//! Events that enter the application loop.

use crossterm::event::KeyEvent;

use crate::error::AppError;
use crate::model::{
    Finding, HealthStatus, LogEntry, ProcessInfo, ServiceInfo, StorageProgress, StorageTree,
    SystemMetrics,
};

#[derive(Debug, Clone)]
pub enum OperationResult {
    Success(String),
    Failure(AppError),
}

#[derive(Debug)]
pub enum AppEvent {
    Tick,
    Input(KeyEvent),
    Resize(u16, u16),
    MetricsUpdated(SystemMetrics),
    ProcessesUpdated(Vec<ProcessInfo>),
    ServicesUpdated(Vec<ServiceInfo>),
    /// Refresh replaces the ring buffer.
    LogsReplaced(Vec<LogEntry>),
    /// Follow appends (dedupe by cursor/fingerprint when possible).
    LogsAppended(Vec<LogEntry>),
    StorageProgress(StorageProgress),
    StorageFinished(Result<StorageTree, AppError>),
    DiagnosticsUpdated {
        findings: Vec<Finding>,
        health: HealthStatus,
        report: String,
        probes_degraded: Vec<String>,
    },
    /// SMART CRC observations for single-writer persist merge.
    SmartCrcObservations(Vec<(String, u64)>),
    ServiceDetails(ServiceInfo),
    ProcessDetails(crate::model::ProcessDetails),
    FilePreviewReady(crate::model::FilePreview),
    PpidMap(Vec<(u32, Option<u32>)>),
    OperationFinished(OperationResult),
    /// Permission denial that may recover via explicit sudo confirmation.
    ElevationRequired {
        unit: String,
        action: crate::model::ServiceActionKind,
    },
    ReportSaved(std::path::PathBuf),
    Error(AppError),
    Quit,
}
