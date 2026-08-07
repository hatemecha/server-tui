//! Application entrypoint and async event loop.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use clap::Parser;
use crossterm::event::{Event, EventStream, KeyEventKind};
use futures::StreamExt;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;
use tracing_subscriber::EnvFilter;

use server_tui::app::event::{AppEvent, OperationResult};
use server_tui::app::state::AppState;
use server_tui::app::update::{apply_event, SideEffect};
use server_tui::cli::{Cli, Commands};
use server_tui::config::{expand_tilde, Config};
use server_tui::diagnostics::{compute_health_status, evaluate, format_text_report};
use server_tui::doctor::run_doctor;
use server_tui::error::AppError;
use server_tui::providers::demo::DemoProviders;
use server_tui::providers::linux::linux_bundle;
use server_tui::providers::ProviderBundle;
use server_tui::terminal::TerminalGuard;
use server_tui::ui;
use server_tui::{APP_NAME, APP_VERSION};

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    if let Some(Commands::Doctor(args)) = cli.command.clone() {
        let code = run_doctor(args).await;
        std::process::exit(i32::from(code));
    }

    if let Err(err) = run(cli).await {
        eprintln!("{APP_NAME} error: {}", err.user_message());
        tracing::error!("{err}");
        std::process::exit(1);
    }
}

async fn run(cli: Cli) -> Result<(), AppError> {
    init_logging(cli.debug_log.as_ref())?;

    let (mut config, cfg_path) = Config::load_or_default(cli.config.as_deref())?;
    tracing::info!(?cfg_path, version = APP_VERSION, "starting");

    if let Some(ms) = cli.refresh_ms {
        if ms < 200 {
            return Err(AppError::Configuration {
                path: cfg_path,
                message: "refresh_ms must be >= 200".into(),
            });
        }
        config.refresh_ms = ms;
    }
    if cli.ascii {
        config.unicode = false;
    }

    let color = cli.color_enabled(config.color);
    let scan_path = cli
        .scan_path
        .clone()
        .unwrap_or_else(|| config.expand_scan_path());

    let providers = if cli.demo {
        DemoProviders::bundle(cli.read_only)
    } else {
        linux_bundle(cli.read_only, &config)
    };

    let mut state = AppState::new(
        config.clone(),
        cli.demo,
        cli.read_only,
        color,
        cli.ascii || !config.unicode,
        scan_path,
    );
    if state.demo {
        state.set_status("DEMO mode: synthetic data only; host is not modified.");
    } else if state.read_only {
        state.set_status("READ ONLY: signals and systemd changes are disabled.");
    } else {
        state.set_status(
            "Admin actions require confirmation. Prefer --read-only for observation-only.",
        );
    }

    let mut terminal = TerminalGuard::enter()?;
    let result = run_loop(&mut terminal, &mut state, providers, config).await;
    terminal.restore()?;
    result
}

fn init_logging(path: Option<&PathBuf>) -> Result<(), AppError> {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    if let Some(path) = path {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .map_err(|e| AppError::Internal(format!("debug log: {e}")))?;
        tracing_subscriber::fmt()
            .with_env_filter(filter)
            .with_writer(file)
            .with_ansi(false)
            .init();
    } else {
        tracing_subscriber::fmt()
            .with_env_filter(filter)
            .with_writer(std::io::sink)
            .init();
    }
    Ok(())
}

