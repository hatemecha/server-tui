//! Diagnostics screen: findings list + details + deep links.

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph, Row, Table, Wrap};

use crate::app::action::FocusPane;
use crate::app::state::AppState;
use crate::ui::theme::Theme;

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

    let filtered: Vec<(usize, &crate::model::Finding)> = state
        .findings
        .iter()
        .enumerate()
        .filter(|(_, f)| crate::model::finding_matches(f, state.current_search()))
        .collect();

    let header = Row::new(vec!["SEV", "ID", "TITLE"]).style(theme.accent());
    let rows = filtered.iter().enumerate().map(|(i, (_, f))| {
        let ack = if state.persist.is_acknowledged(&f.id) {
            " (ack)"
        } else {
            ""
        };
        let row = Row::new(vec![
            f.severity.label().to_string(),
            f.id.clone(),
            format!("{}{ack}", f.title),
        ]);
        if i == state.finding_selected.min(filtered.len().saturating_sub(1)) {
            row.style(theme.highlight())
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
                "Findings ({}){}",
                filtered.len(),
                state.search_title_suffix()
            ))
            .border_style(border),
    );
    frame.render_widget(table, chunks[1]);

    let detail_idx = if filtered.is_empty() {
        0
    } else {
        state.finding_selected.min(filtered.len() - 1)
    };
    let detail = if let Some((_, f)) = filtered.get(detail_idx) {
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
        "No findings. Press r to refresh diagnostics.".into()
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
