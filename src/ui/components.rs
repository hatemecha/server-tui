//! Shared UI widgets and contextual key hints.

use ratatui::prelude::*;
use ratatui::symbols;
use ratatui::widgets::{Block, Borders, Gauge, Paragraph, Sparkline};

use crate::app::state::AppState;
use crate::ui::theme::Theme;

/// ASCII-safe bar ladder for `--ascii` terminals.
const ASCII_BAR_SET: symbols::bar::Set = symbols::bar::Set {
    full: "#",
    seven_eighths: "#",
    three_quarters: "=",
    five_eighths: "=",
    half: "=",
    three_eighths: "-",
    one_quarter: "-",
    one_eighth: ".",
    empty: " ",
};

pub fn gauge(
    theme: Theme,
    title: impl Into<String>,
    ratio: f64,
    label: impl Into<String>,
    ascii: bool,
) -> Gauge<'static> {
    let ratio = ratio.clamp(0.0, 1.0);
    let style = if ratio > 0.9 {
        theme.err()
    } else if ratio > 0.75 {
        theme.warn()
    } else {
        theme.ok()
    };
    Gauge::default()
        .block(Block::default().borders(Borders::ALL).title(title.into()))
        .gauge_style(style)
        .ratio(ratio)
        .label(label.into())
        .use_unicode(!ascii)
}

pub fn sparkline<'a>(
    theme: Theme,
    title: impl Into<String>,
    data: &'a [u64],
    ascii: bool,
) -> Sparkline<'a> {
    let mut spark = Sparkline::default()
        .block(Block::default().borders(Borders::ALL).title(title.into()))
        .style(theme.accent())
        .data(data);
    if ascii {
        spark = spark.bar_set(ASCII_BAR_SET);
    }
    spark
}

#[derive(Debug, Clone, Copy)]
pub struct KeyHint {
    pub key: &'static str,
    pub action: &'static str,
}

pub fn footer_hints(state: &AppState) -> String {
    if state.searching {
        let target = if state.glossary_open() {
            "glossary"
        } else {
            state.screen.label()
        };
        return format!("/{target}: {}_  Enter · Esc", state.current_search());
    }

    let mut hints: Vec<KeyHint> = vec![KeyHint {
        key: "Tab",
        action: "focus",
    }];

    if state.search_supported() {
        hints.push(KeyHint {
            key: "/",
            action: "search",
        });
    }

    hints.extend_from_slice(&[
        KeyHint {
            key: "r",
            action: "refresh",
        },
        KeyHint {
            key: "g",
            action: "glossary",
        },
        KeyHint {
            key: "?",
            action: "help",
        },
        KeyHint {
            key: "q",
            action: "quit",
        },
    ]);

    match state.screen {
        crate::app::Screen::Processes => {
            hints.extend_from_slice(&[
                KeyHint {
                    key: "t",
                    action: "SIGTERM",
                },
                KeyHint {
                    key: "K",
                    action: "SIGKILL",
                },
                KeyHint {
                    key: "s",
                    action: "sort",
                },
            ]);
        }
        crate::app::Screen::Services => {
            hints.extend_from_slice(&[
                KeyHint {
                    key: "s/x/R",
                    action: "start/stop/restart",
                },
                KeyHint {
                    key: "u",
                    action: "reload",
                },
                KeyHint {
                    key: "l",
                    action: "logs",
                },
                KeyHint {
                    key: "f",
                    action: "failed",
                },
            ]);
        }
        crate::app::Screen::Logs => {
            hints.extend_from_slice(&[
                KeyHint {
                    key: "f",
                    action: "follow",
                },
                KeyHint {
                    key: "n/N",
                    action: "match",
                },
                KeyHint {
                    key: "p",
                    action: "priority",
                },
            ]);
        }
        crate::app::Screen::Storage => {
            hints.extend_from_slice(&[
                KeyHint {
                    key: "Enter",
                    action: "open",
                },
                KeyHint {
                    key: "Backspace",
                    action: "up",
                },
                KeyHint {
                    key: "Esc",
                    action: "cancel",
                },
            ]);
        }
        crate::app::Screen::Diagnostics => {
            hints.extend_from_slice(&[
                KeyHint {
                    key: "Enter",
                    action: "open",
                },
                KeyHint {
                    key: "o",
                    action: "report",
                },
                KeyHint {
                    key: "a",
                    action: "ack",
                },
            ]);
        }
        crate::app::Screen::Dashboard => {
            hints.push(KeyHint {
                key: "w",
                action: "wallboard",
            });
        }
        crate::app::Screen::Settings => {
            hints.push(KeyHint {
                key: "j/k",
                action: "section",
            });
        }
    }

    // Show active filter even when not editing.
    if !state.current_search().is_empty()
        && (state.screen.supports_search() || state.glossary_open())
    {
        hints.insert(
            0,
            KeyHint {
                key: "Esc",
                action: "clear-filter",
            },
        );
    }

    format_hints(&hints, state.width)
}

fn format_hints(hints: &[KeyHint], width: u16) -> String {
    let sep = " · ";
    let parts: Vec<String> = hints
        .iter()
        .map(|h| format!("{} {}", h.key, h.action))
        .collect();
    let full = parts.join(sep);
    if width >= 100 {
        return full;
    }
    // Compact: drop lower-priority trailing hints until it fits.
    let budget = width.saturating_sub(4) as usize;
    let mut out = String::new();
    for (i, part) in parts.iter().enumerate() {
        let candidate = if out.is_empty() {
            part.clone()
        } else {
            format!("{out}{sep}{part}")
        };
        if candidate.len() > budget && i > 2 {
            break;
        }
        out = candidate;
    }
    out
}

pub fn keyed_line(theme: Theme, label: &str, value: &str) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("{label}: "), theme.muted()),
        Span::styled(value.to_string(), theme.normal()),
    ])
}

pub fn empty_panel(theme: Theme, msg: &str) -> Paragraph<'_> {
    Paragraph::new(msg).style(theme.muted()).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(theme.border()),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use std::path::PathBuf;

    #[test]
    fn footer_compacts_on_narrow() {
        let mut state = AppState::new(
            Config::default(),
            true,
            true,
            true,
            false,
            PathBuf::from("/tmp"),
        );
        state.screen = crate::app::Screen::Processes;
        state.width = 70;
        let narrow = footer_hints(&state);
        state.width = 120;
        let wide = footer_hints(&state);
        assert!(wide.len() >= narrow.len());
    }

    #[test]
    fn service_footer_uses_capital_r_for_restart() {
        let mut state = AppState::new(
            Config::default(),
            true,
            true,
            true,
            false,
            PathBuf::from("/tmp"),
        );
        state.screen = crate::app::Screen::Services;
        state.width = 160;
        let hints = footer_hints(&state);
        assert!(hints.contains("R") && hints.contains("restart"), "{hints}");
        assert!(
            !hints.contains("s/x/r "),
            "lowercase r must not mean restart: {hints}"
        );
        // Global refresh still present
        assert!(hints.contains("r refresh"), "{hints}");
    }
}
