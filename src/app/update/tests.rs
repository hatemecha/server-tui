use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::path::PathBuf;

use crate::app::action::{AppAction, ConfirmChoice, Screen};
use crate::app::state::{AppState, Dialog};
use crate::app::update::navigation::set_selection;
use crate::app::update::{apply_action, map_key};
use crate::config::Config;
use crate::model::ProcessInfo;

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
    state.process.items.push(ProcessInfo {
        pid: 5,
        user: "u".into(),
        name: "x".into(),
        cmd: "x".into(),
        cpu: 1.0,
        mem_pct: Some(1.0),
        mem_bytes: 1,
        state: "R".into(),
        run_time_secs: 1,
        start_time: 1,
    });
    let _ = apply_action(&mut state, AppAction::SignalTerm);
    assert!(state.status_line().contains("READ ONLY"));
}

#[test]
fn screen_digits() {
    assert_eq!(Screen::from_digit('6'), Some(Screen::Diagnostics));
}

#[test]
fn confirm_defaults_to_cancel() {
    let mut state = demo_state();
    state.screen = Screen::Processes;
    state.process.items.push(ProcessInfo {
        pid: 5,
        user: "u".into(),
        name: "x".into(),
        cmd: "x".into(),
        cpu: 1.0,
        mem_pct: Some(1.0),
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
    state.process.items = vec![
        ProcessInfo {
            pid: 10,
            user: "a".into(),
            name: "low".into(),
            cmd: "low".into(),
            cpu: 1.0,
            mem_pct: Some(1.0),
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
            mem_pct: Some(1.0),
            mem_bytes: 1,
            state: "R".into(),
            run_time_secs: 1,
            start_time: 2,
        },
    ];
    state.process.sort = crate::model::ProcessSort::Cpu;
    set_selection(&mut state, 0);
    assert_eq!(state.process.selected_pid, Some(20));
    let _ = apply_action(&mut state, AppAction::ChangeSort); // -> Memory
                                                             // After sort change, PID selection preserved.
    assert_eq!(state.process.selected_pid, Some(20));
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
    state.process.items = vec![
        ProcessInfo {
            pid: 10,
            user: "a".into(),
            name: "nginx".into(),
            cmd: "nginx".into(),
            cpu: 1.0,
            mem_pct: Some(1.0),
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
            mem_pct: Some(1.0),
            mem_bytes: 1,
            state: "S".into(),
            run_time_secs: 1,
            start_time: 2,
        },
    ];
    state.process.sort = crate::model::ProcessSort::Name;

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
    assert!(state.status_line().contains("no list to filter"));
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
