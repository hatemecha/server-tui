use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::app::state::AppState;
use crate::model::{format_bytes, format_rate, format_uptime};
use crate::sanitize::{sanitize_path_display, truncate_width};
use crate::ui::components::{gauge, sparkline};
use crate::ui::theme::Theme;

pub fn draw(frame: &mut Frame<'_>, area: Rect, state: &AppState) {
    let theme = state.theme();

    if area.height < 8 || area.width < 24 {
        draw_compact(frame, area, state, theme);
        return;
    }

    // Fit sparklines/net on typical 80×24 (content ≈15 with tab strip).
    let (host_h, gauges_h, hist_h, net_h) = if area.height < 12 {
        (3u16, 3, 0, 0)
    } else if area.height < 15 {
        (3, 3, 0, 3)
    } else if area.height < 18 {
        (3, 3, 3, 3)
    } else {
        (3, 4, 4, 4)
    };

    let mut constraints = vec![Constraint::Length(host_h), Constraint::Length(gauges_h)];
    if hist_h > 0 {
        constraints.push(Constraint::Length(hist_h));
    }
    if net_h > 0 {
        constraints.push(Constraint::Length(net_h));
    }
    constraints.push(Constraint::Length(5));
    constraints.push(Constraint::Min(3));

    let cols = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(area);

    let mut idx = 0usize;
    draw_host(frame, cols[idx], state, theme);
    idx += 1;

    draw_resource_gauges(frame, cols[idx], state, theme);
    idx += 1;

    if hist_h > 0 {
        draw_history(frame, cols[idx], state, theme);
        idx += 1;
    }
    if net_h > 0 {
        draw_network(frame, cols[idx], state, theme);
        idx += 1;
    }
    draw_attention_strip(frame, cols[idx], state, theme);
    idx += 1;
    draw_disks_sensors(frame, cols[idx], state, theme);
}

fn draw_compact(frame: &mut Frame<'_>, area: Rect, state: &AppState, theme: Theme) {
    let m = &state.metrics;
    let line = format!(
        "cpu {:.0}%  mem {}  rx {}  ld {:.2}  {}",
        m.cpu_total,
        format_bytes(m.memory_used),
        format_rate(m.net_rx_bps),
        m.load_avg.0,
        state.health_status.label(),
    );
    frame.render_widget(
        Paragraph::new(truncate_width(&line, area.width.saturating_sub(2) as usize)).block(
            Block::default()
                .borders(Borders::ALL)
                .title("Host")
                .border_style(theme.border()),
        ),
        area,
    );
}

fn draw_attention_strip(frame: &mut Frame<'_>, area: Rect, state: &AppState, theme: Theme) {
    let findings = state.visible_findings();
    let mut lines: Vec<Line> = findings
        .iter()
        .take(5)
        .map(|f| {
            Line::from(format!(
                "[{}] {}",
                f.severity.label(),
                truncate_width(&f.title, area.width.saturating_sub(6) as usize)
            ))
        })
        .collect();
    if lines.is_empty() {
        lines.push(Line::from(Span::styled(
            "no attention items",
            theme.muted(),
        )));
    }
    let recent: Vec<Line> = state
        .activity
        .iter()
        .rev()
        .take(5)
        .map(|a| Line::from(truncate_width(a, area.width.saturating_sub(2) as usize)))
        .collect();
    let split = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
        .split(area);
    frame.render_widget(
        Paragraph::new(lines).block(
            Block::default()
                .borders(Borders::ALL)
                .title("Attention (6 Diagnostics)")
                .border_style(theme.border()),
        ),
        split[0],
    );
    frame.render_widget(
        Paragraph::new(if recent.is_empty() {
            vec![Line::from(Span::styled(
                "no recent activity",
                theme.muted(),
            ))]
        } else {
            recent
        })
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Recent")
                .border_style(theme.border()),
        ),
        split[1],
    );
}

fn draw_host(frame: &mut Frame<'_>, area: Rect, state: &AppState, theme: Theme) {
    let m = &state.metrics;
    let w = area.width.saturating_sub(2) as usize;
    let line1 = truncate_width(
        &format!(
            "{}  {}  {}",
            empty(&m.distro),
            empty(&m.kernel),
            empty(&m.arch)
        ),
        w,
    );
    let line2 = truncate_width(
        &format!(
            "load {:.2} {:.2} {:.2}  procs {}  failed {}  up {}",
            m.load_avg.0,
            m.load_avg.1,
            m.load_avg.2,
            m.process_count,
            m.failed_services,
            format_uptime(m.uptime_seconds),
        ),
        w,
    );
    let text = if area.height >= 4 {
        format!(
            "{line1}\n{line2}\nsystemd {}  journal {}  health {}",
            avail(m.systemd_available),
            avail(m.journal_available),
            state.health_status.label(),
        )
    } else {
        format!("{line1}\n{line2}")
    };
    frame.render_widget(
        Paragraph::new(text).block(
            Block::default()
                .borders(Borders::ALL)
                .title("Host")
                .border_style(theme.border()),
        ),
        area,
    );
}

