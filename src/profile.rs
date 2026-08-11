//! Terminal and performance profiles — owned outside `ui/` so app/config can use them.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TerminalProfile {
    #[default]
    Auto,
    Modern,
    Tty16,
    HighContrast,
    Monochrome,
}

impl TerminalProfile {
    pub fn parse_cli(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "auto" => Some(Self::Auto),
            "modern" => Some(Self::Modern),
            "tty16" | "tty-16" | "16" => Some(Self::Tty16),
            "high-contrast" | "highcontrast" | "hc" => Some(Self::HighContrast),
            "monochrome" | "mono" | "bw" => Some(Self::Monochrome),
            _ => None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Modern => "modern",
            Self::Tty16 => "tty16",
            Self::HighContrast => "high-contrast",
            Self::Monochrome => "monochrome",
        }
    }

    pub fn next(self) -> Self {
        match self {
            Self::Auto => Self::Modern,
            Self::Modern => Self::Tty16,
            Self::Tty16 => Self::HighContrast,
            Self::HighContrast => Self::Monochrome,
            Self::Monochrome => Self::Auto,
        }
    }

    pub fn resolve(self, color_enabled: bool) -> Self {
        match self {
            Self::Auto => {
                if !color_enabled {
                    Self::Monochrome
                } else if std::env::var_os("TERM")
                    .map(|t| {
                        let t = t.to_string_lossy().to_ascii_lowercase();
                        t == "linux" || t == "vt100" || t == "ansi" || t.contains("16color")
                    })
                    .unwrap_or(false)
                {
                    Self::Tty16
                } else {
                    Self::Modern
                }
            }
            other => other,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PerformanceProfile {
    #[default]
    Auto,
    LowResource,
    Balanced,
    Responsive,
}

impl PerformanceProfile {
    pub fn parse_cli(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "auto" => Some(Self::Auto),
            "low-resource" | "low" | "slow" => Some(Self::LowResource),
            "balanced" => Some(Self::Balanced),
            "responsive" | "fast" => Some(Self::Responsive),
            _ => None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::LowResource => "low-resource",
            Self::Balanced => "balanced",
            Self::Responsive => "responsive",
        }
    }

    pub fn next(self) -> Self {
        match self {
            Self::Auto => Self::LowResource,
            Self::LowResource => Self::Balanced,
            Self::Balanced => Self::Responsive,
            Self::Responsive => Self::Auto,
        }
    }

    /// Suggested global refresh interval multiplier (1.0 = config default).
    pub fn refresh_scale(self) -> f64 {
        match self.resolve() {
            Self::LowResource => 2.5,
            Self::Balanced | Self::Auto => 1.0,
            Self::Responsive => 0.6,
        }
    }

    pub fn resolve(self) -> Self {
        match self {
            Self::Auto => Self::Balanced,
            other => other,
        }
    }

    pub fn history_size(self, base: usize) -> usize {
        match self.resolve() {
            Self::LowResource => (base / 2).max(10),
            Self::Balanced | Self::Auto => base,
            Self::Responsive => (base * 3 / 2).min(300),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_profiles() {
        assert_eq!(
            TerminalProfile::parse_cli("tty16"),
            Some(TerminalProfile::Tty16)
        );
        assert_eq!(
            PerformanceProfile::parse_cli("low-resource"),
            Some(PerformanceProfile::LowResource)
        );
    }
}
