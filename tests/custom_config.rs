use std::path::PathBuf;

use server_tui::app::event::AppEvent;
use server_tui::app::update::{apply_action, apply_event, SideEffect};
use server_tui::app::{AppAction, AppState};
use server_tui::config::Config;

#[test]
fn custom_config_path_survives_state_and_runtime_save_boundary() {
    let temp = tempfile::tempdir().expect("tempdir");
    let custom = temp.path().join("custom/config.toml");
    let alternate = temp.path().join("default/config.toml");
    let initial = Config {
        color: true,
        ..Config::default()
    };
    initial.save_atomic(&custom).expect("seed custom config");

    let (loaded, resolved) = Config::load_or_default(Some(&custom)).expect("load custom");
    let mut state = AppState::new(
        loaded,
        true,
        true,
        true,
        false,
        PathBuf::from("/tmp"),
        resolved,
    );
    apply_action(
        &mut state,
        AppAction::ToggleSetting(server_tui::settings::SettingId::Color),
    );
    let effects = apply_action(&mut state, AppAction::SaveSettings);
    let [SideEffect::SaveConfig {
        path,
        config,
        reason,
        harden_parent,
    }] = effects.as_slice()
    else {
        panic!("expected one typed SaveConfig effect: {effects:?}");
    };
    assert_eq!(path, &custom);
    assert!(!harden_parent);

    let result = config.save_atomic(path);
    apply_event(
        &mut state,
        AppEvent::ConfigSaveFinished {
            path: path.clone(),
            reason: *reason,
            result,
        },
    );
    let (saved, _) = Config::load_or_default(Some(&custom)).expect("reload custom");
    assert!(!saved.color);
    assert!(!alternate.exists());
}
