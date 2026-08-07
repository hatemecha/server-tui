//! Diagnostic finding types. Pure data — no I/O.

use serde::{Deserialize, Serialize};

/// Overall host health from required/optional subsystem observation + findings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HealthStatus {
    #[default]
    Unknown,
    Ok,
    Warning,
    Critical,
}

impl HealthStatus {
    pub fn label(self) -> &'static str {
        match self {
            Self::Unknown => "unknown",
            Self::Ok => "ok",
            Self::Warning => "warning",
            Self::Critical => "critical",
        }
    }

    pub fn worse(self, other: Self) -> Self {
        use HealthStatus::*;
        let rank = |h: HealthStatus| match h {
            Unknown => 0,
            Ok => 1,
            Warning => 2,
            Critical => 3,
        };
        if rank(other) > rank(self) {
            other
        } else {
            self
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Info,
    Warning,
    Critical,
}

impl Severity {
    pub fn label(self) -> &'static str {
        match self {
            Self::Info => "info",
            Self::Warning => "warning",
            Self::Critical => "critical",
        }
    }

    pub fn to_health(self) -> HealthStatus {
        match self {
            Self::Info => HealthStatus::Ok,
            Self::Warning => HealthStatus::Warning,
            Self::Critical => HealthStatus::Critical,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Confidence {
    Low,
    Medium,
    High,
}

impl Confidence {
    pub fn label(self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Category {
    Services,
    Journal,
    Memory,
    Storage,
    Temperature,
    Clock,
    Boot,
    CoreDump,
    Pressure,
    Config,
    Hardware,
    Kernel,
    Other,
}

impl Category {
    pub fn label(self) -> &'static str {
        match self {
            Self::Services => "services",
            Self::Journal => "journal",
            Self::Memory => "memory",
            Self::Storage => "storage",
            Self::Temperature => "temperature",
            Self::Clock => "clock",
            Self::Boot => "boot",
            Self::CoreDump => "coredump",
            Self::Pressure => "pressure",
            Self::Config => "config",
            Self::Hardware => "hardware",
            Self::Kernel => "kernel",
            Self::Other => "other",
        }
    }
}

/// Deep-link target into an existing screen with optional search prefill.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiagnosticTarget {
    pub screen: ScreenTarget,
    pub search: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScreenTarget {
    Dashboard,
    Processes,
    Services,
    Logs,
    Storage,
    Diagnostics,
}

impl ScreenTarget {
    pub fn label(self) -> &'static str {
        match self {
            Self::Dashboard => "dashboard",
            Self::Processes => "processes",
            Self::Services => "services",
            Self::Logs => "logs",
            Self::Storage => "storage",
            Self::Diagnostics => "diagnostics",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Evidence {
    pub summary: String,
    pub details: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SuggestedCheck {
    pub description: String,
    /// Informational only — never executed by the app.
    pub command_hint: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Finding {
    pub id: String,
    pub title: String,
    pub summary: String,
    pub severity: Severity,
    pub confidence: Confidence,
    pub category: Category,
    pub evidence: Evidence,
    pub targets: Vec<DiagnosticTarget>,
    pub suggested_check: Option<SuggestedCheck>,
    /// When false, absence of the probe must not invent a finding.
    pub degradable: bool,
}

/// Case-insensitive substring match across finding fields used by `/` search.
pub fn finding_matches(f: &Finding, query: &str) -> bool {
    if query.is_empty() {
        return true;
    }
    let q = query.to_lowercase();
    let hit = |s: &str| s.to_lowercase().contains(&q);
    if hit(&f.id)
        || hit(&f.title)
        || hit(&f.summary)
        || hit(f.category.label())
        || hit(f.severity.label())
        || hit(f.confidence.label())
        || hit(&f.evidence.summary)
    {
        return true;
    }
    if f.evidence.details.iter().any(|d| hit(d)) {
        return true;
    }
    if let Some(check) = &f.suggested_check {
        if hit(&check.description) {
            return true;
        }
        if check.command_hint.as_ref().is_some_and(|h| hit(h)) {
            return true;
        }
    }
    f.targets
        .iter()
        .any(|t| hit(t.screen.label()) || t.search.as_ref().is_some_and(|s| hit(s)))
}

/// Pure snapshot built from in-memory app/probe data for the evaluator.
#[derive(Debug, Clone, Default)]
pub struct DiagnosticSnapshot {
    pub boot_id: Option<String>,
    pub previous_boot_id: Option<String>,
    pub failed_units: Vec<FailedUnitSnapshot>,
    pub journal_critical: Vec<JournalCriticalGroup>,
    pub oom_events: Vec<OomEvent>,
    pub previous_boot_summary: Option<PreviousBootSummary>,
    pub pstore_entries: Vec<PstoreEntry>,
    pub coredumps: Vec<CoredumpEntry>,
    pub psi: Option<PsiSnapshot>,
    pub resource_pressure: Option<ResourcePressureSnapshot>,
    pub temperatures: Vec<TempSnapshot>,
    pub clock_sync: Option<ClockSyncSnapshot>,
    pub etc_mtime_notable: Vec<EtcMetaChange>,
    pub smart_disks: Vec<SmartDiskSnapshot>,
    pub systemd_observable: bool,
    pub journal_observable: bool,
    pub probes_degraded: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct SmartDiskSnapshot {
    pub device: String,
    pub available: bool,
    pub passed: Option<bool>,
    pub uda_crc_error_count: Option<u64>,
    pub prev_uda_crc_error_count: Option<u64>,
    pub summary: String,
    pub details: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct FailedUnitSnapshot {
    pub unit: String,
    pub active_state: String,
    pub sub_state: String,
    pub description: String,
}

#[derive(Debug, Clone)]
pub struct JournalCriticalGroup {
    pub unit: String,
    pub count: usize,
    pub sample_message: String,
}

#[derive(Debug, Clone)]
pub struct OomEvent {
    pub process: String,
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct PreviousBootSummary {
    pub boot_id: String,
    /// True only when strong evidence of unclean shutdown WITHOUT claiming kernel panic.
    pub unclean_hint: bool,
    pub note: String,
}

#[derive(Debug, Clone)]
pub struct PstoreEntry {
    pub name: String,
    pub bytes: u64,
}

#[derive(Debug, Clone)]
pub struct CoredumpEntry {
    pub exe: String,
    pub signal: Option<String>,
    pub timestamp: String,
}

#[derive(Debug, Clone)]
pub struct PsiSnapshot {
    pub memory_some_avg10: f32,
    pub memory_full_avg10: f32,
    pub cpu_some_avg10: f32,
    pub io_some_avg10: f32,
}

#[derive(Debug, Clone)]
pub struct ResourcePressureSnapshot {
    pub mem_used_pct: f32,
    pub swap_used_pct: f32,
    pub load1: f64,
    pub n_cpus: usize,
    pub disk_max_used_pct: f32,
}

#[derive(Debug, Clone)]
pub struct TempSnapshot {
    pub label: String,
    pub celsius: f32,
}

#[derive(Debug, Clone)]
pub struct ClockSyncSnapshot {
    pub ntp_synchronized: Option<bool>,
    pub system_time_status: String,
}

#[derive(Debug, Clone)]
pub struct EtcMetaChange {
    pub path: String,
    pub kind: String,
}

/// Conservative named thresholds (percent / ratio). Prefer under-diagnosis.
pub mod thresholds {
    /// Memory used fraction triggering resource-pressure warning.
    pub const MEM_USED_PCT_WARN: f32 = 95.0;
    /// Swap used fraction triggering warning.
    pub const SWAP_USED_PCT_WARN: f32 = 80.0;
    /// Load1 / n_cpus ratio.
    pub const LOAD_PER_CPU_WARN: f64 = 4.0;
    /// Single filesystem used %.
    pub const DISK_USED_PCT_WARN: f32 = 95.0;
    /// Temperature (°C) for "High temperature observed" — not throttling.
    pub const TEMP_CELSIUS_HIGH: f32 = 90.0;
    /// PSI memory some avg10 (%) soft warn.
    pub const PSI_MEMORY_SOME_WARN: f32 = 20.0;
    /// PSI memory full avg10 (%) stronger warn.
    pub const PSI_MEMORY_FULL_WARN: f32 = 10.0;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn health_worse() {
        assert_eq!(
            HealthStatus::Ok.worse(HealthStatus::Critical),
            HealthStatus::Critical
        );
        assert_eq!(
            HealthStatus::Critical.worse(HealthStatus::Warning),
            HealthStatus::Critical
        );
    }
}
