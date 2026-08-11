//! Keybinding registry — single source of truth for scopes, keys, and labels.

use crossterm::event::KeyCode;

use crate::app::action::{AppAction, Screen};
use crate::settings::SettingId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BindingScope {
    Global,
    Screen(Screen),
    Dialog,
    Search,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BindingVisibility {
    Footer,
    Help,
    Hidden,
}

#[derive(Debug, Clone, Copy)]
pub struct Binding {
    pub scope: BindingScope,
    pub key: KeyCode,
    pub action: BindingAction,
    pub label: &'static str,
    pub visibility: BindingVisibility,
}

/// Lightweight action tokens used by the registry (mapped to AppAction at resolve time).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BindingAction {
    Quit,
    ToggleHelp,
    ToggleGlossary,
    ChangeScreen(Screen),
    ToggleSetting(SettingId),
    Refresh,
    SaveSettings,
    ResetSettings,
}

/// Global digit→screen navigation. Screen-local maps take precedence when both claim a key.
pub fn global_digit_bindings() -> &'static [Binding] {
    &[
        Binding {
            scope: BindingScope::Global,
            key: KeyCode::Char('1'),
            action: BindingAction::ChangeScreen(Screen::Dashboard),
            label: "Dashboard",
            visibility: BindingVisibility::Help,
        },
        Binding {
            scope: BindingScope::Global,
            key: KeyCode::Char('2'),
            action: BindingAction::ChangeScreen(Screen::Processes),
            label: "Processes",
            visibility: BindingVisibility::Help,
        },
        Binding {
            scope: BindingScope::Global,
            key: KeyCode::Char('3'),
            action: BindingAction::ChangeScreen(Screen::Services),
            label: "Services",
            visibility: BindingVisibility::Help,
        },
        Binding {
            scope: BindingScope::Global,
            key: KeyCode::Char('4'),
            action: BindingAction::ChangeScreen(Screen::Logs),
            label: "Logs",
            visibility: BindingVisibility::Help,
        },
        Binding {
            scope: BindingScope::Global,
            key: KeyCode::Char('5'),
            action: BindingAction::ChangeScreen(Screen::Storage),
            label: "Storage",
            visibility: BindingVisibility::Help,
        },
        Binding {
            scope: BindingScope::Global,
            key: KeyCode::Char('6'),
            action: BindingAction::ChangeScreen(Screen::Diagnostics),
            label: "Diagnostics",
            visibility: BindingVisibility::Help,
        },
        Binding {
            scope: BindingScope::Global,
            key: KeyCode::Char('7'),
            action: BindingAction::ChangeScreen(Screen::Settings),
            label: "Settings",
            visibility: BindingVisibility::Help,
        },
    ]
}

pub fn settings_bindings() -> &'static [Binding] {
    &[
        Binding {
            scope: BindingScope::Screen(Screen::Settings),
            key: KeyCode::Char('1'),
            action: BindingAction::ToggleSetting(SettingId::ConfirmSigterm),
            label: "toggle confirm SIGTERM",
            visibility: BindingVisibility::Footer,
        },
        Binding {
            scope: BindingScope::Screen(Screen::Settings),
            key: KeyCode::Char('2'),
            action: BindingAction::ToggleSetting(SettingId::ConfirmSigkill),
            label: "toggle confirm SIGKILL",
            visibility: BindingVisibility::Footer,
        },
        Binding {
            scope: BindingScope::Screen(Screen::Settings),
            key: KeyCode::Char('3'),
            action: BindingAction::ToggleSetting(SettingId::ConfirmServiceActions),
            label: "toggle confirm service actions",
            visibility: BindingVisibility::Footer,
        },
        Binding {
            scope: BindingScope::Screen(Screen::Settings),
            key: KeyCode::Char('4'),
            action: BindingAction::ToggleSetting(SettingId::EnableSmartProbes),
            label: "toggle SMART probes",
            visibility: BindingVisibility::Footer,
        },
        Binding {
            scope: BindingScope::Screen(Screen::Settings),
            key: KeyCode::Char('5'),
            action: BindingAction::ToggleSetting(SettingId::DiagnosticsLightScan),
            label: "toggle light diagnostics",
            visibility: BindingVisibility::Footer,
        },
        Binding {
            scope: BindingScope::Screen(Screen::Settings),
            key: KeyCode::Char('S'),
            action: BindingAction::SaveSettings,
            label: "save settings",
            visibility: BindingVisibility::Footer,
        },
        Binding {
            scope: BindingScope::Screen(Screen::Settings),
            key: KeyCode::Char('d'),
            action: BindingAction::ResetSettings,
            label: "reset settings",
            visibility: BindingVisibility::Help,
        },
    ]
}

pub fn binding_to_action(action: BindingAction) -> AppAction {
    match action {
        BindingAction::Quit => AppAction::Quit,
        BindingAction::ToggleHelp => AppAction::ToggleHelp,
        BindingAction::ToggleGlossary => AppAction::ToggleGlossary,
        BindingAction::ChangeScreen(s) => AppAction::ChangeScreen(s),
        BindingAction::ToggleSetting(id) => AppAction::ToggleSetting(id),
        BindingAction::Refresh => AppAction::Refresh,
        BindingAction::SaveSettings => AppAction::SaveSettings,
        BindingAction::ResetSettings => AppAction::ResetSettings,
    }
}

/// Detect duplicate keys within the same scope (collision).
pub fn find_scope_collisions(bindings: &[Binding]) -> Vec<(BindingScope, KeyCode)> {
    let mut seen = Vec::new();
    let mut collisions = Vec::new();
    for b in bindings {
        let key = (b.scope, b.key);
        if seen.contains(&key) {
            collisions.push(key);
        } else {
            seen.push(key);
        }
    }
    collisions
}

/// Screen-local bindings that would be unreachable if globals were resolved first.
pub fn find_unreachable_under_global_first(
    local: &[Binding],
    global: &[Binding],
) -> Vec<&'static str> {
    let mut out = Vec::new();
    for l in local {
        if global.iter().any(|g| g.key == l.key) {
            out.push(l.label);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_digits_collide_with_global_digits() {
        let unreachable =
            find_unreachable_under_global_first(settings_bindings(), global_digit_bindings());
        assert!(
            unreachable.iter().any(|l| l.contains("confirm")),
            "settings digit toggles must be detected as collisions vs global digits"
        );
    }

    #[test]
    fn no_duplicate_keys_inside_settings_scope() {
        let collisions = find_scope_collisions(settings_bindings());
        assert!(
            collisions.is_empty(),
            "duplicate settings bindings: {collisions:?}"
        );
    }

    #[test]
    fn no_duplicate_global_digits() {
        let collisions = find_scope_collisions(global_digit_bindings());
        assert!(collisions.is_empty());
    }
}
