//! `server-tui doctor` one-shot diagnostics CLI.

use std::process::ExitCode;

use clap::Args;

use crate::config::Config;
use crate::diagnostics::{compute_health_status, evaluate, format_json_report, format_text_report};
use crate::model::HealthStatus;
use crate::providers::demo::DemoProviders;
use crate::providers::linux::linux_bundle;
use crate::APP_NAME;

#[derive(Debug, Clone, Args)]
pub struct DoctorArgs {
    /// Print a redacted text report (default when not --json).
    #[arg(long)]
    pub report: bool,

    /// Emit JSON report to stdout.
    #[arg(long)]
    pub json: bool,

    /// Include unredacted evidence details (may contain sensitive paths).
    #[arg(long)]
    pub include_sensitive: bool,

    /// Use synthetic demo probes (no host interaction).
    #[arg(long)]
    pub demo: bool,

    /// Read-only Linux providers (still observation-only for doctor).
    #[arg(long)]
    pub read_only: bool,
}

/// Exit codes: 0 Ok, 1 Warning, 2 Critical, 3 Unknown / probe failure.
pub async fn run_doctor(args: DoctorArgs) -> u8 {
    let config = Config::default();
    let providers = if args.demo {
        DemoProviders::bundle(true)
    } else {
        linux_bundle(true, &config)
    };

    let snap = match providers.diagnostics.probe().await {
        Ok(s) => s,
        Err(e) => {
            eprintln!("{APP_NAME} doctor: probe failed: {}", e.user_message());
            return 3;
        }
    };

    let findings = evaluate(&snap);
    let health = compute_health_status(&findings, snap.systemd_observable, snap.journal_observable);

    let include = args.include_sensitive;
    if args.json {
        match format_json_report(health, &findings, include, &snap.probes_degraded) {
            Ok(j) => println!("{j}"),
            Err(e) => {
                eprintln!("json encode failed: {e}");
                return 3;
            }
        }
    } else {
        print!(
            "{}",
            format_text_report(health, &findings, include, &snap.probes_degraded)
        );
    }

    match health {
        HealthStatus::Ok => 0,
        HealthStatus::Warning => 1,
        HealthStatus::Critical => 2,
        HealthStatus::Unknown => 3,
    }
}

pub fn doctor_exit_code(code: u8) -> ExitCode {
    ExitCode::from(code)
}
