//! Shared UI widgets.

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Gauge, Paragraph, Sparkline};

use crate::app::state::AppState;
use crate::ui::theme::Theme;

pub fn gauge<'a>(theme: Theme, title: &'a str, ratio: f64, label: &'a str) -> Gauge<'a> {
    let ratio = ratio.clamp(0.0, 1.0);
    let style = if ratio > 0.9 {
        theme.err()
    } else if ratio > 0.75 {
        theme.warn()
    } else {
        theme.ok()
    };
    Gauge::default()
        .block(Block::default().borders(Borders::ALL).title(title))
        .gauge_style(style)
        .ratio(ratio)
        .label(label.to_string())
}

pub fn sparkline<'a>(theme: Theme, title: &'a str, data: &'a [u64], _ascii: bool) -> Sparkline<'a> {
    Sparkline::default()
        .block(Block::default().borders(Borders::ALL).title(title))
        .style(theme.accent())
        .data(data)
}

pub fn footer_hints(state: &AppState) -> String {
    if state.searching {
        return format!("search: {}_  Esc clear", state.search_query);
    }
    let base = "Tab focus · / search · r refresh · ? help · q quit";
    let extra = match state.screen {
        crate::app::Screen::Processes => " · t SIGTERM · K SIGKILL · s sort · c cmd",
        crate::app::Screen::Services => " · s start · x stop · r restart · l logs · f failed",
        crate::app::Screen::Logs => " · f follow · n/N match · p priority · w wrap",
        crate::app::Screen::Storage => " · Enter open · Backspace up · s sort · Esc cancel",
        crate::app::Screen::Dashboard => "",
    };
    format!("{base}{extra}")
}

pub fn keyed_line(theme: Theme, label: &str, value: &str) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("{label}: "), theme.muted()),
        Span::styled(value.to_string(), theme.normal()),
    ])
}

pub fn empty_panel(theme: Theme, msg: &str) -> Paragraph<'_> {
    Paragraph::new(msg).style(theme.muted()).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(theme.border()),
    )
}
