//! Typed settings identifiers (no free-form config bool strings).

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SettingId {
    ConfirmSigterm,
    ConfirmSigkill,
    ConfirmServiceActions,
    EnableSmartProbes,
    DiagnosticsLightScan,
    Wallboard,
    Color,
}

impl SettingId {
    pub fn label(self) -> &'static str {
        match self {
            Self::ConfirmSigterm => "confirm_sigterm",
            Self::ConfirmSigkill => "confirm_sigkill",
            Self::ConfirmServiceActions => "confirm_service_actions",
            Self::EnableSmartProbes => "enable_smart_probes",
            Self::DiagnosticsLightScan => "diagnostics_light_scan",
            Self::Wallboard => "wallboard",
            Self::Color => "color",
        }
    }
}

/// Settings screen section labels (owned outside UI so app can navigate).
pub const SETTINGS_SECTIONS: &[&str] = &[
    "General",
    "Appearance",
    "Performance",
    "Dashboard",
    "Logs",
    "Diagnostics",
    "Safety",
    "Startup",
];
