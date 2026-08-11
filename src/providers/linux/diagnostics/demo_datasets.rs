//! Demo datasets A–H for synthetic diagnostics.

use crate::model::{
    ClockSyncSnapshot, CoredumpEntry, DiagnosticSnapshot, FailedUnitSnapshot, JournalCriticalGroup,
    OomEvent, PsiSnapshot, ResourcePressureSnapshot, TempSnapshot,
};

#[derive(Debug, Clone, Copy)]
pub enum DemoDataset {
    AHealthy,
    BFailedService,
    COom,
    DTemp,
    EPressure,
    FCoredump,
    GClock,
    HMixed,
}

impl DemoDataset {
    pub fn from_tick(t: u64) -> Self {
        match t % 8 {
            0 => Self::AHealthy,
            1 => Self::BFailedService,
            2 => Self::COom,
            3 => Self::DTemp,
            4 => Self::EPressure,
            5 => Self::FCoredump,
            6 => Self::GClock,
            _ => Self::HMixed,
        }
    }

    /// One-shot CLI demo (`doctor --demo` / `support --demo`): interesting findings.
    /// Interactive TUI keeps `from_tick` cycling so the UI shows variety over time.
    pub fn for_cli_demo() -> Self {
        Self::HMixed
    }

    pub fn build(self) -> DiagnosticSnapshot {
        let mut snap = DiagnosticSnapshot {
            systemd_observable: true,
            journal_observable: true,
            boot_id: Some("demo-boot".into()),
            ..DiagnosticSnapshot::default()
        };
        match self {
            Self::AHealthy => {}
            Self::BFailedService => {
                snap.failed_units.push(FailedUnitSnapshot {
                    unit: "broken-demo.service".into(),
                    active_state: "failed".into(),
                    sub_state: "failed".into(),
                    description: "Intentionally failed demo unit".into(),
                });
            }
            Self::COom => {
                snap.oom_events.push(OomEvent {
                    process: "hog".into(),
                    message: "Killed process 9999 (hog)".into(),
                });
            }
            Self::DTemp => {
                snap.temperatures.push(TempSnapshot {
                    label: "CPU".into(),
                    celsius: 94.0,
                });
            }
            Self::EPressure => {
                snap.resource_pressure = Some(ResourcePressureSnapshot {
                    mem_used_pct: 97.0,
                    swap_used_pct: 85.0,
                    load1: 20.0,
                    n_cpus: 2,
                    disk_max_used_pct: 96.0,
                });
                snap.psi = Some(PsiSnapshot {
                    memory_some_avg10: 25.0,
                    memory_full_avg10: 12.0,
                    cpu_some_avg10: 5.0,
                    io_some_avg10: 1.0,
                });
            }
            Self::FCoredump => {
                snap.coredumps.push(CoredumpEntry {
                    exe: "/usr/bin/demo-crash".into(),
                    signal: Some("SIGSEGV".into()),
                    timestamp: "demo".into(),
                });
            }
            Self::GClock => {
                snap.clock_sync = Some(ClockSyncSnapshot {
                    ntp_synchronized: Some(false),
                    system_time_status: "NTP synchronized: no".into(),
                });
            }
            Self::HMixed => {
                snap.failed_units.push(FailedUnitSnapshot {
                    unit: "broken-demo.service".into(),
                    active_state: "failed".into(),
                    sub_state: "failed".into(),
                    description: "demo".into(),
                });
                snap.temperatures.push(TempSnapshot {
                    label: "CPU".into(),
                    celsius: 91.0,
                });
                snap.journal_critical.push(JournalCriticalGroup {
                    unit: "nginx.service".into(),
                    count: 3,
                    sample_message: "demo critical".into(),
                });
            }
        }
        snap
    }
}
