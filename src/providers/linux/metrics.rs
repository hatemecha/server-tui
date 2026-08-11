//! Real system metrics via sysinfo with independent subsystem refresh caching.

use std::fs;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use async_trait::async_trait;
use sysinfo::{Disks, Networks, System};

use crate::config::Config;
use crate::error::AppError;
use crate::model::{DiskUsage, SystemMetrics, TemperatureReading};
use crate::providers::MetricsProvider;
use crate::sanitize::{sanitize_cell, sanitize_path_display};

pub struct LinuxMetricsProvider {
    inner: Arc<Mutex<MetricsState>>,
}

#[derive(Debug, Clone)]
struct MetricsIntervals {
    cpu_net_ms: u64,
    memory_ms: u64,
    disk_ms: u64,
    temperature_ms: u64,
    capabilities_ms: u64,
}

impl MetricsIntervals {
    fn from_config(cfg: &Config) -> Self {
        Self {
            cpu_net_ms: cfg.refresh_ms,
            memory_ms: cfg.memory_refresh_ms,
            disk_ms: cfg.disk_refresh_ms,
            temperature_ms: cfg.temperature_refresh_ms,
            // Capabilities change rarely; reuse disk interval as a conservative bound.
            capabilities_ms: cfg.disk_refresh_ms.max(5_000),
        }
    }
}

struct MetricsState {
    intervals: MetricsIntervals,
    system: System,
    networks: Networks,
    disks: Disks,
    last_net: Option<(Instant, u64, u64)>,
    net_rx_bps: f64,
    net_tx_bps: f64,
    last_cpu_net: Option<Instant>,
    last_memory: Option<Instant>,
    last_disk: Option<Instant>,
    last_temp: Option<Instant>,
    last_caps: Option<Instant>,
    cached_disks: Vec<DiskUsage>,
    cached_temps: Vec<TemperatureReading>,
    cached_caps: (bool, bool),
    cached_cpu_total: f32,
    cached_cpu_per_core: Vec<f32>,
    cached_mem_used: u64,
    cached_mem_total: u64,
    cached_swap_used: u64,
    cached_swap_total: u64,
    cached_rx: u64,
    cached_tx: u64,
}

impl LinuxMetricsProvider {
    pub fn new(config: &Config) -> Self {
        let mut system = System::new();
        system.refresh_memory();
        system.refresh_cpu_all();
        let networks = Networks::new_with_refreshed_list();
        let disks = Disks::new_with_refreshed_list();
        Self {
            inner: Arc::new(Mutex::new(MetricsState {
                intervals: MetricsIntervals::from_config(config),
                system,
                networks,
                disks,
                last_net: None,
                net_rx_bps: 0.0,
                net_tx_bps: 0.0,
                last_cpu_net: None,
                last_memory: None,
                last_disk: None,
                last_temp: None,
                last_caps: None,
                cached_disks: Vec::new(),
                cached_temps: Vec::new(),
                cached_caps: (false, false),
                cached_cpu_total: 0.0,
                cached_cpu_per_core: Vec::new(),
                cached_mem_used: 0,
                cached_mem_total: 0,
                cached_swap_used: 0,
                cached_swap_total: 0,
                cached_rx: 0,
                cached_tx: 0,
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

    fn update_refresh_policy(&self, config: &Config) {
        if let Ok(mut state) = self.inner.lock() {
            state.intervals = MetricsIntervals::from_config(config);
        }
    }
}

fn due(last: Option<Instant>, ms: u64) -> bool {
    match last {
        None => true,
        Some(t) => t.elapsed() >= Duration::from_millis(ms),
    }
}

fn collect_blocking(inner: &Mutex<MetricsState>) -> Result<SystemMetrics, AppError> {
    let mut guard = inner
        .lock()
        .map_err(|_| AppError::Internal("metrics lock poisoned".into()))?;
    let state = &mut *guard;
    let intervals = state.intervals.clone();
    let now = Instant::now();

    if due(state.last_cpu_net, intervals.cpu_net_ms) {
        state.system.refresh_cpu_all();
        state.networks.refresh(true);
        state.cached_cpu_per_core = state.system.cpus().iter().map(|c| c.cpu_usage()).collect();
        state.cached_cpu_total = state.system.global_cpu_usage();

        let mut rx = 0u64;
        let mut tx = 0u64;
        for data in state.networks.values() {
            rx = rx.saturating_add(data.total_received());
            tx = tx.saturating_add(data.total_transmitted());
        }
        if let Some((prev_t, prev_rx, prev_tx)) = state.last_net {
            let dt = now.duration_since(prev_t).as_secs_f64().max(0.001);
            state.net_rx_bps = (rx.saturating_sub(prev_rx)) as f64 / dt;
            state.net_tx_bps = (tx.saturating_sub(prev_tx)) as f64 / dt;
        }
        state.last_net = Some((now, rx, tx));
        state.cached_rx = rx;
        state.cached_tx = tx;
        state.last_cpu_net = Some(now);
    }

    if due(state.last_memory, intervals.memory_ms) {
        state.system.refresh_memory();
        state.cached_mem_used = state.system.used_memory();
        state.cached_mem_total = state.system.total_memory();
        state.cached_swap_used = state.system.used_swap();
        state.cached_swap_total = state.system.total_swap();
        state.last_memory = Some(now);
    }

    if due(state.last_disk, intervals.disk_ms) {
        state.disks.refresh(true);
        state.cached_disks = state
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
                mount_point: sanitize_path_display(d.mount_point()),
                total: d.total_space(),
                used: d.total_space().saturating_sub(d.available_space()),
                available: d.available_space(),
            })
            .collect();
        state.last_disk = Some(now);
    }

    if due(state.last_temp, intervals.temperature_ms) {
        state.cached_temps = read_temperatures();
        state.last_temp = Some(now);
    }

    if due(state.last_caps, intervals.capabilities_ms) {
        state.cached_caps = probe_capabilities();
        state.last_caps = Some(now);
    }

    let (systemd_available, journal_available) = state.cached_caps;

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
        cpu_total: state.cached_cpu_total,
        cpu_per_core: state.cached_cpu_per_core.clone(),
        memory_used: state.cached_mem_used,
        memory_total: state.cached_mem_total,
        swap_used: state.cached_swap_used,
        swap_total: state.cached_swap_total,
        net_rx_bytes: state.cached_rx,
        net_tx_bytes: state.cached_tx,
        net_rx_bps: state.net_rx_bps,
        net_tx_bps: state.net_tx_bps,
        disks: state.cached_disks.clone(),
        process_count: 0, // filled from process provider / app state
        failed_services: 0,
        temperatures: state.cached_temps.clone(),
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
