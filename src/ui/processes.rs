use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph, Row, Scrollbar, ScrollbarState, Table};

use crate::app::action::FocusPane;
use crate::app::state::AppState;
use crate::model::{format_bytes, format_mem_pct, format_uptime};
use crate::ui::selection::{marker_for, selected_row_style};
use crate::ui::viewport::sync_table_state;

pub fn draw(frame: &mut Frame<'_>, area: Rect, state: &AppState) {
    let theme = state.theme();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(5), Constraint::Length(6)])
        .split(area);

    let filtered = state.visible_processes();
    let visible = chunks[0].height.saturating_sub(3) as usize;
    let mut vp = state.process.vp;

    let tree_rows = if state.process.tree_mode && !state.process.ppids.is_empty() {
        Some(crate::model::build_process_tree(
            &state.process.items,
            &state.process.ppids,
        ))
    } else {
        None
    };
    let row_count = tree_rows
        .as_ref()
        .map(|t| t.len())
        .unwrap_or(filtered.len());
    vp.ensure_visible(row_count, visible.max(1));

    let follow = state
        .process
        .follow_pid
        .map(|p| format!(" follow:{p}"))
        .unwrap_or_default();
    let tree = if state.process.tree_mode { " tree" } else { "" };

    let header =
        Row::new(vec!["", "PID", "USER", "CPU%", "MEM%", "STATE", "COMMAND"]).style(theme.accent());
    let rows: Vec<Row> = if let Some(tree) = &tree_rows {
        tree.iter()
            .enumerate()
            .map(|(i, (pid, name, depth))| {
                let selected = i == vp.selected;
                let info = state.process.items.iter().find(|p| p.pid == *pid);
                let indent = "  ".repeat(*depth);
                let row = Row::new(vec![
                    marker_for(selected).to_string(),
                    pid.to_string(),
                    info.map(|p| p.user.clone()).unwrap_or_else(|| "?".into()),
                    info.map(|p| format!("{:.1}", p.cpu))
                        .unwrap_or_else(|| "?".into()),
                    info.map(|p| format_mem_pct(p.mem_pct))
                        .unwrap_or_else(|| "?".into()),
                    info.map(|p| p.state.clone()).unwrap_or_else(|| "?".into()),
                    format!("{indent}{name}"),
                ]);
                if selected {
                    row.style(selected_row_style(theme))
                } else {
                    row
                }
            })
            .collect()
    } else {
        filtered
            .iter()
            .enumerate()
            .map(|(i, p)| {
                let cmd = if state.process.show_full_cmd {
                    p.cmd.as_str()
                } else {
                    p.name.as_str()
                };
                let selected = i == vp.selected;
                let row = Row::new(vec![
                    marker_for(selected).to_string(),
                    p.pid.to_string(),
                    p.user.clone(),
                    format!("{:.1}", p.cpu),
                    format_mem_pct(p.mem_pct),
                    p.state.clone(),
                    cmd.to_string(),
                ]);
                if selected {
                    row.style(selected_row_style(theme))
                } else {
                    row
                }
            })
            .collect()
    };

    let border = if state.focus == FocusPane::Content {
        theme.accent()
    } else {
        theme.border()
    };
    let title = format!(
        "Processes ({}) {}{}{} sort:{}{}",
        row_count,
        vp.position_label(row_count),
        follow,
        tree,
        state.process.sort.label(),
        state.search_title_suffix()
    );
    let table = Table::new(
        rows,
        [
            Constraint::Length(1),
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
            .title(title)
            .border_style(border),
    );

    let mut table_state = vp.to_table_state();
    sync_table_state(&vp, &mut table_state);
    frame.render_stateful_widget(table, chunks[0], &mut table_state);

    if row_count > visible && visible > 0 {
        let mut sb = ScrollbarState::new(row_count.saturating_sub(1)).position(vp.selected);
        frame.render_stateful_widget(
            Scrollbar::default()
                .orientation(ratatui::widgets::ScrollbarOrientation::VerticalRight)
                .begin_symbol(None)
                .end_symbol(None),
            chunks[0],
            &mut sb,
        );
    }

    let detail = if state.process.tree_mode && !state.process.ppids.is_empty() {
        let tree = crate::model::build_process_tree(&state.process.items, &state.process.ppids);
        if let Some((pid, name, _)) = tree.get(vp.selected) {
            if let Some(p) = state.process.items.iter().find(|p| p.pid == *pid) {
                format!(
                    "PID {}  user {}  state {}\nCPU {:.1}%  MEM {}% ({})\nruntime {}\ncmd: {}",
                    p.pid,
                    p.user,
                    p.state,
                    p.cpu,
                    format_mem_pct(p.mem_pct),
                    format_bytes(p.mem_bytes),
                    format_uptime(p.run_time_secs),
                    p.cmd
                )
            } else {
                format!("PID {pid} ({name})")
            }
        } else {
            "No process selected".into()
        }
    } else if let Some(p) = filtered.get(vp.selected) {
        format!(
            "PID {}  user {}  state {}\nCPU {:.1}%  MEM {}% ({})\nruntime {}\ncmd: {}",
            p.pid,
            p.user,
            p.state,
            p.cpu,
            format_mem_pct(p.mem_pct),
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
