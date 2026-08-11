//! Settings / profile / onboarding actions (extracted from inspect handlers).

use crate::app::action::{AppAction, ConfirmChoice};
use crate::app::state::{AppState, Dialog};
use crate::app::update::SideEffect;

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
            match state.config.save_atomic(&state.config_path) {
                Ok(()) => {
                    state.settings.onboarding_pending = false;
                    state.set_success(format!(
                        "settings saved → {}",
                        crate::sanitize::sanitize_path_display(&state.config_path)
                    ));
                }
                Err(e) => state.set_error(e),
            }
            Some(vec![SideEffect::PublishConfig])
        }
        AppAction::CycleTerminalProfile => {
            state.terminal_profile = state.terminal_profile.next();
            state.config.terminal_profile = state.terminal_profile;
            state.set_status(format!(
                "terminal_profile: {}",
                state.terminal_profile.label()
            ));
            Some(Vec::new())
        }
        AppAction::CyclePerformanceProfile => {
            state.performance_profile = state.performance_profile.next();
            state.config.performance_profile = state.performance_profile;
            apply_performance(state);
            state.set_status(format!(
                "performance_profile: {}",
                state.performance_profile.label()
            ));
            Some(vec![SideEffect::PublishConfig])
        }
        AppAction::ToggleSetting(id) => {
            use crate::settings::SettingId;
            match id {
                SettingId::ConfirmSigterm => {
                    state.config.confirm_sigterm = !state.config.confirm_sigterm
                }
                SettingId::ConfirmSigkill => {
                    state.config.confirm_sigkill = !state.config.confirm_sigkill
                }
                SettingId::ConfirmServiceActions => {
                    state.config.confirm_service_actions = !state.config.confirm_service_actions
                }
                SettingId::EnableSmartProbes => {
                    state.config.enable_smart_probes = !state.config.enable_smart_probes
                }
                SettingId::DiagnosticsLightScan => {
                    state.config.diagnostics_light_scan = !state.config.diagnostics_light_scan
                }
                SettingId::Wallboard => {
                    state.wallboard = !state.wallboard;
                    state.config.wallboard_default = state.wallboard;
                }
                SettingId::Color => {
                    state.color = !state.color;
                    state.config.color = state.color;
                }
            }
            state.set_status(format!("toggled {} (Save with S)", id.label()));
            Some(Vec::new())
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

fn apply_performance(state: &mut AppState) {
    let scale = state.performance_profile.refresh_scale();
    let base = 1000u64;
    state.config.refresh_ms = ((base as f64) * scale) as u64;
    state.config.metric_history_size = state
        .performance_profile
        .history_size(state.config.metric_history_size);
}
