//! Selection markers that remain visible with --no-color / Tty16.

use ratatui::style::{Modifier, Style};

use crate::ui::theme::{TerminalProfile, Theme};

/// Prefix for the selected row (`>`). Always textual so monochrome works.
pub const SELECT_MARKER: &str = ">";
pub const UNSELECT_MARKER: &str = " ";

/// Style for a selected row: REVERSED + BOLD when colors/styles available.
pub fn selected_row_style(theme: Theme) -> Style {
    let mut style = theme.highlight();
    // Always add REVERSED so selection is visible even with NO_COLOR / Tty16.
    style = style.add_modifier(Modifier::REVERSED | Modifier::BOLD);
    if matches!(
        theme.profile,
        TerminalProfile::Monochrome | TerminalProfile::Tty16
    ) || !theme.color
    {
        // Drop background reliance; reverse video is the signal.
        style = Style::default().add_modifier(Modifier::REVERSED | Modifier::BOLD);
    }
    style
}

/// Prefix cell for table/list rows.
pub fn marker_for(selected: bool) -> &'static str {
    if selected {
        SELECT_MARKER
    } else {
        UNSELECT_MARKER
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;
    use ratatui::prelude::*;
    use ratatui::widgets::{Block, Borders, Row, Table};
    use ratatui::Terminal;

    #[test]
    fn marker_differs() {
        assert_eq!(marker_for(true), ">");
        assert_eq!(marker_for(false), " ");
    }

    #[test]
    fn selected_row_visible_without_color() {
        let backend = TestBackend::new(40, 6);
        let mut terminal = Terminal::new(backend).expect("term");
        let theme = Theme::with_profile(false, TerminalProfile::Monochrome);
        terminal
            .draw(|f| {
                let rows = [
                    Row::new(vec![marker_for(false), "alpha"]),
                    Row::new(vec![marker_for(true), "beta"]).style(selected_row_style(theme)),
                ];
                let table = Table::new(rows, [Constraint::Length(1), Constraint::Min(4)])
                    .block(Block::default().borders(Borders::ALL).title("t"));
                f.render_widget(table, f.area());
            })
            .expect("draw");
        let buf = terminal.backend().buffer().clone();
        let mut found_marker = false;
        let area = buf.area;
        for y in 0..area.height {
            for x in 0..area.width {
                if buf[(x, y)].symbol() == ">" {
                    found_marker = true;
                }
            }
        }
        assert!(found_marker, "expected > marker in buffer");
    }
}
