//! Provider traits — UI never talks to the OS directly.

use std::path::PathBuf;

use async_trait::async_trait;
use tokio_util::sync::CancellationToken;

use crate::error::AppError;
use crate::model::{
    DiagnosticSnapshot, LogEntry, LogPreset, ProcessDetails, ProcessInfo, ProcessSignal,
    ServiceActionKind, ServiceInfo, StorageProgress, StorageTree, SystemMetrics,
};

#[async_trait]
pub trait MetricsProvider: Send + Sync {
    async fn collect(&self) -> Result<SystemMetrics, AppError>;

    /// Update cache TTLs after a runtime profile change.
    fn update_refresh_policy(&self, _config: &crate::config::Config) {}
}

#[async_trait]
pub trait ProcessProvider: Send + Sync {
    async fn list(&self) -> Result<Vec<ProcessInfo>, AppError>;
    /// On-demand details (parent, cmdline, IO, cgroup, fds). List stays cheap.
    async fn details(&self, pid: u32) -> Result<ProcessDetails, AppError> {
        let _ = pid;
        Err(AppError::Unsupported(
            "process details not available".into(),
        ))
    }
}

#[async_trait]
pub trait ServiceProvider: Send + Sync {
    async fn list_services(&self) -> Result<Vec<ServiceInfo>, AppError>;
    /// On-demand unit details (e.g. fragment_path). List path stays cheap.
    async fn details(&self, unit: &str) -> Result<ServiceInfo, AppError>;
    async fn is_available(&self) -> bool;
}

#[async_trait]
pub trait LogProvider: Send + Sync {
    async fn recent(&self, unit: Option<&str>, lines: usize) -> Result<Vec<LogEntry>, AppError>;
    /// Preset-aware fetch (Important / boot / 1h / kernel). Default: ignore preset.
    async fn recent_preset(
        &self,
        unit: Option<&str>,
        lines: usize,
        preset: LogPreset,
    ) -> Result<Vec<LogEntry>, AppError> {
        let _ = preset;
        self.recent(unit, lines).await
    }
    async fn is_available(&self) -> bool;
}

#[async_trait]
pub trait StorageProvider: Send + Sync {
    async fn scan(
        &self,
        root: PathBuf,
        stay_on_fs: bool,
        follow_symlinks: bool,
        cancel: CancellationToken,
        progress: tokio::sync::mpsc::Sender<StorageProgress>,
    ) -> Result<StorageTree, AppError>;
}

/// Collects diagnostic probe data. Must not receive `AppState`.
#[async_trait]
pub trait DiagnosticProbeProvider: Send + Sync {
    /// `deep` enables expensive probes. `enable_smart` gates SMART even when deep.
    async fn probe(&self, deep: bool, enable_smart: bool) -> Result<DiagnosticSnapshot, AppError>;
}

#[async_trait]
pub trait AdministrativeExecutor: Send + Sync {
    async fn send_signal(
        &self,
        pid: u32,
        signal: ProcessSignal,
        expected_start_time: Option<u64>,
    ) -> Result<(), AppError>;
    async fn service_action(&self, unit: &str, action: ServiceActionKind) -> Result<(), AppError>;
    fn read_only(&self) -> bool;
    /// Shared known-unit allowlist, when this executor participates in unit actions.
    fn unit_registry(&self) -> Option<crate::model::UnitRegistry> {
        None
    }
    /// Known-unit registry membership for privileged unit actions (D-Bus and sudo).
    /// Empty registry means the list has not completed — fail closed.
    fn unit_known_for_action(&self, unit: &str) -> bool {
        match self.unit_registry() {
            Some(reg) => !reg.is_empty() && reg.contains(unit),
            None => false,
        }
    }
    /// True when failure is a D-Bus access denial that may recover via sudo.
    fn permission_may_sudo(&self, err: &AppError) -> bool {
        match err {
            AppError::Permission(msg) => {
                let m = msg.to_ascii_lowercase();
                m.contains("access denied")
                    || m.contains("permission denied")
                    || m.contains("interactive authentication required")
                    || m.contains("not allowed")
                    || m.contains("org.freedesktop.dbus.error.accessdenied")
            }
            _ => false,
        }
    }
}

/// Bundle of providers used by the application runtime.
pub struct ProviderBundle {
    pub metrics: Box<dyn MetricsProvider>,
    pub processes: Box<dyn ProcessProvider>,
    pub services: Box<dyn ServiceProvider>,
    pub logs: Box<dyn LogProvider>,
    pub storage: Box<dyn StorageProvider>,
    pub admin: Box<dyn AdministrativeExecutor>,
    pub diagnostics: Box<dyn DiagnosticProbeProvider>,
}
