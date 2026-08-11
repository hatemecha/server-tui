//! Update path: keymap → reducer / events → SideEffect.

mod events;
mod keymap;
mod navigation;
mod reducer;
mod side_effect;

#[cfg(test)]
mod tests;

pub use events::apply_event;
pub use keymap::map_key;
pub use reducer::apply_action;
pub(crate) use side_effect::{
    begin_diagnostics, begin_log_refresh, begin_ppid_map, begin_preview, begin_process_details,
    begin_service_details,
};
pub use side_effect::{ConfigSaveReason, SideEffect};