async fn run_loop(
    terminal: &mut TerminalGuard,
    state: &mut AppState,
    providers: ProviderBundle,
    config: Config,
) -> Result<(), AppError> {
    let providers = Arc::new(providers);
    let (tx, mut rx) = mpsc::channel::<AppEvent>(256);
    let app_cancel = CancellationToken::new();
    let tracker = TaskTracker::new();

    let size = terminal
        .terminal()
        .size()
        .map_err(|e| AppError::Terminal(e.to_string()))?;
    state.width = size.width;
    state.height = size.height;

    spawn_pollers(
        Arc::clone(&providers),
        tx.clone(),
        app_cancel.clone(),
        config.clone(),
        &tracker,
    );

    {
        let p = Arc::clone(&providers);
        let tx = tx.clone();
        tracker.spawn(async move {
            match p.metrics.collect().await {
                Ok(m) => {
                    let _ = tx.send(AppEvent::MetricsUpdated(m)).await;
                }
                Err(e) => {
                    let _ = tx.send(AppEvent::Error(e)).await;
                }
            }
        });
    }

    let mut events = EventStream::new();
    let mut follow_cancel: Option<CancellationToken> = None;
    let mut scan_cancel: Option<CancellationToken> = None;

    let mut redraw = tokio::time::interval(Duration::from_millis(config.refresh_ms.max(200)));
    redraw.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    let mut sigterm = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        .map_err(|e| AppError::Terminal(format!("SIGTERM handler: {e}")))?;
    let mut sighup = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::hangup())
        .map_err(|e| AppError::Terminal(format!("SIGHUP handler: {e}")))?;

    loop {
        terminal
            .terminal()
            .draw(|f| ui::draw(f, state))
            .map_err(|e| AppError::Terminal(e.to_string()))?;

        if state.should_quit {
            break;
        }

        tokio::select! {
            _ = redraw.tick() => {
                let _ = tx.send(AppEvent::Tick).await;
            }
            _ = sigterm.recv() => {
                tracing::info!("received SIGTERM");
                state.should_quit = true;
            }
            _ = sighup.recv() => {
                tracing::info!("received SIGHUP");
                state.should_quit = true;
            }
            maybe_term = events.next() => {
                match maybe_term {
                    Some(Ok(Event::Key(key))) => {
                        if key.kind == KeyEventKind::Press || key.kind == KeyEventKind::Repeat {
                            let effects = apply_event(state, AppEvent::Input(key));
                            handle_effects(
                                effects,
                                state,
                                &providers,
                                &tx,
                                &app_cancel,
                                &mut follow_cancel,
                                &mut scan_cancel,
                                &config,
                                &tracker,
                            ).await;
                        }
                    }
                    Some(Ok(Event::Resize(w, h))) => {
                        let _ = apply_event(state, AppEvent::Resize(w, h));
                    }
                    Some(Ok(_)) => {}
                    Some(Err(e)) => {
                        tracing::warn!("input error: {e}");
                    }
                    None => break,
                }
            }
            maybe_ev = rx.recv() => {
                match maybe_ev {
                    Some(ev) => {
                        let effects = apply_event(state, ev);
                        handle_effects(
                            effects,
                            state,
                            &providers,
                            &tx,
                            &app_cancel,
                            &mut follow_cancel,
                            &mut scan_cancel,
                            &config,
                            &tracker,
                        ).await;
                    }
                    None => break,
                }
            }
        }
    }

    // Structured shutdown: cancel → wait children → timeout → abort.
    app_cancel.cancel();
    if let Some(c) = follow_cancel.take() {
        c.cancel();
    }
    if let Some(c) = scan_cancel.take() {
        c.cancel();
    }
    tracker.close();
    let shutdown = tokio::time::timeout(Duration::from_secs(2), tracker.wait());
    if shutdown.await.is_err() {
        tracing::warn!("task shutdown timed out; aborting remaining tasks");
        // TaskTracker wait timed out; remaining tasks drop on runtime shutdown.
    }
    Ok(())
}

