//! Diagnostics screen: findings list + details + deep links.

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph, Row, Scrollbar, ScrollbarState, Table, Wrap};

use crate::app::action::FocusPane;
use crate::app::state::AppState;
use crate::ui::selection::{marker_for, selected_row_style};
use crate::ui::theme::Theme;
use crate::ui::viewport::sync_table_state;

pub fn draw(frame: &mut Frame<'_>, area: Rect, state: &AppState) {
    let theme = Theme::new(state.color);
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(5),
            Constraint::Length(8),
        ])
        .split(area);

    let health_line = format!(
        "health: {}   systemd: {}   journal: {}   diagnostics: {}{}",
        state.health_status.label(),
        state.subsystem_health.systemd.label(),
        state.subsystem_health.journal.label(),
        state.subsystem_health.diagnostics.label(),
        if state.diagnostic_running {
            "   (running…)"
        } else {
            ""
        }
    );
    frame.render_widget(
        Paragraph::new(health_line).block(
            Block::default()
                .borders(Borders::ALL)
                .title("Host health")
                .border_style(theme.border()),
        ),
        chunks[0],
    );

    let filtered: Vec<&crate::model::Finding> = state.visible_findings();
    let visible = chunks[1].height.saturating_sub(3) as usize;
    let mut vp = state.finding_vp;
    vp.ensure_visible(filtered.len(), visible.max(1));

    let header = Row::new(vec!["", "SEV", "ID", "TITLE"]).style(theme.accent());
    let rows = filtered.iter().enumerate().map(|(i, f)| {
        let selected = i == vp.selected;
        let ack = if state.persist.is_acknowledged(&f.id) {
            " (ack)"
        } else {
            ""
        };
        let row = Row::new(vec![
            marker_for(selected).to_string(),
            f.severity.label().to_string(),
            f.id.clone(),
            format!("{}{ack}", f.title),
        ]);
        if selected {
            row.style(selected_row_style(theme))
        } else {
            row
        }
    });

    let border = if state.focus == FocusPane::Content {
        theme.accent()
    } else {
        theme.border()
    };
    let table = Table::new(
        rows,
        [
            Constraint::Length(1),
            Constraint::Length(9),
            Constraint::Length(28),
            Constraint::Min(20),
        ],
    )
    .header(header)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(format!(
                "Findings ({}) {}{}",
                filtered.len(),
                vp.position_label(filtered.len()),
                state.search_title_suffix()
            ))
            .border_style(border),
    );
    let mut table_state = vp.to_table_state();
    sync_table_state(&vp, &mut table_state);
    frame.render_stateful_widget(table, chunks[1], &mut table_state);

    if filtered.len() > visible && visible > 0 {
        let mut sb = ScrollbarState::new(filtered.len().saturating_sub(1)).position(vp.selected);
        frame.render_stateful_widget(
            Scrollbar::default()
                .orientation(ratatui::widgets::ScrollbarOrientation::VerticalRight)
                .begin_symbol(None)
                .end_symbol(None),
            chunks[1],
            &mut sb,
        );
    }

    let detail = if let Some(f) = filtered.get(vp.selected) {
        let targets = f
            .targets
            .iter()
            .map(|t| {
                format!(
                    "{:?}{}",
                    t.screen,
                    t.search
                        .as_ref()
                        .map(|s| format!(" /{s}"))
                        .unwrap_or_default()
                )
            })
            .collect::<Vec<_>>()
            .join(", ");
        let check = f
            .suggested_check
            .as_ref()
            .map(|c| {
                format!(
                    "\ncheck: {}{}",
                    c.description,
                    c.command_hint
                        .as_ref()
                        .map(|h| format!(" (hint: {h})"))
                        .unwrap_or_default()
                )
            })
            .unwrap_or_default();
        format!(
            "{}\n{}\nconfidence: {}  category: {}\nevidence: {}\ntargets: {targets}{check}",
            f.title,
            f.summary,
            f.confidence.label(),
            f.category.label(),
            f.evidence.summary
        )
    } else {
        "No findings. Press r to refresh diagnostics. Enter opens inspector · e exports · D deep."
            .into()
    };

    frame.render_widget(
        Paragraph::new(detail).wrap(Wrap { trim: false }).block(
            Block::default()
                .borders(Borders::ALL)
                .title("Details")
                .border_style(theme.border()),
        ),
        chunks[2],
    );
}
