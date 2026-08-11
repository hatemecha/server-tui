//! Apply AppEvent → state + SideEffects.

use crossterm::event::KeyCode;

use crate::app::action::ConfirmChoice;
use crate::app::event::{AppEvent, OperationResult};
use crate::app::state::{AppState, Dialog, ScanState};
use crate::app::update::keymap::map_key;
use crate::app::update::reducer::apply_action;
use crate::app::update::side_effect::SideEffect;
use crate::model::{
    count_failed, filter_services, preserve_selection, preserve_service_selection, ServiceFilter,
    SubsystemHealth,
};

fn sync_service_recent_logs(state: &mut AppState, entries: &[crate::model::LogEntry]) {
    if let Some(unit) = &state.service.selected_unit {
        let extra: Vec<_> = entries
            .iter()
            .filter(|e| e.unit.eq_ignore_ascii_case(unit))
            .cloned()
            .collect();
        state.service.recent_logs.extend(extra);
        if state.service.recent_logs.len() > 20 {
            let drain = state.service.recent_logs.len() - 20;
            state.service.recent_logs.drain(0..drain);
        }
    }
}

fn apply_logs_replaced(state: &mut AppState, entries: Vec<crate::model::LogEntry>) {
    sync_service_recent_logs(state, &entries);
    state.log.buffer.clear();
    state.log.buffer.extend(entries);
    if state.log.follow {
        let filtered = state.log.buffer.filtered_preset(
            state.current_search(),
            state.log.min_priority,
            state.log.preset,
            state.log.unit.as_deref(),
        );
        if !filtered.is_empty() {
            state.log.vp.selected = filtered.len().saturating_sub(1);
        }
    }
}

fn apply_logs_appended(state: &mut AppState, entries: Vec<crate::model::LogEntry>) {
    if entries.is_empty() {
        return;
    }
    let recent: Vec<(String, String)> = state
        .log
        .buffer
        .iter()
        .skip(state.log.buffer.len().saturating_sub(64))
        .map(|e| (e.timestamp.clone(), e.message.clone()))
        .collect();
    let mut fresh = Vec::new();
    for e in entries {
        let key = (e.timestamp.clone(), e.message.clone());
        if recent.iter().any(|r| r == &key) {
            continue;
        }
        if fresh
            .iter()
            .any(|f: &crate::model::LogEntry| f.timestamp == key.0 && f.message == key.1)
        {
            continue;
        }
        fresh.push(e);
    }
    sync_service_recent_logs(state, &fresh);
    state.log.buffer.extend(fresh);
    if state.log.follow {
        let filtered = state.log.buffer.filtered_preset(
            state.current_search(),
            state.log.min_priority,
            state.log.preset,
            state.log.unit.as_deref(),
        );
        if !filtered.is_empty() {
            state.log.vp.selected = filtered.len().saturating_sub(1);
        }
    }
}

