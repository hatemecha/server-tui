use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};

use crate::app::action::FocusPane;
use crate::app::state::AppState;
use crate::sanitize::truncate_width;
use crate::ui::theme::Theme;

pub fn draw(frame: &mut Frame<'_>, area: Rect, state: &AppState) {
    let theme = Theme::new(state.color);
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(5)])
        .split(area);

    let follow = if state.log_follow { "ON" } else { "OFF" };
    let unit = state.log_unit.as_deref().unwrap_or("(system)");
    let header = format!(
        "unit: {unit}  follow: {follow}  priority<={}  wrap: {}",
        state.log_min_priority.label(),
        if state.log_wrap { "on" } else { "off" }
    );
    frame.render_widget(
        Paragraph::new(header).block(
            Block::default()
                .borders(Borders::ALL)
                .title("Logs")
                .border_style(theme.border()),
        ),
        chunks[0],
    );

    let filtered = state
        .logs
        .filtered(state.current_search(), state.log_min_priority);
    let items: Vec<ListItem> = filtered
        .iter()
        .enumerate()
        .map(|(i, e)| {
            let pid = e.pid.map(|p| p.to_string()).unwrap_or_else(|| "-".into());
            let line = if state.log_wrap {
                format!(
                    "{} {:>5} {:<22} {:>5} {}",
                    e.timestamp,
                    e.priority.label(),
                    e.unit,
                    pid,
                    e.message
                )
            } else {
                let msg = truncate_width(&e.message, 120);
                let suffix = if msg.chars().count() < e.message.chars().count() {
                    "…"
                } else {
                    ""
                };
                format!(
                    "{} {:>5} {:<22} {:>5} {msg}{suffix}",
                    e.timestamp,
                    e.priority.label(),
                    e.unit,
                    pid
                )
            };
            let style = if i == state.log_selected {
                theme.highlight()
            } else if e.priority as u8 <= 3 {
                theme.err()
            } else if e.priority as u8 == 4 {
                theme.warn()
            } else {
                theme.normal()
            };
            ListItem::new(line).style(style)
        })
        .collect();

    let border = if state.focus == FocusPane::Content {
        theme.accent()
    } else {
        theme.border()
    };
    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(format!(
                "Entries ({}){}",
                filtered.len(),
                state.search_title_suffix()
            ))
            .border_style(border),
    );
    frame.render_widget(list, chunks[1]);
}
