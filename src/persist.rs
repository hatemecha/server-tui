//! Persistent XDG state (`$XDG_STATE_HOME/server-tui/state.toml`).

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::error::AppError;
use crate::CONFIG_DIR_NAME;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AppPersistState {
    pub last_seen_boot_id: Option<String>,
    pub last_diagnostic_at: Option<u64>,
    #[serde(default)]
    pub acknowledged_finding_ids: HashSet<String>,
    /// Last seen SMART UDMA CRC counters keyed by device path.
    #[serde(default)]
    pub smart_crc_counts: std::collections::HashMap<String, u64>,
}

impl AppPersistState {
    pub fn default_path() -> PathBuf {
        dirs::state_dir()
            .or_else(dirs::data_local_dir)
            .unwrap_or_else(|| PathBuf::from("."))
            .join(CONFIG_DIR_NAME)
            .join("state.toml")
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
        let state: AppPersistState = toml::from_str(&raw).map_err(|e| AppError::Configuration {
            path: path.clone(),
            message: e.to_string(),
        })?;
        Ok((state, path))
    }

    pub fn save_atomic(&self, path: &Path) -> Result<(), AppError> {
        let body = toml::to_string_pretty(self)
            .map_err(|e| AppError::Internal(format!("state serialize: {e}")))?;
        crate::fsutil::write_private_atomic(path, body.as_bytes())
    }

    pub fn touch_diagnostic_now(&mut self) {
        let secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        self.last_diagnostic_at = Some(secs);
    }

    pub fn is_acknowledged(&self, id: &str) -> bool {
        self.acknowledged_finding_ids.contains(id)
    }

    pub fn acknowledge(&mut self, id: &str) {
        self.acknowledged_finding_ids.insert(id.to_string());
    }

    pub fn remember_crc(&mut self, device: &str, count: u64) -> Option<u64> {
        self.smart_crc_counts.insert(device.to_string(), count)
    }

    pub fn previous_crc(&self, device: &str) -> Option<u64> {
        self.smart_crc_counts.get(device).copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_atomic() {
        let dir = tempfile::tempdir().expect("tmpdir");
        let path = dir.path().join("state.toml");
        let mut s = AppPersistState {
            last_seen_boot_id: Some("abc".into()),
            ..AppPersistState::default()
        };
        s.acknowledge("finding.oom");
        s.save_atomic(&path).expect("save");
        let (loaded, _) = AppPersistState::load_or_default(Some(&path)).expect("load");
        assert_eq!(loaded.last_seen_boot_id.as_deref(), Some("abc"));
        assert!(loaded.is_acknowledged("finding.oom"));
    }
}
