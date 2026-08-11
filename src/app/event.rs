//! Events that enter the application loop.

use crossterm::event::KeyEvent;

use crate::error::AppError;
use crate::model::{
    Finding, HealthStatus, LogEntry, LogPreset, ProcessInfo, ServiceInfo, StorageProgress,
    StorageTree, SystemMetrics,
};

/// Session-local freshness identity for replaceable asynchronous work.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RequestId(u64);

impl RequestId {
    pub(crate) fn new(value: u64) -> Self {
        Self(value)
    }
}

#[derive(Debug)]
pub struct DiagnosticOutcome {
    pub findings: Vec<Finding>,
    pub health: HealthStatus,
    pub report: String,
    pub probes_degraded: Vec<String>,
    pub smart_crc_observations: Vec<(String, u64)>,
}

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
    LogsRefreshFinished {
        request_id: RequestId,
        unit: Option<String>,
        preset: LogPreset,
        result: Result<Vec<LogEntry>, AppError>,
    },
    LogsAppended {
        session_id: RequestId,
        entries: Vec<LogEntry>,
    },
    StorageProgress {
        request_id: RequestId,
        progress: StorageProgress,
    },
    StorageFinished {
        request_id: RequestId,
        result: Result<StorageTree, AppError>,
    },
    DiagnosticsFinished {
        request_id: RequestId,
        result: Result<DiagnosticOutcome, AppError>,
    },
    ServiceDetailsFinished {
        request_id: RequestId,
        unit: String,
        result: Result<ServiceInfo, AppError>,
    },
    ProcessDetailsFinished {
        request_id: RequestId,
        pid: u32,
        result: Result<crate::model::ProcessDetails, AppError>,
    },
    FilePreviewFinished {
        request_id: RequestId,
        path: std::path::PathBuf,
        result: Result<crate::model::FilePreview, AppError>,
    },
    PpidMapFinished {
        request_id: RequestId,
        result: Result<Vec<(u32, Option<u32>)>, AppError>,
    },
    OperationFinished(OperationResult),
    /// Permission denial that may recover via explicit sudo confirmation.
    ElevationRequired {
        unit: String,
        action: crate::model::ServiceActionKind,
    },
    ReportSaved(std::path::PathBuf),
    ConfigSaveFinished {
        path: std::path::PathBuf,
        reason: crate::app::update::ConfigSaveReason,
        result: Result<(), AppError>,
    },
    Error(AppError),
    Quit,
}
