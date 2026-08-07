//! TestBackend render smoke tests (no real TTY).

use ratatui::backend::TestBackend;
use ratatui::Terminal;
use server_tui::app::action::Screen;
use server_tui::app::state::{AppState, Dialog};
use server_tui::config::Config;
use server_tui::model::{Category, Confidence, Evidence, Finding, HealthStatus, Severity};
use server_tui::ui;
use std::path::PathBuf;

fn state_80x24() -> AppState {
    let mut s = AppState::new(
        Config::default(),
        true,
        true,
        false,
        true,
        PathBuf::from("/tmp"),
    );
    s.width = 80;
    s.height = 24;
    s
}

#[test]
fn renders_dashboard_80x24() {
    let mut state = state_80x24();
    state.screen = Screen::Dashboard;
    state.metrics.cpu_total = 42.0;
    state.metrics.memory_used = 1024 * 1024;
    state.metrics.memory_total = 4 * 1024 * 1024;
    state.metrics.net_rx_bps = 12_000.0;
    state.metrics.net_tx_bps = 8_000.0;
    state.history.record_from_metrics(&state.metrics);
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("terminal");
    terminal.draw(|f| ui::draw(f, &state)).expect("draw");
}

#[test]
fn renders_glossary_overlay_80x24() {
    let mut state = state_80x24();
    state.dialog = Some(Dialog::Glossary);
    state.search.glossary = "demo".into();
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("terminal");
    terminal.draw(|f| ui::draw(f, &state)).expect("draw");
}

#[test]
fn renders_diagnostics_with_finding() {
    let mut state = state_80x24();
    state.screen = Screen::Diagnostics;
    state.health_status = HealthStatus::Warning;
    state.findings.push(Finding {
        id: "demo.x".into(),
        title: "Demo finding".into(),
        summary: "summary".into(),
        severity: Severity::Warning,
        confidence: Confidence::Low,
        category: Category::Other,
        evidence: Evidence {
            summary: "e".into(),
            details: vec![],
        },
        targets: vec![],
        suggested_check: None,
        degradable: true,
    });
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("terminal");
    terminal.draw(|f| ui::draw(f, &state)).expect("draw");
}

#[test]
fn renders_tiny_without_panic() {
    let mut state = state_80x24();
    state.width = 40;
    state.height = 8;
    let backend = TestBackend::new(40, 8);
    let mut terminal = Terminal::new(backend).expect("terminal");
    terminal.draw(|f| ui::draw(f, &state)).expect("draw");
}
