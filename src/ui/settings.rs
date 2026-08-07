//! Settings screen — General / Appearance / Performance / Safety / Startup.

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};

use crate::app::state::AppState;
use crate::setup::{self, ConsoleInstallSpec};
use crate::ui::selection::{marker_for, selected_row_style};

pub const SECTIONS: &[&str] = &[
    "General",
    "Appearance",
    "Performance",
    "Dashboard",
    "Logs",
    "Diagnostics",
    "Safety",
    "Startup",
];

pub fn draw(frame: &mut Frame<'_>, area: Rect, state: &AppState) {
    let theme = state.theme();
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(18), Constraint::Min(20)])
        .split(area);

    let items: Vec<ListItem> = SECTIONS
        .iter()
        .enumerate()
        .map(|(i, name)| {
            let selected = i == state.settings_section;
            let line = format!("{} {name}", marker_for(selected));
            let style = if selected {
                selected_row_style(theme)
            } else {
                theme.normal()
            };
            ListItem::new(line).style(style)
        })
        .collect();
    frame.render_widget(
        List::new(items).block(
            Block::default()
                .borders(Borders::ALL)
                .title("Settings")
                .border_style(theme.border()),
        ),
        chunks[0],
    );

    let body = section_body(state);
    frame.render_widget(
        Paragraph::new(body).block(
            Block::default()
                .borders(Borders::ALL)
                .title(
                    SECTIONS
                        .get(state.settings_section)
                        .copied()
                        .unwrap_or("Settings"),
                )
                .border_style(theme.border()),
        ),
        chunks[1],
    );
}

fn section_body(state: &AppState) -> String {
    match state.settings_section {
        0 => format!(
            "read_only (cli): {}\nonboarding_completed: {}\nonboarding_pending: {}\nconfig: {}\n\nKeys: S save · o complete onboarding · d reset (confirm)\nEnter also saves.",
            state.read_only,
            state.config.onboarding_completed,
            state.onboarding_pending,
            state.config_path.display()
        ),
        1 => format!(
            "terminal_profile: {}  (key t to cycle)\ncolor: {}  (key c)\nascii: {}\n\nApplies immediately to Theme.",
            state.terminal_profile.label(),
            state.color,
            state.ascii
        ),
        2 => format!(
            "performance_profile: {}  (key p)\nrefresh_ms: {}\nhistory: {}\n\nLow-resource slows refresh; Responsive speeds it.",
            state.performance_profile.label(),
            state.config.refresh_ms,
            state.config.metric_history_size
        ),
        3 => format!(
            "wallboard: {}  (key w)\nAttention panel uses diagnostics findings.\nRecent activity buffer: {} events.",
            state.wallboard,
            state.activity.len()
        ),
        4 => format!(
            "log preset: {}\nmax_log_entries: {}\nreport_retention_days: {}\n\nCleanup old reports: run from Startup or leave retention > 0.",
            state.log_preset.label(),
            state.config.max_log_entries,
            state.config.report_retention_days
        ),
        5 => format!(
            "diagnostics_light_scan: {}  (key 5)\nenable_smart_probes: {}  (key 4)\ndeep_requested: {}\n\nLight = skip SMART. Deep (D on Diagnostics) enables expensive probes.",
            state.config.diagnostics_light_scan,
            state.config.enable_smart_probes,
            state.diagnostic_deep
        ),
        6 => format!(
            "confirm_sigterm: {}  (1)\nconfirm_sigkill: {}  (2)\nconfirm_service_actions: {}  (3)\nPrefer --read-only for observation.",
            state.config.confirm_sigterm,
            state.config.confirm_sigkill,
            state.config.confirm_service_actions
        ),
        7 => {
            let spec = ConsoleInstallSpec::default();
            let (unit, _) = setup::install_paths(&spec.tty);
            format!(
                "{}\n\nWould write: {}\nKeys: C cleanup reports (retention {}).\nCLI: server-tui setup console --status | --print-unit\nLibrary: install_console_unit / remove_console_unit via ConsoleFsOps (DryRunFs in tests; RealFs never used by smoke).",
                setup::status_text(&spec),
                unit.display(),
                state.config.report_retention_days
            )
        }
        _ => String::new(),
    }
}
