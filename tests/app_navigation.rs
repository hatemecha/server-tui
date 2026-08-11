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
    s.process.items = vec![
        ProcessInfo {
            pid: 1,
            user: "a".into(),
            name: "a".into(),
            cmd: "a".into(),
            cpu: 1.0,
            mem_pct: Some(1.0),
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
            mem_pct: Some(2.0),
            mem_bytes: 2,
            state: "R".into(),
            run_time_secs: 2,
            start_time: 2,
        },
    ];
    s.process.sort = server_tui::model::ProcessSort::Pid;
    let _ = apply_action(&mut s, AppAction::MoveDown);
    assert_eq!(s.process_selected(), 1);
    assert_eq!(s.process.selected_pid, Some(2));
}

#[test]
fn demo_journey_covers_main_screens_and_quit() {
    use server_tui::app::state::Dialog;
    use server_tui::app::update::SideEffect;

    let mut s = state();
    assert_eq!(s.screen, Screen::Dashboard);

    s.process.items = vec![ProcessInfo {
        pid: 42,
        user: "demo".into(),
        name: "nginx".into(),
        cmd: "nginx".into(),
        cpu: 1.0,
        mem_pct: Some(1.0),
        mem_bytes: 1,
        state: "S".into(),
        run_time_secs: 1,
        start_time: 1,
    }];
    s.process.selected_pid = Some(42);

    let tour = [
        Screen::Processes,
        Screen::Services,
        Screen::Logs,
        Screen::Storage,
        Screen::Diagnostics,
        Screen::Settings,
        Screen::Dashboard,
    ];
    for screen in tour {
        let _ = apply_action(&mut s, AppAction::ChangeScreen(screen));
        assert_eq!(s.screen, screen);
    }

    let _ = apply_action(&mut s, AppAction::ChangeScreen(Screen::Processes));
    let effects = apply_action(&mut s, AppAction::Inspect);
    assert!(
        effects
            .iter()
            .any(|e| matches!(e, SideEffect::FetchProcessDetails { pid: 42 }))
            || matches!(s.dialog, Some(Dialog::Inspector { .. })),
        "inspect on processes should fetch details or open inspector; effects={effects:?} dialog={:?}",
        s.dialog
    );

    let _ = apply_action(&mut s, AppAction::Quit);
    assert!(s.should_quit);
}
