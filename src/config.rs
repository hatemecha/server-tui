//! Application configuration loaded from TOML.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::AppError;
use crate::CONFIG_DIR_NAME;

const MIN_REFRESH_MS: u64 = 200;
const MAX_REFRESH_MS: u64 = 60_000;
const MIN_HISTORY: usize = 10;
const MAX_HISTORY: usize = 300;
const MIN_LOG_ENTRIES: usize = 100;
const MAX_LOG_ENTRIES: usize = 50_000;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub refresh_ms: u64,
    pub process_refresh_ms: u64,
    pub service_refresh_ms: u64,
    pub disk_refresh_ms: u64,
    pub temperature_refresh_ms: u64,
    pub memory_refresh_ms: u64,
    pub default_scan_path: String,
    pub follow_symlinks: bool,
    pub stay_on_filesystem: bool,
    pub color: bool,
    pub unicode: bool,
    pub confirm_sigterm: bool,
    pub confirm_sigkill: bool,
    pub confirm_service_actions: bool,
    pub max_log_entries: usize,
    pub metric_history_size: usize,
    #[serde(default)]
    pub terminal_profile: crate::ui::theme::TerminalProfile,
    #[serde(default)]
    pub performance_profile: crate::ui::theme::PerformanceProfile,
    #[serde(default)]
    pub onboarding_completed: bool,
    #[serde(default = "default_true")]
    pub enable_smart_probes: bool,
    #[serde(default = "default_true")]
    pub diagnostics_light_scan: bool,
    #[serde(default)]
    pub wallboard_default: bool,
    /// Days to keep support reports under XDG_STATE; 0 = never auto-clean.
    #[serde(default = "default_report_days")]
    pub report_retention_days: u64,
    pub config_path_override: Option<String>,
}

fn default_true() -> bool {
    true
}

fn default_report_days() -> u64 {
    30
}

impl Default for Config {
    fn default() -> Self {
        Self {
            refresh_ms: 1000,
            process_refresh_ms: 2000,
            service_refresh_ms: 5000,
            disk_refresh_ms: 10_000,
            temperature_refresh_ms: 5000,
            memory_refresh_ms: 2000,
            default_scan_path: "~".to_string(),
            follow_symlinks: false,
            stay_on_filesystem: true,
            color: true,
            unicode: true,
            confirm_sigterm: true,
            confirm_sigkill: true,
            confirm_service_actions: true,
            max_log_entries: 5000,
            metric_history_size: 60,
            terminal_profile: crate::ui::theme::TerminalProfile::Auto,
            performance_profile: crate::ui::theme::PerformanceProfile::Auto,
            onboarding_completed: false,
            enable_smart_probes: true,
            diagnostics_light_scan: true,
            wallboard_default: false,
            report_retention_days: 30,
            config_path_override: None,
        }
    }
}

impl Config {
    pub fn default_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(CONFIG_DIR_NAME)
            .join("config.toml")
    }

    pub fn load_or_default(path: Option<&Path>) -> Result<(Self, PathBuf), AppError> {
        let path = path
            .map(Path::to_path_buf)
            .unwrap_or_else(Self::default_path);
        if !path.exists() {
            return Ok((Self::default(), path));
        }
        let raw = fs::read_to_string(&path).map_err(|e| AppError::Configuration {
            path: path.clone(),
            message: e.to_string(),
        })?;
        let mut cfg: Config = toml::from_str(&raw).map_err(|e| AppError::Configuration {
            path: path.clone(),
            message: e.to_string(),
        })?;
        cfg.clamp();
        Ok((cfg, path))
    }

    pub fn clamp(&mut self) {
        self.refresh_ms = self.refresh_ms.clamp(MIN_REFRESH_MS, MAX_REFRESH_MS);
        self.process_refresh_ms = self
            .process_refresh_ms
            .clamp(MIN_REFRESH_MS, MAX_REFRESH_MS);
        self.service_refresh_ms = self
            .service_refresh_ms
            .clamp(MIN_REFRESH_MS, MAX_REFRESH_MS);
        self.disk_refresh_ms = self.disk_refresh_ms.clamp(MIN_REFRESH_MS, MAX_REFRESH_MS);
        self.temperature_refresh_ms = self
            .temperature_refresh_ms
            .clamp(MIN_REFRESH_MS, MAX_REFRESH_MS);
        self.memory_refresh_ms = self.memory_refresh_ms.clamp(MIN_REFRESH_MS, MAX_REFRESH_MS);
        self.max_log_entries = self.max_log_entries.clamp(MIN_LOG_ENTRIES, MAX_LOG_ENTRIES);
        self.metric_history_size = self.metric_history_size.clamp(MIN_HISTORY, MAX_HISTORY);
    }

    pub fn expand_scan_path(&self) -> PathBuf {
        expand_tilde(&self.default_scan_path)
    }

    /// Atomic write: tmp + fsync + rename.
    pub fn save_atomic(&self, path: &Path) -> Result<(), AppError> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| AppError::Internal(e.to_string()))?;
        }
        let body = toml::to_string_pretty(self)
            .map_err(|e| AppError::Internal(format!("config serialize: {e}")))?;
        let tmp = path.with_extension("toml.tmp");
        {
            let mut f = fs::File::create(&tmp).map_err(|e| AppError::Internal(e.to_string()))?;
            f.write_all(body.as_bytes())
                .map_err(|e| AppError::Internal(e.to_string()))?;
            f.sync_all()
                .map_err(|e| AppError::Internal(e.to_string()))?;
        }
        fs::rename(&tmp, path).map_err(|e| AppError::Internal(e.to_string()))?;
        Ok(())
    }
}

pub fn expand_tilde(path: &str) -> PathBuf {
    if path == "~" {
        return dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    }
    if let Some(rest) = path.strip_prefix("~/") {
        return dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(rest);
    }
    PathBuf::from(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn loads_defaults_when_missing() {
        let (cfg, _) = Config::load_or_default(Some(Path::new("/tmp/server-tui-missing.toml")))
            .expect("defaults");
        assert_eq!(cfg.refresh_ms, 1000);
    }

    #[test]
    fn rejects_zero_refresh_via_clamp() {
        let mut cfg = Config {
            refresh_ms: 0,
            ..Config::default()
        };
        cfg.clamp();
        assert!(cfg.refresh_ms >= MIN_REFRESH_MS);
    }

    #[test]
    fn parses_valid_toml() {
        let dir = tempfile::tempdir().expect("tmpdir");
        let path = dir.path().join("config.toml");
        let mut f = fs::File::create(&path).expect("create");
        writeln!(f, "refresh_ms = 500\nmax_log_entries = 1000").expect("write");
        let (cfg, _) = Config::load_or_default(Some(&path)).expect("load");
        assert_eq!(cfg.refresh_ms, 500);
        assert_eq!(cfg.max_log_entries, 1000);
    }

    #[test]
    fn invalid_toml_reports_path() {
        let dir = tempfile::tempdir().expect("tmpdir");
        let path = dir.path().join("bad.toml");
        fs::write(&path, "refresh_ms = [oops]").expect("write");
        let err = Config::load_or_default(Some(&path)).unwrap_err();
        match err {
            AppError::Configuration { path: p, .. } => assert_eq!(p, path),
            other => panic!("unexpected {other:?}"),
        }
    }
}