pub fn apply_event(state: &mut AppState, event: AppEvent) -> Vec<SideEffect> {
    match event {
        AppEvent::Tick => Vec::new(),
        AppEvent::Input(key) => {
            // Special-case: 'y' on confirm should confirm even if focus is Cancel.
            if let Some(
                Dialog::ConfirmSignal { choice, .. } | Dialog::ConfirmService { choice, .. },
            ) = state.dialog.as_mut()
            {
                if key.code == KeyCode::Char('y') {
                    *choice = ConfirmChoice::Yes;
                }
            }
            if let Some(action) = map_key(state, key) {
                apply_action(state, action)
            } else {
                Vec::new()
            }
        }
        AppEvent::Resize(w, h) => {
            state.width = w;
            state.height = h;
            state.viewport_rows = crate::viewport::content_rows_from_terminal(h);
            Vec::new()
        }
        AppEvent::MetricsUpdated(mut m) => {
            m.failed_services = count_failed(&state.service.items);
            if m.process_count == 0 {
                m.process_count = state.process.items.len();
            }
            state.history.record_from_metrics(&m);
            state.metrics = m;
            state.refresh_subsystem_health_from_metrics();
            Vec::new()
        }
        AppEvent::ProcessesUpdated(list) => {
            state.process.items = list;
            let selected_pid = state.process.follow_pid.or(state.process.selected_pid);
            let idx = {
                let filtered = state.visible_processes();
                preserve_selection(&filtered, selected_pid)
            };
            let vis = state.viewport_rows.max(1);
            let len = state.visible_processes().len();
            state.process.vp.set_selected(idx, len, vis);
            let filtered = state.visible_processes();
            if let Some(p) = filtered.get(state.process.vp.selected) {
                state.process.selected_pid = Some(p.pid);
            }
            if state.process.tree_mode {
                return vec![SideEffect::FetchPpidMap];
            }
            Vec::new()
        }
        AppEvent::ServicesUpdated(list) => {
            state.service.items = list;
            state.metrics.failed_services = count_failed(&state.service.items);
            let filter = if state.service.failed_only {
                ServiceFilter::Failed
            } else {
                state.service.filter
            };
            let filtered = filter_services(&state.service.items, state.current_search(), filter);
            state.service.vp.selected =
                preserve_service_selection(&filtered, state.service.selected_unit.as_deref());
            if let Some(s) = filtered.get(state.service_selected()) {
                state.service.selected_unit = Some(s.unit.clone());
                return vec![SideEffect::FetchServiceDetails {
                    unit: s.unit.clone(),
                }];
            }
            Vec::new()
        }
        AppEvent::ServiceDetails(info) => {
            let unit = info.unit.clone();
            if let Some(slot) = state.service.items.iter_mut().find(|s| s.unit == info.unit) {
                *slot = info.clone();
            }
            let logs: Vec<_> = state
                .log
                .buffer
                .filtered_preset(
                    "",
                    crate::model::LogPriority::Debug,
                    crate::model::LogPreset::SelectedService,
                    Some(&unit),
                )
                .into_iter()
                .take(10)
                .cloned()
                .collect();
            state.service.recent_logs = logs.clone();
            let bundle = crate::model::ServiceInspectBundle {
                service: info,
                recent_logs: logs,
            };
            if state.service.pending_inspect {
                state.service.pending_inspect = false;
                state.dialog = Some(Dialog::Inspector {
                    title: format!("Service {}", bundle.service.unit),
                    body: bundle.format_body(),
                });
            }
            Vec::new()
        }
        AppEvent::ProcessDetails(details) => {
            let body = details.format_body();
            let title = details
                .info
                .as_ref()
                .map(|p| format!("Process {}", p.pid))
                .unwrap_or_else(|| "Process".into());
            state.process.details = Some(details);
            state.dialog = Some(Dialog::Inspector { title, body });
            Vec::new()
        }
        AppEvent::FilePreviewReady(prev) => {
            let trunc = if prev.truncated { " (truncated)" } else { "" };
            state.dialog = Some(Dialog::Inspector {
                title: format!("Preview {}{trunc}", prev.path),
                body: prev.text.clone(),
            });
            state.storage.file_preview = Some(prev);
            Vec::new()
        }
        AppEvent::PpidMap(map) => {
            state.process.ppids = map;
            state.set_status(format!(
                "tree parents ready ({})",
                state.process.ppids.len()
            ));
            Vec::new()
        }
        AppEvent::LogsReplaced(entries) => {
            apply_logs_replaced(state, entries);
            Vec::new()
        }
        AppEvent::LogsAppended(entries) => {
            apply_logs_appended(state, entries);
            Vec::new()
        }
        AppEvent::SmartCrcObservations(obs) => {
            for (dev, crc) in obs {
                state.persist.remember_crc(&dev, crc);
            }
            state.save_persist();
            Vec::new()
        }
        AppEvent::ReportSaved(path) => {
            state.set_success(format!(
                "exported {}",
                crate::sanitize::sanitize_path_display(&path)
            ));
            Vec::new()
        }
        AppEvent::StorageProgress(p) => {
            state.storage.scan_progress = Some(p);
            Vec::new()
        }
        AppEvent::StorageFinished(result) => match result {
            Ok(mut tree) => {
                tree.root.sort_children(state.storage.sort);
                let truncated = tree.truncated;
                state.storage.tree = Some(tree);
                state.storage.scan_state = ScanState::Finished;
                if truncated {
                    state.set_warning(
                        "storage scan truncated at entry budget — results are partial",
                    );
                } else {
                    state.set_status("storage scan finished");
                }
                Vec::new()
            }
            Err(e) => {
                if e.to_string().contains("cancel") {
                    state.storage.scan_state = ScanState::Cancelled;
                    state.set_status("storage scan cancelled");
                } else {
                    state.storage.scan_state = ScanState::Idle;
                    state.set_error(e);
                }
                Vec::new()
            }
        },
        AppEvent::DiagnosticsUpdated {
            findings,
            health,
            report,
            probes_degraded,
        } => {
            state.diagnostic.findings = findings;
            state.health_status = health;
            state.diagnostic.report = Some(report);
            state.diagnostic.running = false;
            state.subsystem_health.diagnostics = if probes_degraded.is_empty() {
                SubsystemHealth::Healthy
            } else {
                SubsystemHealth::Degraded
            };
            state.persist.touch_diagnostic_now();
            state.save_persist();
            if state.finding_selected() >= state.diagnostic.findings.len() {
                state.diagnostic.vp.selected = state.diagnostic.findings.len().saturating_sub(1);
            }
            if !probes_degraded.is_empty() {
                state.set_status(format!(
                    "diagnostics: {} finding(s); {} probe(s) degraded",
                    state.diagnostic.findings.len(),
                    probes_degraded.len()
                ));
            } else {
                state.set_status(format!(
                    "diagnostics: {} finding(s); health={}",
                    state.diagnostic.findings.len(),
                    health.label()
                ));
            }
            Vec::new()
        }
        AppEvent::ElevationRequired { unit, action } => {
            state.dialog = Some(Dialog::ConfirmElevation {
                unit,
                action,
                choice: ConfirmChoice::Cancel,
            });
            state.set_warning("Administrator permission is required");
            Vec::new()
        }
        AppEvent::OperationFinished(op) => {
            match op {
                OperationResult::Success(msg) => {
                    state.push_activity(msg.clone());
                    state.set_success(msg);
                }
                OperationResult::Failure(err) => state.set_error(err),
            }
            vec![SideEffect::RefreshProcesses, SideEffect::RefreshServices]
        }
        AppEvent::Error(err) => {
            state.set_error(err);
            Vec::new()
        }
        AppEvent::Quit => {
            state.should_quit = true;
            Vec::new()
        }
    }
}
