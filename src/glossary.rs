//! In-app glossary: screens, modes, health, and terms the UI surfaces.

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph, Wrap};

use crate::app::action::Screen;
use crate::app::state::AppState;
use crate::sanitize::truncate_width;
use crate::theme::Theme;
use crate::APP_NAME;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandRisk {
    ReadOnly,
    Privileged,
    Destructive,
}

impl CommandRisk {
    pub fn label(self) -> &'static str {
        match self {
            Self::ReadOnly => "ro",
            Self::Privileged => "priv",
            Self::Destructive => "dest",
        }
    }
}

#[derive(Debug, Clone)]
pub struct GlossaryTerm {
    pub term: &'static str,
    pub definition: &'static str,
    pub aliases: &'static [&'static str],
    /// Screens where this idea shows up (used for contextual ordering).
    pub screens: &'static [Screen],
}

#[derive(Debug, Clone)]
pub struct CommandRef {
    pub command: &'static str,
    pub purpose: &'static str,
    pub risk: CommandRisk,
}

pub fn terms() -> &'static [GlossaryTerm] {
    &[
        GlossaryTerm {
            term: "screens",
            definition: "1 Dashboard · 2 Processes · 3 Services · 4 Logs · 5 Storage · 6 Diagnostics · 7 Settings. Digit keys always jump screens.",
            aliases: &["1", "2", "3", "4", "5", "6", "navigation"],
            screens: &[Screen::Dashboard],
        },
        GlossaryTerm {
            term: "panes",
            definition: "Nav · Content · Details. Tab / Shift+Tab enters content and cycles panes; accent border marks the active pane. Digits switch screens.",
            aliases: &["tab", "focus"],
            screens: &[
                Screen::Dashboard,
                Screen::Processes,
                Screen::Services,
                Screen::Logs,
                Screen::Storage,
                Screen::Diagnostics,
            ],
        },
        GlossaryTerm {
            term: "search",
            definition: "/ filters the current list (name, description, path, evidence, …). Live as you type; Esc clears. Not on Dashboard.",
            aliases: &["/", "filter"],
            screens: &[
                Screen::Processes,
                Screen::Services,
                Screen::Logs,
                Screen::Storage,
                Screen::Diagnostics,
            ],
        },
        GlossaryTerm {
            term: "help vs glossary",
            definition: "? keybindings. g terms and modes. Esc closes either overlay.",
            aliases: &["?", "g", "help"],
            screens: &[Screen::Dashboard],
        },
        GlossaryTerm {
            term: "confirm",
            definition: "Destructive actions: filled Yes/Cancel buttons, focus defaults to Cancel. Enter activates focus; y / n / Esc. ←/→ or Tab moves focus.",
            aliases: &["yes", "cancel", "y", "n"],
            screens: &[Screen::Processes, Screen::Services],
        },
        GlossaryTerm {
            term: "--demo",
            definition: "Synthetic providers only. Badge: DEMO.",
            aliases: &["demo", "synthetic"],
            screens: &[Screen::Dashboard],
        },
        GlossaryTerm {
            term: "--read-only",
            definition: "Blocks SIGTERM/SIGKILL and systemd admin. Lists and filters still work. Badge: READ ONLY.",
            aliases: &["readonly", "ro"],
            screens: &[Screen::Processes, Screen::Services],
        },
        GlossaryTerm {
            term: "doctor",
            definition: "CLI: same diagnostics pipeline (--report/--json/--demo). Exit 0 Ok · 1 Warning · 2 Critical · 3 Unknown.",
            aliases: &["cli", "exit code"],
            screens: &[Screen::Diagnostics],
        },
        GlossaryTerm {
            term: "Dashboard",
            definition: "Host snapshot: CPU, memory, swap, load, net rates, disks, health.",
            aliases: &["host", "metrics"],
            screens: &[Screen::Dashboard],
        },
        GlossaryTerm {
            term: "Processes",
            definition: "Process table with sort and /. t SIGTERM · K SIGKILL (confirm). PIDs 0/1/self refused.",
            aliases: &["pid", "signal", "kill"],
            screens: &[Screen::Processes],
        },
        GlossaryTerm {
            term: "Services",
            definition: "systemd units from a known-unit registry. s/x/r/R/e/d admin; l → Logs; f failed-only.",
            aliases: &["systemd", "unit"],
            screens: &[Screen::Services],
        },
        GlossaryTerm {
            term: "Logs",
            definition: "journal entries (optional unit scope). / text filter; n/N matches; f follow; p min priority.",
            aliases: &["journal", "journalctl"],
            screens: &[Screen::Logs],
        },
        GlossaryTerm {
            term: "Storage",
            definition: "Directory scan under --scan-path (never /). / filters names; Esc cancels scan. No delete/mount.",
            aliases: &["scan", "disk", "directory"],
            screens: &[Screen::Storage],
        },
        GlossaryTerm {
            term: "Diagnostics",
            definition: "Probe → snapshot → evaluate → findings. Prefer Unknown over false alarms. r re-run · o report · a ack · Enter deep link.",
            aliases: &["findings", "probe", "evaluate"],
            screens: &[Screen::Diagnostics, Screen::Dashboard],
        },
        GlossaryTerm {
            term: "health",
            definition: "Ok / Warning / Critical / Unknown. Unknown = probes missing, not healthy.",
            aliases: &["status", "unknown"],
            screens: &[Screen::Diagnostics, Screen::Dashboard],
        },
        GlossaryTerm {
            term: "severity",
            definition: "Finding urgency (Info → Critical). Independent of confidence.",
            aliases: &["critical", "warning", "info"],
            screens: &[Screen::Diagnostics],
        },
        GlossaryTerm {
            term: "confidence",
            definition: "Evaluator certainty. Low/missing evidence → silence or Unknown.",
            aliases: &["sure", "evidence"],
            screens: &[Screen::Diagnostics],
        },
        GlossaryTerm {
            term: "deep link",
            definition: "Finding target opens another screen; may prefill that screen's / filter.",
            aliases: &["target", "enter"],
            screens: &[Screen::Diagnostics],
        },
        GlossaryTerm {
            term: "acknowledge",
            definition: "Persist finding id as seen (XDG state.toml). List mark only.",
            aliases: &["ack", "a"],
            screens: &[Screen::Diagnostics],
        },
        GlossaryTerm {
            term: "suggested check",
            definition: "Finding hint text / example command. Never executed here.",
            aliases: &["hint", "check"],
            screens: &[Screen::Diagnostics],
        },
        GlossaryTerm {
            term: "unit",
            definition: "systemd unit (*.service, …): active state + unit-file state (enabled/disabled/static/masked).",
            aliases: &["systemd", "service", "enabled", "masked"],
            screens: &[Screen::Services, Screen::Diagnostics],
        },
        GlossaryTerm {
            term: "journal",
            definition: "journald source for Logs and some diagnostic rules (via journalctl).",
            aliases: &["journald", "journalctl"],
            screens: &[Screen::Logs, Screen::Diagnostics],
        },
        GlossaryTerm {
            term: "SIGTERM / SIGKILL",
            definition: "Term (polite) / Kill (forced). Confirm required; --read-only blocks.",
            aliases: &["term", "kill", "signal"],
            screens: &[Screen::Processes],
        },
        GlossaryTerm {
            term: "OOM / PSI / load",
            definition: "May flag OOM evidence, PSI pressure, or load vs CPU count when probes succeed.",
            aliases: &["memory", "pressure", "loadavg"],
            screens: &[Screen::Diagnostics, Screen::Dashboard],
        },
        GlossaryTerm {
            term: "coredump / pstore",
            definition: "Crash evidence (coredumpctl; /sys/fs/pstore). Missing tools degrade probe.",
            aliases: &["crash", "panic", "coredumpctl"],
            screens: &[Screen::Diagnostics],
        },
    ]
}

