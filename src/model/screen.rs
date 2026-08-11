//! Navigation screens shared by app update and diagnostic deep-links.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
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

    /// Title-case chrome label.
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

    /// snake_case id used in finding search / report JSON (`serde` rename).
    pub fn slug(self) -> &'static str {
        match self {
            Self::Dashboard => "dashboard",
            Self::Processes => "processes",
            Self::Services => "services",
            Self::Logs => "logs",
            Self::Storage => "storage",
            Self::Diagnostics => "diagnostics",
            Self::Settings => "settings",
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slug_matches_serde_snake_case() {
        for s in Screen::all() {
            let json = serde_json::to_string(s).expect("ser");
            assert_eq!(json, format!("\"{}\"", s.slug()));
            let back: Screen = serde_json::from_str(&json).expect("de");
            assert_eq!(back, *s);
        }
    }
}
