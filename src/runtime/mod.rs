//! Application runtime: event loop, pollers, side effects, privilege.

mod app_loop;
mod effects;
mod privilege;

pub use app_loop::run_loop;
pub use effects::cleanup_old_reports;
pub use privilege::{escalate_sudo_systemctl, TerminalSuspension};
