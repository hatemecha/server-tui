//! Input → AppAction maps.
//! Digit keys `1`–`7` always change screens; Tab cycles focus into content.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::app::action::{AppAction, Screen};
use crate::app::state::{AppState, Dialog, ScanState};

pub fn map_key(state: &AppState, key: KeyEvent) -> Option<AppAction> {
    if let Some(dialog) = &state.dialog {
        return map_dialog_key(state, dialog, key);
    }

    if state.searching {
        return map_search_key(key);
    }

    // Globals that always win.
    match (key.code, key.modifiers) {
        (KeyCode::Char('c'), KeyModifiers::CONTROL) | (KeyCode::Char('q'), _) => {
            return Some(AppAction::Quit);
        }
        (KeyCode::Char('?'), _) => return Some(AppAction::ToggleHelp),
        (KeyCode::Char('g'), _) => return Some(AppAction::ToggleGlossary),
        (KeyCode::Char('w'), _) if matches!(state.screen, Screen::Dashboard) => {
            return Some(AppAction::ToggleWallboard)
        }
        (KeyCode::Tab, KeyModifiers::SHIFT) => return Some(AppAction::PrevFocus),
        (KeyCode::BackTab, _) => return Some(AppAction::PrevFocus),
        (KeyCode::Tab, _) => return Some(AppAction::NextFocus),
        (KeyCode::Char('/'), _) if state.search_supported() => {
            return Some(AppAction::Search);
        }
        (KeyCode::Esc, _) if !state.current_search().is_empty() => {
            return Some(AppAction::ClearSearch);
        }
        _ => {}
    }

    // Screen digits beat screen-local maps so Settings never steals 1–7.
    if let KeyCode::Char(c) = key.code {
        if let Some(screen) = Screen::from_digit(c) {
            return Some(AppAction::ChangeScreen(screen));
        }
    }

    match state.screen {
        Screen::Dashboard => map_dashboard(key),
        Screen::Processes => map_processes(key),
        Screen::Services => map_services(key),
        Screen::Logs => map_logs(key),
        Screen::Storage => map_storage(state, key),
        Screen::Diagnostics => map_diagnostics(key),
        Screen::Settings => map_settings(state, key),
    }
}

