//! Color theme with NO_COLOR / --no-color support.

use ratatui::style::{Color, Modifier, Style};

#[derive(Debug, Clone, Copy)]
pub struct Theme {
    pub color: bool,
}

impl Theme {
    pub fn new(color: bool) -> Self {
        Self { color }
    }

    fn c(self, color: Color) -> Color {
        if self.color {
            color
        } else {
            Color::Reset
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
