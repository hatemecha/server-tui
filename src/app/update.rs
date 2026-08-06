//! Map input to actions and apply actions/events to state.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::app::action::{AppAction, FocusPane, Screen};
use crate::app::event::{AppEvent, OperationResult};
use crate::app::state::{AppState, Dialog, ScanState};
use crate::model::{
    count_failed, filter_processes, filter_services, is_protected_pid, preserve_selection,
    preserve_service_selection, sort_processes, ProcessSignal, ServiceActionKind, ServiceFilter,
};

pub fn map_key(state: &AppState, key: KeyEvent) -> Option<AppAction> {
    if let Some(dialog) = &state.dialog {
        return map_dialog_key(dialog, key);
    }

    if state.searching {
        return map_search_key(key);
    }

    // Global
    match (key.code, key.modifiers) {
        (KeyCode::Char('c'), KeyModifiers::CONTROL) | (KeyCode::Char('q'), _) => {
            return Some(AppAction::Quit);
        }
        (KeyCode::Char('?'), _) => return Some(AppAction::ToggleHelp),
        (KeyCode::Tab, KeyModifiers::SHIFT) => return Some(AppAction::PrevFocus),
        (KeyCode::BackTab, _) => return Some(AppAction::PrevFocus),
        (KeyCode::Tab, _) => return Some(AppAction::NextFocus),
        (KeyCode::Char(c), _) if Screen::from_digit(c).is_some() => {
            return Screen::from_digit(c).map(AppAction::ChangeScreen);
        }
        (KeyCode::Char('/'), _) => return Some(AppAction::Search),
        (KeyCode::Esc, _) if !state.search_query.is_empty() => {
            return Some(AppAction::ClearSearch);
        }
        _ => {}
    }

    // Screen-specific when content focused (or always for navigation keys)
    match state.screen {
        Screen::Dashboard => map_dashboard(key),
        Screen::Processes => map_processes(state, key),
        Screen::Services => map_services(state, key),
        Screen::Logs => map_logs(state, key),
        Screen::Storage => map_storage(state, key),
    }
}

fn map_dialog_key(dialog: &Dialog, key: KeyEvent) -> Option<AppAction> {
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') => Some(AppAction::Cancel),
        KeyCode::Enter | KeyCode::Char('y') => Some(AppAction::Confirm),
        KeyCode::Char('n') => Some(AppAction::Cancel),
        KeyCode::Left | KeyCode::Right | KeyCode::Tab => None,
        _ => {
            if matches!(dialog, Dialog::Help | Dialog::Message { .. }) {
                Some(AppAction::Cancel)
            } else {
                None
            }
        }
    }
}

fn map_search_key(key: KeyEvent) -> Option<AppAction> {
    match key.code {
        KeyCode::Esc => Some(AppAction::ClearSearch),
        KeyCode::Enter => Some(AppAction::Search), // close search mode, keep query
        KeyCode::Backspace => Some(AppAction::SearchBackspace),
        KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
            Some(AppAction::SearchInput(c))
        }
        _ => None,
    }
}

fn map_dashboard(key: KeyEvent) -> Option<AppAction> {
    match key.code {
        KeyCode::Char('r') => Some(AppAction::Refresh),
        _ => None,
    }
}

fn map_processes(state: &AppState, key: KeyEvent) -> Option<AppAction> {
    match key.code {
        KeyCode::Up | KeyCode::Char('k') => Some(AppAction::MoveUp),
        KeyCode::Down | KeyCode::Char('j') => Some(AppAction::MoveDown),
        KeyCode::PageUp => Some(AppAction::PageUp),
        KeyCode::PageDown => Some(AppAction::PageDown),
        KeyCode::Home => Some(AppAction::Home),
        KeyCode::End => Some(AppAction::End),
        KeyCode::Char('s') => Some(AppAction::ChangeSort),
        KeyCode::Char('c') => Some(AppAction::ToggleFullCommand),
        KeyCode::Char('r') => Some(AppAction::Refresh),
        KeyCode::Char('t') => Some(AppAction::SignalTerm),
        KeyCode::Char('K') => Some(AppAction::SignalKill),
        _ => {
            let _ = state;
            None
        }
    }
}

