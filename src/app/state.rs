//! Application state.

use std::path::PathBuf;

use crate::app::action::{FocusPane, Screen};
use crate::config::Config;
use crate::error::AppError;
use crate::model::{
    LogBuffer, LogPriority, MetricHistory, ProcessInfo, ProcessSignal, ProcessSort,
    ServiceActionKind, ServiceFilter, ServiceInfo, StorageNode, StorageProgress, StorageSort,
    StorageTree, SystemMetrics,
};

#[derive(Debug, Clone)]
pub enum Dialog {
    Help,
    ConfirmSignal {
        pid: u32,
        user: String,
        command: String,
        signal: ProcessSignal,
        start_time: u64,
    },
    ConfirmService {
        unit: String,
        action: ServiceActionKind,
    },
    Message {
        title: String,
        body: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScanState {
    Idle,
    Running,
    Cancelled,
    Finished,
}

pub struct AppState {
    pub screen: Screen,
    pub focus: FocusPane,
    pub config: Config,
    pub demo: bool,
    pub read_only: bool,
    pub color: bool,
    pub ascii: bool,
    pub width: u16,
    pub height: u16,
    pub should_quit: bool,
    pub status_message: Option<String>,
    pub last_error: Option<AppError>,
    pub dialog: Option<Dialog>,
    pub searching: bool,
    pub search_query: String,

    pub metrics: SystemMetrics,
    pub history: MetricHistory,

    pub processes: Vec<ProcessInfo>,
    pub process_sort: ProcessSort,
    pub process_selected: usize,
    pub process_selected_pid: Option<u32>,
    pub show_full_cmd: bool,

    pub services: Vec<ServiceInfo>,
    pub service_filter: ServiceFilter,
    pub service_selected: usize,
    pub service_selected_unit: Option<String>,
    pub failed_only: bool,

    pub logs: LogBuffer,
    pub log_unit: Option<String>,
    pub log_follow: bool,
    pub log_wrap: bool,
    pub log_min_priority: LogPriority,
    pub log_selected: usize,
    pub log_match_idx: Option<usize>,

    pub scan_path: PathBuf,
    pub scan_state: ScanState,
    pub scan_progress: Option<StorageProgress>,
    pub storage_tree: Option<StorageTree>,
    pub storage_cwd: Vec<String>, // breadcrumbs under root
    pub storage_selected: usize,
    pub storage_sort: StorageSort,
    pub use_apparent: bool,
    pub stay_on_fs: bool,
}

impl AppState {
    pub fn new(
        config: Config,
        demo: bool,
        read_only: bool,
        color: bool,
        ascii: bool,
        scan_path: PathBuf,
    ) -> Self {
        let history = MetricHistory::new(config.metric_history_size);
        let max_logs = config.max_log_entries;
        let stay_on_fs = config.stay_on_filesystem;
        Self {
            screen: Screen::Dashboard,
            focus: FocusPane::Content,
            config,
            demo,
            read_only,
            color,
            ascii,
            width: 80,
            height: 24,
            should_quit: false,
            status_message: None,
            last_error: None,
            dialog: None,
            searching: false,
            search_query: String::new(),
            metrics: SystemMetrics::default(),
            history,
            processes: Vec::new(),
            process_sort: ProcessSort::Cpu,
            process_selected: 0,
            process_selected_pid: None,
            show_full_cmd: false,
            services: Vec::new(),
            service_filter: ServiceFilter::All,
            service_selected: 0,
            service_selected_unit: None,
            failed_only: false,
            logs: LogBuffer::new(max_logs),
            log_unit: None,
            log_follow: false,
            log_wrap: false,
            log_min_priority: LogPriority::Debug,
            log_selected: 0,
            log_match_idx: None,
            scan_path,
            scan_state: ScanState::Idle,
            scan_progress: None,
            storage_tree: None,
            storage_cwd: Vec::new(),
            storage_selected: 0,
            storage_sort: StorageSort::Size,
            use_apparent: false,
            stay_on_fs,
        }
    }

    pub fn mode_label(&self) -> &'static str {
        if self.demo {
            "DEMO"
        } else if self.read_only {
            "READ ONLY"
        } else {
            "normal"
        }
    }

    pub fn too_small(&self) -> bool {
        self.width < 60 || self.height < 12
    }

    pub fn narrow_nav(&self) -> bool {
        self.width < 100
    }

    pub fn current_storage_node(&self) -> Option<&StorageNode> {
        let tree = self.storage_tree.as_ref()?;
        let mut node = &tree.root;
        for part in &self.storage_cwd {
            node = node.find_child(part)?;
        }
        Some(node)
    }

    pub fn set_status(&mut self, msg: impl Into<String>) {
        self.status_message = Some(crate::sanitize::sanitize_text(&msg.into()));
    }

    pub fn set_error(&mut self, err: AppError) {
        tracing::error!("{}", err);
        self.status_message = Some(crate::sanitize::sanitize_text(&err.user_message()));
        self.last_error = Some(err);
    }
}
