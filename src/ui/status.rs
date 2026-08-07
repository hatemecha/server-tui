//! Toast / status message system (success, warning, error, progress).

use std::time::{Duration, Instant};

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::sanitize::sanitize_text;
use crate::ui::theme::Theme;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToastKind {
    Info,
    Success,
    Warning,
    Error,
    Progress,
}

impl ToastKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Info => "info",
            Self::Success => "ok",
            Self::Warning => "warn",
            Self::Error => "error",
            Self::Progress => "…",
        }
    }
}

#[derive(Debug, Clone)]
pub struct StatusToast {
    pub kind: ToastKind,
    pub message: String,
    pub created: Instant,
    /// Errors that should stick until replaced or dismissed.
    pub sticky: bool,
    pub ttl: Duration,
}

impl StatusToast {
    pub fn new(kind: ToastKind, message: impl Into<String>) -> Self {
        let sticky = matches!(kind, ToastKind::Error);
        let ttl = match kind {
            ToastKind::Error => Duration::from_secs(30),
            ToastKind::Warning => Duration::from_secs(8),
            ToastKind::Progress => Duration::from_secs(60),
            ToastKind::Success | ToastKind::Info => Duration::from_secs(4),
        };
        Self {
            kind,
            message: sanitize_text(&message.into()),
            created: Instant::now(),
            sticky,
            ttl,
        }
    }

    pub fn sticky_error(message: impl Into<String>) -> Self {
        let mut t = Self::new(ToastKind::Error, message);
        t.sticky = true;
        t.ttl = Duration::from_secs(120);
        t
    }

    pub fn expired(&self, now: Instant) -> bool {
        if self.sticky {
            return false;
        }
        now.duration_since(self.created) >= self.ttl
    }

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

pub fn expire_toast(toast: &mut Option<StatusToast>) {
    if let Some(t) = toast.as_ref() {
        if t.expired(Instant::now()) {
            *toast = None;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn success_expires_error_sticks() {
        let ok = StatusToast::new(ToastKind::Success, "done");
        assert!(!ok.sticky);
        let err = StatusToast::sticky_error("failed");
        assert!(err.sticky);
        assert!(!err.expired(Instant::now()));
    }

    #[test]
    fn sanitizes_message() {
        let t = StatusToast::new(ToastKind::Info, "hi\x1b[31m");
        assert!(!t.message.contains('\u{1b}'));
    }
}
