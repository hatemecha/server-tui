//! Pure diagnostic rule modules. No FS / D-Bus / Command / Tokio / network.

mod boot;
mod clock;
mod common;
mod coredumps;
mod etc_meta;
mod failed_services;
mod journal;
mod oom;
mod pressure_psi;
mod pstore;
mod resource_pressure;
mod smart;
mod temperature;

pub use boot::rule_previous_boot;
pub use clock::rule_clock_sync;
pub use coredumps::rule_coredumps;
pub use etc_meta::rule_etc_meta;
pub use failed_services::rule_failed_services;
pub use journal::rule_journal_critical;
pub use oom::rule_oom;
pub use pressure_psi::rule_psi;
pub use pstore::rule_pstore;
pub use resource_pressure::rule_resource_pressure;
pub use smart::rule_smart;
pub use temperature::rule_temperature;
