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
                "Send {} to process?\n\nPID:      {pid}\nUser:     {user}\nCommand:  {command}{warn}",
                signal.label(),
            );
            draw_confirm(
                frame,
                area,
                theme,
                &format!("Confirm {}", signal.label()),
                &body,
                *choice,
                ConfirmButtonSet::YesCancel,
            );
        }
        Dialog::ConfirmService {
            unit,
            action,
            choice,
        } => {
            let body = format!(
                "Perform systemd action?\n\nUnit:    {unit}\nAction:  {}",
                action.label(),
            );
            draw_confirm(
                frame,
                area,
                theme,
                "Confirm service action",
                &body,
                *choice,
                ConfirmButtonSet::YesCancel,
            );
        }
        Dialog::ConfirmElevation {
            unit,
            action,
            choice,
        } => {
            let body = format!(
                "Administrator permission is required.\n\nUnit:    {unit}\nAction:  {}\n\nRun once with sudo (leave TUI → sudo -v → return), or Cancel.",
                action.label(),
            );
            draw_confirm(
                frame,
                area,
                theme,
                "Administrator permission is required",
                &body,
                *choice,
                ConfirmButtonSet::SudoCancel,
            );
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
            draw_confirm(
                frame,
                area,
                theme,
                title,
                msg,
                *choice,
                ConfirmButtonSet::YesCancel,
            );
        }
        Dialog::DiagnosticReport { body } => {
            draw_box(frame, area, theme, "Diagnostic report", body);
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

#[derive(Clone, Copy)]
enum ConfirmButtonSet {
    YesCancel,
    SudoCancel,
}

fn draw_confirm(
    frame: &mut Frame<'_>,
    area: Rect,
    theme: Theme,
    title: &str,
    body: &str,
    choice: ConfirmChoice,
    buttons: ConfirmButtonSet,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(title.to_string())
        .border_style(theme.warn());
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(3),
            Constraint::Length(3),
            Constraint::Length(1),
        ])
        .split(inner);

    frame.render_widget(
        Paragraph::new(body.to_string())
            .wrap(Wrap { trim: false })
            .style(theme.normal()),
        chunks[0],
    );
    frame.render_widget(confirm_button_line(theme, choice, buttons), chunks[1]);
    frame.render_widget(
        Paragraph::new("←/→ or Tab focus · Enter activate · y confirm · n/Esc cancel")
            .style(theme.muted()),
        chunks[2],
    );
}

fn confirm_button_line(
    theme: Theme,
    choice: ConfirmChoice,
    buttons: ConfirmButtonSet,
) -> Paragraph<'static> {
    let (cancel_label, yes_label) = match buttons {
        ConfirmButtonSet::YesCancel => (" Cancel ", " Yes "),
        ConfirmButtonSet::SudoCancel => (" Cancel ", " Run once with sudo "),
    };
    let cancel_focused = matches!(choice, ConfirmChoice::Cancel);
    let yes_focused = matches!(choice, ConfirmChoice::Yes);

    let cancel = Span::styled(
        cancel_label,
        if cancel_focused {
            filled_button_style(theme)
        } else {
            idle_button_style(theme)
        },
    );
    let yes = Span::styled(
        yes_label,
        if yes_focused {
            filled_button_style(theme)
        } else {
            idle_button_style(theme)
        },
    );
    Paragraph::new(Line::from(vec![
        Span::raw("  "),
        cancel,
        Span::raw("   "),
        yes,
    ]))
}

/// Filled control: reverse video + bold so focus reads as a real button.
fn filled_button_style(theme: Theme) -> Style {
    selected_row_style(theme)
}

fn idle_button_style(theme: Theme) -> Style {
    theme.muted().add_modifier(Modifier::DIM)
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
