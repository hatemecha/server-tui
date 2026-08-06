//! Real system metrics via sysinfo.

use std::fs;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use async_trait::async_trait;
use sysinfo::{Disks, Networks, System};

use crate::error::AppError;
use crate::model::{DiskUsage, SystemMetrics, TemperatureReading};
use crate::providers::MetricsProvider;
use crate::sanitize::sanitize_cell;

pub struct LinuxMetricsProvider {
    inner: Arc<Mutex<MetricsState>>,
}

impl Default for LinuxMetricsProvider {
    fn default() -> Self {
        Self::new()
    }
}

struct MetricsState {
    system: System,
    networks: Networks,
    disks: Disks,
    last_net: Option<(Instant, u64, u64)>,
    net_rx_bps: f64,
    net_tx_bps: f64,
}

impl LinuxMetricsProvider {
    pub fn new() -> Self {
        let mut system = System::new();
        system.refresh_memory();
        system.refresh_cpu_all();
        let networks = Networks::new_with_refreshed_list();
        let disks = Disks::new_with_refreshed_list();
        Self {
            inner: Arc::new(Mutex::new(MetricsState {
                system,
                networks,
                disks,
                last_net: None,
                net_rx_bps: 0.0,
                net_tx_bps: 0.0,
            })),
        }
    }
}

#[async_trait]
impl MetricsProvider for LinuxMetricsProvider {
    async fn collect(&self) -> Result<SystemMetrics, AppError> {
        let inner = Arc::clone(&self.inner);
        tokio::task::spawn_blocking(move || collect_blocking(&inner))
            .await
            .map_err(|e| AppError::Internal(format!("metrics join: {e}")))?
    }
}

fn collect_blocking(inner: &Mutex<MetricsState>) -> Result<SystemMetrics, AppError> {
    let mut guard = inner
        .lock()
        .map_err(|_| AppError::Internal("metrics lock poisoned".into()))?;
    let state = &mut *guard;

    state.system.refresh_cpu_all();
    state.system.refresh_memory();
    state.networks.refresh(true);
    state.disks.refresh(true);

    let mut rx = 0u64;
    let mut tx = 0u64;
    for data in state.networks.values() {
        rx = rx.saturating_add(data.total_received());
        tx = tx.saturating_add(data.total_transmitted());
    }
    let now = Instant::now();
    if let Some((prev_t, prev_rx, prev_tx)) = state.last_net {
        let dt = now.duration_since(prev_t).as_secs_f64().max(0.001);
        state.net_rx_bps = (rx.saturating_sub(prev_rx)) as f64 / dt;
        state.net_tx_bps = (tx.saturating_sub(prev_tx)) as f64 / dt;
    }
    state.last_net = Some((now, rx, tx));

    let cpu_per_core: Vec<f32> = state.system.cpus().iter().map(|c| c.cpu_usage()).collect();
    let cpu_total = state.system.global_cpu_usage();

    let disks: Vec<DiskUsage> = state
        .disks
        .list()
        .iter()
        .filter(|d| {
            let mp = d.mount_point().to_string_lossy();
            !mp.starts_with("/proc")
                && !mp.starts_with("/sys")
                && !mp.starts_with("/dev")
                && !mp.starts_with("/run")
        })
        .map(|d| DiskUsage {
            name: sanitize_cell(&d.name().to_string_lossy()),
            mount_point: sanitize_cell(&d.mount_point().to_string_lossy()),
            total: d.total_space(),
            used: d.total_space().saturating_sub(d.available_space()),
            available: d.available_space(),
        })
        .collect();

    let temperatures = read_temperatures();
    let (systemd_available, journal_available) = probe_capabilities();

    Ok(SystemMetrics {
        hostname: sanitize_cell(&System::host_name().unwrap_or_else(|| "unknown".into())),
        distro: sanitize_cell(&read_distro()),
        kernel: sanitize_cell(&System::kernel_version().unwrap_or_else(|| "unknown".into())),
        arch: sanitize_cell(std::env::consts::ARCH),
        uptime_seconds: System::uptime(),
        load_avg: {
            let l = System::load_average();
            (l.one, l.five, l.fifteen)
        },
        cpu_total,
        cpu_per_core,
        memory_used: state.system.used_memory(),
        memory_total: state.system.total_memory(),
        swap_used: state.system.used_swap(),
        swap_total: state.system.total_swap(),
        net_rx_bytes: rx,
        net_tx_bytes: tx,
        net_rx_bps: state.net_rx_bps,
        net_tx_bps: state.net_tx_bps,
        disks,
        process_count: state.system.processes().len(),
        failed_services: 0,
        temperatures,
        systemd_available,
        journal_available,
    })
}

fn read_distro() -> String {
    if let Ok(content) = fs::read_to_string("/etc/os-release") {
        for line in content.lines() {
            if let Some(v) = line.strip_prefix("PRETTY_NAME=") {
                return v.trim_matches('"').to_string();
            }
        }
    }
    "Linux".into()
}

fn read_temperatures() -> Vec<TemperatureReading> {
    let mut out = Vec::new();
    let Ok(entries) = fs::read_dir("/sys/class/thermal") else {
        return out;
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
        let temp_path = path.join("temp");
        let type_path = path.join("type");
        let Ok(raw) = fs::read_to_string(&temp_path) else {
            continue;
        };
        let Ok(milli) = raw.trim().parse::<f32>() else {
            continue;
        };
        let label = fs::read_to_string(&type_path)
            .map(|s| sanitize_cell(s.trim()))
            .unwrap_or_else(|_| "thermal".into());
        out.push(TemperatureReading {
            label,
            celsius: milli / 1000.0,
        });
        if out.len() >= 8 {
            break;
        }
    }
    out
}

fn probe_capabilities() -> (bool, bool) {
    let systemd = std::path::Path::new("/run/systemd/system").exists()
        || std::path::Path::new("/run/dbus/system_bus_socket").exists();
    let journal = which_journalctl();
    (systemd, journal)
}

fn which_journalctl() -> bool {
    std::env::var_os("PATH")
        .map(|paths| {
            for p in std::env::split_paths(&paths) {
                if p.join("journalctl").is_file() {
                    return true;
                }
            }
            false
        })
        .unwrap_or(false)
}

/// sysinfo CPU usage benefits from a short wait between refreshes on cold start.
pub async fn warm_cpu(provider: &LinuxMetricsProvider) -> Result<(), AppError> {
    let _ = provider.collect().await?;
    tokio::time::sleep(Duration::from_millis(200)).await;
    let _ = provider.collect().await?;
    Ok(())
}
