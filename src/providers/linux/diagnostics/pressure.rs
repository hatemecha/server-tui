//! PSI and resource utilization snapshots.

use std::fs;

use crate::error::AppError;
use crate::model::{PsiSnapshot, ResourcePressureSnapshot};

pub(crate) fn probe_psi() -> Result<Option<PsiSnapshot>, AppError> {
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

pub(crate) fn probe_resource_pressure() -> Result<Option<ResourcePressureSnapshot>, AppError> {
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