fn map_services(state: &AppState, key: KeyEvent) -> Option<AppAction> {
    match key.code {
        KeyCode::Up | KeyCode::Char('k') => Some(AppAction::MoveUp),
        KeyCode::Down | KeyCode::Char('j') => Some(AppAction::MoveDown),
        KeyCode::PageUp => Some(AppAction::PageUp),
        KeyCode::PageDown => Some(AppAction::PageDown),
        KeyCode::Home => Some(AppAction::Home),
        KeyCode::End => Some(AppAction::End),
        KeyCode::Char('s') => Some(AppAction::StartService),
        KeyCode::Char('x') => Some(AppAction::StopService),
        KeyCode::Char('r') => {
            if state.focus == FocusPane::Content {
                Some(AppAction::RestartService)
            } else {
                Some(AppAction::Refresh)
            }
        }
        KeyCode::Char('R') => Some(AppAction::ReloadService),
        KeyCode::Char('e') => Some(AppAction::EnableService),
        KeyCode::Char('d') => Some(AppAction::DisableService),
        KeyCode::Char('l') => Some(AppAction::OpenLogsForSelected),
        KeyCode::Char('f') => Some(AppAction::ToggleFailedOnly),
        _ => None,
    }
}

fn map_logs(state: &AppState, key: KeyEvent) -> Option<AppAction> {
    let _ = state;
    match key.code {
        KeyCode::Up | KeyCode::Char('k') => Some(AppAction::MoveUp),
        KeyCode::Down | KeyCode::Char('j') => Some(AppAction::MoveDown),
        KeyCode::PageUp => Some(AppAction::PageUp),
        KeyCode::PageDown => Some(AppAction::PageDown),
        KeyCode::Home => Some(AppAction::Home),
        KeyCode::End => Some(AppAction::End),
        KeyCode::Char('f') => Some(AppAction::ToggleFollow),
        KeyCode::Char('n') => Some(AppAction::NextMatch),
        KeyCode::Char('N') => Some(AppAction::PrevMatch),
        KeyCode::Char('p') => Some(AppAction::ChangePriority),
        KeyCode::Char('w') => Some(AppAction::ToggleWrap),
        KeyCode::Char('r') => Some(AppAction::Refresh),
        _ => None,
    }
}

fn map_storage(state: &AppState, key: KeyEvent) -> Option<AppAction> {
    match key.code {
        KeyCode::Up | KeyCode::Char('k') => Some(AppAction::MoveUp),
        KeyCode::Down | KeyCode::Char('j') => Some(AppAction::MoveDown),
        KeyCode::PageUp => Some(AppAction::PageUp),
        KeyCode::PageDown => Some(AppAction::PageDown),
        KeyCode::Home => Some(AppAction::Home),
        KeyCode::End => Some(AppAction::End),
        KeyCode::Enter => Some(AppAction::EnterDir),
        KeyCode::Backspace => Some(AppAction::ParentDir),
        KeyCode::Char('s') => Some(AppAction::ChangeSort),
        KeyCode::Char('a') => Some(AppAction::ToggleApparentSize),
        KeyCode::Char('x') => Some(AppAction::ToggleStayOnFs),
        KeyCode::Char('r') => Some(AppAction::StartStorageScan(None)),
        KeyCode::Esc if state.scan_state == ScanState::Running => {
            Some(AppAction::CancelStorageScan)
        }
        _ => None,
    }
}

/// Apply a user action. Returns side-effect intents for the runtime to spawn.
#[derive(Debug, Clone)]
pub enum SideEffect {
    RefreshMetrics,
    RefreshProcesses,
    RefreshServices,
    RefreshLogs {
        unit: Option<String>,
    },
    StartFollow {
        unit: Option<String>,
    },
    StopFollow,
    StartScan {
        path: std::path::PathBuf,
    },
    CancelScan,
    SendSignal {
        pid: u32,
        signal: ProcessSignal,
        start_time: u64,
    },
    ServiceAction {
        unit: String,
        action: ServiceActionKind,
    },
}

