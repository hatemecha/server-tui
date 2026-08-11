//! System metrics models and bounded history.

use std::collections::VecDeque;

#[derive(Debug, Clone, Default)]
pub struct SystemMetrics {
    pub hostname: String,
    pub distro: String,
    pub kernel: String,
    pub arch: String,
    pub uptime_seconds: u64,
    pub load_avg: (f64, f64, f64),
    pub cpu_total: f32,
    pub cpu_per_core: Vec<f32>,
    pub memory_used: u64,
    pub memory_total: u64,
    pub swap_used: u64,
    pub swap_total: u64,
    pub net_rx_bytes: u64,
    pub net_tx_bytes: u64,
    pub net_rx_bps: f64,
    pub net_tx_bps: f64,
    pub disks: Vec<DiskUsage>,
    pub process_count: usize,
    pub failed_services: usize,
    pub temperatures: Vec<TemperatureReading>,
    pub systemd_available: bool,
    pub journal_available: bool,
}

#[derive(Debug, Clone)]
pub struct DiskUsage {
    pub name: String,
    pub mount_point: String,
    pub total: u64,
    pub used: u64,
    pub available: u64,
}

#[derive(Debug, Clone)]
pub struct TemperatureReading {
    pub label: String,
    pub celsius: f32,
}

#[derive(Debug, Clone)]
pub struct CircularBuffer<T> {
    capacity: usize,
    data: VecDeque<T>,
}

impl<T> CircularBuffer<T> {
    pub fn new(capacity: usize) -> Self {
        let capacity = capacity.max(1);
        Self {
            capacity,
            data: VecDeque::with_capacity(capacity),
        }
    }

    pub fn push(&mut self, value: T) {
        if self.data.len() >= self.capacity {
            self.data.pop_front();
        }
        self.data.push_back(value);
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.data.iter()
    }

    pub fn as_slice_vec(&self) -> Vec<&T> {
        self.data.iter().collect()
    }

    pub fn resize(&mut self, capacity: usize) {
        let capacity = capacity.max(1);
        while self.data.len() > capacity {
            self.data.pop_front();
        }
        self.capacity = capacity;
    }
}

impl CircularBuffer<f32> {
    pub fn values(&self) -> Vec<f64> {
        self.data.iter().map(|v| f64::from(*v)).collect()
    }
}

impl CircularBuffer<f64> {
    pub fn values(&self) -> Vec<f64> {
        self.data.iter().copied().collect()
    }
}

#[derive(Debug, Clone)]
pub struct MetricHistory {
    pub cpu: CircularBuffer<f32>,
    pub memory: CircularBuffer<f32>,
    pub net_rx: CircularBuffer<f64>,
    pub net_tx: CircularBuffer<f64>,
}

impl MetricHistory {
    pub fn new(capacity: usize) -> Self {
        Self {
            cpu: CircularBuffer::new(capacity),
            memory: CircularBuffer::new(capacity),
            net_rx: CircularBuffer::new(capacity),
            net_tx: CircularBuffer::new(capacity),
        }
    }

    pub fn record_from_metrics(&mut self, metrics: &SystemMetrics) {
        self.cpu.push(metrics.cpu_total);
        let mem_pct = if metrics.memory_total == 0 {
            0.0
        } else {
            (metrics.memory_used as f64 / metrics.memory_total as f64 * 100.0) as f32
        };
        self.memory.push(mem_pct);
        self.net_rx.push(metrics.net_rx_bps);
        self.net_tx.push(metrics.net_tx_bps);
    }

    pub fn resize(&mut self, capacity: usize) {
        self.cpu.resize(capacity);
        self.memory.resize(capacity);
        self.net_rx.resize(capacity);
        self.net_tx.resize(capacity);
    }
}

pub fn format_bytes(bytes: u64) -> String {
    const UNITS: [&str; 6] = ["B", "KiB", "MiB", "GiB", "TiB", "PiB"];
    let mut value = bytes as f64;
    let mut unit = 0usize;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} {}", UNITS[unit])
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}

pub fn format_rate(bps: f64) -> String {
    format!("{}/s", format_bytes(bps.max(0.0) as u64))
}

pub fn format_uptime(seconds: u64) -> String {
    let days = seconds / 86_400;
    let hours = (seconds % 86_400) / 3600;
    let mins = (seconds % 3600) / 60;
    if days > 0 {
        format!("{days}d {hours}h")
    } else if hours > 0 {
        format!("{hours}h {mins}m")
    } else {
        format!("{mins}m")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn circular_buffer_drops_oldest() {
        let mut buf = CircularBuffer::new(3);
        buf.push(1);
        buf.push(2);
        buf.push(3);
        buf.push(4);
        assert_eq!(buf.len(), 3);
        let v: Vec<_> = buf.iter().copied().collect();
        assert_eq!(v, vec![2, 3, 4]);
    }

    #[test]
    fn history_resize_preserves_newest_and_accepts_growth() {
        let mut history = MetricHistory::new(4);
        for value in 1..=4 {
            history.cpu.push(value as f32);
        }
        history.resize(2);
        assert_eq!(history.cpu.values(), vec![3.0, 4.0]);
        history.resize(5);
        history.cpu.push(5.0);
        assert_eq!(history.cpu.values(), vec![3.0, 4.0, 5.0]);
        assert_eq!(history.cpu.capacity(), 5);
    }

    #[test]
    fn format_bytes_scales() {
        assert_eq!(format_bytes(512), "512 B");
        assert_eq!(format_bytes(2048), "2.0 KiB");
    }
}
