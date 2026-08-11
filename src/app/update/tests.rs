use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::path::PathBuf;

use crate::app::action::{AppAction, ConfirmChoice, Screen};
use crate::app::event::{AppEvent, DiagnosticOutcome};
use crate::app::state::{AppState, Dialog};
use crate::app::update::navigation::set_selection;
use crate::app::update::{apply_action, apply_event, map_key, SideEffect};
use crate::config::Config;
use crate::error::AppError;
use crate::model::{
    HealthStatus, LogEntry, LogPreset, LogPriority, ProcessInfo, ServiceActionKind, ServiceInfo,
    StorageNode, StorageTree, UnitFileState,
};

fn demo_state() -> AppState {
    AppState::new(
        Config::default(),
        true,
        false,
        true,
        false,
        PathBuf::from("/tmp"),
        Config::default_path(),
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

fn storage_tree(name: &str) -> StorageTree {
    StorageTree {
        root: StorageNode {
            name: name.into(),
            path: PathBuf::from(name),
            is_dir: true,
            size: 1,
            apparent_size: 1,
            children: Vec::new(),
            inaccessible: false,
            error: None,
        },
        files: 0,
        dirs: 1,
        errors: 0,
        excluded: Vec::new(),
        truncated: false,
    }
}

fn log_entry(message: &str) -> LogEntry {
    LogEntry {
        timestamp: message.into(),
        unit: "unit.service".into(),
        pid: None,
        priority: LogPriority::Info,
        message: message.into(),
    }
}

fn service(unit: &str) -> ServiceInfo {
    ServiceInfo {
        unit: unit.into(),
        description: unit.into(),
        load_state: "loaded".into(),
        active_state: "active".into(),
        sub_state: "running".into(),
        unit_path: String::new(),
        unit_file_state: UnitFileState::Enabled,
        fragment_path: None,
    }
}

#[test]
fn stale_storage_result_and_error_cannot_replace_new_scan() {
    let mut state = demo_state();
    let a = super::side_effect::begin_scan(&mut state, PathBuf::from("a"));
    let b = super::side_effect::begin_scan(&mut state, PathBuf::from("b"));
    let SideEffect::StartScan { request_id: a, .. } = a else {
        panic!("scan A")
    };
    let SideEffect::StartScan { request_id: b, .. } = b else {
        panic!("scan B")
    };
    apply_event(
        &mut state,
        AppEvent::StorageProgress {
            request_id: a,
            progress: crate::model::StorageProgress {
                files: 99,
                ..Default::default()
            },
        },
    );
    assert!(state.storage.scan_progress.is_none());
    apply_event(
        &mut state,
        AppEvent::StorageProgress {
            request_id: b,
            progress: crate::model::StorageProgress {
                files: 2,
                ..Default::default()
            },
        },
    );
    assert_eq!(
        state
            .storage
            .scan_progress
            .as_ref()
            .expect("progress B")
            .files,
        2
    );
    apply_event(
        &mut state,
        AppEvent::StorageFinished {
            request_id: a,
            result: Err(AppError::Storage("old failure".into())),
        },
    );
    assert!(state.storage.tree.is_none());
    assert!(state.last_error.is_none());
    apply_event(
        &mut state,
        AppEvent::StorageFinished {
            request_id: b,
            result: Ok(storage_tree("b")),
        },
    );
    assert_eq!(state.storage.tree.as_ref().expect("tree B").root.name, "b");
}

#[test]
fn stale_log_refresh_and_follow_entries_are_ignored() {
    let mut state = demo_state();
    state.log.preset = LogPreset::SelectedService;
    let a = super::side_effect::begin_log_refresh(&mut state, Some("a.service".into()));
    let b = super::side_effect::begin_log_refresh(&mut state, Some("b.service".into()));
    let SideEffect::RefreshLogs { request_id: a, .. } = a else {
        panic!("logs A")
    };
    let SideEffect::RefreshLogs { request_id: b, .. } = b else {
        panic!("logs B")
    };
    apply_event(
        &mut state,
        AppEvent::LogsRefreshFinished {
            request_id: b,
            unit: Some("b.service".into()),
            preset: LogPreset::SelectedService,
            result: Ok(vec![log_entry("new")]),
        },
    );
    apply_event(
        &mut state,
        AppEvent::LogsRefreshFinished {
            request_id: a,
            unit: Some("a.service".into()),
            preset: LogPreset::SelectedService,
            result: Err(AppError::Journal("old failure".into())),
        },
    );
    assert_eq!(
        state.log.buffer.iter().next().expect("new log").message,
        "new"
    );
    assert!(state.last_error.is_none());

    let follow_a = super::side_effect::begin_follow(&mut state, Some("a.service".into()));
    let follow_b = super::side_effect::begin_follow(&mut state, Some("b.service".into()));
    let SideEffect::StartFollow { session_id: a, .. } = follow_a else {
        panic!("follow A")
    };
    let SideEffect::StartFollow { session_id: b, .. } = follow_b else {
        panic!("follow B")
    };
    apply_event(
        &mut state,
        AppEvent::LogsAppended {
            session_id: a,
            entries: vec![log_entry("stale append")],
        },
    );
    apply_event(
        &mut state,
        AppEvent::LogsAppended {
            session_id: b,
            entries: vec![log_entry("current append")],
        },
    );
    assert!(!state
        .log
        .buffer
        .iter()
        .any(|entry| entry.message == "stale append"));
    assert!(state
        .log
        .buffer
        .iter()
        .any(|entry| entry.message == "current append"));
}

#[test]
fn log_navigation_does_not_invalidate_in_flight_refresh() {
    let mut state = demo_state();
    state.screen = Screen::Logs;
    state.log.preset = LogPreset::All;
    let effect = super::side_effect::begin_log_refresh(&mut state, None);
    let SideEffect::RefreshLogs { request_id, .. } = effect else {
        panic!("expected RefreshLogs")
    };
    let pending = state
        .log
        .refresh_request
        .clone()
        .expect("refresh should remain pending");

    let _ = apply_action(&mut state, AppAction::MoveDown);
    let _ = apply_action(&mut state, AppAction::SearchInput('o'));
    let _ = apply_action(&mut state, AppAction::ClearSearch);
    assert_eq!(state.log.refresh_request.as_ref(), Some(&pending));

    apply_event(
        &mut state,
        AppEvent::LogsRefreshFinished {
            request_id,
            unit: None,
            preset: LogPreset::All,
            result: Ok(vec![log_entry("kept after navigation")]),
        },
    );
    assert_eq!(
        state
            .log
            .buffer
            .iter()
            .next()
            .expect("refresh should apply")
            .message,
        "kept after navigation"
    );
}

#[test]
fn stale_diagnostics_cannot_update_state_or_persistence() {
    let mut state = demo_state();
    let a = super::side_effect::begin_diagnostics(&mut state);
    let b = super::side_effect::begin_diagnostics(&mut state);
    let SideEffect::RefreshDiagnostics { request_id: a, .. } = a else {
        panic!("diagnostics A")
    };
    let SideEffect::RefreshDiagnostics { request_id: b, .. } = b else {
        panic!("diagnostics B")
    };
    let outcome = |report: &str, crc| DiagnosticOutcome {
        findings: Vec::new(),
        health: HealthStatus::Ok,
        report: report.into(),
        probes_degraded: Vec::new(),
        smart_crc_observations: vec![("disk".into(), crc)],
    };
    let effects = apply_event(
        &mut state,
        AppEvent::DiagnosticsFinished {
            request_id: a,
            result: Ok(outcome("old", 1)),
        },
    );
    assert!(effects.is_empty());
    assert!(state.diagnostic.report.is_none());
    assert!(!state.persist.smart_crc_counts.contains_key("disk"));
    apply_event(
        &mut state,
        AppEvent::DiagnosticsFinished {
            request_id: a,
            result: Err(AppError::Internal("stale diagnostic failure".into())),
        },
    );
    assert!(state.last_error.is_none());
    let effects = apply_event(
        &mut state,
        AppEvent::DiagnosticsFinished {
            request_id: b,
            result: Ok(outcome("new", 2)),
        },
    );
    assert!(matches!(
        effects.as_slice(),
        [SideEffect::SavePersist { .. }]
    ));
    assert_eq!(state.diagnostic.report.as_deref(), Some("new"));
    assert_eq!(state.persist.smart_crc_counts.get("disk"), Some(&2));
}

#[test]
fn stale_service_and_process_details_are_ignored() {
    let mut state = demo_state();
    state.service.pending_inspect = true;
    let a = super::side_effect::begin_service_details(&mut state, "nginx.service".into());
    let b = super::side_effect::begin_service_details(&mut state, "ssh.service".into());
    let SideEffect::FetchServiceDetails { request_id: a, .. } = a else {
        panic!("service A")
    };
    let SideEffect::FetchServiceDetails { request_id: b, .. } = b else {
        panic!("service B")
    };
    apply_event(
        &mut state,
        AppEvent::ServiceDetailsFinished {
            request_id: b,
            unit: "ssh.service".into(),
            result: Ok(service("ssh.service")),
        },
    );
    apply_event(
        &mut state,
        AppEvent::ServiceDetailsFinished {
            request_id: a,
            unit: "nginx.service".into(),
            result: Ok(service("nginx.service")),
        },
    );
    assert!(
        matches!(&state.dialog, Some(Dialog::Inspector { title, .. }) if title.contains("ssh.service"))
    );

    state.dialog = None;
    let a = super::side_effect::begin_process_details(&mut state, 10);
    let b = super::side_effect::begin_process_details(&mut state, 20);
    let SideEffect::FetchProcessDetails { request_id: a, .. } = a else {
        panic!("process A")
    };
    let SideEffect::FetchProcessDetails { request_id: b, .. } = b else {
        panic!("process B")
    };
    apply_event(
        &mut state,
        AppEvent::ProcessDetailsFinished {
            request_id: a,
            pid: 10,
            result: Err(AppError::Process("old".into())),
        },
    );
    apply_event(
        &mut state,
        AppEvent::ProcessDetailsFinished {
            request_id: b,
            pid: 20,
            result: Ok(crate::model::ProcessDetails::default()),
        },
    );
    assert!(state.process.details.is_some());
    assert!(state.last_error.is_none());
}

#[test]
fn stale_file_preview_and_ppid_map_are_ignored() {
    let mut state = demo_state();
    let path_a = PathBuf::from("a.txt");
    let path_b = PathBuf::from("b.txt");
    let a = super::side_effect::begin_preview(&mut state, path_a.clone());
    let b = super::side_effect::begin_preview(&mut state, path_b.clone());
    let SideEffect::PreviewFile { request_id: a, .. } = a else {
        panic!("preview A")
    };
    let SideEffect::PreviewFile { request_id: b, .. } = b else {
        panic!("preview B")
    };
    apply_event(
        &mut state,
        AppEvent::FilePreviewFinished {
            request_id: a,
            path: path_a,
            result: Err(AppError::Storage("old preview".into())),
        },
    );
    apply_event(
        &mut state,
        AppEvent::FilePreviewFinished {
            request_id: b,
            path: path_b,
            result: Ok(crate::model::FilePreview {
                path: "b.txt".into(),
                bytes_read: 1,
                truncated: false,
                is_binary: false,
                text: "b".into(),
            }),
        },
    );
    assert_eq!(
        state.storage.file_preview.as_ref().expect("preview B").text,
        "b"
    );
    assert!(state.last_error.is_none());

    state.process.tree_mode = true;
    let a = super::side_effect::begin_ppid_map(&mut state);
    let b = super::side_effect::begin_ppid_map(&mut state);
    let SideEffect::FetchPpidMap { request_id: a, .. } = a else {
        panic!("ppid A")
    };
    let SideEffect::FetchPpidMap { request_id: b, .. } = b else {
        panic!("ppid B")
    };
    apply_event(
        &mut state,
        AppEvent::PpidMapFinished {
            request_id: a,
            result: Ok(vec![(1, None)]),
        },
    );
    assert!(state.process.ppids.is_empty());
    apply_event(
        &mut state,
        AppEvent::PpidMapFinished {
            request_id: b,
            result: Ok(vec![(2, Some(1))]),
        },
    );
    assert_eq!(state.process.ppids, vec![(2, Some(1))]);
}

fn confirmation_dialog(index: usize) -> Dialog {
    match index {
        0 => Dialog::ConfirmSignal {
            pid: 42,
            user: "demo".into(),
            command: "demo".into(),
            signal: crate::model::ProcessSignal::Term,
            start_time: 1,
            choice: ConfirmChoice::Cancel,
        },
        1 => Dialog::ConfirmService {
            unit: "demo.service".into(),
            action: ServiceActionKind::Restart,
            choice: ConfirmChoice::Cancel,
        },
        2 => Dialog::ConfirmElevation {
            unit: "demo.service".into(),
            action: ServiceActionKind::Restart,
            choice: ConfirmChoice::Cancel,
        },
        3 => Dialog::ConfirmResetSettings {
            choice: ConfirmChoice::Cancel,
        },
        _ => Dialog::ConfirmCompleteOnboarding {
            choice: ConfirmChoice::Cancel,
        },
    }
}

#[test]
fn every_confirmation_obeys_shared_keyboard_contract() {
    for index in 0..5 {
        let mut state = demo_state();
        state.dialog = Some(confirmation_dialog(index));
        let enter =
            map_key(&state, KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)).expect("enter");
        assert!(apply_action(&mut state, enter).is_empty(), "index {index}");

        state.dialog = Some(confirmation_dialog(index));
        let yes = map_key(
            &state,
            KeyEvent::new(KeyCode::Char('y'), KeyModifiers::NONE),
        )
        .expect("yes");
        assert!(!apply_action(&mut state, yes).is_empty(), "index {index}");

        state.dialog = Some(confirmation_dialog(index));
        apply_action(&mut state, AppAction::ConfirmFocusRight);
        assert!(
            !apply_action(&mut state, AppAction::Confirm).is_empty(),
            "index {index}"
        );

        state.dialog = Some(confirmation_dialog(index));
        let no = map_key(
            &state,
            KeyEvent::new(KeyCode::Char('n'), KeyModifiers::NONE),
        )
        .expect("no");
        assert!(apply_action(&mut state, no).is_empty(), "index {index}");
        assert!(state.dialog.is_none());
    }
}

#[test]
fn performance_switches_publish_complete_non_compounding_policy() {
    let mut state = demo_state();
    state.performance_profile = crate::profile::PerformanceProfile::Balanced;
    state.config.performance_profile = state.performance_profile;
    state.runtime_config = state.config.effective(state.performance_profile);
    let balanced_refresh = state.runtime_config.refresh_ms;
    let balanced_history = state.runtime_config.metric_history_size;

    for _ in 0..4 {
        let effects = apply_action(&mut state, AppAction::CyclePerformanceProfile);
        let [SideEffect::PublishRuntimeConfig(config)] = effects.as_slice() else {
            panic!("profile must publish one complete policy: {effects:?}");
        };
        assert_eq!(config.refresh_ms, state.runtime_config.refresh_ms);
        assert_eq!(
            config.process_refresh_ms,
            state.runtime_config.process_refresh_ms
        );
        assert_eq!(
            config.service_refresh_ms,
            state.runtime_config.service_refresh_ms
        );
        assert_eq!(config.disk_refresh_ms, state.runtime_config.disk_refresh_ms);
        assert_eq!(
            config.temperature_refresh_ms,
            state.runtime_config.temperature_refresh_ms
        );
        assert_eq!(
            config.memory_refresh_ms,
            state.runtime_config.memory_refresh_ms
        );
        assert_eq!(config.metric_history_size, state.history.cpu.capacity());
    }
    assert_eq!(
        state.performance_profile,
        crate::profile::PerformanceProfile::Balanced
    );
    assert_eq!(state.runtime_config.refresh_ms, balanced_refresh);
    assert_eq!(state.runtime_config.metric_history_size, balanced_history);

    apply_action(&mut state, AppAction::ToggleWallboard);
    assert_eq!(
        state.performance_profile,
        crate::profile::PerformanceProfile::Balanced
    );
}

#[test]
fn config_save_failure_reports_session_only_application() {
    let mut state = demo_state();
    let path = state.config_path.clone();
    apply_event(
        &mut state,
        AppEvent::ConfigSaveFinished {
            path,
            reason: crate::app::update::ConfigSaveReason::Settings,
            result: Err(AppError::Internal("disk full".into())),
        },
    );
    assert!(state
        .last_error
        .as_ref()
        .is_some_and(|error| error.user_message().contains("applied for this session")));
}
