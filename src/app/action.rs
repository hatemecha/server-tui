//! Typed application actions produced by input mapping.

use std::path::PathBuf;

use crate::model::{ProcessSignal, ServiceActionKind};
use crate::settings::SettingId;

pub use crate::model::Screen;

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
    /// Explicit affirmative confirmation, independent of focused button.
    ConfirmYes,
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
    ToggleSetting(SettingId),
    /// Space / Enter on the focused Settings row.
    ActivateFocusedSetting,
    /// ←/→ on a cycle row (or checkbox set off/on). Negative = left/prev.
    AdjustFocusedSetting(i8),
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
