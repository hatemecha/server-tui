//! Application state shell: navigation, overlays, and screen substates.

use std::path::PathBuf;

use crate::app::action::{ConfirmChoice, FocusPane, Screen};
use crate::app::substates::{
    DiagnosticState, LogState, ProcessState, ServiceState, SettingsState, StorageState,
};
use crate::config::Config;
use crate::error::AppError;
use crate::model::{
    finding_matches, visible_processes, Finding, HealthStatus, MetricHistory, ProcessInfo,
    ServiceActionKind, StorageNode, SubsystemHealth, SubsystemHealthMap, SystemMetrics,
};
use crate::persist::AppPersistState;
use crate::profile::{PerformanceProfile, TerminalProfile};
use crate::status::{expire_toast, StatusToast, ToastKind};
use crate::viewport::ViewportState;

pub use crate::app::substates::ScanState;

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
    /// Explicit elevation: administrator permission is required (never silent).
    ConfirmElevation {
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
    pub last_error: Option<AppError>,
    pub dialog: Option<Dialog>,
    pub searching: bool,
    pub search: SearchQueries,

    pub metrics: SystemMetrics,
    pub history: MetricHistory,
    pub health_status: HealthStatus,
    pub subsystem_health: SubsystemHealthMap,

    pub process: ProcessState,
    pub service: ServiceState,
    pub log: LogState,
    pub storage: StorageState,
    pub diagnostic: DiagnosticState,
    pub settings: SettingsState,

    pub persist: AppPersistState,
    pub persist_path: PathBuf,
    pub screen_before_overlay: Option<Screen>,
    pub glossary_vp: ViewportState,
    pub toast: Option<StatusToast>,
    pub terminal_profile: TerminalProfile,
    pub performance_profile: PerformanceProfile,
    pub wallboard: bool,
    pub activity: std::collections::VecDeque<String>,
    pub viewport_rows: usize,
    pub config_path: PathBuf,
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
            last_error: None,
            dialog: None,
            searching: false,
            search: SearchQueries::default(),
            metrics: SystemMetrics::default(),
            history,
            health_status: HealthStatus::Unknown,
            subsystem_health: SubsystemHealthMap::default(),
            process: ProcessState::new(),
            service: ServiceState::new(),
            log: LogState::new(max_logs),
            storage: StorageState::new(scan_path, stay_on_fs),
            diagnostic: DiagnosticState::new(),
            settings: SettingsState::default(),
            persist,
            persist_path,
            screen_before_overlay: None,
            glossary_vp: ViewportState::new(),
            toast: None,
            terminal_profile: TerminalProfile::Auto,
            performance_profile: PerformanceProfile::Auto,
            wallboard: false,
            activity: std::collections::VecDeque::with_capacity(100),
            viewport_rows: 20,
            config_path: Config::default_path(),
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

    pub fn search_title_suffix(&self) -> String {
        let q = self.current_search();
        if q.is_empty() {
            String::new()
        } else {
            format!("  /{q}")
        }
    }

    pub fn visible_processes(&self) -> Vec<&ProcessInfo> {
        visible_processes(
            &self.process.items,
            self.current_search(),
            self.process.sort,
        )
    }

    pub fn visible_findings(&self) -> Vec<&Finding> {
        let q = self.current_search();
        self.diagnostic
            .findings
            .iter()
            .filter(|f| finding_matches(f, q))
            .collect()
    }

    pub fn current_storage_node(&self) -> Option<&StorageNode> {
        let tree = self.storage.tree.as_ref()?;
        let mut node = &tree.root;
        for part in &self.storage.cwd {
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
        self.process.vp.selected
    }

    pub fn service_selected(&self) -> usize {
        self.service.vp.selected
    }

    pub fn log_selected(&self) -> usize {
        self.log.vp.selected
    }

    pub fn storage_selected(&self) -> usize {
        self.storage.vp.selected
    }

    pub fn finding_selected(&self) -> usize {
        self.diagnostic.vp.selected
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
        self.toast = Some(StatusToast::new(kind, msg));
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
        self.toast = Some(StatusToast::sticky_error(&msg));
        self.last_error = Some(err);
    }

    /// Chrome status line — single source of truth is `toast`.
    pub fn status_line(&self) -> &str {
        self.toast
            .as_ref()
            .map(|t| t.message.as_str())
            .unwrap_or("ready")
    }

    pub fn tick_toasts(&mut self) {
        expire_toast(&mut self.toast);
    }

    pub fn theme(&self) -> crate::theme::Theme {
        crate::theme::Theme::with_profile(self.color, self.terminal_profile)
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

#[cfg(test)]
mod status_line_tests {
    use super::*;
    use crate::config::Config;
    use crate::status::ToastKind;
    use std::path::PathBuf;

    #[test]
    fn status_line_follows_toast_only() {
        let mut state = AppState::new(
            Config::default(),
            true,
            true,
            true,
            false,
            PathBuf::from("/tmp"),
        );
        assert_eq!(state.status_line(), "ready");
        state.set_status("hello");
        assert_eq!(state.status_line(), "hello");
        state.set_toast(ToastKind::Success, "saved");
        assert_eq!(state.status_line(), "saved");
        state.toast = None;
        assert_eq!(state.status_line(), "ready");
    }
}
