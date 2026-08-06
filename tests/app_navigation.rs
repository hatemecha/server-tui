//! Navigation and selection preservation integration-style tests.

use server_tui::app::action::{AppAction, Screen};
use server_tui::app::state::AppState;
use server_tui::app::update::apply_action;
use server_tui::config::Config;
use server_tui::model::ProcessInfo;
use std::path::PathBuf;

fn state() -> AppState {
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
fn changes_screens_with_digits() {
    let mut s = state();
    let _ = apply_action(&mut s, AppAction::ChangeScreen(Screen::Services));
    assert_eq!(s.screen, Screen::Services);
    let _ = apply_action(&mut s, AppAction::ChangeScreen(Screen::Logs));
    assert_eq!(s.screen, Screen::Logs);
}

#[test]
fn quit_action_sets_flag() {
    let mut s = state();
    let _ = apply_action(&mut s, AppAction::Quit);
    assert!(s.should_quit);
}

#[test]
fn selection_moves_in_process_list() {
    let mut s = state();
    s.screen = Screen::Processes;
    s.processes = vec![
        ProcessInfo {
            pid: 1,
            user: "a".into(),
            name: "a".into(),
            cmd: "a".into(),
            cpu: 1.0,
            mem_pct: 1.0,
            mem_bytes: 1,
            state: "S".into(),
            run_time_secs: 1,
            start_time: 1,
        },
        ProcessInfo {
            pid: 2,
            user: "b".into(),
            name: "b".into(),
            cmd: "b".into(),
            cpu: 2.0,
            mem_pct: 2.0,
            mem_bytes: 2,
            state: "R".into(),
            run_time_secs: 2,
            start_time: 2,
        },
    ];
    let _ = apply_action(&mut s, AppAction::MoveDown);
    assert_eq!(s.process_selected, 1);
    assert_eq!(s.process_selected_pid, Some(2));
}
