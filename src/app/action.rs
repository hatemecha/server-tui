//! Typed application actions produced by input mapping.

use std::path::PathBuf;

use crate::model::{ProcessSignal, ServiceActionKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Dashboard,
    Processes,
    Services,
    Logs,
    Storage,
    Diagnostics,
    Settings,
}

impl Screen {
    pub fn from_digit(c: char) -> Option<Self> {
        match c {
            '1' => Some(Self::Dashboard),
            '2' => Some(Self::Processes),
            '3' => Some(Self::Services),
            '4' => Some(Self::Logs),
            '5' => Some(Self::Storage),
            '6' => Some(Self::Diagnostics),
            '7' => Some(Self::Settings),
            _ => None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Dashboard => "Dashboard",
            Self::Processes => "Processes",
            Self::Services => "Services",
            Self::Logs => "Logs",
            Self::Storage => "Storage",
            Self::Diagnostics => "Diagnostics",
            Self::Settings => "Settings",
        }
    }

    pub fn all() -> &'static [Screen] {
        &[
            Self::Dashboard,
            Self::Processes,
            Self::Services,
            Self::Logs,
            Self::Storage,
            Self::Diagnostics,
            Self::Settings,
        ]
    }

    /// Screens with a filterable list (Dashboard/Settings are overview-only).
    pub fn supports_search(self) -> bool {
        !matches!(self, Self::Dashboard | Self::Settings)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusPane {
    Nav,
    Content,
    Details,
}

/// Focused button in a Yes/Cancel confirmation dialog. Default is Cancel (safer).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ConfirmChoice {
    Yes,
    #[default]
    Cancel,
}

impl ConfirmChoice {
    pub fn toggle(self) -> Self {
        match self {
            Self::Yes => Self::Cancel,
            Self::Cancel => Self::Yes,
        }
    }
}

#[derive(Debug, Clone)]
pub enum AppAction {
    ChangeScreen(Screen),
    Refresh,
    MoveUp,
    MoveDown,
    PageUp,
    PageDown,
    Home,
    End,
    NextFocus,
    PrevFocus,
    Search,
    ClearSearch,
    SearchInput(char),
    SearchBackspace,
    ChangeSort,
    ToggleFullCommand,
    ToggleFailedOnly,
    ToggleFollow,
    NextMatch,
    PrevMatch,
    ChangePriority,
    ToggleWrap,
    ToggleApparentSize,
    ToggleStayOnFs,
    EnterDir,
    ParentDir,
    StartService,
    StopService,
    RestartService,
    ReloadService,
    EnableService,
    DisableService,
    OpenLogsForSelected,
    SignalTerm,
    SignalKill,
    StartStorageScan(Option<PathBuf>),
    CancelStorageScan,
    Confirm,
    Cancel,
    ConfirmFocusLeft,
    ConfirmFocusRight,
    ToggleHelp,
    ToggleGlossary,
    ToggleWallboard,
    OpenActionMenu,
    Inspect,
    CycleLogPreset,
    CycleStorageTab,
    DeepDiagnostics,
    ResetSettings,
    SettingsMove,
    OpenDiagnosticReport,
    AcknowledgeFinding,
    FollowDiagnosticTarget,
    SignalStop,
    SignalCont,
    ExportContext,
    ActivateMenu,
    MenuSelect,
    ToggleProcessTree,
    ToggleProcessFollow,
    CompleteOnboarding,
    SaveSettings,
    CycleTerminalProfile,
    CyclePerformanceProfile,
    ToggleConfigBool(&'static str),
    CleanupReportsNow,
    Quit,
    // Internal typed admin intents after confirmation.
    ExecuteSignal {
        pid: u32,
        signal: ProcessSignal,
        start_time: u64,
    },
    ExecuteService {
        unit: String,
        action: ServiceActionKind,
    },
}
