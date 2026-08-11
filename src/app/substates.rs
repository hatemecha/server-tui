//! Per-screen AppState substates (lists + viewport + screen-local UI).

use std::path::PathBuf;

use crate::app::event::RequestId;
use crate::model::{
    Finding, LogBuffer, LogPriority, ProcessInfo, ProcessSort, ServiceFilter, ServiceInfo,
    StorageProgress, StorageSort, StorageTree,
};
use crate::viewport::ViewportState;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScanState {
    Idle,
    Running,
    Cancelled,
    Finished,
}

#[derive(Debug)]
pub struct ProcessState {
    pub items: Vec<ProcessInfo>,
    pub sort: ProcessSort,
    pub vp: ViewportState,
    pub selected_pid: Option<u32>,
    pub follow_pid: Option<u32>,
    pub tree_mode: bool,
    pub show_full_cmd: bool,
    pub details: Option<crate::model::ProcessDetails>,
    pub ppids: Vec<(u32, Option<u32>)>,
    pub details_request: Option<(RequestId, u32)>,
    pub ppid_request: Option<RequestId>,
}

impl ProcessState {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            sort: ProcessSort::Cpu,
            vp: ViewportState::new(),
            selected_pid: None,
            follow_pid: None,
            tree_mode: false,
            show_full_cmd: false,
            details: None,
            ppids: Vec::new(),
            details_request: None,
            ppid_request: None,
        }
    }
}

impl Default for ProcessState {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
pub struct ServiceState {
    pub items: Vec<ServiceInfo>,
    pub filter: ServiceFilter,
    pub vp: ViewportState,
    pub selected_unit: Option<String>,
    pub failed_only: bool,
    pub recent_logs: Vec<crate::model::LogEntry>,
    pub pending_inspect: bool,
    pub details_request: Option<(RequestId, String)>,
}

impl ServiceState {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            filter: ServiceFilter::All,
            vp: ViewportState::new(),
            selected_unit: None,
            failed_only: false,
            recent_logs: Vec::new(),
            pending_inspect: false,
            details_request: None,
        }
    }
}

impl Default for ServiceState {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
pub struct LogState {
    pub buffer: LogBuffer,
    pub unit: Option<String>,
    pub follow: bool,
    pub wrap: bool,
    pub min_priority: LogPriority,
    pub vp: ViewportState,
    pub match_idx: Option<usize>,
    pub preset: crate::model::LogPreset,
    pub refresh_request: Option<(RequestId, Option<String>, crate::model::LogPreset)>,
    pub follow_session: Option<(RequestId, Option<String>)>,
}

impl LogState {
    pub fn new(capacity: usize) -> Self {
        Self {
            buffer: LogBuffer::new(capacity),
            unit: None,
            follow: false,
            wrap: false,
            min_priority: LogPriority::Debug,
            vp: ViewportState::new(),
            match_idx: None,
            preset: crate::model::LogPreset::default(),
            refresh_request: None,
            follow_session: None,
        }
    }
}

#[derive(Debug)]
pub struct StorageState {
    pub scan_path: PathBuf,
    pub scan_state: ScanState,
    pub scan_progress: Option<StorageProgress>,
    pub tree: Option<StorageTree>,
    pub cwd: Vec<String>,
    pub vp: ViewportState,
    pub tab: crate::model::StorageTab,
    pub sort: StorageSort,
    pub use_apparent: bool,
    pub stay_on_fs: bool,
    pub file_preview: Option<crate::model::FilePreview>,
    pub scan_request: Option<RequestId>,
    pub preview_request: Option<(RequestId, PathBuf)>,
}

impl StorageState {
    pub fn new(scan_path: PathBuf, stay_on_fs: bool) -> Self {
        Self {
            scan_path,
            scan_state: ScanState::Idle,
            scan_progress: None,
            tree: None,
            cwd: Vec::new(),
            vp: ViewportState::new(),
            tab: crate::model::StorageTab::default(),
            sort: StorageSort::Size,
            use_apparent: false,
            stay_on_fs,
            file_preview: None,
            scan_request: None,
            preview_request: None,
        }
    }
}

#[derive(Debug)]
pub struct DiagnosticState {
    pub findings: Vec<Finding>,
    pub vp: ViewportState,
    pub deep: bool,
    pub running: bool,
    pub report: Option<String>,
    pub request: Option<RequestId>,
}

impl DiagnosticState {
    pub fn new() -> Self {
        Self {
            findings: Vec::new(),
            vp: ViewportState::new(),
            deep: false,
            running: false,
            report: None,
            request: None,
        }
    }
}

impl Default for DiagnosticState {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Default)]
pub struct SettingsState {
    pub section: usize,
    pub selected: usize,
    pub onboarding_pending: bool,
}
