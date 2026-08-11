//! Typed settings catalog — sections, rows, and bool ids (no free-form config strings).

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

    pub fn title(self) -> &'static str {
        match self {
            Self::ConfirmSigterm => "Confirm before SIGTERM",
            Self::ConfirmSigkill => "Confirm before SIGKILL",
            Self::ConfirmServiceActions => "Confirm service actions",
            Self::EnableSmartProbes => "Enable SMART probes",
            Self::DiagnosticsLightScan => "Light diagnostics (skip SMART)",
            Self::Wallboard => "Wallboard by default",
            Self::Color => "Color",
        }
    }
}

/// Interactive row in the Settings detail pane.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingRow {
    Checkbox(SettingId),
    TerminalProfile,
    PerformanceProfile,
    LogPreset,
    Save,
    CompleteOnboarding,
    Reset,
    CleanupReports,
}

impl SettingRow {
    pub fn title(self) -> &'static str {
        match self {
            Self::Checkbox(id) => id.title(),
            Self::TerminalProfile => "Terminal profile",
            Self::PerformanceProfile => "Performance profile",
            Self::LogPreset => "Log preset",
            Self::Save => "Save settings",
            Self::CompleteOnboarding => "Complete onboarding",
            Self::Reset => "Reset to defaults",
            Self::CleanupReports => "Cleanup old reports",
        }
    }

    pub fn is_checkbox(self) -> bool {
        matches!(self, Self::Checkbox(_))
    }

    pub fn is_cycle(self) -> bool {
        matches!(
            self,
            Self::TerminalProfile | Self::PerformanceProfile | Self::LogPreset
        )
    }

    pub fn is_action(self) -> bool {
        matches!(
            self,
            Self::Save | Self::CompleteOnboarding | Self::Reset | Self::CleanupReports
        )
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

/// Interactive rows for a section (selection targets).
pub fn rows_for_section(section: usize) -> &'static [SettingRow] {
    match section {
        0 => &[
            SettingRow::Save,
            SettingRow::CompleteOnboarding,
            SettingRow::Reset,
        ],
        1 => &[
            SettingRow::TerminalProfile,
            SettingRow::Checkbox(SettingId::Color),
        ],
        2 => &[SettingRow::PerformanceProfile],
        3 => &[SettingRow::Checkbox(SettingId::Wallboard)],
        4 => &[SettingRow::LogPreset],
        5 => &[
            SettingRow::Checkbox(SettingId::DiagnosticsLightScan),
            SettingRow::Checkbox(SettingId::EnableSmartProbes),
        ],
        6 => &[
            SettingRow::Checkbox(SettingId::ConfirmSigterm),
            SettingRow::Checkbox(SettingId::ConfirmSigkill),
            SettingRow::Checkbox(SettingId::ConfirmServiceActions),
        ],
        7 => &[SettingRow::CleanupReports],
        _ => &[],
    }
}

/// Short helper text rendered under the interactive list.
pub fn section_footer(section: usize) -> &'static str {
    match section {
        0 => "Tab enters options · Space activates · S also saves.",
        1 => "←/→ cycle profile · Space toggles color. Applies to Theme immediately.",
        2 => "←/→ cycle profile. Low-resource slows refresh; Responsive speeds it.",
        3 => "Attention panel uses diagnostics findings when wallboard is on.",
        4 => "Preset applies when you open Logs. Report retention is in config.toml.",
        5 => "Light skips SMART. Deep scan (D on Diagnostics) enables expensive probes.",
        6 => "Safer defaults keep confirms on. Prefer --read-only for observation-only.",
        7 => "CLI: server-tui setup console --status | --print-unit",
        _ => "",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_section_has_rows_or_is_empty_only_out_of_range() {
        for i in 0..SETTINGS_SECTIONS.len() {
            assert!(
                !rows_for_section(i).is_empty(),
                "section {i} should expose at least one row"
            );
        }
        assert!(rows_for_section(99).is_empty());
    }
}
