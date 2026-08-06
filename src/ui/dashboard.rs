use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::app::state::AppState;
use crate::model::{format_bytes, format_rate, format_uptime};
use crate::ui::components::{gauge, sparkline};
use crate::ui::theme::Theme;

pub fn draw(frame: &mut Frame<'_>, area: Rect, state: &AppState) {
    let theme = Theme::new(state.color);
    let cols = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4),
            Constraint::Length(5),
            Constraint::Length(5),
            Constraint::Min(6),
        ])
        .split(area);

    let m = &state.metrics;
    let info = format!(
        "distro: {}  kernel: {}  arch: {}\nload: {:.2} {:.2} {:.2}  procs: {}  failed services: {}\nsystemd: {}  journal: {}  uptime: {}",
        empty(&m.distro),
        empty(&m.kernel),
        empty(&m.arch),
        m.load_avg.0,
        m.load_avg.1,
        m.load_avg.2,
        m.process_count,
        m.failed_services,
        avail(m.systemd_available),
        avail(m.journal_available),
        format_uptime(m.uptime_seconds),
    );
    frame.render_widget(
        Paragraph::new(info).block(
            Block::default()
                .borders(Borders::ALL)
                .title("Host")
                .border_style(theme.border()),
        ),
        cols[0],
    );

    let mem_ratio = if m.memory_total == 0 {
        0.0
    } else {
        m.memory_used as f64 / m.memory_total as f64
    };
    let swap_ratio = if m.swap_total == 0 {
        0.0
    } else {
        m.swap_used as f64 / m.swap_total as f64
    };
    let row = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(cols[1]);
    frame.render_widget(
        gauge(
            theme,
            "CPU",
            f64::from(m.cpu_total) / 100.0,
            &format!("{:.1}%", m.cpu_total),
        ),
        row[0],
    );
    frame.render_widget(
        gauge(
            theme,
            "Memory",
            mem_ratio,
            &format!(
                "{} / {}",
                format_bytes(m.memory_used),
                format_bytes(m.memory_total)
            ),
        ),
        row[1],
    );

    let row2 = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(33),
            Constraint::Percentage(33),
            Constraint::Percentage(34),
        ])
        .split(cols[2]);
    frame.render_widget(
        gauge(
            theme,
            "Swap",
            swap_ratio,
            &format!(
                "{} / {}",
                format_bytes(m.swap_used),
                format_bytes(m.swap_total)
            ),
        ),
        row2[0],
    );
    let cpu_hist: Vec<u64> = state
        .history
        .cpu
        .iter()
        .map(|v| (*v as u64).min(100))
        .collect();
    frame.render_widget(
        sparkline(theme, "CPU hist", &cpu_hist, state.ascii),
        row2[1],
    );
    let mem_hist: Vec<u64> = state
        .history
        .memory
        .iter()
        .map(|v| (*v as u64).min(100))
        .collect();
    frame.render_widget(
        sparkline(theme, "MEM hist", &mem_hist, state.ascii),
        row2[2],
    );

    let mut lines = vec![Line::from(format!(
        "net rx {} ({})  tx {} ({})",
        format_bytes(m.net_rx_bytes),
        format_rate(m.net_rx_bps),
        format_bytes(m.net_tx_bytes),
        format_rate(m.net_tx_bps),
    ))];
    if !m.cpu_per_core.is_empty() && area.width > 80 {
        let cores: String = m
            .cpu_per_core
            .iter()
            .enumerate()
            .map(|(i, c)| format!("c{i}:{c:.0}%"))
            .collect::<Vec<_>>()
            .join(" ");
        lines.push(Line::from(cores));
    }
    for d in m.disks.iter().take(6) {
        let pct = if d.total == 0 {
            0.0
        } else {
            d.used as f64 / d.total as f64 * 100.0
        };
        lines.push(Line::from(format!(
            "{} {}  {:.0}%  {} / {}",
            d.mount_point,
            d.name,
            pct,
            format_bytes(d.used),
            format_bytes(d.total)
        )));
    }
    if m.temperatures.is_empty() {
        lines.push(Line::from("temperatures: no disponible").style(theme.muted()));
    } else {
        let temps: String = m
            .temperatures
            .iter()
            .map(|t| format!("{}:{:.0}C", t.label, t.celsius))
            .collect::<Vec<_>>()
            .join("  ");
        lines.push(Line::from(temps));
    }
    frame.render_widget(
        Paragraph::new(lines).block(
            Block::default()
                .borders(Borders::ALL)
                .title("Network / Disks / Sensors")
                .border_style(theme.border()),
        ),
        cols[3],
    );
}

fn empty(s: &str) -> &str {
    if s.is_empty() {
        "..."
    } else {
        s
    }
}

fn avail(v: bool) -> &'static str {
    if v {
        "ok"
    } else {
        "no disponible"
    }
}