/// External command hints shown in-app; not executed by server-tui.
pub fn commands() -> &'static [CommandRef] {
    &[
        CommandRef {
            command: "server-tui --demo",
            purpose: "UI with synthetic data",
            risk: CommandRisk::ReadOnly,
        },
        CommandRef {
            command: "server-tui --read-only",
            purpose: "Inspect without admin actions",
            risk: CommandRisk::ReadOnly,
        },
        CommandRef {
            command: "server-tui doctor --demo",
            purpose: "Non-interactive diagnostics",
            risk: CommandRisk::ReadOnly,
        },
        CommandRef {
            command: "systemctl status UNIT",
            purpose: "Related host check",
            risk: CommandRisk::ReadOnly,
        },
        CommandRef {
            command: "journalctl -u UNIT -n 50",
            purpose: "Related host check",
            risk: CommandRisk::ReadOnly,
        },
        CommandRef {
            command: "systemctl restart UNIT",
            purpose: "Services restart equivalent",
            risk: CommandRisk::Privileged,
        },
        CommandRef {
            command: "kill -TERM/-KILL PID",
            purpose: "Processes t/K equivalent",
            risk: CommandRisk::Destructive,
        },
    ]
}

fn contains_ci(hay: &str, needle: &str) -> bool {
    hay.to_lowercase().contains(needle)
}

