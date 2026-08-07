//! Linux diagnostic probes — each probe fails independently.

use std::collections::HashMap;
use std::ffi::OsString;
use std::fs;
use std::path::Path;
use std::process::Stdio;
use std::time::{Duration, SystemTime};

use async_trait::async_trait;
use tokio::process::Command;

use crate::error::AppError;
use crate::model::{
    ClockSyncSnapshot, CoredumpEntry, DiagnosticSnapshot, EtcMetaChange, FailedUnitSnapshot,
    JournalCriticalGroup, OomEvent, PreviousBootSummary, PsiSnapshot, PstoreEntry,
    ResourcePressureSnapshot, SmartDiskSnapshot, TempSnapshot,
};
use crate::providers::linux::systemd::LinuxServiceProvider;
use crate::providers::DiagnosticProbeProvider;
use crate::providers::ServiceProvider;
use crate::sanitize::{sanitize_cell, sanitize_path_display, sanitize_text};

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

        // Independent provider instance: probe failures must not depend on UI list state.
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

        match probe_journal().await {
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

        match probe_boot_ids().await {
            Ok((boot, prev_summary)) => {
                snap.boot_id = boot;
                snap.previous_boot_summary = prev_summary;
            }
            Err(e) => degraded.push(format!("boot: {e}")),
        }

        match probe_pstore() {
            Ok(entries) => snap.pstore_entries = entries,
            Err(e) => degraded.push(format!("pstore: {e}")),
        }

        match probe_coredumps().await {
            Ok(entries) => snap.coredumps = entries,
            Err(e) => degraded.push(format!("coredump: {e}")),
        }

        match probe_psi() {
            Ok(psi) => snap.psi = psi,
            Err(e) => degraded.push(format!("psi: {e}")),
        }

        match probe_resource_pressure() {
            Ok(r) => snap.resource_pressure = r,
            Err(e) => degraded.push(format!("resources: {e}")),
        }

        match probe_temperatures() {
            Ok(t) => snap.temperatures = t,
            Err(e) => degraded.push(format!("temp: {e}")),
        }

        match probe_clock().await {
            Ok(c) => snap.clock_sync = c,
            Err(e) => degraded.push(format!("clock: {e}")),
        }

        match probe_etc_meta() {
            Ok(e) => snap.etc_mtime_notable = e,
            Err(e) => degraded.push(format!("etc: {e}")),
        }

        if deep && enable_smart {
            match probe_smart_readonly().await {
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

/// Typed journalctl query → OsString args (no shell).
#[derive(Debug, Clone)]
pub struct JournalQuery {
    pub args: Vec<OsString>,
}

impl JournalQuery {
    pub fn critical_recent(lines: usize) -> Self {
        Self {
            args: vec![
                OsString::from("--no-pager"),
                OsString::from("--output=json"),
                OsString::from("-p"),
                OsString::from("0..2"),
                OsString::from("-n"),
                OsString::from(lines.to_string()),
                OsString::from("--boot=0"),
            ],
        }
    }

    pub fn kernel_oom(lines: usize) -> Self {
        Self {
            args: vec![
                OsString::from("--no-pager"),
                OsString::from("--output=cat"),
                OsString::from("-k"),
                OsString::from("-g"),
                OsString::from("oom|Out of memory|Killed process"),
                OsString::from("-n"),
                OsString::from(lines.to_string()),
                OsString::from("--boot=0"),
            ],
        }
    }

    pub fn previous_boot_tail(lines: usize) -> Self {
        Self {
            args: vec![
                OsString::from("--no-pager"),
                OsString::from("--output=short-iso"),
                OsString::from("-b"),
                OsString::from("-1"),
                OsString::from("-n"),
                OsString::from(lines.to_string()),
            ],
        }
    }
}

async fn run_journalctl(query: &JournalQuery) -> Result<String, AppError> {
    let mut cmd = Command::new("journalctl");
    for a in &query.args {
        cmd.arg(a);
    }
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    let output = cmd
        .output()
        .await
        .map_err(|e| AppError::Journal(format!("journalctl: {e}")))?;
    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        return Err(AppError::Journal(format!(
            "journalctl exited {}: {}",
            output.status, err
        )));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

async fn probe_journal() -> Result<(Vec<JournalCriticalGroup>, Vec<OomEvent>, bool), AppError> {
    let raw = match run_journalctl(&JournalQuery::critical_recent(200)).await {
        Ok(s) => s,
        Err(_) => return Ok((Vec::new(), Vec::new(), false)),
    };

    let mut groups: HashMap<String, (usize, String)> = HashMap::new();
    for line in raw.lines() {
        let Ok(v) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        let unit = v
            .get("_SYSTEMD_UNIT")
            .or_else(|| v.get("SYSLOG_IDENTIFIER"))
            .and_then(|x| x.as_str())
            .unwrap_or("unknown");
        let msg = v
            .get("MESSAGE")
            .map(|m| match m {
                serde_json::Value::String(s) => sanitize_text(s),
                serde_json::Value::Array(arr) => {
                    let bytes: Vec<u8> = arr
                        .iter()
                        .filter_map(|b| b.as_u64().map(|n| n as u8))
                        .collect();
                    sanitize_text(&String::from_utf8_lossy(&bytes))
                }
                _ => String::new(),
            })
            .unwrap_or_default();
        let entry = groups
            .entry(sanitize_cell(unit))
            .or_insert((0, msg.clone()));
        entry.0 += 1;
        if entry.1.is_empty() {
            entry.1 = msg;
        }
    }

    let critical: Vec<JournalCriticalGroup> = groups
        .into_iter()
        .map(|(unit, (count, sample_message))| JournalCriticalGroup {
            unit,
            count,
            sample_message,
        })
        .collect();

    let mut ooms = Vec::new();
    if let Ok(oom_raw) = run_journalctl(&JournalQuery::kernel_oom(40)).await {
        for line in oom_raw.lines().take(20) {
            let msg = sanitize_text(line);
            if msg.is_empty() {
                continue;
            }
            let process = msg
                .split_whitespace()
                .find(|w| w.starts_with("process") || w.contains('('))
                .unwrap_or("unknown")
                .to_string();
            ooms.push(OomEvent {
                process: sanitize_cell(&process),
                message: msg,
            });
        }
    }

    Ok((critical, ooms, true))
}

async fn probe_boot_ids() -> Result<(Option<String>, Option<PreviousBootSummary>), AppError> {
    let boot = fs::read_to_string("/proc/sys/kernel/random/boot_id")
        .ok()
        .map(|s| sanitize_cell(s.trim()));

    let prev = match run_journalctl(&JournalQuery::previous_boot_tail(40)).await {
        Ok(text) => {
            let lower = text.to_ascii_lowercase();
            let clean = lower.contains("reached target shutdown")
                || lower.contains("powering off")
                || lower.contains("reboot: restarting system")
                || lower.contains("systemd-shutdown");
            // Unclean hint only — never claim kernel panic.
            let unclean_hint = !text.trim().is_empty() && !clean;
            Some(PreviousBootSummary {
                boot_id: "previous".into(),
                unclean_hint,
                note: if unclean_hint {
                    "Previous boot journal ended without an obvious clean shutdown marker".into()
                } else {
                    "Previous boot journal looks consistent with a normal shutdown".into()
                },
            })
        }
        Err(_) => None,
    };

    Ok((boot, prev))
}

fn probe_pstore() -> Result<Vec<PstoreEntry>, AppError> {
    let dir = Path::new("/sys/fs/pstore");
    if !dir.is_dir() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    let entries = fs::read_dir(dir).map_err(|e| AppError::Internal(e.to_string()))?;
    for entry in entries.flatten() {
        let meta = entry.metadata().ok();
        let bytes = meta.as_ref().map(|m| m.len()).unwrap_or(0);
        out.push(PstoreEntry {
            name: sanitize_cell(&entry.file_name().to_string_lossy()),
            bytes,
        });
        if out.len() >= 32 {
            break;
        }
    }
    Ok(out)
}

async fn probe_coredumps() -> Result<Vec<CoredumpEntry>, AppError> {
    let mut cmd = Command::new("coredumpctl");
    cmd.args(["--no-pager", "--json=short", "-n", "10"]);
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    let output = match cmd.output().await {
        Ok(o) => o,
        Err(_) => return Ok(Vec::new()),
    };
    if !output.status.success() {
        return Ok(Vec::new());
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let mut out = Vec::new();
    // coredumpctl --json=short may emit a JSON array or line-delimited objects.
    if let Ok(arr) = serde_json::from_str::<Vec<serde_json::Value>>(&text) {
        for v in arr.into_iter().take(10) {
            out.push(parse_coredump_json(&v));
        }
    } else {
        for line in text.lines().take(10) {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(line) {
                out.push(parse_coredump_json(&v));
            }
        }
    }
    Ok(out)
}

fn parse_coredump_json(v: &serde_json::Value) -> CoredumpEntry {
    let exe = v
        .get("exe")
        .or_else(|| v.get("COREDUMP_EXE"))
        .and_then(|x| x.as_str())
        .unwrap_or("unknown");
    let signal = v
        .get("signal")
        .or_else(|| v.get("COREDUMP_SIGNAL"))
        .map(|x| match x {
            serde_json::Value::String(s) => s.clone(),
            serde_json::Value::Number(n) => n.to_string(),
            _ => "?".into(),
        });
    let timestamp = v
        .get("time")
        .or_else(|| v.get("COREDUMP_TIMESTAMP"))
        .map(|x| x.to_string())
        .unwrap_or_else(|| "?".into());
    CoredumpEntry {
        exe: sanitize_path_display(Path::new(exe)),
        signal: signal.map(|s| sanitize_cell(&s)),
        timestamp: sanitize_cell(&timestamp),
    }
}

fn probe_psi() -> Result<Option<PsiSnapshot>, AppError> {
    let mem = fs::read_to_string("/proc/pressure/memory").ok();
    let cpu = fs::read_to_string("/proc/pressure/cpu").ok();
    let io = fs::read_to_string("/proc/pressure/io").ok();
    if mem.is_none() && cpu.is_none() && io.is_none() {
        return Ok(None);
    }
    Ok(Some(PsiSnapshot {
        memory_some_avg10: parse_psi_avg10(mem.as_deref(), "some").unwrap_or(0.0),
        memory_full_avg10: parse_psi_avg10(mem.as_deref(), "full").unwrap_or(0.0),
        cpu_some_avg10: parse_psi_avg10(cpu.as_deref(), "some").unwrap_or(0.0),
        io_some_avg10: parse_psi_avg10(io.as_deref(), "some").unwrap_or(0.0),
    }))
}

fn parse_psi_avg10(content: Option<&str>, kind: &str) -> Option<f32> {
    let content = content?;
    for line in content.lines() {
        if !line.starts_with(kind) {
            continue;
        }
        for part in line.split_whitespace() {
            if let Some(v) = part.strip_prefix("avg10=") {
                return v.parse().ok();
            }
        }
    }
    None
}

fn probe_resource_pressure() -> Result<Option<ResourcePressureSnapshot>, AppError> {
    use sysinfo::{Disks, System};
    let mut sys = System::new();
    sys.refresh_memory();
    sys.refresh_cpu_all();
    let mem_total = sys.total_memory().max(1) as f32;
    let mem_used_pct = sys.used_memory() as f32 / mem_total * 100.0;
    let swap_total = sys.total_swap();
    let swap_used_pct = if swap_total == 0 {
        0.0
    } else {
        sys.used_swap() as f32 / swap_total as f32 * 100.0
    };
    let load1 = System::load_average().one;
    let n_cpus = sys.cpus().len().max(1);
    let disks = Disks::new_with_refreshed_list();
    let disk_max_used_pct = disks
        .list()
        .iter()
        .filter(|d| {
            let mp = d.mount_point().to_string_lossy();
            !mp.starts_with("/proc")
                && !mp.starts_with("/sys")
                && !mp.starts_with("/dev")
                && !mp.starts_with("/run")
        })
        .map(|d| {
            let total = d.total_space().max(1) as f32;
            let used = d.total_space().saturating_sub(d.available_space()) as f32;
            used / total * 100.0
        })
        .fold(0.0f32, f32::max);

    Ok(Some(ResourcePressureSnapshot {
        mem_used_pct,
        swap_used_pct,
        load1,
        n_cpus,
        disk_max_used_pct,
    }))
}

fn probe_temperatures() -> Result<Vec<TempSnapshot>, AppError> {
    let mut out = Vec::new();
    let Ok(entries) = fs::read_dir("/sys/class/thermal") else {
        return Ok(out);
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path
            .file_name()
            .map(|n| n.to_string_lossy().starts_with("thermal_zone"))
            .unwrap_or(false)
        {
            continue;
        }
        let Ok(raw) = fs::read_to_string(path.join("temp")) else {
            continue;
        };
        let Ok(milli) = raw.trim().parse::<f32>() else {
            continue;
        };
        let label = fs::read_to_string(path.join("type"))
            .map(|s| sanitize_cell(s.trim()))
            .unwrap_or_else(|_| "thermal".into());
        out.push(TempSnapshot {
            label,
            celsius: milli / 1000.0,
        });
        if out.len() >= 8 {
            break;
        }
    }
    Ok(out)
}

async fn probe_clock() -> Result<Option<ClockSyncSnapshot>, AppError> {
    let mut cmd = Command::new("timedatectl");
    cmd.args([
        "show",
        "--property=NTPSynchronized",
        "--property=TimeUSec",
        "--value",
    ]);
    // Fallback: status text parse
    let output = Command::new("timedatectl")
        .arg("status")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await;
    let Ok(output) = output else {
        return Ok(None);
    };
    if !output.status.success() {
        return Ok(None);
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let mut ntp = None;
    for line in text.lines() {
        let l = line.trim().to_ascii_lowercase();
        if l.contains("system clock synchronized") || l.contains("ntp synchronized") {
            if l.contains("yes") {
                ntp = Some(true);
            } else if l.contains("no") {
                ntp = Some(false);
            }
        }
    }
    Ok(Some(ClockSyncSnapshot {
        ntp_synchronized: ntp,
        system_time_status: sanitize_text(text.lines().next().unwrap_or("timedatectl status")),
    }))
}

fn probe_etc_meta() -> Result<Vec<EtcMetaChange>, AppError> {
    let etc = Path::new("/etc");
    if !etc.is_dir() {
        return Ok(Vec::new());
    }
    let cutoff = SystemTime::now()
        .checked_sub(Duration::from_secs(7 * 24 * 3600))
        .unwrap_or(SystemTime::UNIX_EPOCH);
    let mut out = Vec::new();
    let Ok(entries) = fs::read_dir(etc) else {
        return Ok(out);
    };
    for entry in entries.flatten() {
        let Ok(meta) = entry.metadata() else {
            continue;
        };
        let Ok(mtime) = meta.modified() else {
            continue;
        };
        if mtime < cutoff {
            continue;
        }
        let kind = if meta.is_dir() { "dir" } else { "file" };
        out.push(EtcMetaChange {
            path: sanitize_path_display(&entry.path()),
            kind: kind.into(),
        });
        if out.len() >= 24 {
            break;
        }
    }
    Ok(out)
}

/// Demo datasets A–H for synthetic diagnostics.
pub mod demo_datasets {
    use super::*;

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
}

/// Read-only SMART via smartctl (-H -A -l error). Never runs self-tests.
async fn probe_smart_readonly() -> Result<Vec<SmartDiskSnapshot>, AppError> {
    let smartctl = match crate::actions::trusted::resolve_trusted("smartctl") {
        Ok(p) => p,
        Err(_) => return Ok(Vec::new()), // soft: smartctl optional
    };
    let mut disks = Vec::new();
    // Conservative device enumeration: only common block names under /dev.
    let candidates = list_smart_device_candidates();
    for dev in candidates.into_iter().take(8) {
        let output = Command::new(&smartctl)
            .arg("-H")
            .arg("-A")
            .arg("-l")
            .arg("error")
            .arg(&dev)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await;
        let Ok(out) = output else {
            continue;
        };
        let stdout = String::from_utf8_lossy(&out.stdout);
        let passed = if stdout.to_ascii_lowercase().contains("result: passed") {
            Some(true)
        } else if stdout.to_ascii_lowercase().contains("result: failed") {
            Some(false)
        } else {
            None
        };
        let crc = parse_uda_crc(&stdout);
        let device_key = sanitize_path_display(Path::new(&dev));
        let prev = crate::persist::AppPersistState::load_or_default(None)
            .ok()
            .and_then(|(p, _)| p.previous_crc(&device_key));
        disks.push(SmartDiskSnapshot {
            device: device_key.clone(),
            available: true,
            passed,
            uda_crc_error_count: crc,
            prev_uda_crc_error_count: prev,
            summary: if passed == Some(false) {
                "SMART FAILED".into()
            } else {
                "SMART probed (read-only)".into()
            },
            details: stdout.lines().take(20).map(sanitize_text).collect(),
        });
        if let Some(c) = crc {
            if let Ok((mut persist, path)) = crate::persist::AppPersistState::load_or_default(None)
            {
                persist.remember_crc(&device_key, c);
                let _ = persist.save_atomic(&path);
            }
        }
    }
    Ok(disks)
}

fn list_smart_device_candidates() -> Vec<String> {
    let mut out = Vec::new();
    let Ok(rd) = fs::read_dir("/dev") else {
        return out;
    };
    for ent in rd.flatten() {
        let name = ent.file_name().to_string_lossy().into_owned();
        // sdX / nvmeXn1 only — never partitions like sda1.
        let ok = (name.starts_with("sd") && name.len() == 3)
            || (name.starts_with("nvme") && name.ends_with("n1") && !name.contains('p'));
        if ok {
            out.push(format!("/dev/{name}"));
        }
    }
    out.sort();
    out
}

fn parse_uda_crc(attrs: &str) -> Option<u64> {
    for line in attrs.lines() {
        if line.contains("UDMA_CRC_Error_Count") || line.contains("CRC_Error_Count") {
            let parts: Vec<_> = line.split_whitespace().collect();
            if let Some(last) = parts.last() {
                if let Ok(v) = last.parse::<u64>() {
                    return Some(v);
                }
            }
        }
    }
    None
}
