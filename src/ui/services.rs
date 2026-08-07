use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph, Row, Table};

use crate::app::action::FocusPane;
use crate::app::state::AppState;
use crate::model::{filter_services, ServiceFilter};
use crate::ui::theme::{status_style, Theme};

pub fn draw(frame: &mut Frame<'_>, area: Rect, state: &AppState) {
    let theme = Theme::new(state.color);
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(5), Constraint::Length(7)])
        .split(area);

    let filter = if state.failed_only {
        ServiceFilter::Failed
    } else {
        state.service_filter
    };
    let filtered = filter_services(&state.services, state.current_search(), filter);

    let header = Row::new(vec![
        "UNIT",
        "LOAD",
        "ACTIVE",
        "SUB",
        "ENABLED",
        "DESCRIPTION",
    ])
    .style(theme.accent());

    let rows = filtered.iter().enumerate().map(|(i, s)| {
        let en = s.unit_file_state.short_label();
        let mut row = Row::new(vec![
            s.unit.clone(),
            s.load_state.clone(),
            s.active_state.clone(),
            s.sub_state.clone(),
            en.into(),
            s.description.clone(),
        ]);
        if i == state.service_selected {
            row = row.style(theme.highlight());
        } else {
            row = row.style(status_style(theme, &s.active_state));
        }
        row
    });

    let border = if state.focus == FocusPane::Content {
        theme.accent()
    } else {
        theme.border()
    };
    let readonly = if state.read_only { " [READ ONLY]" } else { "" };
    let table = Table::new(
        rows,
        [
            Constraint::Length(28),
            Constraint::Length(8),
            Constraint::Length(8),
            Constraint::Length(10),
            Constraint::Length(8),
            Constraint::Min(10),
        ],
    )
    .header(header)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(format!(
                "Services ({}) filter:{}{}{}",
                filtered.len(),
                filter.label(),
                state.search_title_suffix(),
                readonly
            ))
            .border_style(border),
    );
    frame.render_widget(table, chunks[0]);

    let detail = if let Some(s) = filtered.get(state.service_selected) {
        format!(
            "unit: {}\nactive: {} ({})  load: {}  unit-file: {}\nfragment: {}\n{}{}",
            s.unit,
            s.active_state,
            s.sub_state,
            s.load_state,
            s.unit_file_state.label(),
            s.fragment_path
                .as_deref()
                .unwrap_or("(press refresh / wait for details)"),
            s.description,
            if state.read_only {
                ""
            } else {
                "\nTip: confirmations default to Cancel — use arrows then Enter."
            }
        )
    } else if !state.metrics.systemd_available && !state.demo {
        "systemd: no disponible".into()
    } else {
        "No service selected".into()
    };
    frame.render_widget(
        Paragraph::new(detail).block(
            Block::default()
                .borders(Borders::ALL)
                .title("Details")
                .border_style(theme.border()),
        ),
        chunks[1],
    );
}