pub fn apply_action(state: &mut AppState, action: AppAction) -> Vec<SideEffect> {
    let mut effects = Vec::new();
    match action {
        AppAction::Quit => state.should_quit = true,
        AppAction::ChangeScreen(screen) => {
            let leaving_logs = state.screen == Screen::Logs && screen != Screen::Logs;
            if leaving_logs && state.log_follow {
                state.log_follow = false;
                effects.push(SideEffect::StopFollow);
            }
            state.screen = screen;
            state.searching = false;
            if screen == Screen::Storage
                && state.storage_tree.is_none()
                && state.scan_state == ScanState::Idle
            {
                effects.push(SideEffect::StartScan {
                    path: state.scan_path.clone(),
                });
                state.scan_state = ScanState::Running;
            }
            if screen == Screen::Logs {
                effects.push(SideEffect::RefreshLogs {
                    unit: state.log_unit.clone(),
                });
            }
        }
        AppAction::Refresh => match state.screen {
            Screen::Dashboard => effects.push(SideEffect::RefreshMetrics),
            Screen::Processes => effects.push(SideEffect::RefreshProcesses),
            Screen::Services => effects.push(SideEffect::RefreshServices),
            Screen::Logs => effects.push(SideEffect::RefreshLogs {
                unit: state.log_unit.clone(),
            }),
            Screen::Storage => {
                effects.push(SideEffect::StartScan {
                    path: state.scan_path.clone(),
                });
                state.scan_state = ScanState::Running;
            }
        },
        AppAction::NextFocus => {
            state.focus = match state.focus {
                FocusPane::Nav => FocusPane::Content,
                FocusPane::Content => FocusPane::Details,
                FocusPane::Details => FocusPane::Nav,
            };
        }
        AppAction::PrevFocus => {
            state.focus = match state.focus {
                FocusPane::Nav => FocusPane::Details,
                FocusPane::Content => FocusPane::Nav,
                FocusPane::Details => FocusPane::Content,
            };
        }
        AppAction::MoveUp => move_selection(state, -1),
        AppAction::MoveDown => move_selection(state, 1),
        AppAction::PageUp => move_selection(state, -10),
        AppAction::PageDown => move_selection(state, 10),
        AppAction::Home => set_selection(state, 0),
        AppAction::End => set_selection(state, usize::MAX),
        AppAction::Search => {
            state.searching = !state.searching;
        }
        AppAction::ClearSearch => {
            state.searching = false;
            state.search_query.clear();
        }
        AppAction::SearchInput(c) => {
            state.search_query.push(c);
        }
        AppAction::SearchBackspace => {
            state.search_query.pop();
        }
        AppAction::ChangeSort => match state.screen {
            Screen::Processes => {
                state.process_sort = state.process_sort.next();
                state.set_status(format!("sort: {}", state.process_sort.label()));
            }
            Screen::Storage => {
                state.storage_sort = state.storage_sort.next();
                if let Some(tree) = state.storage_tree.as_mut() {
                    tree.root.sort_children(state.storage_sort);
                }
                state.set_status(format!("sort: {}", state.storage_sort.label()));
            }
            _ => {}
        },
        AppAction::ToggleFullCommand => state.show_full_cmd = !state.show_full_cmd,
        AppAction::ToggleFailedOnly => {
            state.failed_only = !state.failed_only;
            state.service_filter = if state.failed_only {
                ServiceFilter::Failed
            } else {
                ServiceFilter::All
            };
        }
        AppAction::ToggleFollow => {
            state.log_follow = !state.log_follow;
            if state.log_follow {
                effects.push(SideEffect::StartFollow {
                    unit: state.log_unit.clone(),
                });
            } else {
                effects.push(SideEffect::StopFollow);
            }
        }
        AppAction::NextMatch => step_log_match(state, true),
        AppAction::PrevMatch => step_log_match(state, false),
        AppAction::ChangePriority => {
            state.log_min_priority = state.log_min_priority.next_minimum();
            state.set_status(format!("min priority: {}", state.log_min_priority.label()));
        }
        AppAction::ToggleWrap => state.log_wrap = !state.log_wrap,
        AppAction::ToggleApparentSize => state.use_apparent = !state.use_apparent,
        AppAction::ToggleStayOnFs => {
            state.stay_on_fs = !state.stay_on_fs;
            state.set_status(format!(
                "stay on filesystem: {}",
                if state.stay_on_fs { "on" } else { "off" }
            ));
        }
        AppAction::EnterDir => enter_storage_dir(state),
        AppAction::ParentDir => {
            state.storage_cwd.pop();
            state.storage_selected = 0;
        }
        AppAction::StartService => maybe_confirm_service(state, ServiceActionKind::Start),
        AppAction::StopService => maybe_confirm_service(state, ServiceActionKind::Stop),
        AppAction::RestartService => maybe_confirm_service(state, ServiceActionKind::Restart),
        AppAction::ReloadService => maybe_confirm_service(state, ServiceActionKind::Reload),
        AppAction::EnableService => maybe_confirm_service(state, ServiceActionKind::Enable),
        AppAction::DisableService => maybe_confirm_service(state, ServiceActionKind::Disable),
        AppAction::OpenLogsForSelected => {
            if let Some(unit) = selected_service_unit(state) {
                state.log_unit = Some(unit.clone());
                state.screen = Screen::Logs;
                state.logs.clear();
                effects.push(SideEffect::RefreshLogs { unit: Some(unit) });
            }
        }
        AppAction::SignalTerm => maybe_confirm_signal(state, ProcessSignal::Term),
        AppAction::SignalKill => maybe_confirm_signal(state, ProcessSignal::Kill),
        AppAction::StartStorageScan(path) => {
            let path = path.unwrap_or_else(|| state.scan_path.clone());
            state.scan_path = path.clone();
            state.scan_state = ScanState::Running;
            state.storage_cwd.clear();
            effects.push(SideEffect::StartScan { path });
        }
        AppAction::CancelStorageScan => {
            if state.scan_state == ScanState::Running {
                state.scan_state = ScanState::Cancelled;
                effects.push(SideEffect::CancelScan);
                state.set_status("scan cancel requested");
            }
        }
        AppAction::Confirm => {
            if let Some(dialog) = state.dialog.take() {
                match dialog {
                    Dialog::ConfirmSignal {
                        pid,
                        signal,
                        start_time,
                        ..
                    } => {
                        effects.push(SideEffect::SendSignal {
                            pid,
                            signal,
                            start_time,
                        });
                    }
                    Dialog::ConfirmService { unit, action } => {
                        effects.push(SideEffect::ServiceAction { unit, action });
                    }
                    _ => {}
                }
            }
        }
        AppAction::Cancel => {
            state.dialog = None;
        }
        AppAction::ToggleHelp => {
            if matches!(state.dialog, Some(Dialog::Help)) {
                state.dialog = None;
            } else {
                state.dialog = Some(Dialog::Help);
            }
        }
        AppAction::ExecuteSignal {
            pid,
            signal,
            start_time,
        } => {
            if state.read_only {
                state.set_status("READ ONLY: signals disabled");
            } else {
                effects.push(SideEffect::SendSignal {
                    pid,
                    signal,
                    start_time,
                });
            }
        }
        AppAction::ExecuteService { unit, action } => {
            if state.read_only {
                state.set_status("READ ONLY: service actions disabled");
            } else {
                effects.push(SideEffect::ServiceAction { unit, action });
            }
        }
    }
    effects
}

