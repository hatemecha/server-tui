use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph, Row, Table};

use crate::app::action::FocusPane;
use crate::app::state::AppState;
use crate::model::{format_bytes, format_uptime};
use crate::ui::theme::Theme;

pub fn draw(frame: &mut Frame<'_>, area: Rect, state: &AppState) {
    let theme = Theme::new(state.color);
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(5), Constraint::Length(6)])
        .split(area);

    let filtered = state.visible_processes();

    let header =
        Row::new(vec!["PID", "USER", "CPU%", "MEM%", "STATE", "COMMAND"]).style(theme.accent());
    let rows = filtered.iter().enumerate().map(|(i, p)| {
        let cmd = if state.show_full_cmd {
            p.cmd.as_str()
        } else {
            p.name.as_str()
        };
        let row = Row::new(vec![
            p.pid.to_string(),
            p.user.clone(),
            format!("{:.1}", p.cpu),
            format!("{:.1}", p.mem_pct),
            p.state.clone(),
            cmd.to_string(),
        ]);
        if i == state.process_selected {
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
            Constraint::Length(7),
            Constraint::Length(10),
            Constraint::Length(6),
            Constraint::Length(6),
            Constraint::Length(8),
            Constraint::Min(10),
        ],
    )
    .header(header)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(format!(
                "Processes ({}) sort:{}{}",
                filtered.len(),
                state.process_sort.label(),
                state.search_title_suffix()
            ))
            .border_style(border),
    );
    frame.render_widget(table, chunks[0]);

    let detail = if let Some(p) = filtered.get(state.process_selected) {
        format!(
            "PID {}  user {}  state {}\nCPU {:.1}%  MEM {:.1}% ({})\nruntime {}\ncmd: {}",
            p.pid,
            p.user,
            p.state,
            p.cpu,
            p.mem_pct,
            format_bytes(p.mem_bytes),
            format_uptime(p.run_time_secs),
            p.cmd
        )
    } else {
        "No process selected".into()
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
