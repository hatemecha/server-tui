use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph, Row, Table};

use crate::app::action::FocusPane;
use crate::app::state::{AppState, ScanState};
use crate::model::format_size;
use crate::ui::theme::Theme;

pub fn draw(frame: &mut Frame<'_>, area: Rect, state: &AppState) {
    let theme = Theme::new(state.color);
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(4), Constraint::Min(5)])
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
                    "done  files:{} dirs:{} errors:{}  excluded: /proc /sys /dev /run",
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
        "path: {cwd}\nsort:{}  size:{}  stay_fs:{}  {}",
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

    let children = state
        .current_storage_node()
        .map(|n| n.children.as_slice())
        .unwrap_or(&[]);
    let header_row = Row::new(vec!["NAME", "TYPE", "SIZE"]).style(theme.accent());
    let rows = children.iter().enumerate().map(|(i, c)| {
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
        let row = Row::new(vec![name, kind.into(), format_size(size)]);
        if i == state.storage_selected {
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
            Constraint::Min(20),
            Constraint::Length(6),
            Constraint::Length(12),
        ],
    )
    .header(header_row)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(format!("Entries ({})", children.len()))
            .border_style(border),
    );
    frame.render_widget(table, chunks[1]);
}
