//! Map input to actions and apply actions/events to state.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::app::action::{AppAction, ConfirmChoice, FocusPane, Screen};
use crate::app::event::{AppEvent, OperationResult};
use crate::app::state::{AppState, Dialog, ScanState};
use crate::model::{
    count_failed, filter_services, is_protected_pid, preserve_selection,
    preserve_service_selection, ProcessInfo, ProcessSignal, ScreenTarget, ServiceActionKind,
    ServiceFilter, SubsystemHealth,
};

pub fn map_key(state: &AppState, key: KeyEvent) -> Option<AppAction> {
    if let Some(dialog) = &state.dialog {
        return map_dialog_key(state, dialog, key);
    }

    if state.searching {
        return map_search_key(key);
    }

    match (key.code, key.modifiers) {
        (KeyCode::Char('c'), KeyModifiers::CONTROL) | (KeyCode::Char('q'), _) => {
            return Some(AppAction::Quit);
        }
        (KeyCode::Char('?'), _) => return Some(AppAction::ToggleHelp),
        (KeyCode::Char('g'), _) => return Some(AppAction::ToggleGlossary),
        (KeyCode::Tab, KeyModifiers::SHIFT) => return Some(AppAction::PrevFocus),
        (KeyCode::BackTab, _) => return Some(AppAction::PrevFocus),
        (KeyCode::Tab, _) => return Some(AppAction::NextFocus),
        (KeyCode::Char(c), _) if Screen::from_digit(c).is_some() => {
            return Screen::from_digit(c).map(AppAction::ChangeScreen);
        }
        (KeyCode::Char('/'), _) if state.search_supported() => {
            return Some(AppAction::Search);
        }
        (KeyCode::Esc, _) if !state.current_search().is_empty() => {
            return Some(AppAction::ClearSearch);
        }
        _ => {}
    }

    match state.screen {
        Screen::Dashboard => map_dashboard(key),
        Screen::Processes => map_processes(key),
        Screen::Services => map_services(state, key),
        Screen::Logs => map_logs(key),
        Screen::Storage => map_storage(state, key),
        Screen::Diagnostics => map_diagnostics(key),
    }
}

fn map_dialog_key(state: &AppState, dialog: &Dialog, key: KeyEvent) -> Option<AppAction> {
    match dialog {
        Dialog::ConfirmSignal { .. } | Dialog::ConfirmService { .. } => match key.code {
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('n') => Some(AppAction::Cancel),
            KeyCode::Char('y') => Some(AppAction::Confirm),
            KeyCode::Left | KeyCode::BackTab => Some(AppAction::ConfirmFocusLeft),
            KeyCode::Right | KeyCode::Tab => Some(AppAction::ConfirmFocusRight),
            KeyCode::Enter => Some(AppAction::Confirm),
            _ => None,
        },
        Dialog::Glossary => {
            if state.searching {
                return map_search_key(key);
            }
            match key.code {
                KeyCode::Esc if !state.current_search().is_empty() => Some(AppAction::ClearSearch),
                KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('g') => Some(AppAction::Cancel),
                KeyCode::Char('/') => Some(AppAction::Search),
                KeyCode::Up | KeyCode::Char('k') => Some(AppAction::MoveUp),
                KeyCode::Down | KeyCode::Char('j') => Some(AppAction::MoveDown),
                KeyCode::PageUp => Some(AppAction::PageUp),
                KeyCode::PageDown => Some(AppAction::PageDown),
                KeyCode::Home => Some(AppAction::Home),
                KeyCode::End => Some(AppAction::End),
                _ => None,
            }
        }
        Dialog::Help | Dialog::Message { .. } | Dialog::DiagnosticReport { .. } => match key.code {
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Enter => Some(AppAction::Cancel),
            KeyCode::Char('?') if matches!(dialog, Dialog::Help) => Some(AppAction::Cancel),
            _ => {
                if matches!(dialog, Dialog::Help | Dialog::Message { .. }) {
                    Some(AppAction::Cancel)
                } else {
                    None
                }
            }
        },
    }
}

