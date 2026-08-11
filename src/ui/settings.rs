//! Settings screen — section menu + interactive checkbox / cycle list.

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph, Wrap};

use crate::app::action::FocusPane;
use crate::app::settings_actions::setting_bool;
use crate::app::state::AppState;
use crate::settings::{rows_for_section, section_footer, SettingRow, SETTINGS_SECTIONS};
use crate::setup::{self, ConsoleInstallSpec};
use crate::ui::selection::{marker_for, selected_row_style};

pub use crate::settings::SETTINGS_SECTIONS as SECTIONS;

pub fn draw(frame: &mut Frame<'_>, area: Rect, state: &AppState) {
    let theme = state.theme();
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(18), Constraint::Min(20)])
        .split(area);

    draw_sections(frame, chunks[0], state, theme);
    draw_detail(frame, chunks[1], state, theme);
}

fn draw_sections(frame: &mut Frame<'_>, area: Rect, state: &AppState, theme: crate::theme::Theme) {
    let items: Vec<ListItem> = SETTINGS_SECTIONS
        .iter()
        .enumerate()
        .map(|(i, name)| {
            let selected = i == state.settings.section;
            let line = format!("{} {name}", marker_for(selected));
            let style = if selected {
                selected_row_style(theme)
            } else {
                theme.normal()
            };
            ListItem::new(line).style(style)
        })
        .collect();
    let border = if state.focus == FocusPane::Nav {
        theme.accent()
    } else {
        theme.border()
    };
    frame.render_widget(
        List::new(items).block(
            Block::default()
                .borders(Borders::ALL)
                .title("Sections")
                .border_style(border),
        ),
        area,
    );
}

fn draw_detail(frame: &mut Frame<'_>, area: Rect, state: &AppState, theme: crate::theme::Theme) {
    let border = if state.focus == FocusPane::Content {
        theme.accent()
    } else {
        theme.border()
    };
    let title = SETTINGS_SECTIONS
        .get(state.settings.section)
        .copied()
        .unwrap_or("Settings");
    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .border_style(border);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let rows = rows_for_section(state.settings.section);
    let footer = detail_footer(state);
    let footer_lines = footer.lines().count().max(1) as u16;
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(3),
            Constraint::Length(footer_lines.min(6).saturating_add(1)),
        ])
        .split(inner);

    let items: Vec<ListItem> = rows
        .iter()
        .enumerate()
        .map(|(i, row)| {
            let focused = state.focus == FocusPane::Content && i == state.settings.selected;
            let line = format_row(state, *row, focused);
            let style = if focused {
                selected_row_style(theme)
            } else {
                theme.normal()
            };
            ListItem::new(line).style(style)
        })
        .collect();
    frame.render_widget(List::new(items), chunks[0]);
    frame.render_widget(
        Paragraph::new(footer)
            .wrap(Wrap { trim: false })
            .style(theme.muted()),
        chunks[1],
    );
}

fn format_row(state: &AppState, row: SettingRow, focused: bool) -> String {
    let mark = marker_for(focused);
    let (open, close) = if state.ascii {
        ("<", ">")
    } else {
        ("‹", "›")
    };
    match row {
        SettingRow::Checkbox(id) => {
            let box_ = if setting_bool(state, id) {
                "[x]"
            } else {
                "[ ]"
            };
            format!("{mark} {box_}  {}", row.title())
        }
        SettingRow::TerminalProfile => format!(
            "{mark} {open} {} {close}  {}",
            state.terminal_profile.label(),
            row.title()
        ),
        SettingRow::PerformanceProfile => format!(
            "{mark} {open} {} {close}  {}",
            state.performance_profile.label(),
            row.title()
        ),
        SettingRow::LogPreset => format!(
            "{mark} {open} {} {close}  {}",
            state.log.preset.label(),
            row.title()
        ),
        SettingRow::Save
        | SettingRow::CompleteOnboarding
        | SettingRow::Reset
        | SettingRow::CleanupReports => {
            if focused {
                if state.ascii {
                    format!("{mark} > {} <", row.title())
                } else {
                    format!("{mark} ▸ {} ◂", row.title())
                }
            } else {
                format!("{mark}   {}", row.title())
            }
        }
    }
}

fn detail_footer(state: &AppState) -> String {
    let hint = section_footer(state.settings.section);
    let extra = match state.settings.section {
        0 => format!(
            "read_only: {} · onboarding: {} · pending: {}\nconfig: {}",
            state.read_only,
            state.config.onboarding_completed,
            state.settings.onboarding_pending,
            crate::sanitize::sanitize_path_display(&state.config_path)
        ),
        1 => format!("ascii (cli): {}", state.ascii),
        2 => format!(
            "refresh_ms: {} · history: {}",
            state.config.refresh_ms, state.config.metric_history_size
        ),
        3 => format!("activity events: {}", state.activity.len()),
        4 => format!(
            "max_log_entries: {} · report_retention_days: {}",
            state.config.max_log_entries, state.config.report_retention_days
        ),
        5 => format!("deep_requested: {}", state.diagnostic.deep),
        6 => "Prefer --read-only for observation-only installs.".into(),
        7 => {
            let spec = ConsoleInstallSpec::default();
            let (unit, _) = setup::install_paths(&spec.tty);
            format!(
                "{}\nWould write: {}\nretention days: {}",
                setup::status_text(&spec),
                unit.display(),
                state.config.report_retention_days
            )
        }
        _ => String::new(),
    };
    if extra.is_empty() {
        hint.to_string()
    } else {
        format!("{hint}\n{extra}")
    }
}
