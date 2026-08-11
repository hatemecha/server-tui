//! Application entrypoint: parse CLI → bootstrap → run → exit.

use std::path::PathBuf;

use clap::Parser;
use tracing_subscriber::EnvFilter;

use server_tui::app::state::AppState;
use server_tui::cli::{Cli, Commands, SupportArgs};
use server_tui::config::Config;
use server_tui::diagnostics::{compute_health_status, evaluate};
use server_tui::doctor::run_doctor;
use server_tui::error::AppError;
use server_tui::providers::demo::DemoProviders;
use server_tui::providers::linux::linux_bundle;
use server_tui::runtime::run_loop;
use server_tui::support::{build_report, render, save_report};
use server_tui::terminal::TerminalGuard;
use server_tui::{APP_NAME, APP_VERSION};

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    match cli.command.clone() {
        Some(Commands::Doctor(args)) => {
            let code = run_doctor(args).await;
            std::process::exit(i32::from(code));
        }
        Some(Commands::Support(args)) => {
            let code = run_support_cli(args).await;
            std::process::exit(code);
        }
        Some(Commands::Setup { command }) => {
            let code = match command {
                server_tui::cli::SetupCommands::Console {
                    status,
                    install,
                    remove,
                    tty,
                    user,
                    wallboard,
                    read_only,
                    print_unit,
                } => server_tui::cli::run_setup_console(
                    status, install, remove, tty, user, wallboard, read_only, print_unit,
                ),
            };
            std::process::exit(code);
        }
        None => {}
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
    if let Some(profile) = cli.performance_profile {
        config.performance_profile = profile;
    }
    if let Some(profile) = cli.terminal_profile {
        config.terminal_profile = profile;
    }
    let effective_config = config.effective(config.performance_profile);

    let color = cli.color_enabled(config.color);
    let scan_path = cli
        .scan_path
        .clone()
        .unwrap_or_else(|| config.expand_scan_path());

    let providers = if cli.demo {
        DemoProviders::bundle(cli.read_only)
    } else {
        linux_bundle(cli.read_only, &effective_config)
    };

    let mut state = AppState::new(
        config.clone(),
        cli.demo,
        cli.read_only,
        color,
        cli.ascii || !config.unicode,
        scan_path,
        cfg_path.clone(),
    );
    state.config_dir_owned = cli.config.is_none();
    state.terminal_profile = config.terminal_profile.resolve(color);
    state.performance_profile = config.performance_profile;
    state.wallboard = cli.wallboard || config.wallboard_default;
    if state.wallboard {
        state.screen = server_tui::app::Screen::Dashboard;
        state.set_status("WALLBOARD: dashboard mode (w to toggle)");
    } else if !state.config.onboarding_completed {
        // Always land on Dashboard; Settings stays optional via digit 7.
        state.settings.onboarding_pending = true;
        state.screen = server_tui::app::Screen::Dashboard;
        state.set_status(
            "First run: Dashboard ready. Press 7 for Settings (optional) · Tab enters content · 1–7 switch screens.",
        );
    } else if state.demo {
        state.set_status("DEMO mode: synthetic data only; host is not modified.");
    } else if state.read_only {
        state.set_status("READ ONLY: signals and systemd changes are disabled.");
    } else {
        state.set_status(
            "Admin actions require confirmation. Prefer --read-only for observation-only.",
        );
    }

    let mut terminal = TerminalGuard::enter()?;
    let result = run_loop(&mut terminal, &mut state, providers, effective_config).await;
    terminal.restore()?;
    result
}

async fn run_support_cli(args: SupportArgs) -> i32 {
    let config = Config::default();
    let providers = if args.demo {
        DemoProviders::bundle(true)
    } else {
        linux_bundle(true, &config)
    };
    let metrics = providers.metrics.collect().await.unwrap_or_default();
    let processes = providers.processes.list().await.unwrap_or_default();
    let services = providers.services.list_services().await.unwrap_or_default();
    // CLI demo: fixed mixed dataset (same as doctor --demo). TUI keeps tick cycling.
    let snap = if args.demo {
        Some(DemoProviders::cli_diagnostic_snapshot())
    } else {
        providers.diagnostics.probe(true, true).await.ok()
    };
    let findings = snap.as_ref().map(evaluate).unwrap_or_default();
    let health = snap
        .as_ref()
        .map(|s| compute_health_status(&findings, s.systemd_observable, s.journal_observable))
        .unwrap_or(server_tui::model::HealthStatus::Unknown);
    let report = build_report(
        &metrics,
        &findings,
        &processes,
        &services,
        health,
        args.demo,
        true,
        args.include_sensitive,
    );
    let fmt = args.format.into();
    match render(&report, fmt) {
        Ok(body) => match save_report(&body, fmt, args.output.as_deref()) {
            Ok(path) => {
                println!("{}", server_tui::sanitize::sanitize_path_display(&path));
                0
            }
            Err(e) => {
                eprintln!("{}", e.user_message());
                1
            }
        },
        Err(e) => {
            eprintln!("{}", e.user_message());
            1
        }
    }
}

fn init_logging(path: Option<&PathBuf>) -> Result<(), AppError> {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    if let Some(path) = path {
        let file = server_tui::fsutil::open_private_append(path)
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
