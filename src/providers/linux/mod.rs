pub mod diagnostics;
pub mod journal;
pub mod metrics;
pub mod proc_details;
pub mod processes;
pub mod storage;
pub mod systemd;

use crate::actions::{LinuxAdminExecutor, ReadOnlyExecutor};
use crate::config::Config;
use crate::model::UnitRegistry;
use crate::providers::traits::ProviderBundle;

use self::diagnostics::LinuxDiagnosticProbes;
use self::journal::LinuxLogProvider;
use self::metrics::LinuxMetricsProvider;
use self::processes::LinuxProcessProvider;
use self::storage::LinuxStorageProvider;
use self::systemd::LinuxServiceProvider;

pub fn linux_bundle(read_only: bool, config: &Config) -> ProviderBundle {
    let registry = UnitRegistry::new();
    let admin: Box<dyn crate::providers::AdministrativeExecutor> = if read_only {
        Box::new(ReadOnlyExecutor)
    } else {
        Box::new(LinuxAdminExecutor::new(registry.clone()))
    };
    ProviderBundle {
        metrics: Box::new(LinuxMetricsProvider::new(config)),
        processes: Box::new(LinuxProcessProvider::new()),
        services: Box::new(LinuxServiceProvider::new(registry)),
        logs: Box::new(LinuxLogProvider),
        storage: Box::new(LinuxStorageProvider),
        admin,
        diagnostics: Box::new(LinuxDiagnosticProbes::new()),
    }
}
