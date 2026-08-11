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
pub use side_effect::SideEffect;