pub fn term_matches(term: &GlossaryTerm, query: &str) -> bool {
    if query.is_empty() {
        return true;
    }
    let q = query.to_lowercase();
    contains_ci(term.term, &q)
        || contains_ci(term.definition, &q)
        || term.aliases.iter().any(|a| contains_ci(a, &q))
}

pub fn command_matches(cmd: &CommandRef, query: &str) -> bool {
    if query.is_empty() {
        return true;
    }
    let q = query.to_lowercase();
    contains_ci(cmd.command, &q)
        || contains_ci(cmd.purpose, &q)
        || contains_ci(cmd.risk.label(), &q)
}

pub fn filtered_terms<'a>(screen: Screen, query: &str) -> Vec<&'a GlossaryTerm> {
    let mut ordered: Vec<&GlossaryTerm> =
        terms().iter().filter(|t| term_matches(t, query)).collect();
    ordered.sort_by_key(|t| if t.screens.contains(&screen) { 0 } else { 1 });
    ordered
}

pub fn filtered_commands<'a>(query: &str) -> Vec<&'a CommandRef> {
    commands()
        .iter()
        .filter(|c| command_matches(c, query))
        .collect()
}

pub fn draw(frame: &mut Frame<'_>, area: Rect, state: &AppState, theme: Theme) {
    let screen = state.screen;
    let query = state.current_search();
    let searching = state.searching;
    let ordered = filtered_terms(screen, query);
    let cmds = filtered_commands(query);

    let selected = if ordered.is_empty() {
        0
    } else {
        state.glossary_selected().min(ordered.len() - 1)
    };

    let title = if searching {
        format!("Glossary ({})  /{query}_", ordered.len())
    } else if !query.is_empty() {
        format!("Glossary ({})  /{query}", ordered.len())
    } else {
        format!("Glossary ({})", ordered.len())
    };

    let outer = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .border_style(theme.accent());
    let inner = outer.inner(area);
    frame.render_widget(outer, area);

    if inner.width < 20 || inner.height < 4 {
        let msg = if ordered.is_empty() {
            "(no match)"
        } else {
            ordered.get(selected).map(|t| t.term).unwrap_or("?")
        };
        frame.render_widget(Paragraph::new(msg).style(theme.muted()), inner);
        return;
    }

    let list_w = (inner.width / 3).clamp(14, 28);
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(list_w), Constraint::Min(20)])
        .split(inner);

    let list_width = cols[0].width.saturating_sub(2) as usize;
    let items: Vec<ListItem> = if ordered.is_empty() {
        vec![ListItem::new(Span::styled("(no match)", theme.muted()))]
    } else {
        ordered
            .iter()
            .enumerate()
            .map(|(i, t)| {
                let mark = if t.screens.contains(&screen) {
                    "•"
                } else {
                    " "
                };
                let label = truncate_width(&format!("{mark} {}", t.term), list_width.max(1));
                let item = ListItem::new(label);
                if i == selected {
                    item.style(theme.highlight())
                } else {
                    item
                }
            })
            .collect()
    };

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Terms")
            .border_style(theme.border()),
    );
    frame.render_widget(list, cols[0]);

    let detail_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(3),
            Constraint::Length(cmds_pane_h(cmds.len(), cols[1].height)),
        ])
        .split(cols[1]);

    let mut detail_lines: Vec<Line> = Vec::new();
    if let Some(t) = ordered.get(selected) {
        detail_lines.push(Line::from(Span::styled(t.term.to_string(), theme.title())));
        detail_lines.push(Line::from(""));
        detail_lines.push(Line::from(t.definition.to_string()));
        if !t.aliases.is_empty() {
            detail_lines.push(Line::from(""));
            detail_lines.push(Line::from(Span::styled(
                format!("aka {}", t.aliases.join(", ")),
                theme.muted(),
            )));
        }
    } else {
        detail_lines.push(Line::from(Span::styled("No terms match.", theme.muted())));
    }

    let hint = if searching {
        format!("{APP_NAME}  Esc clears filter")
    } else if !query.is_empty() {
        format!("{APP_NAME}  Esc clears · / edit")
    } else {
        format!("{APP_NAME}  / filter · j/k select · Esc close")
    };
    detail_lines.push(Line::from(""));
    detail_lines.push(Line::from(Span::styled(hint, theme.muted())));

    frame.render_widget(
        Paragraph::new(detail_lines)
            .wrap(Wrap { trim: false })
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Definition")
                    .border_style(theme.border()),
            ),
        detail_chunks[0],
    );

    if detail_chunks[1].height >= 2 {
        let cmd_width = detail_chunks[1].width.saturating_sub(2) as usize;
        let mut cmd_lines: Vec<Line> = Vec::new();
        if cmds.is_empty() {
            cmd_lines.push(Line::from(Span::styled("(none)", theme.muted())));
        } else {
            for c in cmds
                .iter()
                .take(detail_chunks[1].height.saturating_sub(2) as usize)
            {
                let row = format!("[{}] {}  —  {}", c.risk.label(), c.command, c.purpose);
                cmd_lines.push(Line::from(truncate_width(&row, cmd_width.max(1))));
            }
        }
        frame.render_widget(
            Paragraph::new(cmd_lines).block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(format!("Commands ({})", cmds.len()))
                    .border_style(theme.border()),
            ),
            detail_chunks[1],
        );
    }
}

fn cmds_pane_h(cmd_count: usize, available: u16) -> u16 {
    if available < 8 {
        0
    } else {
        let want = (cmd_count.min(6) as u16).saturating_add(2);
        want.min(available / 2).max(3)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filter_matches_product_terms() {
        let hits = filtered_terms(Screen::Dashboard, "demo");
        assert!(hits.iter().any(|t| t.term.contains("demo")));
    }

    #[test]
    fn filter_matches_aliases_and_body() {
        let hits = filtered_terms(Screen::Diagnostics, "ack");
        assert!(hits.iter().any(|t| t.term.contains("acknowledge")));
        let body = filtered_terms(Screen::Diagnostics, "silence");
        assert!(body.iter().any(|t| t.term.contains("confidence")));
    }

    #[test]
    fn empty_query_returns_all() {
        assert_eq!(filtered_terms(Screen::Logs, "").len(), terms().len());
    }

    #[test]
    fn command_filter_matches_purpose() {
        let hits = filtered_commands("synthetic");
        assert!(hits.iter().any(|c| c.command.contains("--demo")));
    }
}
