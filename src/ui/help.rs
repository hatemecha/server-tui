use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

use crate::ui::theme::Theme;
use crate::APP_NAME;

pub fn draw_help(frame: &mut Frame<'_>, area: Rect, theme: Theme) {
    let text = format!(
        "{APP_NAME} — help\n\n\
Screens: 1 Dashboard  2 Processes  3 Services  4 Logs  5 Storage  6 Diagnostics  7 Settings\n\
Global: Tab enter content / cycle panes  / filter (list screens)  r refresh  g glossary  ? help  q quit  Esc cancel\n\n\
Filter: / on Processes, Services, Logs, Storage, Diagnostics, Glossary.\n\
  Type to filter; Enter keeps filter; Esc clears. Dashboard has no list filter.\n\n\
Processes: t SIGTERM  K SIGKILL  z STOP  Z CONT  s sort  c full command  T tree  F follow\n\
Services:  s start  x stop  r refresh  R restart  u reload  e enable  d disable  l logs  f failed\n\
Logs:      f follow  n/N next/prev match  p min priority  w wrap\n\
Storage:   Enter open  Backspace parent  s sort  a apparent/disk  x stay-on-fs  Esc cancel scan\n\
Diagnostics: Enter open related screen  o report  a acknowledge  r refresh\n\
Settings:  Tab sections↔options  j/k move  Space toggle/activate  ←/→ cycle  S save\n\n\
CLI: server-tui doctor [--report|--json|--include-sensitive|--demo]\n\
Modes: --demo  --read-only  --no-color  --ascii\n\
Config: ~/.config/server-tui/config.toml\n\n\
? keybindings · g glossary\n\
Esc closes."
    );
    let p = Paragraph::new(text).wrap(Wrap { trim: false }).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Help")
            .border_style(theme.accent()),
    );
    frame.render_widget(p, area);
}
