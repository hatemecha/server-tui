//! Side-effect execution (I/O, admin, reports) off the reducer path.

use std::sync::Arc;
use std::time::Duration;

use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;

use crate::app::event::{AppEvent, DiagnosticOutcome, OperationResult};
use crate::app::state::AppState;
use crate::app::update::SideEffect;
use crate::config::{expand_tilde, Config};
use crate::diagnostics::{compute_health_status, evaluate, format_text_report};
use crate::error::AppError;
use crate::providers::ProviderBundle;
use crate::runtime::privilege::escalate_sudo_systemctl;
use crate::terminal::TerminalGuard;

#[allow(clippy::too_many_arguments)]
pub(crate) async fn handle_effects(
    effects: Vec<SideEffect>,
    state: &mut AppState,
    providers: &Arc<ProviderBundle>,
    tx: &mpsc::Sender<AppEvent>,
    _app_cancel: &CancellationToken,
    follow_cancel: &mut Option<CancellationToken>,
    scan_cancel: &mut Option<CancellationToken>,
    config_tx: &tokio::sync::watch::Sender<Config>,
    tracker: &TaskTracker,
    terminal: &mut TerminalGuard,
) {
    for effect in effects {
        match effect {
            SideEffect::RefreshMetrics => {
                let p = Arc::clone(providers);
                let tx = tx.clone();
                tracker.spawn(async move {
                    match p.metrics.collect().await {
                        Ok(m) => {
                            let _ = tx.send(AppEvent::MetricsUpdated(m)).await;
                        }
                        Err(e) => {
                            let _ = tx.send(AppEvent::Error(e)).await;
                        }
                    }
                });
            }
            SideEffect::RefreshProcesses => {
                let p = Arc::clone(providers);
                let tx = tx.clone();
                tracker.spawn(async move {
                    match p.processes.list().await {
                        Ok(list) => {
                            let _ = tx.send(AppEvent::ProcessesUpdated(list)).await;
                        }
                        Err(e) => {
                            let _ = tx.send(AppEvent::Error(e)).await;
                        }
                    }
                });
            }
            SideEffect::RefreshServices => {
                let p = Arc::clone(providers);
                let tx = tx.clone();
                tracker.spawn(async move {
                    match p.services.list_services().await {
                        Ok(list) => {
                            let _ = tx.send(AppEvent::ServicesUpdated(list)).await;
                        }
                        Err(e) => {
                            let _ = tx.send(AppEvent::Error(e)).await;
                        }
                    }
                });
            }
            SideEffect::RefreshLogs {
                request_id,
                unit,
                preset,
            } => {
                let p = Arc::clone(providers);
                let tx = tx.clone();
                tracker.spawn(async move {
                    let result = p.logs.recent_preset(unit.as_deref(), 200, preset).await;
                    let _ = tx
                        .send(AppEvent::LogsRefreshFinished {
                            request_id,
                            unit,
                            preset,
                            result,
                        })
                        .await;
                });
            }
            SideEffect::RefreshDiagnostics {
                request_id,
                deep,
                enable_smart,
                previous_crc,
            } => {
                let p = Arc::clone(providers);
                let tx = tx.clone();
                tracker.spawn(async move {
                    let result = p
                        .diagnostics
                        .probe(deep, enable_smart)
                        .await
                        .map(|mut snap| {
                            for disk in &mut snap.smart_disks {
                                disk.prev_uda_crc_error_count =
                                    previous_crc.get(&disk.device).copied();
                            }
                            let smart_crc_observations = snap
                                .smart_disks
                                .iter()
                                .filter_map(|disk| {
                                    disk.uda_crc_error_count
                                        .map(|count| (disk.device.clone(), count))
                                })
                                .collect();
                            let findings = evaluate(&snap);
                            let health = compute_health_status(
                                &findings,
                                snap.systemd_observable,
                                snap.journal_observable,
                            );
                            let report =
                                format_text_report(health, &findings, false, &snap.probes_degraded);
                            DiagnosticOutcome {
                                findings,
                                health,
                                report,
                                probes_degraded: snap.probes_degraded,
                                smart_crc_observations,
                            }
                        });
                    let _ = tx
                        .send(AppEvent::DiagnosticsFinished { request_id, result })
                        .await;
                });
            }
            SideEffect::FetchServiceDetails { request_id, unit } => {
                let p = Arc::clone(providers);
                let tx = tx.clone();
                tracker.spawn(async move {
                    let result = p.services.details(&unit).await;
                    let _ = tx
                        .send(AppEvent::ServiceDetailsFinished {
                            request_id,
                            unit,
                            result,
                        })
                        .await;
                });
            }
            SideEffect::StartFollow { session_id, unit } => {
                if let Some(c) = follow_cancel.take() {
                    c.cancel();
                }
                let token = CancellationToken::new();
                *follow_cancel = Some(token.clone());
                let tx = tx.clone();
                if state.demo {
                    let p = Arc::clone(providers);
                    tracker.spawn(async move {
                        while !token.is_cancelled() {
                            tokio::time::sleep(Duration::from_millis(500)).await;
                            if let Ok(entries) = p.logs.recent(unit.as_deref(), 1).await {
                                let _ = tx
                                    .send(AppEvent::LogsAppended {
                                        session_id,
                                        entries,
                                    })
                                    .await;
                            }
                        }
                    });
                } else {
                    let (log_tx, mut log_rx) = mpsc::channel(32);
                    let follow_token = token.clone();
                    tracker.spawn(async move {
                        let _ = crate::providers::linux::journal::follow_journal(
                            unit,
                            follow_token,
                            log_tx,
                        )
                        .await;
                    });
                    let tx2 = tx.clone();
                    let token2 = token.clone();
                    tracker.spawn(async move {
                        loop {
                            tokio::select! {
                                _ = token2.cancelled() => break,
                                msg = log_rx.recv() => {
                                    match msg {
                                        Some(entries) => {
                                            let _ = tx2.send(AppEvent::LogsAppended {
                                                session_id,
                                                entries,
                                            }).await;
                                        }
                                        None => break,
                                    }
                                }
                            }
                        }
                    });
                }
            }
            SideEffect::StopFollow => {
                if let Some(c) = follow_cancel.take() {
                    c.cancel();
                }
            }
            SideEffect::StartScan {
                request_id,
                path,
                stay_on_fs,
                follow_symlinks,
            } => {
                if let Some(c) = scan_cancel.take() {
                    c.cancel();
                }
                let token = CancellationToken::new();
                *scan_cancel = Some(token.clone());
                let p = Arc::clone(providers);
                let tx_prog = tx.clone();
                let (prog_tx, mut prog_rx) = mpsc::channel(32);
                tracker.spawn(async move {
                    while let Some(prog) = prog_rx.recv().await {
                        let _ = tx_prog
                            .send(AppEvent::StorageProgress {
                                request_id,
                                progress: prog,
                            })
                            .await;
                    }
                });
                let tx2 = tx.clone();
                let path = if path.as_os_str() == "~" {
                    expand_tilde("~")
                } else {
                    path
                };
                tracker.spawn(async move {
                    let result = p
                        .storage
                        .scan(path, stay_on_fs, follow_symlinks, token, prog_tx)
                        .await;
                    let _ = tx2
                        .send(AppEvent::StorageFinished { request_id, result })
                        .await;
                });
            }
            SideEffect::CancelScan => {
                if let Some(c) = scan_cancel.take() {
                    c.cancel();
                }
            }
            SideEffect::SendSignal {
                pid,
                signal,
                start_time,
            } => {
                let p = Arc::clone(providers);
                let tx = tx.clone();
                tracker.spawn(async move {
                    let result = match p.admin.send_signal(pid, signal, Some(start_time)).await {
                        Ok(()) => OperationResult::Success(format!(
                            "{} delivered to PID {pid}",
                            signal.label()
                        )),
                        Err(e) => OperationResult::Failure(e),
                    };
                    let _ = tx.send(AppEvent::OperationFinished(result)).await;
                });
            }
            SideEffect::ServiceAction { unit, action } => {
                let p = Arc::clone(providers);
                let tx = tx.clone();
                let demo = state.demo;
                let read_only = state.read_only;
                tracker.spawn(async move {
                    match p.admin.service_action(&unit, action).await {
                        Ok(()) => {
                            let _ = tx
                                .send(AppEvent::OperationFinished(OperationResult::Success(
                                    format!("{} {} ok", action.label(), unit),
                                )))
                                .await;
                        }
                        Err(e) if p.admin.permission_may_sudo(&e) && !demo && !read_only => {
                            let _ = tx.send(AppEvent::ElevationRequired { unit, action }).await;
                        }
                        Err(e) => {
                            let _ = tx
                                .send(AppEvent::OperationFinished(OperationResult::Failure(e)))
                                .await;
                        }
                    }
                });
            }
            SideEffect::SudoServiceAction { unit, action } => {
                let Some(registry) = providers.admin.unit_registry() else {
                    state.set_error(AppError::Permission(
                        "elevated systemd actions are unavailable in this mode".into(),
                    ));
                    continue;
                };
                if let Err(err) = escalate_sudo_systemctl(terminal, &unit, action, &registry).await
                {
                    state.set_error(err);
                } else {
                    state.set_success(format!("sudo {} {} ok", action.label(), unit));
                }
            }
            SideEffect::FetchProcessDetails { request_id, pid } => {
                let p = Arc::clone(providers);
                let tx = tx.clone();
                tracker.spawn(async move {
                    let result = p.processes.details(pid).await;
                    let _ = tx
                        .send(AppEvent::ProcessDetailsFinished {
                            request_id,
                            pid,
                            result,
                        })
                        .await;
                });
            }
            SideEffect::FetchPpidMap {
                request_id,
                pids,
                demo,
            } => {
                let tx = tx.clone();
                tracker.spawn(async move {
                    let map = if demo {
                        // Synthetic parents for demo PIDs (no /proc).
                        Ok(pids
                            .into_iter()
                            .map(|pid| {
                                let ppid = if pid <= 2 { None } else { Some(1) };
                                (pid, ppid)
                            })
                            .collect())
                    } else {
                        crate::providers::linux::processes::ppid_map(pids).await
                    };
                    let _ = tx
                        .send(AppEvent::PpidMapFinished {
                            request_id,
                            result: map,
                        })
                        .await;
                });
            }
            SideEffect::PreviewFile { request_id, path } => {
                let tx = tx.clone();
                let event_path = path.clone();
                tracker.spawn(async move {
                    let result =
                        tokio::task::spawn_blocking(move || crate::preview::preview_file(&path))
                            .await;
                    let result = match result {
                        Ok(result) => result,
                        Err(error) => Err(crate::error::AppError::Internal(error.to_string())),
                    };
                    let _ = tx
                        .send(AppEvent::FilePreviewFinished {
                            request_id,
                            path: event_path,
                            result,
                        })
                        .await;
                });
            }
            SideEffect::CleanupReports => {
                let days = state.config.report_retention_days;
                if days > 0 {
                    let _ = cleanup_old_reports(days);
                    state.set_status(format!("report cleanup (>={days}d)"));
                }
            }
            SideEffect::SaveReport { body, format } => {
                let tx = tx.clone();
                tracker.spawn(async move {
                    let result = tokio::task::spawn_blocking(move || {
                        crate::support::save_report(&body, format, None)
                    })
                    .await;
                    match result {
                        Ok(Ok(path)) => {
                            let _ = tx.send(AppEvent::ReportSaved(path)).await;
                        }
                        Ok(Err(e)) => {
                            let _ = tx
                                .send(AppEvent::OperationFinished(OperationResult::Failure(e)))
                                .await;
                        }
                        Err(e) => {
                            let _ = tx
                                .send(AppEvent::OperationFinished(OperationResult::Failure(
                                    crate::error::AppError::Internal(e.to_string()),
                                )))
                                .await;
                        }
                    }
                });
            }
            SideEffect::SaveConfig {
                path,
                config,
                reason,
                harden_parent,
            } => {
                let tx = tx.clone();
                let event_path = path.clone();
                tracker.spawn(async move {
                    let result = tokio::task::spawn_blocking(move || {
                        if harden_parent {
                            if let Some(parent) = path.parent() {
                                crate::fsutil::ensure_private_app_dir(parent)?;
                            }
                        }
                        config.save_atomic(&path)
                    })
                    .await
                    .unwrap_or_else(|error| {
                        Err(AppError::Internal(format!("config save join: {error}")))
                    });
                    let _ = tx
                        .send(AppEvent::ConfigSaveFinished {
                            path: event_path,
                            reason,
                            result,
                        })
                        .await;
                });
            }
            SideEffect::SavePersist { path, persist } => {
                let tx = tx.clone();
                tracker.spawn(async move {
                    let result = tokio::task::spawn_blocking(move || {
                        if let Some(parent) = path.parent() {
                            crate::fsutil::ensure_private_app_dir(parent)?;
                        }
                        persist.save_atomic(&path)
                    })
                    .await
                    .unwrap_or_else(|error| {
                        Err(AppError::Internal(format!("state save join: {error}")))
                    });
                    if let Err(error) = result {
                        let _ = tx.send(AppEvent::Error(error)).await;
                    }
                });
            }
            SideEffect::PublishRuntimeConfig(config) => {
                providers.metrics.update_refresh_policy(&config);
                let _ = config_tx.send(config);
            }
        }
    }
}

pub fn cleanup_old_reports(days: u64) -> Result<(), crate::error::AppError> {
    use std::time::{Duration, SystemTime};
    let dir = crate::support::default_reports_dir();
    if !dir.exists() {
        return Ok(());
    }
    let cutoff = SystemTime::now() - Duration::from_secs(days.saturating_mul(86400));
    for ent in
        std::fs::read_dir(&dir).map_err(|e| crate::error::AppError::Internal(e.to_string()))?
    {
        let ent = ent.map_err(|e| crate::error::AppError::Internal(e.to_string()))?;
        let meta = ent.metadata().ok();
        let old = meta
            .and_then(|m| m.modified().ok())
            .map(|m| m < cutoff)
            .unwrap_or(false);
        if old {
            let _ = std::fs::remove_file(ent.path());
        }
    }
    Ok(())
}