pub fn apply_event(state: &mut AppState, event: AppEvent) -> Vec<SideEffect> {
    match event {
        AppEvent::Tick => Vec::new(),
        AppEvent::Input(key) => {
            if let Some(action) = map_key(state, key) {
                apply_action(state, action)
            } else {
                Vec::new()
            }
        }
        AppEvent::Resize(w, h) => {
            state.width = w;
            state.height = h;
            Vec::new()
        }
        AppEvent::MetricsUpdated(mut m) => {
            m.failed_services = count_failed(&state.services);
            state.history.record_from_metrics(&m);
            state.metrics = m;
            Vec::new()
        }
        AppEvent::ProcessesUpdated(list) => {
            state.processes = list;
            let filtered = {
                let mut f = filter_processes(&state.processes, &state.search_query);
                sort_processes(&mut f, state.process_sort);
                f
            };
            state.process_selected = preserve_selection(&filtered, state.process_selected_pid);
            if let Some(p) = filtered.get(state.process_selected) {
                state.process_selected_pid = Some(p.pid);
            }
            Vec::new()
        }
        AppEvent::ServicesUpdated(list) => {
            state.services = list;
            state.metrics.failed_services = count_failed(&state.services);
            let filter = if state.failed_only {
                ServiceFilter::Failed
            } else {
                state.service_filter
            };
            let filtered = filter_services(&state.services, &state.search_query, filter);
            state.service_selected =
                preserve_service_selection(&filtered, state.service_selected_unit.as_deref());
            if let Some(s) = filtered.get(state.service_selected) {
                state.service_selected_unit = Some(s.unit.clone());
            }
            Vec::new()
        }
        AppEvent::LogsUpdated(entries) => {
            state.logs.extend(entries);
            if state.log_follow {
                let filtered = state
                    .logs
                    .filtered(&state.search_query, state.log_min_priority);
                if !filtered.is_empty() {
                    state.log_selected = filtered.len().saturating_sub(1);
                }
            }
            Vec::new()
        }
        AppEvent::StorageProgress(p) => {
            state.scan_progress = Some(p);
            Vec::new()
        }
        AppEvent::StorageFinished(result) => match result {
            Ok(mut tree) => {
                tree.root.sort_children(state.storage_sort);
                state.storage_tree = Some(tree);
                state.scan_state = ScanState::Finished;
                state.set_status("storage scan finished");
                Vec::new()
            }
            Err(e) => {
                if e.to_string().contains("cancel") {
                    state.scan_state = ScanState::Cancelled;
                    state.set_status("storage scan cancelled");
                } else {
                    state.scan_state = ScanState::Idle;
                    state.set_error(e);
                }
                Vec::new()
            }
        },
        AppEvent::OperationFinished(op) => {
            match op {
                OperationResult::Success(msg) => state.set_status(msg),
                OperationResult::Failure(err) => state.set_error(err),
            }
            // Refresh lists after admin ops.
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

fn list_len(state: &AppState) -> usize {
    match state.screen {
        Screen::Processes => filter_processes(&state.processes, &state.search_query).len(),
        Screen::Services => {
            let filter = if state.failed_only {
                ServiceFilter::Failed
            } else {
                state.service_filter
            };
            filter_services(&state.services, &state.search_query, filter).len()
        }
        Screen::Logs => state
            .logs
            .filtered(&state.search_query, state.log_min_priority)
            .len(),
        Screen::Storage => state
            .current_storage_node()
            .map(|n| n.children.len())
            .unwrap_or(0),
        Screen::Dashboard => 0,
    }
}

fn move_selection(state: &mut AppState, delta: i32) {
    let len = list_len(state);
    if len == 0 {
        return;
    }
    let cur = match state.screen {
        Screen::Processes => state.process_selected,
        Screen::Services => state.service_selected,
        Screen::Logs => state.log_selected,
        Screen::Storage => state.storage_selected,
        Screen::Dashboard => return,
    };
    let next = if delta < 0 {
        cur.saturating_sub((-delta) as usize)
    } else {
        (cur + delta as usize).min(len - 1)
    };
    set_selection(state, next);
}

fn set_selection(state: &mut AppState, idx: usize) {
    let len = list_len(state);
    let idx = if len == 0 { 0 } else { idx.min(len - 1) };
    match state.screen {
        Screen::Processes => {
            state.process_selected = idx;
            let filtered = filter_processes(&state.processes, &state.search_query);
            if let Some(p) = filtered.get(idx) {
                state.process_selected_pid = Some(p.pid);
            }
        }
        Screen::Services => {
            state.service_selected = idx;
            let filter = if state.failed_only {
                ServiceFilter::Failed
            } else {
                state.service_filter
            };
            let filtered = filter_services(&state.services, &state.search_query, filter);
            if let Some(s) = filtered.get(idx) {
                state.service_selected_unit = Some(s.unit.clone());
            }
        }
        Screen::Logs => state.log_selected = idx,
        Screen::Storage => state.storage_selected = idx,
        Screen::Dashboard => {}
    }
}

fn selected_service_unit(state: &AppState) -> Option<String> {
    let filter = if state.failed_only {
        ServiceFilter::Failed
    } else {
        state.service_filter
    };
    let filtered = filter_services(&state.services, &state.search_query, filter);
    filtered.get(state.service_selected).map(|s| s.unit.clone())
}

use crate::model::ProcessInfo;

fn selected_process_info(state: &AppState) -> Option<ProcessInfo> {
    let mut filtered = filter_processes(&state.processes, &state.search_query);
    sort_processes(&mut filtered, state.process_sort);
    filtered.get(state.process_selected).map(|p| (*p).clone())
}

fn maybe_confirm_signal(state: &mut AppState, signal: ProcessSignal) {
    if state.read_only {
        state.set_status("READ ONLY: signals disabled");
        return;
    }
    let Some(proc_) = selected_process_info(state) else {
        return;
    };
    if is_protected_pid(proc_.pid, std::process::id()) {
        state.set_status(format!(
            "refusing {}: protected PID {}",
            signal.label(),
            proc_.pid
        ));
        return;
    }
    // MVP always confirms; config flags reserved for a future “skip confirm” mode.
    let _ = match signal {
        ProcessSignal::Term => state.config.confirm_sigterm,
        ProcessSignal::Kill => state.config.confirm_sigkill,
    };
    state.dialog = Some(Dialog::ConfirmSignal {
        pid: proc_.pid,
        user: proc_.user.clone(),
        command: proc_.name.clone(),
        signal,
        start_time: proc_.start_time,
    });
}

fn maybe_confirm_service(state: &mut AppState, action: ServiceActionKind) {
    if state.read_only {
        state.set_status("READ ONLY: service actions disabled");
        return;
    }
    let Some(unit) = selected_service_unit(state) else {
        return;
    };
    let _ = state.config.confirm_service_actions;
    state.dialog = Some(Dialog::ConfirmService { unit, action });
}

fn enter_storage_dir(state: &mut AppState) {
    let name = {
        let node = match state.current_storage_node() {
            Some(n) => n,
            None => return,
        };
        let child = match node.children.get(state.storage_selected) {
            Some(c) => c,
            None => return,
        };
        if !child.is_dir {
            return;
        }
        child.name.clone()
    };
    state.storage_cwd.push(name);
    state.storage_selected = 0;
}

fn step_log_match(state: &mut AppState, forward: bool) {
    if state.search_query.is_empty() {
        return;
    }
    let filtered = state
        .logs
        .filtered(&state.search_query, state.log_min_priority);
    if filtered.is_empty() {
        return;
    }
    let cur = state.log_match_idx.unwrap_or(state.log_selected);
    let next = if forward {
        (cur + 1) % filtered.len()
    } else if cur == 0 {
        filtered.len() - 1
    } else {
        cur - 1
    };
    state.log_match_idx = Some(next);
    state.log_selected = next;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use std::path::PathBuf;

    fn demo_state() -> AppState {
        AppState::new(
            Config::default(),
            true,
            false,
            true,
            false,
            PathBuf::from("/tmp"),
        )
    }

    #[test]
    fn read_only_blocks_signal_dialog_status() {
        let mut state = demo_state();
        state.read_only = true;
        state.screen = Screen::Processes;
        state.processes.push(ProcessInfo {
            pid: 5,
            user: "u".into(),
            name: "x".into(),
            cmd: "x".into(),
            cpu: 1.0,
            mem_pct: 1.0,
            mem_bytes: 1,
            state: "R".into(),
            run_time_secs: 1,
            start_time: 1,
        });
        let _ = apply_action(&mut state, AppAction::SignalTerm);
        assert!(state
            .status_message
            .as_deref()
            .unwrap_or("")
            .contains("READ ONLY"));
    }

    #[test]
    fn screen_digits() {
        assert_eq!(Screen::from_digit('3'), Some(Screen::Services));
    }
}
