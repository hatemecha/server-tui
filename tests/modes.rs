use server_tui::app::action::AppAction;
use server_tui::app::state::AppState;
use server_tui::app::update::apply_action;
use server_tui::config::Config;
use server_tui::model::{ProcessInfo, ProcessSignal};
use server_tui::providers::demo::DemoProviders;
use server_tui::providers::AdministrativeExecutor;
use std::path::PathBuf;

#[tokio::test]
async fn demo_bundle_never_needs_root() {
    let bundle = DemoProviders::bundle(false);
    let metrics = bundle.metrics.collect().await.expect("demo metrics");
    assert_eq!(metrics.hostname, "demo-host");
    let procs = bundle.processes.list().await.expect("demo procs");
    assert!(!procs.is_empty());
    let services = bundle
        .services
        .list_services()
        .await
        .expect("demo services");
    assert!(services.iter().any(|s| s.is_failed()));
}

#[tokio::test]
async fn read_only_executor_blocks_admin() {
    let bundle = DemoProviders::bundle(true);
    assert!(AdministrativeExecutor::read_only(bundle.admin.as_ref()));
    let err = AdministrativeExecutor::send_signal(
        bundle.admin.as_ref(),
        4821,
        ProcessSignal::Term,
        Some(2_000),
    )
    .await
    .expect_err("should block");
    assert!(
        err.to_string().contains("read-only")
            || err.user_message().contains("READ ONLY")
            || err.user_message().contains("read-only")
            || err.user_message().contains("denegado")
            || matches!(err, server_tui::error::AppError::Permission(_))
    );
}

#[test]
fn confirmations_required_path_opens_dialog() {
    let mut state = AppState::new(
        Config::default(),
        true,
        false,
        true,
        false,
        PathBuf::from("/tmp"),
    );
    state.screen = server_tui::app::Screen::Processes;
    state.processes.push(ProcessInfo {
        pid: 4821,
        user: "alex".into(),
        name: "test-worker".into(),
        cmd: "test-worker".into(),
        cpu: 1.0,
        mem_pct: 1.0,
        mem_bytes: 1,
        state: "R".into(),
        run_time_secs: 1,
        start_time: 1,
    });
    let _ = apply_action(&mut state, AppAction::SignalKill);
    assert!(matches!(
        state.dialog,
        Some(server_tui::app::state::Dialog::ConfirmSignal {
            signal: ProcessSignal::Kill,
            ..
        })
    ));
}
