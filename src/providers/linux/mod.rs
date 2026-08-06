pub mod journal;
pub mod metrics;
pub mod processes;
pub mod storage;
pub mod systemd;

use crate::actions::{LinuxAdminExecutor, ReadOnlyExecutor};
use crate::providers::traits::ProviderBundle;

use self::journal::LinuxLogProvider;
use self::metrics::LinuxMetricsProvider;
use self::processes::LinuxProcessProvider;
use self::storage::LinuxStorageProvider;
use self::systemd::LinuxServiceProvider;

pub fn linux_bundle(read_only: bool) -> ProviderBundle {
    let admin: Box<dyn crate::providers::AdministrativeExecutor> = if read_only {
        Box::new(ReadOnlyExecutor)
    } else {
        Box::new(LinuxAdminExecutor::new())
    };
    ProviderBundle {
        metrics: Box::new(LinuxMetricsProvider::new()),
        processes: Box::new(LinuxProcessProvider::new()),
        services: Box::new(LinuxServiceProvider::new()),
        logs: Box::new(LinuxLogProvider),
        storage: Box::new(LinuxStorageProvider),
        admin,
    }
}
