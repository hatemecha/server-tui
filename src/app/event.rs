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
    LogsUpdated(Vec<LogEntry>),
    StorageProgress(StorageProgress),
    StorageFinished(Result<StorageTree, AppError>),
    DiagnosticsUpdated {
        findings: Vec<Finding>,
        health: HealthStatus,
        report: String,
        probes_degraded: Vec<String>,
    },
    ServiceDetails(ServiceInfo),
    OperationFinished(OperationResult),
    Error(AppError),
    Quit,
}
