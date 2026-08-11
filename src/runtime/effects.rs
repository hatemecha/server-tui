//! Side-effect execution (I/O, admin, reports) off the reducer path.

use std::sync::Arc;
use std::time::Duration;

use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;

use crate::app::event::{AppEvent, OperationResult};
use crate::app::state::AppState;
use crate::app::update::SideEffect;
use crate::config::{expand_tilde, Config};
use crate::diagnostics::{compute_health_status, evaluate, format_text_report};
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
            SideEffect::RefreshLogs { unit } => {
                let p = Arc::clone(providers);
                let tx = tx.clone();
                let preset = state.log.preset;
                tracker.spawn(async move {
                    match p.logs.recent_preset(unit.as_deref(), 200, preset).await {
                        Ok(entries) => {
                            let _ = tx.send(AppEvent::LogsReplaced(entries)).await;
                        }
                        Err(e) => {
                            let _ = tx.send(AppEvent::Error(e)).await;
                        }
                    }
                });
            }
            SideEffect::RefreshDiagnostics => {
                let p = Arc::clone(providers);
                let tx = tx.clone();
                let deep = state.diagnostic.deep || !state.config.diagnostics_light_scan;
                let enable_smart = state.config.enable_smart_probes;
                let prev_crc = state.persist.smart_crc_counts.clone();
                tracker.spawn(async move {
                    match p.diagnostics.probe(deep, enable_smart).await {
                        Ok(mut snap) => {
                            // Runtime owns persist: inject previous CRC for pure evaluate.
                            for disk in &mut snap.smart_disks {
                                disk.prev_uda_crc_error_count = prev_crc.get(&disk.device).copied();
                            }
                            let observations: Vec<_> = snap
                                .smart_disks
                                .iter()
                                .filter_map(|d| {
                                    d.uda_crc_error_count.map(|c| (d.device.clone(), c))
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
                            if !observations.is_empty() {
                                let _ = tx.send(AppEvent::SmartCrcObservations(observations)).await;
                            }
                            let _ = tx
                                .send(AppEvent::DiagnosticsUpdated {
                                    findings,
                                    health,
                                    report,
                                    probes_degraded: snap.probes_degraded,
                                })
                                .await;
                        }
                        Err(e) => {
                            let _ = tx.send(AppEvent::Error(e)).await;
                        }
                    }
                });
            }
            SideEffect::FetchServiceDetails { unit } => {
                let p = Arc::clone(providers);
                let tx = tx.clone();
                tracker.spawn(async move {
                    match p.services.details(&unit).await {
                        Ok(info) => {
                            let _ = tx.send(AppEvent::ServiceDetails(info)).await;
                        }
                        Err(e) => {
                            tracing::debug!("service details: {e}");
                        }
                    }
                });
            }
            SideEffect::StartFollow { unit } => {
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
                                let _ = tx.send(AppEvent::LogsAppended(entries)).await;
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
                                            let _ = tx2.send(AppEvent::LogsAppended(entries)).await;
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
            SideEffect::StartScan { path } => {
                if let Some(c) = scan_cancel.take() {
                    c.cancel();
                }
                let token = CancellationToken::new();
                *scan_cancel = Some(token.clone());
                let p = Arc::clone(providers);
                let tx_prog = tx.clone();
                let stay = state.storage.stay_on_fs;
                let follow = state.config.follow_symlinks;
                let (prog_tx, mut prog_rx) = mpsc::channel(32);
                tracker.spawn(async move {
                    while let Some(prog) = prog_rx.recv().await {
                        let _ = tx_prog.send(AppEvent::StorageProgress(prog)).await;
                    }
                });
                let tx2 = tx.clone();
                let path = if path.as_os_str() == "~" {
                    expand_tilde("~")
                } else {
                    path
                };
                tracker.spawn(async move {
                    let result = p.storage.scan(path, stay, follow, token, prog_tx).await;
                    let _ = tx2.send(AppEvent::StorageFinished(result)).await;
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
                if let Err(err) = escalate_sudo_systemctl(terminal, &unit, action).await {
                    state.set_error(err);
                } else {
                    state.set_success(format!("sudo {} {} ok", action.label(), unit));
                }
            }
            SideEffect::FetchProcessDetails { pid } => {
                let p = Arc::clone(providers);
                let tx = tx.clone();
                tracker.spawn(async move {
                    match p.processes.details(pid).await {
                        Ok(d) => {
                            let _ = tx.send(AppEvent::ProcessDetails(d)).await;
                        }
                        Err(e) => {
                            let _ = tx.send(AppEvent::Error(e)).await;
                        }
                    }
                });
            }
            SideEffect::FetchPpidMap => {
                let pids: Vec<u32> = state.process.items.iter().map(|p| p.pid).collect();
                let demo = state.demo;
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
                    match map {
                        Ok(map) => {
                            let _ = tx.send(AppEvent::PpidMap(map)).await;
                        }
                        Err(e) => {
                            let _ = tx.send(AppEvent::Error(e)).await;
                        }
                    }
                });
            }
            SideEffect::PreviewFile { path } => {
                let tx = tx.clone();
                tracker.spawn(async move {
                    let result =
                        tokio::task::spawn_blocking(move || crate::preview::preview_file(&path))
                            .await;
                    match result {
                        Ok(Ok(prev)) => {
                            let _ = tx.send(AppEvent::FilePreviewReady(prev)).await;
                        }
                        Ok(Err(e)) => {
                            let _ = tx.send(AppEvent::Error(e)).await;
                        }
                        Err(e) => {
                            let _ = tx
                                .send(AppEvent::Error(crate::error::AppError::Internal(
                                    e.to_string(),
                                )))
                                .await;
                        }
                    }
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
            SideEffect::PublishConfig => {
                let _ = config_tx.send(state.config.clone());
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