fn spawn_pollers(
    providers: Arc<ProviderBundle>,
    tx: mpsc::Sender<AppEvent>,
    cancel: CancellationToken,
    config: Config,
    tracker: &TaskTracker,
) {
    {
        let providers = Arc::clone(&providers);
        let tx = tx.clone();
        let cancel = cancel.clone();
        let ms = config.refresh_ms;
        tracker.spawn(async move {
            let mut ticker = tokio::time::interval(Duration::from_millis(ms));
            loop {
                tokio::select! {
                    _ = cancel.cancelled() => break,
                    _ = ticker.tick() => {
                        match providers.metrics.collect().await {
                            Ok(m) => { let _ = tx.send(AppEvent::MetricsUpdated(m)).await; }
                            Err(e) => { let _ = tx.send(AppEvent::Error(e)).await; }
                        }
                    }
                }
            }
        });
    }
    {
        let providers = Arc::clone(&providers);
        let tx = tx.clone();
        let cancel = cancel.clone();
        let ms = config.process_refresh_ms;
        tracker.spawn(async move {
            let mut ticker = tokio::time::interval(Duration::from_millis(ms));
            loop {
                tokio::select! {
                    _ = cancel.cancelled() => break,
                    _ = ticker.tick() => {
                        match providers.processes.list().await {
                            Ok(p) => { let _ = tx.send(AppEvent::ProcessesUpdated(p)).await; }
                            Err(e) => { let _ = tx.send(AppEvent::Error(e)).await; }
                        }
                    }
                }
            }
        });
    }
    {
        let providers = Arc::clone(&providers);
        let tx = tx.clone();
        let cancel = cancel.clone();
        let ms = config.service_refresh_ms;
        tracker.spawn(async move {
            let mut ticker = tokio::time::interval(Duration::from_millis(ms));
            loop {
                tokio::select! {
                    _ = cancel.cancelled() => break,
                    _ = ticker.tick() => {
                        match providers.services.list_services().await {
                            Ok(s) => { let _ = tx.send(AppEvent::ServicesUpdated(s)).await; }
                            Err(e) => { let _ = tx.send(AppEvent::Error(e)).await; }
                        }
                    }
                }
            }
        });
    }
}