fn draw_resource_gauges(frame: &mut Frame<'_>, area: Rect, state: &AppState, theme: Theme) {
    let m = &state.metrics;
    let ascii = state.ascii;
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
    let cores = m.cpu_per_core.len().max(1) as f64;
    let load_ratio = (m.load_avg.0 / cores).clamp(0.0, 1.5) / 1.5;

    if area.width >= 72 {
        let row = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(25),
                Constraint::Percentage(25),
                Constraint::Percentage(25),
                Constraint::Percentage(25),
            ])
            .split(area);
        frame.render_widget(
            gauge(
                theme,
                "CPU",
                f64::from(m.cpu_total) / 100.0,
                format!("{:.1}%", m.cpu_total),
                ascii,
            ),
            row[0],
        );
        frame.render_widget(
            gauge(
                theme,
                "Memory",
                mem_ratio,
                format!(
                    "{} / {}",
                    format_bytes(m.memory_used),
                    format_bytes(m.memory_total)
                ),
                ascii,
            ),
            row[1],
        );
        frame.render_widget(
            gauge(
                theme,
                "Swap",
                swap_ratio,
                if m.swap_total == 0 {
                    "—".into()
                } else {
                    format!(
                        "{} / {}",
                        format_bytes(m.swap_used),
                        format_bytes(m.swap_total)
                    )
                },
                ascii,
            ),
            row[2],
        );
        frame.render_widget(
            gauge(
                theme,
                "Load",
                load_ratio,
                format!("{:.2} / {:.0}c", m.load_avg.0, cores),
                ascii,
            ),
            row[3],
        );
    } else {
        let row = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(34),
                Constraint::Percentage(33),
                Constraint::Percentage(33),
            ])
            .split(area);
        frame.render_widget(
            gauge(
                theme,
                "CPU",
                f64::from(m.cpu_total) / 100.0,
                format!("{:.1}%", m.cpu_total),
                ascii,
            ),
            row[0],
        );
        frame.render_widget(
            gauge(
                theme,
                "Memory",
                mem_ratio,
                format!(
                    "{} / {}",
                    format_bytes(m.memory_used),
                    format_bytes(m.memory_total)
                ),
                ascii,
            ),
            row[1],
        );
        frame.render_widget(
            gauge(
                theme,
                "Swap / load",
                swap_ratio.max(load_ratio),
                format!("sw {}  ld {:.2}", format_bytes(m.swap_used), m.load_avg.0),
                ascii,
            ),
            row[2],
        );
    }
}

fn draw_history(frame: &mut Frame<'_>, area: Rect, state: &AppState, theme: Theme) {
    let row = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);
    let cpu_hist: Vec<u64> = state
        .history
        .cpu
        .iter()
        .map(|v| (*v as u64).min(100))
        .collect();
    let mem_hist: Vec<u64> = state
        .history
        .memory
        .iter()
        .map(|v| (*v as u64).min(100))
        .collect();
    frame.render_widget(sparkline(theme, "CPU", &cpu_hist, state.ascii), row[0]);
    frame.render_widget(sparkline(theme, "MEM", &mem_hist, state.ascii), row[1]);
}

fn draw_network(frame: &mut Frame<'_>, area: Rect, state: &AppState, theme: Theme) {
    let m = &state.metrics;
    let ascii = state.ascii;
    let rx_hist = normalize_rate_hist(state.history.net_rx.iter().copied());
    let tx_hist = normalize_rate_hist(state.history.net_tx.iter().copied());
    let rx_peak = state
        .history
        .net_rx
        .iter()
        .copied()
        .fold(0.0_f64, f64::max)
        .max(1.0);
    let tx_peak = state
        .history
        .net_tx
        .iter()
        .copied()
        .fold(0.0_f64, f64::max)
        .max(1.0);
    let rx_ratio = (m.net_rx_bps / rx_peak).clamp(0.0, 1.0);
    let tx_ratio = (m.net_tx_bps / tx_peak).clamp(0.0, 1.0);

    if area.height >= 6 && area.width >= 56 {
        // Rate gauges + trend sparklines stacked per direction.
        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(area);
        for (col, title, rate, ratio, hist) in [
            (
                cols[0],
                "RX",
                format_rate(m.net_rx_bps),
                rx_ratio,
                &rx_hist as &[u64],
            ),
            (
                cols[1],
                "TX",
                format_rate(m.net_tx_bps),
                tx_ratio,
                &tx_hist as &[u64],
            ),
        ] {
            let rows = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(3), Constraint::Min(2)])
                .split(col);
            frame.render_widget(gauge(theme, title, ratio, rate.clone(), ascii), rows[0]);
            frame.render_widget(
                sparkline(theme, format!("{title} hist"), hist, ascii),
                rows[1],
            );
        }
    } else if area.width >= 56 {
        // Compact: relative rate gauge + sparkline side by side.
        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(25),
                Constraint::Percentage(25),
                Constraint::Percentage(25),
                Constraint::Percentage(25),
            ])
            .split(area);
        frame.render_widget(
            gauge(theme, "RX", rx_ratio, format_rate(m.net_rx_bps), ascii),
            cols[0],
        );
        frame.render_widget(sparkline(theme, "RX", &rx_hist, ascii), cols[1]);
        frame.render_widget(
            gauge(theme, "TX", tx_ratio, format_rate(m.net_tx_bps), ascii),
            cols[2],
        );
        frame.render_widget(sparkline(theme, "TX", &tx_hist, ascii), cols[3]);
    } else {
        let row = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(area);
        frame.render_widget(
            sparkline(
                theme,
                format!("RX {}", format_rate(m.net_rx_bps)),
                &rx_hist,
                ascii,
            ),
            row[0],
        );
        frame.render_widget(
            sparkline(
                theme,
                format!("TX {}", format_rate(m.net_tx_bps)),
                &tx_hist,
                ascii,
            ),
            row[1],
        );
    }
}

