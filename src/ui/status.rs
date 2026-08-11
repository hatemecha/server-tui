//! Toast / status rendering (model lives in `crate::status`).

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph};

pub use crate::status::{expire_toast, StatusToast, ToastKind};
use crate::ui::theme::Theme;

impl StatusToast {
    pub fn style(&self, theme: Theme) -> Style {
        match self.kind {
            ToastKind::Success => theme.ok(),
            ToastKind::Warning => theme.warn(),
            ToastKind::Error => theme.err(),
            ToastKind::Progress => theme.accent(),
            ToastKind::Info => theme.muted(),
        }
    }
}

pub fn render_toast_line(toast: &StatusToast, theme: Theme) -> Line<'static> {
    Line::from(vec![
        Span::styled(
            format!("[{}] ", toast.kind.label()),
            toast.style(theme).add_modifier(Modifier::BOLD),
        ),
        Span::styled(toast.message.clone(), toast.style(theme)),
    ])
}

pub fn toast_paragraph<'a>(toast: &'a StatusToast, theme: Theme) -> Paragraph<'a> {
    Paragraph::new(render_toast_line(toast, theme)).block(
        Block::default()
            .borders(Borders::NONE)
            .style(toast.style(theme)),
    )
}
