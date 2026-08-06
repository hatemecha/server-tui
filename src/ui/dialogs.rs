//! Modal dialogs: help, confirmations, messages.

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};

use crate::app::state::{AppState, Dialog};
use crate::model::ProcessSignal;
use crate::ui::help;
use crate::ui::theme::Theme;

pub fn draw(frame: &mut Frame<'_>, state: &AppState) {
    let Some(dialog) = &state.dialog else {
        return;
    };
    let theme = Theme::new(state.color);
    let area = centered_rect(70, 70, frame.area());
    Clear.render(area, frame.buffer_mut());

    match dialog {
        Dialog::Help => help::draw_help(frame, area, theme),
        Dialog::ConfirmSignal {
            pid,
            user,
            command,
            signal,
            start_time: _,
        } => {
            let warn = if *signal == ProcessSignal::Kill {
                "\nWARNING: SIGKILL cannot be caught. Prefer SIGTERM when possible.\n"
            } else {
                "\n"
            };
            let body = format!(
                "Send {} to process?\n\nPID:      {pid}\nUser:     {user}\nCommand:  {command}{warn}\n[Esc] Cancel    [Enter] Confirm",
                signal.label()
            );
            draw_box(
                frame,
                area,
                theme,
                &format!("Confirm {}", signal.label()),
                &body,
            );
        }
        Dialog::ConfirmService { unit, action } => {
            let body = format!(
                "Perform systemd action?\n\nUnit:    {unit}\nAction:  {}\n\n[Esc] Cancel    [Enter] Confirm",
                action.label()
            );
            draw_box(frame, area, theme, "Confirm service action", &body);
        }
        Dialog::Message { title, body } => {
            draw_box(frame, area, theme, title, body);
        }
    }
}

fn draw_box(frame: &mut Frame<'_>, area: Rect, theme: Theme, title: &str, body: &str) {
    let p = Paragraph::new(body.to_string())
        .wrap(Wrap { trim: false })
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(title)
                .border_style(theme.warn()),
        );
    frame.render_widget(p, area);
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let popup = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup[1])[1]
}