#[allow(clippy::too_many_arguments)]
async fn handle_effects(
    effects: Vec<SideEffect>,
    state: &mut AppState,
    providers: &Arc<ProviderBundle>,
    tx: &mpsc::Sender<AppEvent>,
    _app_cancel: &CancellationToken,
    follow_cancel: &mut Option<CancellationToken>,
    scan_cancel: &mut Option<CancellationToken>,
    config: &Config,
    tracker: &TaskTracker,
) {
    for effect in effects {
        match effect {
            SideEffect::RefreshMetrics => {
                let p = Arc::clone(providers);
                let tx = tx.clone();
                tracker.spawn(async move {
                    match p.metrics.collect().await {
                        Ok(m) => {
                            let _ = tx.send(AppEvent::MetricsUpdated(m)).await;
                        }
                        Err(e) => {
                            let _ = tx.send(AppEvent::Error(e)).await;
                        }
                    }
                });
            }
            SideEffect::RefreshProcesses => {
                let p = Arc::clone(providers);
                let tx = tx.clone();
                tracker.spawn(async move {
                    match p.processes.list().await {
                        Ok(list) => {
                            let _ = tx.send(AppEvent::ProcessesUpdated(list)).await;
                        }
                        Err(e) => {
                            let _ = tx.send(AppEvent::Error(e)).await;
                        }
                    }
                });
            }
            SideEffect::RefreshServices => {
                let p = Arc::clone(providers);
                let tx = tx.clone();
                tracker.spawn(async move {
                    match p.services.list_services().await {
                        Ok(list) => {
                            let _ = tx.send(AppEvent::ServicesUpdated(list)).await;
                        }
                        Err(e) => {
                            let _ = tx.send(AppEvent::Error(e)).await;
                        }
                    }
                });
            }
            SideEffect::RefreshLogs { unit } => {
                let p = Arc::clone(providers);
                let tx = tx.clone();
                tracker.spawn(async move {
                    match p.logs.recent(unit.as_deref(), 200).await {
                        Ok(entries) => {
                            let _ = tx.send(AppEvent::LogsUpdated(entries)).await;
                        }
                        Err(e) => {
                            let _ = tx.send(AppEvent::Error(e)).await;
                        }
                    }
                });
            }
            SideEffect::RefreshDiagnostics => {
                let p = Arc::clone(providers);
                let tx = tx.clone();
                tracker.spawn(async move {
                    match p.diagnostics.probe().await {
                        Ok(snap) => {
                            let findings = evaluate(&snap);
                            let health = compute_health_status(
                                &findings,
                                snap.systemd_observable,
                                snap.journal_observable,
                            );
                            let report =
                                format_text_report(health, &findings, false, &snap.probes_degraded);
                            let _ = tx
                                .send(AppEvent::DiagnosticsUpdated {
                                    findings,
                                    health,
                                    report,
                                    probes_degraded: snap.probes_degraded,
                                })
                                .await;
                        }
                        Err(e) => {
                            let _ = tx.send(AppEvent::Error(e)).await;
                        }
                    }
                });
            }
            SideEffect::FetchServiceDetails { unit } => {
                let p = Arc::clone(providers);
                let tx = tx.clone();
                tracker.spawn(async move {
                    match p.services.details(&unit).await {
                        Ok(info) => {
                            let _ = tx.send(AppEvent::ServiceDetails(info)).await;
                        }
                        Err(e) => {
                            tracing::debug!("service details: {e}");
                        }
                    }
                });
            }
            SideEffect::StartFollow { unit } => {
                if let Some(c) = follow_cancel.take() {
                    c.cancel();
                }
                let token = CancellationToken::new();
                *follow_cancel = Some(token.clone());
                let tx = tx.clone();
                if state.demo {
                    let p = Arc::clone(providers);
                    tracker.spawn(async move {
                        while !token.is_cancelled() {
                            tokio::time::sleep(Duration::from_millis(500)).await;
                            if let Ok(entries) = p.logs.recent(unit.as_deref(), 1).await {
                                let _ = tx.send(AppEvent::LogsUpdated(entries)).await;
                            }
                        }
                    });
                } else {
                    let (log_tx, mut log_rx) = mpsc::channel(32);
                    let follow_token = token.clone();
                    tracker.spawn(async move {
                        let _ = server_tui::providers::linux::journal::follow_journal(
                            unit,
                            follow_token,
                            log_tx,
                        )
                        .await;
                    });
                    let tx2 = tx.clone();
                    let token2 = token.clone();
                    tracker.spawn(async move {
                        loop {
                            tokio::select! {
                                _ = token2.cancelled() => break,
                                msg = log_rx.recv() => {
                                    match msg {
                                        Some(entries) => {
                                            let _ = tx2.send(AppEvent::LogsUpdated(entries)).await;
                                        }
                                        None => break,
                                    }
                                }
                            }
                        }
                    });
                }
            }
            SideEffect::StopFollow => {
                if let Some(c) = follow_cancel.take() {
                    c.cancel();
                }
            }
            SideEffect::StartScan { path } => {
                if let Some(c) = scan_cancel.take() {
                    c.cancel();
                }
                let token = CancellationToken::new();
                *scan_cancel = Some(token.clone());
                let p = Arc::clone(providers);
                let tx_prog = tx.clone();
                let stay = state.stay_on_fs;
                let follow = config.follow_symlinks;
                let (prog_tx, mut prog_rx) = mpsc::channel(32);
                tracker.spawn(async move {
                    while let Some(prog) = prog_rx.recv().await {
                        let _ = tx_prog.send(AppEvent::StorageProgress(prog)).await;
                    }
                });
                let tx2 = tx.clone();
                let path = if path.as_os_str() == "~" {
                    expand_tilde("~")
                } else {
                    path
                };
                tracker.spawn(async move {
                    let result = p.storage.scan(path, stay, follow, token, prog_tx).await;
                    let _ = tx2.send(AppEvent::StorageFinished(result)).await;
                });
            }
            SideEffect::CancelScan => {
                if let Some(c) = scan_cancel.take() {
                    c.cancel();
                }
            }
            SideEffect::SendSignal {
                pid,
                signal,
                start_time,
            } => {
                let p = Arc::clone(providers);
                let tx = tx.clone();
                tracker.spawn(async move {
                    let result = match p.admin.send_signal(pid, signal, Some(start_time)).await {
                        Ok(()) => OperationResult::Success(format!(
                            "{} delivered to PID {pid}",
                            signal.label()
                        )),
                        Err(e) => OperationResult::Failure(e),
                    };
                    let _ = tx.send(AppEvent::OperationFinished(result)).await;
                });
            }
            SideEffect::ServiceAction { unit, action } => {
                let p = Arc::clone(providers);
                let tx = tx.clone();
                tracker.spawn(async move {
                    let result = match p.admin.service_action(&unit, action).await {
                        Ok(()) => {
                            OperationResult::Success(format!("{} {} ok", action.label(), unit))
                        }
                        Err(e) => OperationResult::Failure(e),
                    };
                    let _ = tx.send(AppEvent::OperationFinished(result)).await;
                });
            }
        }
    }
}