fn draw_disks_sensors(frame: &mut Frame<'_>, area: Rect, state: &AppState, theme: Theme) {
    let m = &state.metrics;
    let ascii = state.ascii;
    if area.height >= 6 && !m.disks.is_empty() && area.width >= 40 {
        let disk_n = ((area.height.saturating_sub(2)) / 3).clamp(1, 3) as usize;
        let take = m.disks.len().min(disk_n);
        let mut constraints: Vec<Constraint> = (0..take).map(|_| Constraint::Length(3)).collect();
        constraints.push(Constraint::Min(1));
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .split(area);

        for (i, d) in m.disks.iter().take(take).enumerate() {
            let ratio = if d.total == 0 {
                0.0
            } else {
                d.used as f64 / d.total as f64
            };
            let mount = sanitize_path_display(std::path::Path::new(&d.mount_point));
            let title = truncate_width(&format!("{mount} {}", d.name), 24);
            frame.render_widget(
                gauge(
                    theme,
                    title,
                    ratio,
                    format!(
                        "{:.0}%  {} / {}",
                        ratio * 100.0,
                        format_bytes(d.used),
                        format_bytes(d.total)
                    ),
                    ascii,
                ),
                rows[i],
            );
        }

        let mut extra = vec![Line::from(format!(
            "Σ rx {} · tx {}",
            format_bytes(m.net_rx_bytes),
            format_bytes(m.net_tx_bytes),
        ))];
        if !m.cpu_per_core.is_empty() && area.width > 60 {
            let cores: String = m
                .cpu_per_core
                .iter()
                .enumerate()
                .map(|(i, c)| format!("c{i}:{c:.0}%"))
                .collect::<Vec<_>>()
                .join(" ");
            extra.push(Line::from(truncate_width(&cores, area.width as usize)));
        }
        if m.temperatures.is_empty() {
            extra.push(Line::from(Span::styled("temps —", theme.muted())));
        } else {
            let temps: String = m
                .temperatures
                .iter()
                .map(|t| format!("{}:{:.0}C", t.label, t.celsius))
                .collect::<Vec<_>>()
                .join("  ");
            extra.push(Line::from(truncate_width(&temps, area.width as usize)));
        }
        frame.render_widget(
            Paragraph::new(extra).block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("I/O · sensors")
                    .border_style(theme.border()),
            ),
            rows[take],
        );
    } else {
        let mut lines = Vec::new();
        if area.height < 6 || m.disks.is_empty() {
            lines.push(Line::from(format!(
                "net {} / {}",
                format_rate(m.net_rx_bps),
                format_rate(m.net_tx_bps),
            )));
        }
        for d in m.disks.iter().take(4) {
            let pct = if d.total == 0 {
                0.0
            } else {
                d.used as f64 / d.total as f64 * 100.0
            };
            let mount = sanitize_path_display(std::path::Path::new(&d.mount_point));
            lines.push(Line::from(truncate_width(
                &format!(
                    "{mount}  {:.0}%  {} / {}",
                    pct,
                    format_bytes(d.used),
                    format_bytes(d.total)
                ),
                area.width.saturating_sub(2) as usize,
            )));
        }
        if m.temperatures.is_empty() {
            lines.push(Line::from(Span::styled("temps —", theme.muted())));
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
                    .title("Disks / sensors")
                    .border_style(theme.border()),
            ),
            area,
        );
    }
}

/// Normalize byte rates to 0..=100 relative to the peak in the window.
fn normalize_rate_hist(values: impl Iterator<Item = f64>) -> Vec<u64> {
    let vals: Vec<f64> = values.map(|v| v.max(0.0)).collect();
    let max = vals.iter().copied().fold(0.0_f64, f64::max).max(1.0);
    vals.iter()
        .map(|v| ((*v / max) * 100.0).round() as u64)
        .collect()
}

fn empty(s: &str) -> &str {
    if s.is_empty() {
        "—"
    } else {
        s
    }
}

fn avail(v: bool) -> &'static str {
    if v {
        "ok"
    } else {
        "—"
    }
}
