//! Color theme with terminal profile support (Auto / Modern / Tty16 / HighContrast / Mono).

use ratatui::style::{Color, Modifier, Style};
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

    pub fn history_size(self, configured: usize) -> usize {
        match self.resolve() {
            Self::LowResource => configured.min(30),
            Self::Balanced | Self::Auto => configured,
            Self::Responsive => configured.max(60),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Theme {
    pub color: bool,
    pub profile: TerminalProfile,
}

impl Theme {
    pub fn new(color: bool) -> Self {
        Self::with_profile(color, TerminalProfile::Auto)
    }

    pub fn with_profile(color: bool, profile: TerminalProfile) -> Self {
        let resolved = profile.resolve(color);
        let color = color
            && !matches!(resolved, TerminalProfile::Monochrome)
            && (matches!(
                resolved,
                TerminalProfile::Modern | TerminalProfile::Tty16 | TerminalProfile::HighContrast
            ) || color);
        // Monochrome forces no color paint.
        let color = if matches!(resolved, TerminalProfile::Monochrome) {
            false
        } else {
            color
        };
        Self {
            color,
            profile: resolved,
        }
    }

    fn c(self, color: Color) -> Color {
        if !self.color {
            return Color::Reset;
        }
        match self.profile {
            TerminalProfile::Tty16 => simplify_16(color),
            TerminalProfile::HighContrast => high_contrast(color),
            TerminalProfile::Modern | TerminalProfile::Auto | TerminalProfile::Monochrome => color,
        }
    }

    pub fn border(self) -> Style {
        Style::default().fg(self.c(Color::DarkGray))
    }

    pub fn title(self) -> Style {
        Style::default()
            .fg(self.c(Color::Cyan))
            .add_modifier(Modifier::BOLD)
    }

    pub fn highlight(self) -> Style {
        if matches!(self.profile, TerminalProfile::HighContrast) {
            return Style::default()
                .fg(Color::Black)
                .bg(Color::White)
                .add_modifier(Modifier::BOLD);
        }
        Style::default()
            .bg(self.c(Color::DarkGray))
            .fg(self.c(Color::White))
            .add_modifier(Modifier::BOLD)
    }

    pub fn muted(self) -> Style {
        Style::default().fg(self.c(Color::DarkGray))
    }

    pub fn ok(self) -> Style {
        Style::default().fg(self.c(Color::Green))
    }

    pub fn warn(self) -> Style {
        Style::default().fg(self.c(Color::Yellow))
    }

    pub fn err(self) -> Style {
        Style::default().fg(self.c(Color::Red))
    }

    pub fn accent(self) -> Style {
        Style::default().fg(self.c(Color::Blue))
    }

    pub fn normal(self) -> Style {
        Style::default().fg(self.c(Color::White))
    }
}

fn simplify_16(color: Color) -> Color {
    match color {
        Color::Cyan | Color::LightCyan => Color::Cyan,
        Color::Blue | Color::LightBlue => Color::Blue,
        Color::Green | Color::LightGreen => Color::Green,
        Color::Yellow | Color::LightYellow => Color::Yellow,
        Color::Red | Color::LightRed => Color::Red,
        Color::Magenta | Color::LightMagenta => Color::Magenta,
        Color::White | Color::Gray => Color::White,
        Color::DarkGray | Color::Black => Color::DarkGray,
        other => other,
    }
}

fn high_contrast(color: Color) -> Color {
    match color {
        Color::DarkGray | Color::Gray => Color::White,
        Color::Blue | Color::Cyan => Color::LightCyan,
        Color::Green => Color::LightGreen,
        Color::Yellow => Color::LightYellow,
        Color::Red => Color::LightRed,
        other => other,
    }
}

pub fn status_style(theme: Theme, state: &str) -> Style {
    let lower = state.to_lowercase();
    if lower.contains("active") || lower.contains("running") {
        theme.ok()
    } else if lower.contains("fail") {
        theme.err()
    } else if lower.contains("inactive") || lower.contains("dead") {
        theme.muted()
    } else {
        theme.normal()
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

    #[test]
    fn auto_monochrome_when_nocolor() {
        assert_eq!(
            TerminalProfile::Auto.resolve(false),
            TerminalProfile::Monochrome
        );
    }
}
