use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph, Row, Scrollbar, ScrollbarState, Table};

use crate::app::action::FocusPane;
use crate::app::state::{AppState, ScanState};
use crate::model::{format_bytes, format_size, StorageTab};
use crate::sanitize::sanitize_path_display;
use crate::ui::selection::{marker_for, selected_row_style};
use crate::ui::viewport::sync_table_state;

pub fn draw(frame: &mut Frame<'_>, area: Rect, state: &AppState) {
    let theme = state.theme();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(5), Constraint::Min(5)])
        .split(area);

    let status = match state.scan_state {
        ScanState::Idle => "idle — press r to scan".into(),
        ScanState::Running => {
            if let Some(p) = &state.scan_progress {
                format!(
                    "scanning {}  files:{} dirs:{} bytes:{} errors:{}  Esc cancel",
                    p.current.display(),
                    p.files,
                    p.dirs,
                    format_size(p.bytes),
                    p.errors
                )
            } else {
                "scanning...".into()
            }
        }
        ScanState::Cancelled => "cancelled".into(),
        ScanState::Finished => {
            if let Some(t) = &state.storage_tree {
                format!(
                    "done  files:{} dirs:{} errors:{}",
                    t.files, t.dirs, t.errors
                )
            } else {
                "done".into()
            }
        }
    };
    let cwd = {
        let mut p = state.scan_path.display().to_string();
        for part in &state.storage_cwd {
            p.push('/');
            p.push_str(part);
        }
        p
    };
    let header = format!(
        "tab:{}  path: {cwd}\nsort:{}  size:{}  stay_fs:{}  {}\nEnter inspect/preview · t cycle tabs · Backspace up",
        state.storage_tab.label(),
        state.storage_sort.label(),
        if state.use_apparent {
            "apparent"
        } else {
            "disk"
        },
        if state.stay_on_fs { "on" } else { "off" },
        status
    );
    frame.render_widget(
        Paragraph::new(header).block(
            Block::default()
                .borders(Borders::ALL)
                .title("Storage")
                .border_style(theme.border()),
        ),
        chunks[0],
    );

    let visible = chunks[1].height.saturating_sub(3) as usize;
    let mut vp = state.storage_vp;
    let border = if state.focus == FocusPane::Content {
        theme.accent()
    } else {
        theme.border()
    };

    match state.storage_tab {
        StorageTab::Mounts => {
            let disks = &state.metrics.disks;
            vp.ensure_visible(disks.len(), visible.max(1));
            let header_row =
                Row::new(vec!["", "MOUNT", "DEVICE", "USED", "TOTAL", "%"]).style(theme.accent());
            let rows = disks.iter().enumerate().map(|(i, d)| {
                let selected = i == vp.selected;
                let pct = if d.total == 0 {
                    0.0
                } else {
                    d.used as f64 / d.total as f64 * 100.0
                };
                let row = Row::new(vec![
                    marker_for(selected).to_string(),
                    sanitize_path_display(std::path::Path::new(&d.mount_point)),
                    d.name.clone(),
                    format_bytes(d.used),
                    format_bytes(d.total),
                    format!("{pct:.0}"),
                ]);
                if selected {
                    row.style(selected_row_style(theme))
                } else {
                    row
                }
            });
            let table = Table::new(
                rows,
                [
                    Constraint::Length(1),
                    Constraint::Min(12),
                    Constraint::Length(14),
                    Constraint::Length(10),
                    Constraint::Length(10),
                    Constraint::Length(5),
                ],
            )
            .header(header_row)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(format!(
                        "Mounts {} {}",
                        vp.position_label(disks.len()),
                        state.search_title_suffix()
                    ))
                    .border_style(border),
            );
            let mut ts = vp.to_table_state();
            sync_table_state(&vp, &mut ts);
            frame.render_stateful_widget(table, chunks[1], &mut ts);
        }
        StorageTab::LargestFiles => {
            let files = state
                .storage_tree
                .as_ref()
                .map(|t| crate::preview::largest_files_from_tree(&t.root, 50))
                .unwrap_or_default();
            vp.ensure_visible(files.len(), visible.max(1));
            let header_row = Row::new(vec!["", "SIZE", "PATH"]).style(theme.accent());
            let rows = files.iter().enumerate().map(|(i, f)| {
                let selected = i == vp.selected;
                let size = if state.use_apparent {
                    f.apparent_size
                } else {
                    f.size
                };
                let row = Row::new(vec![
                    marker_for(selected).to_string(),
                    format_size(size),
                    sanitize_path_display(&f.path),
                ]);
                if selected {
                    row.style(selected_row_style(theme))
                } else {
                    row
                }
            });
            let table = Table::new(
                rows,
                [
                    Constraint::Length(1),
                    Constraint::Length(12),
                    Constraint::Min(20),
                ],
            )
            .header(header_row)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(format!(
                        "Largest files {} {}",
                        vp.position_label(files.len()),
                        state.search_title_suffix()
                    ))
                    .border_style(border),
            );
            let mut ts = vp.to_table_state();
            sync_table_state(&vp, &mut ts);
            frame.render_stateful_widget(table, chunks[1], &mut ts);
        }
        StorageTab::DirectoryUsage => {
            let children = state.visible_storage_children();
            vp.ensure_visible(children.len(), visible.max(1));
            let max_size = children
                .iter()
                .map(|c| {
                    if state.use_apparent {
                        c.apparent_size
                    } else {
                        c.size
                    }
                })
                .max()
                .unwrap_or(1)
                .max(1);
            let header_row =
                Row::new(vec!["", "NAME", "TYPE", "SIZE", "BAR"]).style(theme.accent());
            let rows = children.iter().enumerate().map(|(i, c)| {
                let selected = i == vp.selected;
                let size = if state.use_apparent {
                    c.apparent_size
                } else {
                    c.size
                };
                let kind = if c.is_dir { "dir" } else { "file" };
                let name = if c.inaccessible {
                    format!("{} [denied]", c.name)
                } else {
                    c.name.clone()
                };
                let filled = ((size as f64 / max_size as f64) * 10.0) as usize;
                let bar = format!(
                    "{}{}",
                    "#".repeat(filled.min(10)),
                    ".".repeat(10 - filled.min(10))
                );
                let row = Row::new(vec![
                    marker_for(selected).to_string(),
                    name,
                    kind.into(),
                    format_size(size),
                    bar,
                ]);
                if selected {
                    row.style(selected_row_style(theme))
                } else {
                    row
                }
            });
            let table = Table::new(
                rows,
                [
                    Constraint::Length(1),
                    Constraint::Min(16),
                    Constraint::Length(6),
                    Constraint::Length(12),
                    Constraint::Length(10),
                ],
            )
            .header(header_row)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(format!(
                        "Entries {} {}",
                        vp.position_label(children.len()),
                        state.search_title_suffix()
                    ))
                    .border_style(border),
            );
            let mut ts = vp.to_table_state();
            sync_table_state(&vp, &mut ts);
            frame.render_stateful_widget(table, chunks[1], &mut ts);
            if children.len() > visible {
                let mut sb =
                    ScrollbarState::new(children.len().saturating_sub(1)).position(vp.selected);
                frame.render_stateful_widget(
                    Scrollbar::default()
                        .orientation(ratatui::widgets::ScrollbarOrientation::VerticalRight)
                        .begin_symbol(None)
                        .end_symbol(None),
                    chunks[1],
                    &mut sb,
                );
            }
        }
    }
}
