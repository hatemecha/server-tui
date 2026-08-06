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
}

impl Screen {
    pub fn from_digit(c: char) -> Option<Self> {
        match c {
            '1' => Some(Self::Dashboard),
            '2' => Some(Self::Processes),
            '3' => Some(Self::Services),
            '4' => Some(Self::Logs),
            '5' => Some(Self::Storage),
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
        }
    }

    pub fn all() -> &'static [Screen] {
        &[
            Self::Dashboard,
            Self::Processes,
            Self::Services,
            Self::Logs,
            Self::Storage,
        ]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusPane {
    Nav,
    Content,
    Details,
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
    ToggleHelp,
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
