//! Application chrome: header, nav, footer.

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap};

use crate::app::action::{FocusPane, Screen};
use crate::app::state::AppState;
use crate::model::format_uptime;
use crate::ui::components::footer_hints;
use crate::ui::theme::Theme;
use crate::APP_NAME;

pub fn draw_shell<F>(frame: &mut Frame<'_>, state: &AppState, mut content: F)
where
    F: FnMut(&mut Frame<'_>, Rect, &AppState),
{
    let theme = Theme::new(state.color);
    let area = frame.area();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(5),
            Constraint::Length(3),
        ])
        .split(area);

    draw_header(frame, chunks[0], state, theme);

    if state.narrow_nav() {
        let mid = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(3)])
            .split(chunks[1]);
        draw_tabs(frame, mid[0], state, theme);
        content(frame, mid[1], state);
    } else {
        let mid = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(16), Constraint::Min(20)])
            .split(chunks[1]);
        draw_nav(frame, mid[0], state, theme);
        content(frame, mid[1], state);
    }

    draw_footer(frame, chunks[2], state, theme);
}

pub fn draw_too_small(frame: &mut Frame<'_>, state: &AppState) {
    let theme = Theme::new(state.color);
    Clear.render(frame.area(), frame.buffer_mut());
    let msg = format!(
        "{APP_NAME}: terminal too small ({}x{}). Need at least 60x12.",
        state.width, state.height
    );
    let p = Paragraph::new(msg)
        .style(theme.warn())
        .wrap(Wrap { trim: true });
    frame.render_widget(p, frame.area());
}

fn draw_header(frame: &mut Frame<'_>, area: Rect, state: &AppState, theme: Theme) {
    let host = if state.metrics.hostname.is_empty() {
        "..."
    } else {
        state.metrics.hostname.as_str()
    };
    let uptime = format_uptime(state.metrics.uptime_seconds);
    let title = format!(
        " {APP_NAME}   host: {host}   uptime: {uptime}   mode: {} ",
        state.mode_label()
    );
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme.border())
        .title(Span::styled(title, theme.title()));
    let status = state.status_message.as_deref().unwrap_or("ready");
    let p = Paragraph::new(status).style(theme.muted()).block(block);
    frame.render_widget(p, area);
}

fn draw_nav(frame: &mut Frame<'_>, area: Rect, state: &AppState, theme: Theme) {
    let items: Vec<ListItem> = Screen::all()
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let label = format!("{} {}", i + 1, s.label());
            let style = if *s == state.screen {
                theme.highlight()
            } else {
                theme.normal()
            };
            ListItem::new(label).style(style)
        })
        .collect();
    let border = if state.focus == FocusPane::Nav {
        theme.accent()
    } else {
        theme.border()
    };
    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Menu")
            .border_style(border),
    );
    frame.render_widget(list, area);
}

fn draw_tabs(frame: &mut Frame<'_>, area: Rect, state: &AppState, theme: Theme) {
    let labels: Vec<Span> = Screen::all()
        .iter()
        .enumerate()
        .flat_map(|(i, s)| {
            let style = if *s == state.screen {
                theme.highlight()
            } else {
                theme.muted()
            };
            [
                Span::styled(format!(" {} {} ", i + 1, s.label()), style),
                Span::raw(" "),
            ]
        })
        .collect();
    let p = Paragraph::new(Line::from(labels)).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(theme.border()),
    );
    frame.render_widget(p, area);
}

fn draw_footer(frame: &mut Frame<'_>, area: Rect, state: &AppState, theme: Theme) {
    let hints = footer_hints(state);
    let p = Paragraph::new(hints).style(theme.muted()).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(theme.border()),
    );
    frame.render_widget(p, area);
}
