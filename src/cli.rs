//! Command-line interface.

use std::path::PathBuf;

use clap::Parser;

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

    /// Initial storage scan path (overrides config).
    #[arg(long, value_name = "PATH")]
    pub scan_path: Option<PathBuf>,

    /// Disable colors (also respects NO_COLOR).
    #[arg(long)]
    pub no_color: bool,

    /// Prefer ASCII box-drawing and sparklines.
    #[arg(long)]
    pub ascii: bool,

    /// Path to TOML configuration file.
    #[arg(long, value_name = "PATH")]
    pub config: Option<PathBuf>,

    /// Write internal tracing logs to this file (never to the TUI).
    #[arg(long, value_name = "PATH")]
    pub debug_log: Option<PathBuf>,

    /// Override global refresh interval in milliseconds (minimum 200).
    #[arg(long, value_name = "MS")]
    pub refresh_ms: Option<u64>,
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
