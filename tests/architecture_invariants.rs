//! Architecture / security invariant tests (demo, read-only, layers, keymap).

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use server_tui::app::action::{AppAction, Screen};
use server_tui::app::state::AppState;
use server_tui::app::update::map_key;
use server_tui::config::Config;
use server_tui::keymap::{
    find_scope_collisions, find_unreachable_under_global_first, global_digit_bindings,
    settings_bindings,
};
use server_tui::settings::SettingId;
use server_tui::viewport::content_rows_from_terminal;

fn key(c: char) -> KeyEvent {
    KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE)
}

fn demo_state() -> AppState {
    AppState::new(
        Config::default(),
        true,
        true,
        true,
        false,
        std::path::PathBuf::from("/tmp"),
    )
}

#[test]
fn settings_digits_toggle_not_navigate() {
    let mut state = demo_state();
    state.screen = Screen::Settings;
    let action = map_key(&state, key('1')).expect("settings 1");
    assert!(matches!(
        action,
        AppAction::ToggleSetting(SettingId::ConfirmSigterm)
    ));
    // Digit 6 has no settings mapping → global ChangeScreen(Diagnostics)
    let action = map_key(&state, key('6')).expect("settings 6");
    assert!(matches!(
        action,
        AppAction::ChangeScreen(Screen::Diagnostics)
    ));
}

#[test]
fn global_digits_change_screen_outside_settings() {
    let state = demo_state();
    assert!(matches!(
        map_key(&state, key('2')),
        Some(AppAction::ChangeScreen(Screen::Processes))
    ));
}

#[test]
fn registry_detects_settings_vs_global_digit_overlap() {
    let unreachable =
        find_unreachable_under_global_first(settings_bindings(), global_digit_bindings());
    assert!(!unreachable.is_empty());
    assert!(find_scope_collisions(settings_bindings()).is_empty());
    assert!(find_scope_collisions(global_digit_bindings()).is_empty());
}

#[test]
fn viewport_rows_scale_with_terminal_height() {
    assert_eq!(content_rows_from_terminal(8), 3);
    assert!(content_rows_from_terminal(40) >= 20);
    assert!(content_rows_from_terminal(120) > content_rows_from_terminal(40));
}

#[test]
fn read_only_demo_flags_set() {
    let state = demo_state();
    assert!(state.demo);
    assert!(state.read_only);
}

#[test]
fn active_state_inactive_not_active() {
    use server_tui::model::ActiveState;
    assert!(!ActiveState::parse("inactive").is_active());
    assert!(ActiveState::parse("inactive").is_inactive());
    assert!(ActiveState::parse("active").is_active());
}

fn walk_rs_files(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    for ent in rd.flatten() {
        let path = ent.path();
        if path.is_dir() {
            walk_rs_files(&path, out);
        } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
            out.push(path);
        }
    }
}

#[test]
fn layer_files_forbid_app_importing_ui_internals() {
    // Soft check: app/ modules should not use `crate::ui::` after dependency inversion.
    let mut files = Vec::new();
    walk_rs_files(std::path::Path::new("src/app"), &mut files);
    let mut offenders = Vec::new();
    for path in files {
        let body = std::fs::read_to_string(&path).unwrap();
        for (i, line) in body.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.starts_with("//") {
                continue;
            }
            if trimmed.contains("crate::ui::") {
                offenders.push(format!("{}:{}", path.display(), i + 1));
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "app must not depend on ui (found: {offenders:?})"
    );
}

#[test]
fn diagnostic_rules_stay_pure() {
    let mut files = Vec::new();
    walk_rs_files(std::path::Path::new("src/diagnostics"), &mut files);
    let banned = [
        "std::fs::",
        "tokio::",
        "zbus::",
        "Command::new",
        "std::process::Command",
        "state.toml",
    ];
    let mut offenders = Vec::new();
    for path in files {
        let body = std::fs::read_to_string(&path).unwrap();
        for (i, line) in body.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.starts_with("//") {
                continue;
            }
            for b in banned {
                if trimmed.contains(b) {
                    offenders.push(format!("{}:{} ({b})", path.display(), i + 1));
                }
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "diagnostics must stay pure (found: {offenders:?})"
    );
}
