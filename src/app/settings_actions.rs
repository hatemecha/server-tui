//! Settings / profile / onboarding actions (extracted from inspect handlers).

use crate::app::action::{AppAction, ConfirmChoice};
use crate::app::state::{AppState, Dialog};
use crate::app::update::{ConfigSaveReason, SideEffect};
use crate::settings::{rows_for_section, SettingId, SettingRow};

pub fn apply_settings_action(state: &mut AppState, action: &AppAction) -> Option<Vec<SideEffect>> {
    match action {
        AppAction::CompleteOnboarding => {
            state.dialog = Some(Dialog::ConfirmCompleteOnboarding {
                choice: ConfirmChoice::Cancel,
            });
            Some(Vec::new())
        }
        AppAction::SaveSettings => {
            state.config.terminal_profile = state.terminal_profile;
            state.config.performance_profile = state.performance_profile;
            state.config.wallboard_default = state.wallboard;
            state.settings.onboarding_pending = false;
            Some(vec![SideEffect::SaveConfig {
                path: state.config_path.clone(),
                config: state.config.clone(),
                reason: ConfigSaveReason::Settings,
                harden_parent: state.config_dir_owned,
            }])
        }
        AppAction::CycleTerminalProfile => {
            cycle_terminal(state, 1);
            Some(Vec::new())
        }
        AppAction::CyclePerformanceProfile => Some(cycle_performance(state, 1)),
        AppAction::ToggleSetting(id) => {
            toggle_bool(state, *id);
            Some(Vec::new())
        }
        AppAction::ActivateFocusedSetting => {
            let row = focused_row(state)?;
            Some(activate_row(state, row))
        }
        AppAction::AdjustFocusedSetting(delta) => {
            let row = focused_row(state)?;
            Some(adjust_row(state, row, *delta))
        }
        AppAction::ResetSettings => {
            state.dialog = Some(Dialog::ConfirmResetSettings {
                choice: ConfirmChoice::Cancel,
            });
            Some(Vec::new())
        }
        _ => None,
    }
}

fn focused_row(state: &AppState) -> Option<SettingRow> {
    let rows = rows_for_section(state.settings.section);
    rows.get(state.settings.selected).copied()
}

fn activate_row(state: &mut AppState, row: SettingRow) -> Vec<SideEffect> {
    match row {
        SettingRow::Checkbox(id) => {
            toggle_bool(state, id);
            Vec::new()
        }
        SettingRow::TerminalProfile => {
            cycle_terminal(state, 1);
            Vec::new()
        }
        SettingRow::PerformanceProfile => cycle_performance(state, 1),
        SettingRow::LogPreset => {
            cycle_log_preset(state, 1);
            Vec::new()
        }
        SettingRow::Save => {
            apply_settings_action(state, &AppAction::SaveSettings).unwrap_or_default()
        }
        SettingRow::CompleteOnboarding => {
            apply_settings_action(state, &AppAction::CompleteOnboarding).unwrap_or_default()
        }
        SettingRow::Reset => {
            apply_settings_action(state, &AppAction::ResetSettings).unwrap_or_default()
        }
        SettingRow::CleanupReports => vec![SideEffect::CleanupReports],
    }
}

fn adjust_row(state: &mut AppState, row: SettingRow, delta: i8) -> Vec<SideEffect> {
    if delta == 0 {
        return Vec::new();
    }
    match row {
        SettingRow::Checkbox(id) => {
            set_bool(state, id, delta > 0);
            Vec::new()
        }
        SettingRow::TerminalProfile => {
            cycle_terminal(state, delta);
            Vec::new()
        }
        SettingRow::PerformanceProfile => cycle_performance(state, delta),
        SettingRow::LogPreset => {
            cycle_log_preset(state, delta);
            Vec::new()
        }
        SettingRow::Save
        | SettingRow::CompleteOnboarding
        | SettingRow::Reset
        | SettingRow::CleanupReports => {
            // Actions ignore ←/→.
            Vec::new()
        }
    }
}

fn toggle_bool(state: &mut AppState, id: SettingId) {
    let next = !bool_value(state, id);
    set_bool(state, id, next);
}

fn bool_value(state: &AppState, id: SettingId) -> bool {
    match id {
        SettingId::ConfirmSigterm => state.config.confirm_sigterm,
        SettingId::ConfirmSigkill => state.config.confirm_sigkill,
        SettingId::ConfirmServiceActions => state.config.confirm_service_actions,
        SettingId::EnableSmartProbes => state.config.enable_smart_probes,
        SettingId::DiagnosticsLightScan => state.config.diagnostics_light_scan,
        SettingId::Wallboard => state.wallboard,
        SettingId::Color => state.color,
    }
}

fn set_bool(state: &mut AppState, id: SettingId, value: bool) {
    match id {
        SettingId::ConfirmSigterm => state.config.confirm_sigterm = value,
        SettingId::ConfirmSigkill => state.config.confirm_sigkill = value,
        SettingId::ConfirmServiceActions => state.config.confirm_service_actions = value,
        SettingId::EnableSmartProbes => state.config.enable_smart_probes = value,
        SettingId::DiagnosticsLightScan => state.config.diagnostics_light_scan = value,
        SettingId::Wallboard => {
            state.wallboard = value;
            state.config.wallboard_default = value;
        }
        SettingId::Color => {
            state.color = value;
            state.config.color = value;
        }
    }
    state.set_status(format!(
        "{}: {} (Save to keep)",
        id.title(),
        if value { "on" } else { "off" }
    ));
}

fn cycle_terminal(state: &mut AppState, delta: i8) {
    state.terminal_profile = if delta < 0 {
        state.terminal_profile.prev()
    } else {
        state.terminal_profile.next()
    };
    state.config.terminal_profile = state.terminal_profile;
    state.set_status(format!(
        "terminal profile: {}",
        state.terminal_profile.label()
    ));
}

fn cycle_performance(state: &mut AppState, delta: i8) -> Vec<SideEffect> {
    state.performance_profile = if delta < 0 {
        state.performance_profile.prev()
    } else {
        state.performance_profile.next()
    };
    state.config.performance_profile = state.performance_profile;
    state.runtime_config = state.config.effective(state.performance_profile);
    state
        .history
        .resize(state.runtime_config.metric_history_size);
    state.set_status(format!(
        "performance profile: {}",
        state.performance_profile.label()
    ));
    vec![SideEffect::PublishRuntimeConfig(
        state.runtime_config.clone(),
    )]
}

fn cycle_log_preset(state: &mut AppState, delta: i8) {
    state.log.preset = if delta < 0 {
        state.log.preset.prev()
    } else {
        state.log.preset.next()
    };
    if state.log.preset == crate::model::LogPreset::Important {
        state.log.min_priority = crate::model::LogPriority::Warning;
    }
    state.set_status(format!("log preset: {}", state.log.preset.label()));
}

/// Shared read of checkbox state for the UI.
pub fn setting_bool(state: &AppState, id: SettingId) -> bool {
    bool_value(state, id)
}
