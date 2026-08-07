//! Diagnostics: pure evaluator + report helpers.

pub mod evaluator;
pub mod report;

pub use evaluator::{compute_health_status, evaluate};
pub use report::{format_json_report, format_text_report};
