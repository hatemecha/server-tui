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
    /// True when the last failure was a D-Bus permission denial that may recover via sudo.
    fn permission_may_sudo(&self, err: &AppError) -> bool {
        matches!(err, AppError::Permission(_))
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
