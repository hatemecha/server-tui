//! Command-line interface.

use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};

use crate::doctor::DoctorArgs;
use crate::setup::{self, ConsoleInstallSpec};
use crate::support::SupportFormat;
use crate::ui::theme::{PerformanceProfile, TerminalProfile};
use crate::{APP_NAME, APP_VERSION};

#[derive(Debug, Clone, Parser)]
#[command(
    name = APP_NAME,
    version = APP_VERSION,
    about = "Interactive TUI for observing and administering a local Linux server",
    long_about = None
)]
pub struct Cli {
    /// Use synthetic providers; never touch the real system.
    #[arg(long)]
    pub demo: bool,

    /// Disable administrative actions (signals and systemd changes).
    #[arg(long)]
    pub read_only: bool,

    /// Wallboard mode: dashboard focus, less chrome, efficient refresh.
    #[arg(long)]
    pub wallboard: bool,

    /// Initial storage scan path (overrides config).
    #[arg(long, value_name = "PATH")]
    pub scan_path: Option<PathBuf>,

    /// Disable colors (also respects NO_COLOR).
    #[arg(long)]
    pub no_color: bool,

    /// Prefer ASCII box-drawing and sparklines.
    #[arg(long)]
    pub ascii: bool,

    /// Terminal color/style profile.
    #[arg(long, value_name = "PROFILE", value_parser = parse_terminal_profile)]
    pub terminal_profile: Option<TerminalProfile>,

    /// Refresh / history aggressiveness.
    #[arg(long, value_name = "PROFILE", value_parser = parse_performance_profile)]
    pub performance_profile: Option<PerformanceProfile>,

    /// Path to TOML configuration file.
    #[arg(long, value_name = "PATH")]
    pub config: Option<PathBuf>,

    /// Write internal tracing logs to this file (never to the TUI).
    #[arg(long, value_name = "PATH")]
    pub debug_log: Option<PathBuf>,

    /// Override global refresh interval in milliseconds (minimum 200).
    #[arg(long, value_name = "MS")]
    pub refresh_ms: Option<u64>,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

fn parse_terminal_profile(s: &str) -> Result<TerminalProfile, String> {
    TerminalProfile::parse_cli(s).ok_or_else(|| format!("unknown terminal profile: {s}"))
}

fn parse_performance_profile(s: &str) -> Result<PerformanceProfile, String> {
    PerformanceProfile::parse_cli(s).ok_or_else(|| format!("unknown performance profile: {s}"))
}

#[derive(Debug, Clone, Subcommand)]
pub enum Commands {
    /// Run one-shot host diagnostics and exit.
    Doctor(DoctorArgs),
    /// Generate a redacted support report.
    Support(SupportArgs),
    /// Host setup helpers (console unit generator).
    Setup {
        #[command(subcommand)]
        command: SetupCommands,
    },
}

#[derive(Debug, Clone, Parser)]
pub struct SupportArgs {
    #[arg(long, value_enum, default_value_t = SupportFormatCli::Text)]
    pub format: SupportFormatCli,
    #[arg(long, value_name = "PATH")]
    pub output: Option<PathBuf>,
    #[arg(long)]
    pub include_sensitive: bool,
    #[arg(long)]
    pub demo: bool,
}

#[derive(Debug, Clone, Copy, ValueEnum, Default)]
pub enum SupportFormatCli {
    #[default]
    Text,
    Markdown,
    Json,
}

impl From<SupportFormatCli> for SupportFormat {
    fn from(v: SupportFormatCli) -> Self {
        match v {
            SupportFormatCli::Text => SupportFormat::Text,
            SupportFormatCli::Markdown => SupportFormat::Markdown,
            SupportFormatCli::Json => SupportFormat::Json,
        }
    }
}

#[derive(Debug, Clone, Subcommand)]
pub enum SetupCommands {
    /// Generate / inspect startup console unit (no host writes unless --install).
    Console {
        #[arg(long)]
        status: bool,
        #[arg(long)]
        install: bool,
        #[arg(long)]
        remove: bool,
        #[arg(long, default_value = "tty2")]
        tty: String,
        #[arg(long)]
        user: Option<String>,
        #[arg(long, default_value_t = true)]
        wallboard: bool,
        #[arg(long, default_value_t = true)]
        read_only: bool,
        /// Print generated unit to stdout (safe; no host modification).
        #[arg(long)]
        print_unit: bool,
    },
}

impl Cli {
    pub fn color_enabled(&self, config_color: bool) -> bool {
        if self.no_color {
            return false;
        }
        if std::env::var_os("NO_COLOR").is_some() {
            return false;
        }
        config_color
    }
}

#[allow(clippy::too_many_arguments)]
pub fn run_setup_console(
    status: bool,
    install: bool,
    remove: bool,
    tty: String,
    user: Option<String>,
    wallboard: bool,
    read_only: bool,
    print_unit: bool,
) -> i32 {
    let mut spec = ConsoleInstallSpec {
        tty,
        wallboard,
        read_only,
        ..ConsoleInstallSpec::default()
    };
    if let Some(u) = user {
        spec.user = u;
    }

    if install || remove {
        eprintln!(
            "refusing to modify host systemd from this CLI path in development smoke.\n\
Use `--print-unit` to review the generated unit, then install manually after confirmation.\n\
Requested: install={install} remove={remove}"
        );
        eprintln!("{}", setup::status_text(&spec));
        return 2;
    }

    if print_unit {
        match setup::generate_unit(&spec) {
            Ok(body) => {
                print!("{body}");
                return 0;
            }
            Err(e) => {
                eprintln!("{}", e.user_message());
                return 1;
            }
        }
    }

    // status is default
    let _ = status;
    println!("{}", setup::status_text(&spec));
    0
}