fn map_dialog_key(state: &AppState, dialog: &Dialog, key: KeyEvent) -> Option<AppAction> {
    match dialog {
        Dialog::ActionMenu { .. } => match key.code {
            KeyCode::Esc | KeyCode::Char('q') => Some(AppAction::Cancel),
            KeyCode::Up | KeyCode::Char('k') => Some(AppAction::MoveUp),
            KeyCode::Down | KeyCode::Char('j') => Some(AppAction::MoveDown),
            KeyCode::Enter => Some(AppAction::ActivateMenu),
            _ => None,
        },
        Dialog::Inspector { .. } => match key.code {
            KeyCode::Esc | KeyCode::Char('q') => Some(AppAction::Cancel),
            KeyCode::Up | KeyCode::Char('k') => Some(AppAction::MoveUp),
            KeyCode::Down | KeyCode::Char('j') => Some(AppAction::MoveDown),
            KeyCode::PageUp => Some(AppAction::PageUp),
            KeyCode::PageDown => Some(AppAction::PageDown),
            KeyCode::Home => Some(AppAction::Home),
            KeyCode::End => Some(AppAction::End),
            // Enter must NOT close inspectors (accidental dismiss).
            KeyCode::Enter => None,
            _ => None,
        },
        Dialog::ConfirmSignal { .. }
        | Dialog::ConfirmService { .. }
        | Dialog::ConfirmElevation { .. }
        | Dialog::ConfirmResetSettings { .. }
        | Dialog::ConfirmCompleteOnboarding { .. } => match key.code {
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
        KeyCode::Char('w') => Some(AppAction::ToggleWallboard),
        KeyCode::Enter => Some(AppAction::Inspect),
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
        KeyCode::Char('z') => Some(AppAction::SignalStop),
        KeyCode::Char('Z') => Some(AppAction::SignalCont),
        KeyCode::Enter => Some(AppAction::Inspect),
        KeyCode::Char('m') => Some(AppAction::OpenActionMenu),
        KeyCode::Char('e') => Some(AppAction::ExportContext),
        KeyCode::Char('F') => Some(AppAction::ToggleProcessFollow),
        KeyCode::Char('T') => Some(AppAction::ToggleProcessTree),
        _ => None,
    }
}

fn map_services(key: KeyEvent) -> Option<AppAction> {
    match key.code {
        KeyCode::Enter => Some(AppAction::Inspect),
        KeyCode::Char('m') => Some(AppAction::OpenActionMenu),
        KeyCode::Char('E') => Some(AppAction::ExportContext),
        KeyCode::Up | KeyCode::Char('k') => Some(AppAction::MoveUp),
        KeyCode::Down | KeyCode::Char('j') => Some(AppAction::MoveDown),
        KeyCode::PageUp => Some(AppAction::PageUp),
        KeyCode::PageDown => Some(AppAction::PageDown),
        KeyCode::Home => Some(AppAction::Home),
        KeyCode::End => Some(AppAction::End),
        KeyCode::Char('s') => Some(AppAction::StartService),
        KeyCode::Char('x') => Some(AppAction::StopService),
        KeyCode::Char('r') => Some(AppAction::Refresh),
        KeyCode::Char('R') => Some(AppAction::RestartService),
        KeyCode::Char('u') => Some(AppAction::ReloadService),
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
        KeyCode::Char('[') => Some(AppAction::CycleLogPreset),
        KeyCode::Enter => Some(AppAction::Inspect),
        KeyCode::Char('e') => Some(AppAction::ExportContext),
        KeyCode::Char('m') => Some(AppAction::OpenActionMenu),
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
        KeyCode::Enter => match state.storage.tab {
            crate::model::StorageTab::DirectoryUsage => {
                let children = state.visible_storage_children();
                match children.get(state.storage_selected()) {
                    Some(n) if n.is_dir => Some(AppAction::EnterDir),
                    Some(_) => Some(AppAction::Inspect),
                    None => None,
                }
            }
            _ => Some(AppAction::Inspect),
        },
        KeyCode::Backspace => Some(AppAction::ParentDir),
        KeyCode::Char('s') => Some(AppAction::ChangeSort),
        KeyCode::Char('a') => Some(AppAction::ToggleApparentSize),
        KeyCode::Char('x') => Some(AppAction::ToggleStayOnFs),
        KeyCode::Char('r') => Some(AppAction::StartStorageScan(None)),
        KeyCode::Char('t') => Some(AppAction::CycleStorageTab),
        KeyCode::Char('e') => Some(AppAction::ExportContext),
        KeyCode::Char('m') => Some(AppAction::OpenActionMenu),
        KeyCode::Esc if state.storage.scan_state == ScanState::Running => {
            Some(AppAction::CancelStorageScan)
        }
        _ => None,
    }
}

fn map_settings(state: &AppState, key: KeyEvent) -> Option<AppAction> {
    use crate::app::action::FocusPane;
    match key.code {
        KeyCode::Up | KeyCode::Char('k') => Some(AppAction::MoveUp),
        KeyCode::Down | KeyCode::Char('j') => Some(AppAction::MoveDown),
        KeyCode::Home => Some(AppAction::Home),
        KeyCode::End => Some(AppAction::End),
        KeyCode::Left if state.focus == FocusPane::Content => {
            Some(AppAction::AdjustFocusedSetting(-1))
        }
        KeyCode::Right if state.focus == FocusPane::Content => {
            Some(AppAction::AdjustFocusedSetting(1))
        }
        KeyCode::Char(' ') | KeyCode::Enter if state.focus == FocusPane::Content => {
            Some(AppAction::ActivateFocusedSetting)
        }
        // From the sections pane, ← / Space / Enter jump into options.
        KeyCode::Right | KeyCode::Char(' ') | KeyCode::Enter if state.focus == FocusPane::Nav => {
            Some(AppAction::NextFocus)
        }
        KeyCode::Char('r') => Some(AppAction::Refresh),
        KeyCode::Char('S') => Some(AppAction::SaveSettings),
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
        KeyCode::Char('D') => Some(AppAction::DeepDiagnostics),
        KeyCode::Enter => Some(AppAction::Inspect),
        KeyCode::Char('f') => Some(AppAction::FollowDiagnosticTarget),
        KeyCode::Char('m') => Some(AppAction::OpenActionMenu),
        KeyCode::Char('e') => Some(AppAction::ExportContext),
        _ => None,
    }
}
