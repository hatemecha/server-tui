//! Color theme with terminal profile support (Auto / Modern / Tty16 / HighContrast / Mono).

use ratatui::style::{Color, Modifier, Style};

use crate::model::ActiveState;
pub use crate::profile::{PerformanceProfile, TerminalProfile};

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
        let color = if matches!(resolved, TerminalProfile::Monochrome) {
            false
        } else {
            color
                && !matches!(resolved, TerminalProfile::Monochrome)
                && (matches!(
                    resolved,
                    TerminalProfile::Modern
                        | TerminalProfile::Tty16
                        | TerminalProfile::HighContrast
                ) || color)
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

/// Style a systemd ActiveState string with exact enum semantics (inactive ≠ active).
pub fn status_style(theme: Theme, state: &str) -> Style {
    match ActiveState::parse(state) {
        ActiveState::Active => theme.ok(),
        ActiveState::Failed => theme.err(),
        ActiveState::Inactive => theme.muted(),
        ActiveState::Reloading | ActiveState::Activating | ActiveState::Deactivating => {
            theme.warn()
        }
        ActiveState::Maintenance | ActiveState::Unknown => {
            // Sub-state "running" alone is not a substitute for ActiveState.
            let lower = state.to_ascii_lowercase();
            if lower == "running" {
                theme.ok()
            } else {
                theme.normal()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inactive_is_not_painted_as_active() {
        let theme = Theme::new(true);
        // Critical regression: "inactive".contains("active") must NOT mean ok/green.
        let inactive = status_style(theme, "inactive");
        let active = status_style(theme, "active");
        let failed = status_style(theme, "failed");
        let dead = status_style(theme, "dead");
        assert_eq!(inactive, theme.muted());
        assert_eq!(active, theme.ok());
        assert_eq!(failed, theme.err());
        assert_eq!(dead, theme.muted());
        assert_ne!(inactive, active);
    }

    #[test]
    fn auto_monochrome_when_nocolor() {
        assert_eq!(
            TerminalProfile::Auto.resolve(false),
            TerminalProfile::Monochrome
        );
    }
}
