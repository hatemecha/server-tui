use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};

use crate::app::action::FocusPane;
use crate::app::state::AppState;
use crate::sanitize::truncate_width;
use crate::ui::selection::{marker_for, selected_row_style};
use crate::ui::viewport::sync_list_state;

pub fn draw(frame: &mut Frame<'_>, area: Rect, state: &AppState) {
    let theme = state.theme();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(5)])
        .split(area);

    let follow = if state.log_follow { "ON" } else { "OFF" };
    let unit = state.log_unit.as_deref().unwrap_or("(system)");
    let header = format!(
        "preset:{}  unit:{unit}  follow:{follow}  priority<={}  wrap:{}  [ cycle preset",
        state.log_preset.label(),
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

    let filtered = state.logs.filtered_preset(
        state.current_search(),
        state.log_min_priority,
        state.log_preset,
        state.log_unit.as_deref(),
    );
    let visible = chunks[1].height.saturating_sub(2) as usize;
    let mut vp = state.log_vp;
    vp.ensure_visible(filtered.len(), visible.max(1));

    let items: Vec<ListItem> = filtered
        .iter()
        .enumerate()
        .map(|(i, e)| {
            let selected = i == vp.selected;
            let pid = e.pid.map(|p| p.to_string()).unwrap_or_else(|| "-".into());
            let msg = if state.log_wrap {
                e.message.clone()
            } else {
                truncate_width(&e.message, 120)
            };
            let line = format!(
                "{}{} {:>5} {:<18} {:>5} {msg}",
                marker_for(selected),
                e.timestamp,
                e.priority.label(),
                e.unit,
                pid
            );
            let style = if selected {
                selected_row_style(theme)
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
                "Entries {} {}",
                vp.position_label(filtered.len()),
                state.search_title_suffix()
            ))
            .border_style(border),
    );
    let mut ls = vp.to_list_state();
    sync_list_state(&vp, &mut ls);
    frame.render_stateful_widget(list, chunks[1], &mut ls);
}