fn map_search_key(key: KeyEvent) -> Option<AppAction> {
    match key.code {
        KeyCode::Esc => Some(AppAction::ClearSearch),
        KeyCode::Enter => Some(AppAction::Search),
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

fn map_processes(key: KeyEvent) -> Option<AppAction> {
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
        _ => None,
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

fn map_logs(key: KeyEvent) -> Option<AppAction> {
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

fn map_diagnostics(key: KeyEvent) -> Option<AppAction> {
    match key.code {
        KeyCode::Up | KeyCode::Char('k') => Some(AppAction::MoveUp),
        KeyCode::Down | KeyCode::Char('j') => Some(AppAction::MoveDown),
        KeyCode::PageUp => Some(AppAction::PageUp),
        KeyCode::PageDown => Some(AppAction::PageDown),
        KeyCode::Home => Some(AppAction::Home),
        KeyCode::End => Some(AppAction::End),
        KeyCode::Char('r') => Some(AppAction::Refresh),
        KeyCode::Char('o') => Some(AppAction::OpenDiagnosticReport),
        KeyCode::Char('a') => Some(AppAction::AcknowledgeFinding),
        KeyCode::Enter => Some(AppAction::FollowDiagnosticTarget),
        _ => None,
    }
}

#[derive(Debug, Clone)]
pub enum SideEffect {
    RefreshMetrics,
    RefreshProcesses,
    RefreshServices,
    RefreshLogs {
        unit: Option<String>,
    },
    RefreshDiagnostics,
    FetchServiceDetails {
        unit: String,
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
            if screen == Screen::Diagnostics {
                effects.push(SideEffect::RefreshDiagnostics);
                state.diagnostic_running = true;
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
            Screen::Diagnostics => {
                state.diagnostic_running = true;
                effects.push(SideEffect::RefreshDiagnostics);
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
            if state.searching {
                // Enter while editing: keep filter, leave input mode.
                state.searching = false;
            } else if state.search_supported() {
                state.searching = true;
                let target = if state.glossary_open() {
                    "glossary"
                } else {
                    state.screen.label()
                };
                state.set_status(format!("/{target}"));
            } else {
                state.searching = false;
                state.set_status("no list to filter (2–6 or g)");
            }
        }
        AppAction::ClearSearch => {
            state.searching = false;
            if let Some(q) = state.current_search_mut() {
                q.clear();
            }
            clamp_selection_to_visible(state);
            state.set_status("filter cleared");
        }
        AppAction::SearchInput(c) => {
            if let Some(q) = state.current_search_mut() {
                q.push(c);
            }
            clamp_selection_to_visible(state);
        }
        AppAction::SearchBackspace => {
            if let Some(q) = state.current_search_mut() {
                q.pop();
            }
            clamp_selection_to_visible(state);
        }
        AppAction::ChangeSort => match state.screen {
            Screen::Processes => {
                state.process_sort = state.process_sort.next();
                let selected_pid = state.process_selected_pid;
                let idx = {
                    let visible = state.visible_processes();
                    preserve_selection(&visible, selected_pid)
                };
                state.process_selected = idx;
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
        AppAction::ConfirmFocusLeft | AppAction::ConfirmFocusRight => {
            if let Some(
                Dialog::ConfirmSignal { choice, .. } | Dialog::ConfirmService { choice, .. },
            ) = state.dialog.as_mut()
            {
                *choice = choice.toggle();
            }
        }
        AppAction::Confirm => {
            if let Some(dialog) = state.dialog.take() {
                match dialog {
                    Dialog::ConfirmService {
                        unit,
                        action,
                        choice: ConfirmChoice::Yes,
                        ..
                    } => {
                        effects.push(SideEffect::ServiceAction { unit, action });
                    }
                    Dialog::ConfirmSignal {
                        pid,
                        signal,
                        start_time,
                        choice: ConfirmChoice::Yes,
                        ..
                    } => {
                        effects.push(SideEffect::SendSignal {
                            pid,
                            signal,
                            start_time,
                        });
                    }
                    _ => {}
                }
            }
        }
        AppAction::Cancel => {
            state.dialog = None;
            state.searching = false;
        }
        AppAction::ToggleHelp => {
            if matches!(state.dialog, Some(Dialog::Help)) {
                state.dialog = None;
            } else {
                state.searching = false;
                state.dialog = Some(Dialog::Help);
            }
        }
        AppAction::ToggleGlossary => {
            if matches!(state.dialog, Some(Dialog::Glossary)) {
                state.dialog = None;
                state.searching = false;
            } else {
                state.searching = false;
                state.glossary_selected = 0;
                state.dialog = Some(Dialog::Glossary);
            }
        }
        AppAction::OpenDiagnosticReport => {
            let body = state
                .diagnostic_report
                .clone()
                .unwrap_or_else(|| "No report yet. Press r to run diagnostics.".into());
            state.dialog = Some(Dialog::DiagnosticReport { body });
        }
        AppAction::AcknowledgeFinding => {
            let id = state
                .visible_findings()
                .get(state.finding_selected)
                .map(|f| f.id.clone());
            if let Some(id) = id {
                state.persist.acknowledge(&id);
                state.save_persist();
                state.set_status(format!("acknowledged {id}"));
            }
        }
        AppAction::FollowDiagnosticTarget => {
            let target = state
                .visible_findings()
                .get(state.finding_selected)
                .and_then(|f| f.targets.first())
                .cloned();
            if let Some(target) = target {
                let screen = screen_from_target(target.screen);
                if let Some(q) = target.search.clone() {
                    if let Some(slot) = state.search.get_mut(screen) {
                        *slot = q;
                    }
                }
                state.screen = screen;
                state.searching = false;
                if screen == Screen::Logs {
                    effects.push(SideEffect::RefreshLogs {
                        unit: state.log_unit.clone(),
                    });
                }
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

fn screen_from_target(t: ScreenTarget) -> Screen {
    match t {
        ScreenTarget::Dashboard => Screen::Dashboard,
        ScreenTarget::Processes => Screen::Processes,
        ScreenTarget::Services => Screen::Services,
        ScreenTarget::Logs => Screen::Logs,
        ScreenTarget::Storage => Screen::Storage,
        ScreenTarget::Diagnostics => Screen::Diagnostics,
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
            Vec::new()
        }
        AppEvent::MetricsUpdated(mut m) => {
            m.failed_services = count_failed(&state.services);
            if m.process_count == 0 {
                m.process_count = state.processes.len();
            }
            state.history.record_from_metrics(&m);
            state.metrics = m;
            state.refresh_subsystem_health_from_metrics();
            Vec::new()
        }
        AppEvent::ProcessesUpdated(list) => {
            state.processes = list;
            let selected_pid = state.process_selected_pid;
            let idx = {
                let filtered = state.visible_processes();
                preserve_selection(&filtered, selected_pid)
            };
            state.process_selected = idx;
            let filtered = state.visible_processes();
            if let Some(p) = filtered.get(idx) {
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
            let filtered = filter_services(&state.services, state.current_search(), filter);
            state.service_selected =
                preserve_service_selection(&filtered, state.service_selected_unit.as_deref());
            if let Some(s) = filtered.get(state.service_selected) {
                state.service_selected_unit = Some(s.unit.clone());
                return vec![SideEffect::FetchServiceDetails {
                    unit: s.unit.clone(),
                }];
            }
            Vec::new()
        }
        AppEvent::ServiceDetails(info) => {
            if let Some(slot) = state.services.iter_mut().find(|s| s.unit == info.unit) {
                *slot = info;
            }
            Vec::new()
        }
        AppEvent::LogsUpdated(entries) => {
            state.logs.extend(entries);
            if state.log_follow {
                let filtered = state
                    .logs
                    .filtered(state.current_search(), state.log_min_priority);
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
        AppEvent::DiagnosticsUpdated {
            findings,
            health,
            report,
            probes_degraded,
        } => {
            state.findings = findings;
            state.health_status = health;
            state.diagnostic_report = Some(report);
            state.diagnostic_running = false;
            state.subsystem_health.diagnostics = if probes_degraded.is_empty() {
                SubsystemHealth::Healthy
            } else {
                SubsystemHealth::Degraded
            };
            state.persist.touch_diagnostic_now();
            state.save_persist();
            if state.finding_selected >= state.findings.len() {
                state.finding_selected = state.findings.len().saturating_sub(1);
            }
            if !probes_degraded.is_empty() {
                state.set_status(format!(
                    "diagnostics: {} finding(s); {} probe(s) degraded",
                    state.findings.len(),
                    probes_degraded.len()
                ));
            } else {
                state.set_status(format!(
                    "diagnostics: {} finding(s); health={}",
                    state.findings.len(),
                    health.label()
                ));
            }
            Vec::new()
        }
        AppEvent::OperationFinished(op) => {
            match op {
                OperationResult::Success(msg) => state.set_status(msg),
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

fn list_len(state: &AppState) -> usize {
    if state.glossary_open() {
        return crate::glossary::filtered_terms(state.screen, state.current_search()).len();
    }
    match state.screen {
        Screen::Processes => state.visible_processes().len(),
        Screen::Services => {
            let filter = if state.failed_only {
                ServiceFilter::Failed
            } else {
                state.service_filter
            };
            filter_services(&state.services, state.current_search(), filter).len()
        }
        Screen::Logs => state
            .logs
            .filtered(state.current_search(), state.log_min_priority)
            .len(),
        Screen::Storage => state.visible_storage_children().len(),
        Screen::Diagnostics => state.visible_findings().len(),
        Screen::Dashboard => 0,
    }
}

fn move_selection(state: &mut AppState, delta: i32) {
    let len = list_len(state);
    if len == 0 {
        return;
    }
    let cur = current_selection(state);
    let next = if delta < 0 {
        cur.saturating_sub((-delta) as usize)
    } else {
        (cur + delta as usize).min(len - 1)
    };
    set_selection(state, next);
}

fn current_selection(state: &AppState) -> usize {
    if state.glossary_open() {
        return state.glossary_selected;
    }
    match state.screen {
        Screen::Processes => state.process_selected,
        Screen::Services => state.service_selected,
        Screen::Logs => state.log_selected,
        Screen::Storage => state.storage_selected,
        Screen::Diagnostics => state.finding_selected,
        Screen::Dashboard => 0,
    }
}

fn set_selection(state: &mut AppState, idx: usize) {
    let len = list_len(state);
    let idx = if len == 0 { 0 } else { idx.min(len - 1) };
    if state.glossary_open() {
        state.glossary_selected = idx;
        return;
    }
    match state.screen {
        Screen::Processes => {
            state.process_selected = idx;
            let pid = {
                let filtered = state.visible_processes();
                filtered.get(idx).map(|p| p.pid)
            };
            if let Some(pid) = pid {
                state.process_selected_pid = Some(pid);
            }
        }
        Screen::Services => {
            state.service_selected = idx;
            let filter = if state.failed_only {
                ServiceFilter::Failed
            } else {
                state.service_filter
            };
            let filtered = filter_services(&state.services, state.current_search(), filter);
            if let Some(s) = filtered.get(idx) {
                state.service_selected_unit = Some(s.unit.clone());
            }
        }
        Screen::Logs => state.log_selected = idx,
        Screen::Storage => state.storage_selected = idx,
        Screen::Diagnostics => state.finding_selected = idx,
        Screen::Dashboard => {}
    }
}

fn clamp_selection_to_visible(state: &mut AppState) {
    let len = list_len(state);
    let idx = if len == 0 {
        0
    } else {
        current_selection(state).min(len - 1)
    };
    set_selection(state, idx);
}

fn selected_service_unit(state: &AppState) -> Option<String> {
    let filter = if state.failed_only {
        ServiceFilter::Failed
    } else {
        state.service_filter
    };
    let filtered = filter_services(&state.services, state.current_search(), filter);
    filtered.get(state.service_selected).map(|s| s.unit.clone())
}

fn selected_process_info(state: &AppState) -> Option<ProcessInfo> {
    state
        .visible_processes()
        .get(state.process_selected)
        .map(|p| (*p).clone())
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
        choice: ConfirmChoice::Cancel,
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
    state.dialog = Some(Dialog::ConfirmService {
        unit,
        action,
        choice: ConfirmChoice::Cancel,
    });
}

fn enter_storage_dir(state: &mut AppState) {
    let name = {
        let children = state.visible_storage_children();
        let child = match children.get(state.storage_selected) {
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
    let q = state.current_search().to_string();
    if q.is_empty() {
        return;
    }
    let filtered = state.logs.filtered(&q, state.log_min_priority);
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
        assert_eq!(Screen::from_digit('6'), Some(Screen::Diagnostics));
    }

    #[test]
    fn confirm_defaults_to_cancel() {
        let mut state = demo_state();
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
        let _ = apply_action(&mut state, AppAction::SignalKill);
        match state.dialog {
            Some(Dialog::ConfirmSignal { choice, .. }) => {
                assert_eq!(choice, ConfirmChoice::Cancel);
            }
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn visible_processes_used_for_selection_after_sort() {
        let mut state = demo_state();
        state.screen = Screen::Processes;
        state.processes = vec![
            ProcessInfo {
                pid: 10,
                user: "a".into(),
                name: "low".into(),
                cmd: "low".into(),
                cpu: 1.0,
                mem_pct: 1.0,
                mem_bytes: 1,
                state: "S".into(),
                run_time_secs: 1,
                start_time: 1,
            },
            ProcessInfo {
                pid: 20,
                user: "b".into(),
                name: "high".into(),
                cmd: "high".into(),
                cpu: 90.0,
                mem_pct: 1.0,
                mem_bytes: 1,
                state: "R".into(),
                run_time_secs: 1,
                start_time: 2,
            },
        ];
        state.process_sort = crate::model::ProcessSort::Cpu;
        set_selection(&mut state, 0);
        assert_eq!(state.process_selected_pid, Some(20));
        let _ = apply_action(&mut state, AppAction::ChangeSort); // -> Memory
                                                                 // After sort change, PID selection preserved.
        assert_eq!(state.process_selected_pid, Some(20));
    }

    #[test]
    fn per_screen_search_is_independent() {
        let mut state = demo_state();
        state.screen = Screen::Processes;
        let _ = apply_action(&mut state, AppAction::SearchInput('a'));
        state.screen = Screen::Services;
        let _ = apply_action(&mut state, AppAction::SearchInput('b'));
        assert_eq!(state.search.processes, "a");
        assert_eq!(state.search.services, "b");
    }

    #[test]
    fn search_activates_on_list_screen_and_filters() {
        let mut state = demo_state();
        state.screen = Screen::Processes;
        state.processes = vec![
            ProcessInfo {
                pid: 10,
                user: "a".into(),
                name: "nginx".into(),
                cmd: "nginx".into(),
                cpu: 1.0,
                mem_pct: 1.0,
                mem_bytes: 1,
                state: "S".into(),
                run_time_secs: 1,
                start_time: 1,
            },
            ProcessInfo {
                pid: 20,
                user: "b".into(),
                name: "sshd".into(),
                cmd: "sshd".into(),
                cpu: 1.0,
                mem_pct: 1.0,
                mem_bytes: 1,
                state: "S".into(),
                run_time_secs: 1,
                start_time: 2,
            },
        ];
        state.process_sort = crate::model::ProcessSort::Name;

        let key = KeyEvent::new(KeyCode::Char('/'), KeyModifiers::NONE);
        let action = map_key(&state, key).expect("search action");
        assert!(matches!(action, AppAction::Search));
        let _ = apply_action(&mut state, action);
        assert!(state.searching);

        let _ = apply_action(&mut state, AppAction::SearchInput('n'));
        let _ = apply_action(&mut state, AppAction::SearchInput('g'));
        assert_eq!(state.search.processes, "ng");
        assert_eq!(state.visible_processes().len(), 1);
        assert_eq!(state.visible_processes()[0].name, "nginx");

        let esc = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
        let clear = map_key(&state, esc).expect("clear");
        assert!(matches!(clear, AppAction::ClearSearch));
        let _ = apply_action(&mut state, clear);
        assert!(!state.searching);
        assert!(state.search.processes.is_empty());
        assert_eq!(state.visible_processes().len(), 2);
    }

    #[test]
    fn search_on_dashboard_does_not_stick() {
        let mut state = demo_state();
        assert_eq!(state.screen, Screen::Dashboard);
        let slash = KeyEvent::new(KeyCode::Char('/'), KeyModifiers::NONE);
        assert!(map_key(&state, slash).is_none());
        let _ = apply_action(&mut state, AppAction::Search);
        assert!(!state.searching);
        assert!(state
            .status_message
            .as_deref()
            .unwrap_or("")
            .contains("no list to filter"));
    }

    #[test]
    fn glossary_search_filters_and_esc_exits() {
        let mut state = demo_state();
        let _ = apply_action(&mut state, AppAction::ToggleGlossary);
        assert!(matches!(state.dialog, Some(Dialog::Glossary)));

        let slash = KeyEvent::new(KeyCode::Char('/'), KeyModifiers::NONE);
        let action = map_key(&state, slash).expect("/");
        assert!(matches!(action, AppAction::Search));
        let _ = apply_action(&mut state, action);
        assert!(state.searching);

        let _ = apply_action(&mut state, AppAction::SearchInput('d'));
        let _ = apply_action(&mut state, AppAction::SearchInput('e'));
        let _ = apply_action(&mut state, AppAction::SearchInput('m'));
        let _ = apply_action(&mut state, AppAction::SearchInput('o'));
        assert_eq!(state.current_search(), "demo");
        assert!(!crate::glossary::filtered_terms(state.screen, state.current_search()).is_empty());

        let esc = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
        let clear = map_key(&state, esc).expect("esc");
        let _ = apply_action(&mut state, clear);
        assert!(!state.searching);
        assert!(state.search.glossary.is_empty());
        assert!(matches!(state.dialog, Some(Dialog::Glossary)));
    }
}
