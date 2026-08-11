//! Privileged elevation with RAII terminal suspension.

use std::io::{self, Write};
use std::process::Command;

use crate::actions::trusted::{resolve_trusted, run_sudo_systemctl, RealCommandRunner};
use crate::error::AppError;
use crate::model::{ServiceActionKind, UnitRegistry};
use crate::terminal::TerminalGuard;

/// RAII: leave alternate screen for an external prompt; Drop always reenters.
pub struct TerminalSuspension<'a> {
    terminal: &'a mut TerminalGuard,
    active: bool,
}

impl<'a> TerminalSuspension<'a> {
    pub fn leave(terminal: &'a mut TerminalGuard) -> Result<Self, AppError> {
        terminal.leave_for_external()?;
        Ok(Self {
            terminal,
            active: true,
        })
    }

    /// Mark reenter already done (Drop becomes a no-op).
    pub fn disarm(&mut self) {
        self.active = false;
    }
}

impl Drop for TerminalSuspension<'_> {
    fn drop(&mut self) {
        if self.active {
            let _ = self.terminal.reenter();
            self.active = false;
        }
    }
}

/// Leave TUI → interactive `sudo -v` → reenter → `sudo -n systemctl <action> <unit>`.
/// `registry` must be the shared known-unit allowlist (SECURITY.md).
pub async fn escalate_sudo_systemctl(
    terminal: &mut TerminalGuard,
    unit: &str,
    action: ServiceActionKind,
    registry: &UnitRegistry,
) -> Result<(), AppError> {
    let mut guard = TerminalSuspension::leave(terminal)?;
    println!();
    println!("server-tui: elevated systemd action requested.");
    println!("  unit:   {unit}");
    println!("  action: {}", action.label());
    println!("Running `sudo -v` (cached credentials). Ctrl+C to abort.");
    let _ = io::stdout().flush();

    let sudo = resolve_trusted("sudo")?;
    let status = Command::new(&sudo)
        .arg("-v")
        .status()
        .map_err(|e| AppError::Permission(format!("sudo -v failed to start: {e}")))?;
    if !status.success() {
        return Err(AppError::Permission(
            "sudo -v failed or was cancelled".into(),
        ));
    }

    // Re-enter before running non-interactive systemctl under sudo -n.
    let _ = guard.terminal.reenter();
    guard.disarm();

    run_sudo_systemctl(&RealCommandRunner, action, unit, registry)
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// Tiny fake that records leave/reenter without touching the real TTY.
    struct FakeTerm {
        leaves: AtomicUsize,
        reenters: AtomicUsize,
    }

    impl FakeTerm {
        fn leave(&self) {
            self.leaves.fetch_add(1, Ordering::SeqCst);
        }
        fn reenter(&self) {
            self.reenters.fetch_add(1, Ordering::SeqCst);
        }
    }

    struct FakeSuspension<'a> {
        term: &'a FakeTerm,
        active: bool,
    }

    impl<'a> FakeSuspension<'a> {
        fn leave(term: &'a FakeTerm) -> Self {
            term.leave();
            Self { term, active: true }
        }
        fn disarm(&mut self) {
            self.active = false;
        }
    }

    impl Drop for FakeSuspension<'_> {
        fn drop(&mut self) {
            if self.active {
                self.term.reenter();
                self.active = false;
            }
        }
    }

    #[test]
    fn drop_always_reenters() {
        let t = FakeTerm {
            leaves: AtomicUsize::new(0),
            reenters: AtomicUsize::new(0),
        };
        {
            let _g = FakeSuspension::leave(&t);
        }
        assert_eq!(t.leaves.load(Ordering::SeqCst), 1);
        assert_eq!(t.reenters.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn disarm_skips_reenter_on_drop() {
        let t = FakeTerm {
            leaves: AtomicUsize::new(0),
            reenters: AtomicUsize::new(0),
        };
        {
            let mut g = FakeSuspension::leave(&t);
            t.reenter();
            g.disarm();
        }
        assert_eq!(t.leaves.load(Ordering::SeqCst), 1);
        assert_eq!(t.reenters.load(Ordering::SeqCst), 1);
    }
}
