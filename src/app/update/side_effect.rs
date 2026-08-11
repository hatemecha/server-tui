//! Side effects requested by the reducer — executed by runtime.

use crate::model::{ProcessSignal, ServiceActionKind};

#[derive(Debug, Clone)]
pub enum SideEffect {
    RefreshMetrics,
    RefreshProcesses,
    RefreshServices,
    RefreshLogs {
        unit: Option<String>,
    },
    RefreshDiagnostics,
    FetchServiceDetails {
        unit: String,
    },
    FetchProcessDetails {
        pid: u32,
    },
    FetchPpidMap,
    PreviewFile {
        path: std::path::PathBuf,
    },
    StartFollow {
        unit: Option<String>,
    },
    StopFollow,
    StartScan {
        path: std::path::PathBuf,
    },
    CancelScan,
    SendSignal {
        pid: u32,
        signal: ProcessSignal,
        start_time: u64,
    },
    ServiceAction {
        unit: String,
        action: ServiceActionKind,
    },
    /// After D-Bus permission failure: restore TTY → sudo -v → re-enter → sudo -n systemctl.
    SudoServiceAction {
        unit: String,
        action: ServiceActionKind,
    },
    CleanupReports,
    /// Write a report body off the reducer path (runtime owns I/O).
    SaveReport {
        body: String,
        format: crate::support::SupportFormat,
    },
    /// Publish latest config to pollers (watch channel).
    PublishConfig,
}
