//! Subsystem observation health (orthogonal to diagnostic findings).

use serde::{Deserialize, Serialize};

/// Whether a subsystem can be observed, independent of problem severity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SubsystemHealth {
    #[default]
    Unknown,
    Healthy,
    Degraded,
    Unavailable,
}

impl SubsystemHealth {
    pub fn label(self) -> &'static str {
        match self {
            Self::Unknown => "unknown",
            Self::Healthy => "healthy",
            Self::Degraded => "degraded",
            Self::Unavailable => "unavailable",
        }
    }

    pub fn is_observable(self) -> bool {
        matches!(self, Self::Healthy | Self::Degraded)
    }
}

#[derive(Debug, Clone, Default)]
pub struct SubsystemHealthMap {
    pub systemd: SubsystemHealth,
    pub journal: SubsystemHealth,
    pub diagnostics: SubsystemHealth,
}

impl SubsystemHealthMap {
    pub fn from_capabilities(systemd_available: bool, journal_available: bool) -> Self {
        Self {
            systemd: if systemd_available {
                SubsystemHealth::Healthy
            } else {
                SubsystemHealth::Unavailable
            },
            journal: if journal_available {
                SubsystemHealth::Healthy
            } else {
                SubsystemHealth::Unavailable
            },
            diagnostics: SubsystemHealth::Unknown,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capabilities_map() {
        let m = SubsystemHealthMap::from_capabilities(true, false);
        assert_eq!(m.systemd, SubsystemHealth::Healthy);
        assert_eq!(m.journal, SubsystemHealth::Unavailable);
    }
}
