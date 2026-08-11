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
    state.diagnostic.findings.push(Finding {
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
fn renders_all_screens_matrix() {
    let screens = [
        Screen::Dashboard,
        Screen::Processes,
        Screen::Services,
        Screen::Logs,
        Screen::Storage,
        Screen::Diagnostics,
        Screen::Settings,
    ];
    let sizes = [(40, 8), (80, 24), (120, 40)];
    for (w, h) in sizes {
        for screen in screens {
            let mut state = state_80x24();
            state.screen = screen;
            state.width = w;
            state.height = h;
            state.viewport_rows = server_tui::viewport::content_rows_from_terminal(h);
            let backend = TestBackend::new(w, h);
            let mut terminal = Terminal::new(backend).expect("terminal");
            terminal.draw(|f| ui::draw(f, &state)).expect("draw");
        }
    }
}

#[test]
fn renders_elevation_and_inspector_overlays() {
    let mut state = state_80x24();
    state.dialog = Some(Dialog::ConfirmElevation {
        unit: "ssh.service".into(),
        action: server_tui::model::ServiceActionKind::Restart,
        choice: server_tui::app::action::ConfirmChoice::Cancel,
    });
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("terminal");
    terminal.draw(|f| ui::draw(f, &state)).expect("draw");

    state.dialog = Some(Dialog::Inspector {
        title: "demo".into(),
        body: "line1\nline2".into(),
    });
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
    let text = buffer_text(terminal.backend());
    assert!(
        text.contains("too small"),
        "tiny terminal should show fallback; got {text:?}"
    );
}

fn buffer_text(backend: &TestBackend) -> String {
    let buf = backend.buffer();
    let area = buf.area;
    let mut out = String::new();
    for y in 0..area.height {
        for x in 0..area.width {
            out.push_str(buf[(x, y)].symbol());
        }
        out.push('\n');
    }
    out
}

#[test]
fn render_shows_read_only_on_services() {
    let mut state = state_80x24();
    state.screen = Screen::Services;
    state.read_only = true;
    state.service.items = vec![server_tui::model::ServiceInfo {
        unit: "ssh.service".into(),
        description: "OpenSSH".into(),
        load_state: "loaded".into(),
        active_state: "active".into(),
        sub_state: "running".into(),
        unit_path: "/org/freedesktop/systemd1/unit/ssh".into(),
        unit_file_state: server_tui::model::UnitFileState::Enabled,
        fragment_path: None,
    }];
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("terminal");
    terminal.draw(|f| ui::draw(f, &state)).expect("draw");
    let text = buffer_text(terminal.backend());
    assert!(
        text.contains("READ ONLY"),
        "expected READ ONLY chrome when read_only; got {text}"
    );
}

#[test]
fn render_shows_health_and_finding() {
    let mut state = state_80x24();
    state.screen = Screen::Diagnostics;
    state.health_status = HealthStatus::Warning;
    state.diagnostic.findings.push(Finding {
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
    let text = buffer_text(terminal.backend());
    assert!(
        text.contains("Demo finding"),
        "expected finding title visibility; got {text}"
    );
}

#[test]
fn render_settings_screen_identifiable() {
    let mut state = state_80x24();
    state.screen = Screen::Settings;
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("terminal");
    terminal.draw(|f| ui::draw(f, &state)).expect("draw");
    let text = buffer_text(terminal.backend());
    assert!(
        text.contains("Settings") || text.contains("General") || text.contains("Appearance"),
        "expected Settings content; got {text}"
    );
}
