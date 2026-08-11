//! Action menu builders for list screens.

use crate::app::state::AppState;
use crate::model::{MenuAction, MenuItem, ProcessSignal, ServiceActionKind, UnitFileState};

pub fn process_menu(state: &AppState) -> Vec<MenuItem> {
    let mut items = vec![
        MenuItem::enabled("Details (Enter)", MenuAction::ProcessInspect),
        MenuItem::enabled(
            if state.process.follow_pid.is_some() {
                "Stop follow PID"
            } else {
                "Follow selected PID"
            },
            MenuAction::ProcessFollow,
        ),
        MenuItem::enabled(
            if state.process.tree_mode {
                "Flat list view"
            } else {
                "Tree view (on-demand)"
            },
            MenuAction::ProcessTree,
        ),
        MenuItem::enabled("Open related logs", MenuAction::ProcessLogs),
        MenuItem::enabled("Jump to service (if unit)", MenuAction::ProcessService),
        MenuItem::enabled("Diagnostics", MenuAction::ProcessDiagnostics),
        MenuItem::enabled(
            "Export context (command argv0 only)",
            MenuAction::ProcessExport,
        ),
    ];
    if state.read_only {
        items.push(MenuItem::disabled(
            "SIGTERM",
            MenuAction::ProcessSignal(ProcessSignal::Term),
            "READ ONLY",
        ));
        items.push(MenuItem::disabled(
            "SIGKILL",
            MenuAction::ProcessSignal(ProcessSignal::Kill),
            "READ ONLY",
        ));
        items.push(MenuItem::disabled(
            "SIGSTOP",
            MenuAction::ProcessSignal(ProcessSignal::Stop),
            "READ ONLY",
        ));
        items.push(MenuItem::disabled(
            "SIGCONT",
            MenuAction::ProcessSignal(ProcessSignal::Cont),
            "READ ONLY",
        ));
    } else {
        items.push(MenuItem::enabled(
            "SIGTERM",
            MenuAction::ProcessSignal(ProcessSignal::Term),
        ));
        items.push(MenuItem::enabled(
            "SIGKILL",
            MenuAction::ProcessSignal(ProcessSignal::Kill),
        ));
        items.push(MenuItem::enabled(
            "SIGSTOP",
            MenuAction::ProcessSignal(ProcessSignal::Stop),
        ));
        items.push(MenuItem::enabled(
            "SIGCONT",
            MenuAction::ProcessSignal(ProcessSignal::Cont),
        ));
    }
    items.push(MenuItem::enabled("Close", MenuAction::Close));
    items
}

pub fn service_menu(state: &AppState, active: &str, unit_file: UnitFileState) -> Vec<MenuItem> {
    let ro = state.read_only;
    let mut items = vec![
        MenuItem::enabled("Details + logs (Enter)", MenuAction::ServiceInspect),
        MenuItem::enabled("Open logs", MenuAction::ServiceLogs),
        MenuItem::enabled("Diagnostics", MenuAction::ServiceDiagnostics),
        MenuItem::enabled("Export context", MenuAction::ServiceExport),
    ];
    let add_action = |items: &mut Vec<MenuItem>, label: &str, kind: ServiceActionKind| {
        if ro {
            items.push(MenuItem::disabled(
                label,
                MenuAction::ServiceAction(kind),
                "READ ONLY",
            ));
            return;
        }
        // Soft disable examples
        if kind == ServiceActionKind::Reload && active.eq_ignore_ascii_case("inactive") {
            items.push(MenuItem::disabled(
                label,
                MenuAction::ServiceAction(kind),
                "unit inactive — reload not useful",
            ));
            return;
        }
        if kind == ServiceActionKind::Enable
            && matches!(
                unit_file,
                UnitFileState::Enabled | UnitFileState::EnabledRuntime | UnitFileState::Static
            )
        {
            items.push(MenuItem::disabled(
                label,
                MenuAction::ServiceAction(kind),
                format!("already {}", unit_file.label()),
            ));
            return;
        }
        if kind == ServiceActionKind::Disable
            && matches!(
                unit_file,
                UnitFileState::Disabled | UnitFileState::Static | UnitFileState::Masked
            )
        {
            items.push(MenuItem::disabled(
                label,
                MenuAction::ServiceAction(kind),
                format!("not disableable ({})", unit_file.label()),
            ));
            return;
        }
        items.push(MenuItem::enabled(label, MenuAction::ServiceAction(kind)));
    };
    add_action(&mut items, "Start", ServiceActionKind::Start);
    add_action(&mut items, "Stop", ServiceActionKind::Stop);
    add_action(&mut items, "Restart", ServiceActionKind::Restart);
    add_action(&mut items, "Reload", ServiceActionKind::Reload);
    add_action(&mut items, "Enable", ServiceActionKind::Enable);
    add_action(&mut items, "Disable", ServiceActionKind::Disable);
    items.push(MenuItem::enabled("Close", MenuAction::Close));
    items
}

pub fn log_menu() -> Vec<MenuItem> {
    vec![
        MenuItem::enabled(
            "Export selected (markdown)",
            MenuAction::LogExportSelected(crate::model::ExportFormat::Markdown),
        ),
        MenuItem::enabled(
            "Export visible (text)",
            MenuAction::LogExportVisible(crate::model::ExportFormat::Text),
        ),
        MenuItem::enabled(
            "Export visible (markdown)",
            MenuAction::LogExportVisible(crate::model::ExportFormat::Markdown),
        ),
        MenuItem::enabled(
            "Export visible (json)",
            MenuAction::LogExportVisible(crate::model::ExportFormat::Json),
        ),
        MenuItem::enabled(
            "Export context ±5 (markdown)",
            MenuAction::LogExportContext(crate::model::ExportFormat::Markdown),
        ),
        MenuItem::enabled("Close", MenuAction::Close),
    ]
}

pub fn finding_menu() -> Vec<MenuItem> {
    vec![
        MenuItem::enabled("Export finding context", MenuAction::FindingExport),
        MenuItem::enabled("Close", MenuAction::Close),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use std::path::PathBuf;

    #[test]
    fn readonly_disables_signals() {
        let state = AppState::new(
            Config::default(),
            true,
            true,
            true,
            false,
            PathBuf::from("/tmp"),
            Config::default_path(),
        );
        let m = process_menu(&state);
        assert!(m.iter().any(|i| !i.enabled && i.label.contains("SIGTERM")));
    }
}
