//! Apply typed AppAction → state mutations + SideEffect list.

use crate::app::action::{AppAction, ConfirmChoice, FocusPane, Screen};
use crate::app::state::{AppState, Dialog, ScanState};
use crate::app::update::navigation::{
    clamp_selection_to_visible, enter_storage_dir, maybe_confirm_service, maybe_confirm_signal,
    move_selection, selected_service_unit, set_selection, step_log_match,
};
use crate::app::update::side_effect::SideEffect;
use crate::model::{
    preserve_selection, ProcessSignal, ScreenTarget, ServiceActionKind, ServiceFilter,
};

pub fn apply_action(state: &mut AppState, action: AppAction) -> Vec<SideEffect> {
    let mut effects = Vec::new();
    match action {
        AppAction::Quit => state.should_quit = true,
        AppAction::ChangeScreen(screen) => {
            let leaving_logs = state.screen == Screen::Logs && screen != Screen::Logs;
            if leaving_logs && state.log.follow {
                state.log.follow = false;
                effects.push(SideEffect::StopFollow);
            }
            state.screen = screen;
            state.searching = false;
            if screen == Screen::Storage
                && state.storage.tree.is_none()
                && state.storage.scan_state == ScanState::Idle
            {
                effects.push(SideEffect::StartScan {
                    path: state.storage.scan_path.clone(),
                });
                state.storage.scan_state = ScanState::Running;
            }
            if screen == Screen::Logs {
                effects.push(SideEffect::RefreshLogs {
                    unit: state.log.unit.clone(),
                });
            }
            if screen == Screen::Diagnostics {
                effects.push(SideEffect::RefreshDiagnostics);
                state.diagnostic.running = true;
            }
        }
        AppAction::Refresh => match state.screen {
            Screen::Dashboard => effects.push(SideEffect::RefreshMetrics),
            Screen::Processes => effects.push(SideEffect::RefreshProcesses),
            Screen::Services => effects.push(SideEffect::RefreshServices),
            Screen::Logs => effects.push(SideEffect::RefreshLogs {
                unit: state.log.unit.clone(),
            }),
            Screen::Storage => {
                effects.push(SideEffect::StartScan {
                    path: state.storage.scan_path.clone(),
                });
                state.storage.scan_state = ScanState::Running;
            }
            Screen::Diagnostics => {
                state.diagnostic.running = true;
                effects.push(SideEffect::RefreshDiagnostics);
            }
            Screen::Settings => state.set_status("settings"),
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
                state.process.sort = state.process.sort.next();
                let selected_pid = state.process.selected_pid;
                let idx = {
                    let visible = state.visible_processes();
                    preserve_selection(&visible, selected_pid)
                };
                state.process.vp.selected = idx;
                state.set_status(format!("sort: {}", state.process.sort.label()));
            }
            Screen::Storage => {
                state.storage.sort = state.storage.sort.next();
                if let Some(tree) = state.storage.tree.as_mut() {
                    tree.root.sort_children(state.storage.sort);
                }
                state.set_status(format!("sort: {}", state.storage.sort.label()));
            }
            _ => {}
        },
        AppAction::ToggleFullCommand => state.process.show_full_cmd = !state.process.show_full_cmd,
        AppAction::ToggleFailedOnly => {
            state.service.failed_only = !state.service.failed_only;
            state.service.filter = if state.service.failed_only {
                ServiceFilter::Failed
            } else {
                ServiceFilter::All
            };
        }
        AppAction::ToggleFollow => {
            state.log.follow = !state.log.follow;
            if state.log.follow {
                effects.push(SideEffect::StartFollow {
                    unit: state.log.unit.clone(),
                });
            } else {
                effects.push(SideEffect::StopFollow);
            }
        }
        AppAction::NextMatch => step_log_match(state, true),
        AppAction::PrevMatch => step_log_match(state, false),
        AppAction::ChangePriority => {
            state.log.min_priority = state.log.min_priority.next_minimum();
            state.set_status(format!("min priority: {}", state.log.min_priority.label()));
        }
        AppAction::ToggleWrap => state.log.wrap = !state.log.wrap,
        AppAction::ToggleApparentSize => state.storage.use_apparent = !state.storage.use_apparent,
        AppAction::ToggleStayOnFs => {
            state.storage.stay_on_fs = !state.storage.stay_on_fs;
            state.set_status(format!(
                "stay on filesystem: {}",
                if state.storage.stay_on_fs {
                    "on"
                } else {
                    "off"
                }
            ));
        }
        AppAction::EnterDir => enter_storage_dir(state),
        AppAction::ParentDir => {
            state.storage.cwd.pop();
            state.storage.vp.selected = 0;
        }
        AppAction::StartService => {
            if let Some(e) = maybe_confirm_service(state, ServiceActionKind::Start) {
                effects.push(e);
            }
        }
        AppAction::StopService => {
            if let Some(e) = maybe_confirm_service(state, ServiceActionKind::Stop) {
                effects.push(e);
            }
        }
        AppAction::RestartService => {
            if let Some(e) = maybe_confirm_service(state, ServiceActionKind::Restart) {
                effects.push(e);
            }
        }
        AppAction::ReloadService => {
            if let Some(e) = maybe_confirm_service(state, ServiceActionKind::Reload) {
                effects.push(e);
            }
        }
        AppAction::EnableService => {
            if let Some(e) = maybe_confirm_service(state, ServiceActionKind::Enable) {
                effects.push(e);
            }
        }
        AppAction::DisableService => {
            if let Some(e) = maybe_confirm_service(state, ServiceActionKind::Disable) {
                effects.push(e);
            }
        }
        AppAction::OpenLogsForSelected => {
            if let Some(unit) = selected_service_unit(state) {
                state.log.unit = Some(unit.clone());
                state.screen = Screen::Logs;
                state.log.buffer.clear();
                effects.push(SideEffect::RefreshLogs { unit: Some(unit) });
            }
        }
        AppAction::SignalTerm => {
            if let Some(e) = maybe_confirm_signal(state, ProcessSignal::Term) {
                effects.push(e);
            }
        }
        AppAction::SignalKill => {
            if let Some(e) = maybe_confirm_signal(state, ProcessSignal::Kill) {
                effects.push(e);
            }
        }
        AppAction::StartStorageScan(path) => {
            let path = path.unwrap_or_else(|| state.storage.scan_path.clone());
            state.storage.scan_path = path.clone();
            state.storage.scan_state = ScanState::Running;
            state.storage.cwd.clear();
            effects.push(SideEffect::StartScan { path });
        }
        AppAction::CancelStorageScan => {
            if state.storage.scan_state == ScanState::Running {
                state.storage.scan_state = ScanState::Cancelled;
                effects.push(SideEffect::CancelScan);
                state.set_status("scan cancel requested");
            }
        }
        AppAction::ConfirmFocusLeft | AppAction::ConfirmFocusRight => {
            if let Some(
                Dialog::ConfirmSignal { choice, .. }
                | Dialog::ConfirmService { choice, .. }
                | Dialog::ConfirmElevation { choice, .. }
                | Dialog::ConfirmResetSettings { choice, .. }
                | Dialog::ConfirmCompleteOnboarding { choice, .. },
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
                    Dialog::ConfirmElevation {
                        unit,
                        action,
                        choice: ConfirmChoice::Yes,
                        ..
                    } => {
                        effects.push(SideEffect::SudoServiceAction { unit, action });
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
                    Dialog::ConfirmResetSettings {
                        choice: ConfirmChoice::Yes,
                    } => {
                        let path = state.config_path.clone();
                        state.config = crate::config::Config::default();
                        state.terminal_profile = state.config.terminal_profile;
                        state.performance_profile = state.config.performance_profile;
                        if let Err(e) = state.config.save_atomic(&path) {
                            state.set_error(e);
                        } else {
                            state.set_success("settings reset to defaults");
                        }
                    }
                    Dialog::ConfirmCompleteOnboarding {
                        choice: ConfirmChoice::Yes,
                    } => {
                        state.config.onboarding_completed = true;
                        state.settings.onboarding_pending = false;
                        if let Err(e) = state.config.save_atomic(&state.config_path.clone()) {
                            state.set_error(e);
                        } else {
                            state.set_success("onboarding completed");
                            state.screen = Screen::Dashboard;
                        }
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
                state.glossary_vp.selected = 0;
                state.dialog = Some(Dialog::Glossary);
            }
        }
        AppAction::OpenDiagnosticReport => {
            let body = state
                .diagnostic
                .report
                .clone()
                .unwrap_or_else(|| "No report yet. Press r to run diagnostics.".into());
            state.dialog = Some(Dialog::DiagnosticReport { body });
        }
        AppAction::AcknowledgeFinding => {
            let id = state
                .visible_findings()
                .get(state.finding_selected())
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
                .get(state.finding_selected())
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
                        unit: state.log.unit.clone(),
                    });
                }
            }
        }
        AppAction::ToggleWallboard => {
            state.wallboard = !state.wallboard;
            state.config.wallboard_default = state.wallboard;
            state.set_status(if state.wallboard {
                "wallboard ON"
            } else {
                "wallboard OFF"
            });
        }
        AppAction::OpenActionMenu
        | AppAction::Inspect
        | AppAction::ExportContext
        | AppAction::ActivateMenu
        | AppAction::MenuSelect
        | AppAction::ToggleProcessFollow
        | AppAction::ToggleProcessTree
        | AppAction::CompleteOnboarding
        | AppAction::SaveSettings
        | AppAction::CycleTerminalProfile
        | AppAction::CyclePerformanceProfile
        | AppAction::ToggleSetting(_)
        | AppAction::ResetSettings => {
            if let Some(extra) = crate::app::inspect_actions::apply_inspect_action(state, &action) {
                effects.extend(extra);
            }
        }
        AppAction::CleanupReportsNow => {
            effects.push(SideEffect::CleanupReports);
        }
        AppAction::CycleLogPreset => {
            state.log.preset = state.log.preset.next();
            if state.log.preset == crate::model::LogPreset::Important {
                state.log.min_priority = crate::model::LogPriority::Warning;
            }
            state.log.buffer.clear();
            state.set_status(format!("log preset: {}", state.log.preset.label()));
            effects.push(SideEffect::RefreshLogs {
                unit: state.log.unit.clone(),
            });
        }
        AppAction::CycleStorageTab => {
            state.storage.tab = state.storage.tab.next();
            state.storage.vp = crate::viewport::ViewportState::new();
            state.set_status(format!("storage tab: {}", state.storage.tab.label()));
        }
        AppAction::DeepDiagnostics => {
            state.diagnostic.deep = true;
            state.set_status("deep diagnostics requested");
            effects.push(SideEffect::RefreshDiagnostics);
        }
        AppAction::SettingsMove => {}
        AppAction::SignalStop => {
            if let Some(e) = maybe_confirm_signal(state, ProcessSignal::Stop) {
                effects.push(e);
            }
        }
        AppAction::SignalCont => {
            if let Some(e) = maybe_confirm_signal(state, ProcessSignal::Cont) {
                effects.push(e);
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
