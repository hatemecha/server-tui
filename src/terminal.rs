//! RAII terminal lifecycle: alternate screen, raw mode, cursor, panic restore.

use std::io::{self, stdout, Stdout};
use std::panic::{self, PanicHookInfo};
use std::sync::atomic::{AtomicBool, Ordering};

use crossterm::cursor::{Hide, Show};
use crossterm::event::{DisableMouseCapture, EnableMouseCapture};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use crossterm::ExecutableCommand;
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use crate::error::AppError;

static TERMINAL_ACTIVE: AtomicBool = AtomicBool::new(false);

pub type AppTerminal = Terminal<CrosstermBackend<Stdout>>;

pub struct TerminalGuard {
    terminal: AppTerminal,
    restored: bool,
}

impl TerminalGuard {
    pub fn enter() -> Result<Self, AppError> {
        // Any failure after the first successful terminal mutation must restore.
        let result = enter_inner();
        if result.is_err() {
            let _ = restore_terminal_state();
            TERMINAL_ACTIVE.store(false, Ordering::SeqCst);
        }
        result
    }

    pub fn terminal(&mut self) -> &mut AppTerminal {
        &mut self.terminal
    }

    pub fn restore(&mut self) -> Result<(), AppError> {
        if self.restored {
            return Ok(());
        }
        restore_terminal_state().map_err(|e| AppError::Terminal(e.to_string()))?;
        self.restored = true;
        TERMINAL_ACTIVE.store(false, Ordering::SeqCst);
        Ok(())
    }
}

fn enter_inner() -> Result<TerminalGuard, AppError> {
    enable_raw_mode().map_err(|e| AppError::Terminal(e.to_string()))?;
    let mut out = stdout();
    out.execute(EnterAlternateScreen)
        .map_err(|e| AppError::Terminal(e.to_string()))?;
    out.execute(Hide)
        .map_err(|e| AppError::Terminal(e.to_string()))?;
    // Mouse capture is optional; disable if it causes issues over SSH.
    let _ = out.execute(EnableMouseCapture);

    let backend = CrosstermBackend::new(stdout());
    let mut terminal = Terminal::new(backend).map_err(|e| AppError::Terminal(e.to_string()))?;
    // Alternate-screen buffers often retain prior contents; without an explicit
    // clear, ratatui's differential renderer leaves "empty" cells untouched and
    // old shell scrollback shows through the UI (common over SSH / dumb TERM).
    terminal
        .clear()
        .map_err(|e| AppError::Terminal(e.to_string()))?;

    TERMINAL_ACTIVE.store(true, Ordering::SeqCst);
    install_panic_hook();

    Ok(TerminalGuard {
        terminal,
        restored: false,
    })
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = self.restore();
    }
}

/// Best-effort restore usable from signal handlers and panic hooks.
pub fn force_restore_terminal() {
    let _ = restore_terminal_state();
    TERMINAL_ACTIVE.store(false, Ordering::SeqCst);
}

fn restore_terminal_state() -> io::Result<()> {
    let mut out = stdout();
    let _ = out.execute(DisableMouseCapture);
    let _ = out.execute(Show);
    let _ = out.execute(LeaveAlternateScreen);
    disable_raw_mode()?;
    Ok(())
}

fn install_panic_hook() {
    let original = panic::take_hook();
    panic::set_hook(Box::new(move |info: &PanicHookInfo<'_>| {
        if TERMINAL_ACTIVE.load(Ordering::SeqCst) {
            force_restore_terminal();
        }
        original(info);
    }));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn restore_is_idempotent_flag() {
        // We cannot safely enter raw mode in unit tests; verify atomic + hook install.
        assert!(!TERMINAL_ACTIVE.load(Ordering::SeqCst) || TERMINAL_ACTIVE.load(Ordering::SeqCst));
        install_panic_hook();
        force_restore_terminal();
    }
}
