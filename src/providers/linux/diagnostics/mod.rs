//! Linux diagnostic probes — each probe fails independently.

mod boot;
mod clock;
mod config_changes;
mod coredump;
pub mod demo_datasets;
mod journal;
mod pressure;
mod pstore;
mod smart;
mod thermal;

use async_trait::async_trait;

use crate::error::AppError;
use crate::model::{DiagnosticSnapshot, FailedUnitSnapshot};
use crate::providers::linux::systemd::LinuxServiceProvider;
use crate::providers::DiagnosticProbeProvider;
use crate::providers::ServiceProvider;

pub use journal::JournalQuery;

pub struct LinuxDiagnosticProbes;

impl Default for LinuxDiagnosticProbes {
    fn default() -> Self {
        Self
    }
}

impl LinuxDiagnosticProbes {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl DiagnosticProbeProvider for LinuxDiagnosticProbes {
    async fn probe(&self, deep: bool, enable_smart: bool) -> Result<DiagnosticSnapshot, AppError> {
        let mut snap = DiagnosticSnapshot::default();
        let mut degraded = Vec::new();

        let svc = LinuxServiceProvider::new(crate::model::UnitRegistry::new());
        match svc.list_services().await {
            Ok(list) => {
                snap.systemd_observable = true;
                snap.failed_units = list
                    .into_iter()
                    .filter(|s| s.is_failed())
                    .map(|s| FailedUnitSnapshot {
                        unit: s.unit,
                        active_state: s.active_state,
                        sub_state: s.sub_state,
                        description: s.description,
                    })
                    .collect();
            }
            Err(e) => {
                snap.systemd_observable = false;
                degraded.push(format!("systemd: {e}"));
            }
        }

        match journal::probe_journal().await {
            Ok((critical, ooms, journal_ok)) => {
                snap.journal_observable = journal_ok;
                snap.journal_critical = critical;
                snap.oom_events = ooms;
            }
            Err(e) => {
                snap.journal_observable = false;
                degraded.push(format!("journal: {e}"));
            }
        }

        match boot::probe_boot_ids().await {
            Ok((boot, prev_summary)) => {
                snap.boot_id = boot;
                snap.previous_boot_summary = prev_summary;
            }
            Err(e) => degraded.push(format!("boot: {e}")),
        }

        match pstore::probe_pstore() {
            Ok(entries) => snap.pstore_entries = entries,
            Err(e) => degraded.push(format!("pstore: {e}")),
        }

        match coredump::probe_coredumps().await {
            Ok(entries) => snap.coredumps = entries,
            Err(e) => degraded.push(format!("coredump: {e}")),
        }

        match pressure::probe_psi() {
            Ok(psi) => snap.psi = psi,
            Err(e) => degraded.push(format!("psi: {e}")),
        }

        match pressure::probe_resource_pressure() {
            Ok(r) => snap.resource_pressure = r,
            Err(e) => degraded.push(format!("resources: {e}")),
        }

        match thermal::probe_temperatures() {
            Ok(t) => snap.temperatures = t,
            Err(e) => degraded.push(format!("temp: {e}")),
        }

        match clock::probe_clock().await {
            Ok(c) => snap.clock_sync = c,
            Err(e) => degraded.push(format!("clock: {e}")),
        }

        match config_changes::probe_etc_meta() {
            Ok(e) => snap.etc_mtime_notable = e,
            Err(e) => degraded.push(format!("etc: {e}")),
        }

        if deep && enable_smart {
            match smart::probe_smart_readonly().await {
                Ok(disks) => snap.smart_disks = disks,
                Err(e) => degraded.push(format!("smart: {e}")),
            }
        } else if deep && !enable_smart {
            degraded.push("smart: disabled in settings".into());
        }

        snap.probes_degraded = degraded;
        Ok(snap)
    }
}
