use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph, Row, Scrollbar, ScrollbarState, Table};

use crate::app::action::FocusPane;
use crate::app::state::AppState;
use crate::model::{filter_services, ServiceFilter};
use crate::ui::selection::{marker_for, selected_row_style};
use crate::ui::theme::status_style;
use crate::ui::viewport::sync_table_state;

pub fn draw(frame: &mut Frame<'_>, area: Rect, state: &AppState) {
    let theme = state.theme();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(5), Constraint::Length(7)])
        .split(area);

    let filter = if state.service.failed_only {
        ServiceFilter::Failed
    } else {
        state.service.filter
    };
    let filtered = filter_services(&state.service.items, state.current_search(), filter);
    let visible = chunks[0].height.saturating_sub(3) as usize;
    let mut vp = state.service.vp;
    vp.ensure_visible(filtered.len(), visible.max(1));

    let header = Row::new(vec![
        "",
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
        let selected = i == vp.selected;
        let mut row = Row::new(vec![
            marker_for(selected).to_string(),
            s.unit.clone(),
            s.load_state.clone(),
            s.active_state.clone(),
            s.sub_state.clone(),
            en.into(),
            s.description.clone(),
        ]);
        if selected {
            row = row.style(selected_row_style(theme));
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
            Constraint::Length(1),
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
                "Services ({}) {} filter:{}{}{}",
                filtered.len(),
                vp.position_label(filtered.len()),
                filter.label(),
                state.search_title_suffix(),
                readonly
            ))
            .border_style(border),
    );
    let mut table_state = vp.to_table_state();
    sync_table_state(&vp, &mut table_state);
    frame.render_stateful_widget(table, chunks[0], &mut table_state);
    if filtered.len() > visible && visible > 0 {
        let mut sb = ScrollbarState::new(filtered.len().saturating_sub(1)).position(vp.selected);
        frame.render_stateful_widget(
            Scrollbar::default()
                .orientation(ratatui::widgets::ScrollbarOrientation::VerticalRight)
                .begin_symbol(None)
                .end_symbol(None),
            chunks[0],
            &mut sb,
        );
    }

    let detail = if let Some(s) = filtered.get(vp.selected) {
        format!(
            "unit: {}\nactive: {} ({})  load: {}  unit-file: {}\nfragment: {}\n{}{}",
            s.unit,
            s.active_state,
            s.sub_state,
            s.load_state,
            s.unit_file_state.label(),
            s.fragment_path
                .as_deref()
                .unwrap_or("(press Enter / wait for details)"),
            s.description,
            if state.read_only {
                ""
            } else {
                "\nKeys: s start · x stop · R restart · u reload · l logs"
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
