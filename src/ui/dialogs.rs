//! Modal dialogs: help, glossary, confirmations, inspectors, action menus.

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap};

use crate::app::action::ConfirmChoice;
use crate::app::state::{AppState, Dialog};
use crate::glossary;
use crate::model::ProcessSignal;
use crate::ui::help;
use crate::ui::selection::{marker_for, selected_row_style};
use crate::ui::theme::Theme;

pub fn draw(frame: &mut Frame<'_>, state: &AppState) {
    let Some(dialog) = &state.dialog else {
        return;
    };
    let theme = state.theme();
    let area = centered_rect(78, 78, frame.area());
    Clear.render(area, frame.buffer_mut());

    match dialog {
        Dialog::Help => help::draw_help(frame, area, theme),
        Dialog::Glossary => glossary::draw(frame, area, state, theme),
        Dialog::ConfirmSignal {
            pid,
            user,
            command,
            signal,
            start_time: _,
            choice,
        } => {
            let warn = if *signal == ProcessSignal::Kill {
                "\nWARNING: SIGKILL cannot be caught or ignored. Prefer SIGTERM when possible.\nSuccess means the signal was delivered; the process may still appear briefly.\n"
            } else {
                "\n"
            };
            let body = format!(
                "Send {} to process?\n\nPID:      {pid}\nUser:     {user}\nCommand:  {command}{warn}\n{}",
                signal.label(),
                confirm_buttons(*choice)
            );
            draw_box(
                frame,
                area,
                theme,
                &format!("Confirm {}", signal.label()),
                &body,
            );
        }
        Dialog::ConfirmService {
            unit,
            action,
            choice,
        } => {
            let body = format!(
                "Perform systemd action?\n\nUnit:    {unit}\nAction:  {}\n\n{}",
                action.label(),
                confirm_buttons(*choice)
            );
            draw_box(frame, area, theme, "Confirm service action", &body);
        }
        Dialog::ConfirmResetSettings { choice } | Dialog::ConfirmCompleteOnboarding { choice } => {
            let (title, msg) = match dialog {
                Dialog::ConfirmResetSettings { .. } => (
                    "Reset settings",
                    "Reset all settings to defaults and write config.toml?",
                ),
                _ => (
                    "Complete onboarding",
                    "Mark onboarding complete and save config?",
                ),
            };
            let body = format!("{msg}\n\n{}", confirm_buttons(*choice));
            draw_box(frame, area, theme, title, &body);
        }
        Dialog::DiagnosticReport { body } => {
            draw_box(frame, area, theme, "Diagnostic report (redacted)", body);
        }
        Dialog::Message { title, body } => {
            draw_box(frame, area, theme, title, body);
        }
        Dialog::Inspector { title, body } => {
            draw_box(frame, area, theme, title, body);
        }
        Dialog::ActionMenu {
            title,
            items,
            selected,
        } => {
            let list_items: Vec<ListItem> = items
                .iter()
                .enumerate()
                .map(|(i, it)| {
                    let mark = marker_for(i == *selected);
                    let label = if it.enabled {
                        format!("{mark} {}", it.label)
                    } else {
                        format!(
                            "{mark} {} — {}",
                            it.label,
                            it.disabled_reason.as_deref().unwrap_or("disabled")
                        )
                    };
                    let style = if i == *selected {
                        selected_row_style(theme)
                    } else if it.enabled {
                        theme.normal()
                    } else {
                        theme.muted()
                    };
                    ListItem::new(label).style(style)
                })
                .collect();
            let list = List::new(list_items).block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(title.as_str())
                    .border_style(theme.warn()),
            );
            frame.render_widget(list, area);
        }
    }
}

fn confirm_buttons(choice: ConfirmChoice) -> String {
    let (yes, cancel) = match choice {
        ConfirmChoice::Yes => ("[ YES ]", "  Cancel  "),
        ConfirmChoice::Cancel => ("  Yes  ", "[ CANCEL ]"),
    };
    format!("{cancel}   {yes}\n\n←/→ focus · Enter · y/n")
}

fn draw_box(frame: &mut Frame<'_>, area: Rect, theme: Theme, title: &str, body: &str) {
    let p = Paragraph::new(body.to_string())
        .wrap(Wrap { trim: false })
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(title.to_string())
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
