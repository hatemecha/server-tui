//! Side effects requested by the reducer — executed by runtime.

use crate::app::event::RequestId;
use crate::app::state::AppState;
use crate::model::{LogPreset, ProcessSignal, ServiceActionKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigSaveReason {
    Settings,
    Reset,
    Onboarding,
}

impl ConfigSaveReason {
    pub(crate) fn subject(self) -> &'static str {
        match self {
            Self::Settings => "settings",
            Self::Reset => "settings reset",
            Self::Onboarding => "onboarding completion",
        }
    }

    pub(crate) fn success_message(self) -> &'static str {
        match self {
            Self::Settings => "settings saved",
            Self::Reset => "settings reset to defaults",
            Self::Onboarding => "onboarding completed",
        }
    }
}

#[derive(Debug, Clone)]
pub enum SideEffect {
    RefreshMetrics,
    RefreshProcesses,
    RefreshServices,
    RefreshLogs {
        request_id: RequestId,
        unit: Option<String>,
        preset: LogPreset,
    },
    RefreshDiagnostics {
        request_id: RequestId,
        deep: bool,
        enable_smart: bool,
        previous_crc: std::collections::HashMap<String, u64>,
    },
    FetchServiceDetails {
        request_id: RequestId,
        unit: String,
    },
    FetchProcessDetails {
        request_id: RequestId,
        pid: u32,
    },
    FetchPpidMap {
        request_id: RequestId,
        pids: Vec<u32>,
        demo: bool,
    },
    PreviewFile {
        request_id: RequestId,
        path: std::path::PathBuf,
    },
    StartFollow {
        session_id: RequestId,
        unit: Option<String>,
    },
    StopFollow,
    StartScan {
        request_id: RequestId,
        path: std::path::PathBuf,
        stay_on_fs: bool,
        follow_symlinks: bool,
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
    SaveConfig {
        path: std::path::PathBuf,
        config: crate::config::Config,
        reason: ConfigSaveReason,
        harden_parent: bool,
    },
    SavePersist {
        path: std::path::PathBuf,
        persist: crate::persist::AppPersistState,
    },
    /// Publish a fully-derived config to pollers and provider caches.
    PublishRuntimeConfig(crate::config::Config),
}

pub(crate) fn begin_log_refresh(state: &mut AppState, unit: Option<String>) -> SideEffect {
    let request_id = state.next_request_id();
    state.log.refresh_request = Some((request_id, unit.clone(), state.log.preset));
    SideEffect::RefreshLogs {
        request_id,
        unit,
        preset: state.log.preset,
    }
}

pub(crate) fn begin_current_log_refresh(state: &mut AppState) -> SideEffect {
    begin_log_refresh(state, state.log.unit.clone())
}

pub(crate) fn begin_diagnostics(state: &mut AppState) -> SideEffect {
    let request_id = state.next_request_id();
    state.diagnostic.request = Some(request_id);
    state.diagnostic.running = true;
    SideEffect::RefreshDiagnostics {
        request_id,
        deep: state.diagnostic.deep || !state.config.diagnostics_light_scan,
        enable_smart: state.config.enable_smart_probes,
        previous_crc: state.persist.smart_crc_counts.clone(),
    }
}

pub(crate) fn begin_service_details(state: &mut AppState, unit: String) -> SideEffect {
    let request_id = state.next_request_id();
    state.service.details_request = Some((request_id, unit.clone()));
    SideEffect::FetchServiceDetails { request_id, unit }
}

pub(crate) fn begin_process_details(state: &mut AppState, pid: u32) -> SideEffect {
    let request_id = state.next_request_id();
    state.process.details_request = Some((request_id, pid));
    SideEffect::FetchProcessDetails { request_id, pid }
}

pub(crate) fn begin_ppid_map(state: &mut AppState) -> SideEffect {
    let request_id = state.next_request_id();
    state.process.ppid_request = Some(request_id);
    SideEffect::FetchPpidMap {
        request_id,
        pids: state.process.items.iter().map(|p| p.pid).collect(),
        demo: state.demo,
    }
}

pub(crate) fn begin_preview(state: &mut AppState, path: std::path::PathBuf) -> SideEffect {
    let request_id = state.next_request_id();
    state.storage.preview_request = Some((request_id, path.clone()));
    SideEffect::PreviewFile { request_id, path }
}

pub(crate) fn begin_scan(state: &mut AppState, path: std::path::PathBuf) -> SideEffect {
    let request_id = state.next_request_id();
    state.storage.scan_request = Some(request_id);
    SideEffect::StartScan {
        request_id,
        path,
        stay_on_fs: state.storage.stay_on_fs,
        follow_symlinks: state.config.follow_symlinks,
    }
}

pub(crate) fn begin_current_scan(state: &mut AppState) -> SideEffect {
    begin_scan(state, state.storage.scan_path.clone())
}

pub(crate) fn begin_follow(state: &mut AppState, unit: Option<String>) -> SideEffect {
    let session_id = state.next_request_id();
    state.log.follow_session = Some((session_id, unit.clone()));
    SideEffect::StartFollow { session_id, unit }
}

pub(crate) fn begin_current_follow(state: &mut AppState) -> SideEffect {
    begin_follow(state, state.log.unit.clone())
}
