//! Centralized product identity. Change these constants to rename the app.
pub const APP_NAME: &str = "server-tui";
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const BINARY_NAME: &str = "server-tui";
pub const CONFIG_DIR_NAME: &str = "server-tui";

pub mod actions;
pub mod app;
pub mod cli;
pub mod config;
pub mod error;
pub mod model;
pub mod providers;
pub mod sanitize;
pub mod terminal;
pub mod ui;
