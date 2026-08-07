//! Application state.

use std::path::PathBuf;

use crate::app::action::{ConfirmChoice, FocusPane, Screen};
use crate::config::Config;
use crate::error::AppError;
use crate::model::{
    finding_matches, visible_processes, Finding, HealthStatus, LogBuffer, LogPriority,
    MetricHistory, ProcessInfo, ProcessSort, ServiceActionKind, ServiceFilter, ServiceInfo,
    StorageNode, StorageProgress, StorageSort, StorageTree, SubsystemHealth, SubsystemHealthMap,
    SystemMetrics,
};
use crate::persist::AppPersistState;
use crate::ui::status::{expire_toast, StatusToast, ToastKind};
use crate::ui::theme::{PerformanceProfile, TerminalProfile};
use crate::ui::viewport::ViewportState;

#[derive(Debug, Clone)]
pub enum Dialog {
    Help,
    Glossary,
    ConfirmSignal {
        pid: u32,
        user: String,
        command: String,
        signal: crate::model::ProcessSignal,
        start_time: u64,
        choice: ConfirmChoice,
    },
    ConfirmService {
        unit: String,
        action: ServiceActionKind,
        choice: ConfirmChoice,
    },
    ConfirmResetSettings {
        choice: ConfirmChoice,
    },
    ConfirmCompleteOnboarding {
        choice: ConfirmChoice,
    },
    DiagnosticReport {
        body: String,
    },
    Message {
        title: String,
        body: String,
    },
    Inspector {
        title: String,
        body: String,
    },
    ActionMenu {
        title: String,
        items: Vec<crate::model::MenuItem>,
        selected: usize,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScanState {
    Idle,
    Running,
    Cancelled,
    Finished,
}

/// Per-screen search queries (not a single shared string), plus glossary overlay.
#[derive(Debug, Clone, Default)]
pub struct SearchQueries {
    pub processes: String,
    pub services: String,
    pub logs: String,
    pub storage: String,
    pub diagnostics: String,
    pub glossary: String,
}

impl SearchQueries {
    pub fn get(&self, screen: Screen) -> &str {
        match screen {
            Screen::Processes => self.processes.as_str(),
            Screen::Services => self.services.as_str(),
            Screen::Logs => self.logs.as_str(),
            Screen::Storage => self.storage.as_str(),
            Screen::Diagnostics => self.diagnostics.as_str(),
            Screen::Dashboard | Screen::Settings => "",
        }
    }

    pub fn get_mut(&mut self, screen: Screen) -> Option<&mut String> {
        match screen {
            Screen::Processes => Some(&mut self.processes),
            Screen::Services => Some(&mut self.services),
            Screen::Logs => Some(&mut self.logs),
            Screen::Storage => Some(&mut self.storage),
            Screen::Diagnostics => Some(&mut self.diagnostics),
            Screen::Dashboard | Screen::Settings => None,
        }
    }
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
    pub status_message: Option<String>, // legacy mirror of toast message
    pub last_error: Option<AppError>,
    pub dialog: Option<Dialog>,
    pub searching: bool,
    pub search: SearchQueries,

    pub metrics: SystemMetrics,
    pub history: MetricHistory,
    pub health_status: HealthStatus,
    pub subsystem_health: SubsystemHealthMap,

    pub processes: Vec<ProcessInfo>,
    pub process_sort: ProcessSort,
    pub process_vp: ViewportState,
    pub process_selected_pid: Option<u32>,
    pub process_follow_pid: Option<u32>,
    pub process_tree_mode: bool,
    pub show_full_cmd: bool,

    pub services: Vec<ServiceInfo>,
    pub service_filter: ServiceFilter,
    pub service_vp: ViewportState,
    pub service_selected_unit: Option<String>,
    pub failed_only: bool,

    pub logs: LogBuffer,
    pub log_unit: Option<String>,
    pub log_follow: bool,
    pub log_wrap: bool,
    pub log_min_priority: LogPriority,
    pub log_vp: ViewportState,
    pub log_match_idx: Option<usize>,
    pub log_preset: crate::model::LogPreset,

    pub scan_path: PathBuf,
    pub scan_state: ScanState,
    pub scan_progress: Option<StorageProgress>,
    pub storage_tree: Option<StorageTree>,
    pub storage_cwd: Vec<String>,
    pub storage_vp: ViewportState,
    pub storage_tab: crate::model::StorageTab,
    pub storage_sort: StorageSort,
    pub use_apparent: bool,
    pub stay_on_fs: bool,

    pub findings: Vec<Finding>,
    pub finding_vp: ViewportState,
    pub diagnostic_deep: bool,
    pub diagnostic_running: bool,
    pub diagnostic_report: Option<String>,
    pub persist: AppPersistState,
    pub persist_path: PathBuf,
    pub screen_before_overlay: Option<Screen>,
    pub glossary_vp: ViewportState,
    pub toast: Option<StatusToast>,
    pub terminal_profile: TerminalProfile,
    pub performance_profile: PerformanceProfile,
    pub wallboard: bool,
    pub settings_section: usize,
    pub settings_selected: usize,
    pub onboarding_pending: bool,
    pub activity: std::collections::VecDeque<String>,
    pub viewport_rows: usize,
    pub config_path: PathBuf,
    pub process_details: Option<crate::model::ProcessDetails>,
    pub process_ppids: Vec<(u32, Option<u32>)>,
    pub file_preview: Option<crate::model::FilePreview>,
    pub service_recent_logs: Vec<crate::model::LogEntry>,
    pub pending_service_inspect: bool,
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
        let (persist, persist_path) = AppPersistState::load_or_default(None)
            .unwrap_or_else(|_| (AppPersistState::default(), AppPersistState::default_path()));
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
            search: SearchQueries::default(),
            metrics: SystemMetrics::default(),
            history,
            health_status: HealthStatus::Unknown,
            subsystem_health: SubsystemHealthMap::default(),
            processes: Vec::new(),
            process_sort: ProcessSort::Cpu,
            process_vp: ViewportState::new(),
            process_selected_pid: None,
            process_follow_pid: None,
            process_tree_mode: false,
            show_full_cmd: false,
            services: Vec::new(),
            service_filter: ServiceFilter::All,
            service_vp: ViewportState::new(),
            service_selected_unit: None,
            failed_only: false,
            logs: LogBuffer::new(max_logs),
            log_unit: None,
            log_follow: false,
            log_wrap: false,
            log_min_priority: LogPriority::Debug,
            log_vp: ViewportState::new(),
            log_match_idx: None,
            log_preset: crate::model::LogPreset::default(),
            scan_path,
            scan_state: ScanState::Idle,
            scan_progress: None,
            storage_tree: None,
            storage_cwd: Vec::new(),
            storage_vp: ViewportState::new(),
            storage_tab: crate::model::StorageTab::default(),
            storage_sort: StorageSort::Size,
            use_apparent: false,
            stay_on_fs,
            findings: Vec::new(),
            finding_vp: ViewportState::new(),
            diagnostic_deep: false,
            diagnostic_running: false,
            diagnostic_report: None,
            persist,
            persist_path,
            screen_before_overlay: None,
            glossary_vp: ViewportState::new(),
            toast: None,
            terminal_profile: TerminalProfile::Auto,
            performance_profile: PerformanceProfile::Auto,
            wallboard: false,
            settings_section: 0,
            settings_selected: 0,
            onboarding_pending: false,
            activity: std::collections::VecDeque::with_capacity(100),
            viewport_rows: 20,
            config_path: Config::default_path(),
            process_details: None,
            process_ppids: Vec::new(),
            file_preview: None,
            service_recent_logs: Vec::new(),
            pending_service_inspect: false,
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

    pub fn compact_footer(&self) -> bool {
        self.width < 80 || self.height < 24
    }

    pub fn glossary_open(&self) -> bool {
        matches!(self.dialog, Some(Dialog::Glossary))
    }

    /// Whether `/` should enter a searchable input (list screens or glossary overlay).
    pub fn search_supported(&self) -> bool {
        self.glossary_open() || self.screen.supports_search()
    }

    pub fn current_search(&self) -> &str {
        if self.glossary_open() {
            self.search.glossary.as_str()
        } else {
            self.search.get(self.screen)
        }
    }

    pub fn current_search_mut(&mut self) -> Option<&mut String> {
        if matches!(self.dialog, Some(Dialog::Glossary)) {
            Some(&mut self.search.glossary)
        } else {
            let screen = self.screen;
            self.search.get_mut(screen)
        }
    }

    /// Short title suffix when a filter is active, e.g. `  /nginx`.
    pub fn search_title_suffix(&self) -> String {
        let q = self.current_search();
        if q.is_empty() {
            String::new()
        } else {
            format!("  /{q}")
        }
    }

    pub fn visible_processes(&self) -> Vec<&ProcessInfo> {
        visible_processes(&self.processes, self.current_search(), self.process_sort)
    }

    pub fn visible_findings(&self) -> Vec<&Finding> {
        let q = self.current_search();
        self.findings
            .iter()
            .filter(|f| finding_matches(f, q))
            .collect()
    }

    pub fn current_storage_node(&self) -> Option<&StorageNode> {
        let tree = self.storage_tree.as_ref()?;
        let mut node = &tree.root;
        for part in &self.storage_cwd {
            node = node.find_child(part)?;
        }
        Some(node)
    }

    pub fn visible_storage_children(&self) -> Vec<&StorageNode> {
        let Some(node) = self.current_storage_node() else {
            return Vec::new();
        };
        let q = self.current_search().to_lowercase();
        node.children
            .iter()
            .filter(|c| {
                if q.is_empty() {
                    true
                } else {
                    c.name.to_lowercase().contains(&q)
                        || c.path.to_string_lossy().to_lowercase().contains(&q)
                }
            })
            .collect()
    }

    pub fn process_selected(&self) -> usize {
        self.process_vp.selected
    }

    pub fn service_selected(&self) -> usize {
        self.service_vp.selected
    }

    pub fn log_selected(&self) -> usize {
        self.log_vp.selected
    }

    pub fn storage_selected(&self) -> usize {
        self.storage_vp.selected
    }

    pub fn finding_selected(&self) -> usize {
        self.finding_vp.selected
    }

    pub fn glossary_selected(&self) -> usize {
        self.glossary_vp.selected
    }

    pub fn push_activity(&mut self, msg: impl Into<String>) {
        let msg = crate::sanitize::sanitize_text(&msg.into());
        if self.activity.len() >= 100 {
            self.activity.pop_front();
        }
        self.activity.push_back(msg);
    }

    pub fn set_toast(&mut self, kind: ToastKind, msg: impl Into<String>) {
        let toast = StatusToast::new(kind, msg);
        self.status_message = Some(toast.message.clone());
        self.toast = Some(toast);
    }

    pub fn set_status(&mut self, msg: impl Into<String>) {
        self.set_toast(ToastKind::Info, msg);
    }

    pub fn set_success(&mut self, msg: impl Into<String>) {
        self.set_toast(ToastKind::Success, msg);
    }

    pub fn set_warning(&mut self, msg: impl Into<String>) {
        self.set_toast(ToastKind::Warning, msg);
    }

    pub fn set_error(&mut self, err: AppError) {
        tracing::error!("{}", err);
        let msg = err.user_message();
        self.status_message = Some(crate::sanitize::sanitize_text(&msg));
        self.toast = Some(StatusToast::sticky_error(&msg));
        self.last_error = Some(err);
    }

    pub fn tick_toasts(&mut self) {
        expire_toast(&mut self.toast);
        if self.toast.is_none() {
            // keep status_message as last info until cleared by next toast
        }
    }

    pub fn theme(&self) -> crate::ui::theme::Theme {
        crate::ui::theme::Theme::with_profile(self.color, self.terminal_profile)
    }

    pub fn refresh_subsystem_health_from_metrics(&mut self) {
        self.subsystem_health.systemd = if self.metrics.systemd_available {
            SubsystemHealth::Healthy
        } else {
            SubsystemHealth::Unavailable
        };
        self.subsystem_health.journal = if self.metrics.journal_available {
            SubsystemHealth::Healthy
        } else {
            SubsystemHealth::Unavailable
        };
    }

    pub fn save_persist(&self) {
        if let Err(e) = self.persist.save_atomic(&self.persist_path) {
            tracing::warn!("persist save failed: {e}");
        }
    }
}
