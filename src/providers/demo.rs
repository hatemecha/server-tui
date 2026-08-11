//! Synthetic providers for --demo mode. Never touch the real system.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use tokio_util::sync::CancellationToken;

use crate::error::AppError;
use crate::model::{
    aggregate_sizes, default_exclusions, DiagnosticSnapshot, DiskUsage, LogEntry, LogPreset,
    LogPriority, ProcessInfo, ProcessSignal, ServiceActionKind, ServiceInfo, StorageNode,
    StorageProgress, StorageTree, SystemMetrics, TemperatureReading, UnitFileState, UnitRegistry,
};
use crate::providers::linux::diagnostics::demo_datasets::DemoDataset;
use crate::providers::traits::{
    AdministrativeExecutor, DiagnosticProbeProvider, LogProvider, MetricsProvider, ProcessProvider,
    ProviderBundle, ServiceProvider, StorageProvider,
};

static TICK: AtomicU64 = AtomicU64::new(0);

fn tick() -> u64 {
    TICK.fetch_add(1, Ordering::Relaxed)
}

pub struct DemoProviders;

impl DemoProviders {
    pub fn bundle(read_only: bool) -> ProviderBundle {
        let registry = UnitRegistry::new();
        // Pre-register demo units so admin path can accept them.
        registry.replace_all([
            "ssh.service".into(),
            "nginx.service".into(),
            "broken-demo.service".into(),
            "cron.service".into(),
        ]);
        ProviderBundle {
            metrics: Box::new(DemoMetrics),
            processes: Box::new(DemoProcesses),
            services: Box::new(DemoServices {
                registry: registry.clone(),
            }),
            logs: Box::new(DemoLogs),
            storage: Box::new(DemoStorage),
            admin: Box::new(DemoAdmin {
                read_only,
                registry,
            }),
            diagnostics: Box::new(DemoDiagnostics),
        }
    }
}

struct DemoMetrics;

#[async_trait]
impl MetricsProvider for DemoMetrics {
    async fn collect(&self) -> Result<SystemMetrics, AppError> {
        let t = tick() as f32;
        let cpu = 20.0 + (t % 40.0);
        Ok(SystemMetrics {
            hostname: "demo-host".into(),
            distro: "Demo Linux 1.0".into(),
            kernel: "6.6.0-demo".into(),
            arch: "x86_64".into(),
            uptime_seconds: 3600 * 24 * 4 + 11 * 3600,
            load_avg: (0.42, 0.55, 0.61),
            cpu_total: cpu,
            cpu_per_core: vec![cpu - 5.0, cpu, cpu + 3.0, cpu - 2.0],
            memory_used: 4_294_967_296 + (t as u64 * 1024 * 1024),
            memory_total: 16_106_127_360,
            swap_used: 0,
            swap_total: 2_147_483_648,
            net_rx_bytes: 1_000_000_000,
            net_tx_bytes: 500_000_000,
            net_rx_bps: 125_000.0 + f64::from(t) * 100.0,
            net_tx_bps: 64_000.0 + f64::from(t) * 50.0,
            disks: vec![DiskUsage {
                name: "demo-root".into(),
                mount_point: "/".into(),
                total: 100_000_000_000,
                used: 42_000_000_000,
                available: 58_000_000_000,
            }],
            process_count: 128,
            failed_services: 1,
            temperatures: vec![TemperatureReading {
                label: "CPU".into(),
                celsius: 45.0 + (t % 10.0),
            }],
            systemd_available: true,
            journal_available: true,
        })
    }
}

struct DemoProcesses;

#[async_trait]
impl ProcessProvider for DemoProcesses {
    async fn list(&self) -> Result<Vec<ProcessInfo>, AppError> {
        let t = tick();
        Ok(vec![
            ProcessInfo {
                pid: 1001,
                user: "root".into(),
                name: "systemd".into(),
                cmd: "/sbin/init".into(),
                cpu: 0.1,
                mem_pct: Some(0.5),
                mem_bytes: 20_000_000,
                state: "S".into(),
                run_time_secs: 86400,
                start_time: 1_000,
            },
            ProcessInfo {
                pid: 4821,
                user: "alex".into(),
                name: "test-worker".into(),
                cmd: "test-worker --demo".into(),
                cpu: 12.5 + (t % 10) as f32,
                mem_pct: Some(3.2),
                mem_bytes: 120_000_000,
                state: "R".into(),
                run_time_secs: 420,
                start_time: 2_000,
            },
            ProcessInfo {
                pid: 9001,
                user: "www-data".into(),
                name: "nginx".into(),
                cmd: "nginx: worker process".into(),
                cpu: 1.2,
                mem_pct: Some(1.1),
                mem_bytes: 40_000_000,
                state: "S".into(),
                run_time_secs: 3600,
                start_time: 3_000,
            },
            ProcessInfo {
                pid: 4242,
                user: "demo".into(),
                name: "server-tui".into(),
                cmd: "server-tui --demo".into(),
                cpu: 2.0,
                mem_pct: Some(0.8),
                mem_bytes: 30_000_000,
                state: "R".into(),
                run_time_secs: 60,
                start_time: 4_000,
            },
        ])
    }

    async fn details(&self, pid: u32) -> Result<crate::model::ProcessDetails, AppError> {
        let list = self.list().await?;
        let info = list.into_iter().find(|p| p.pid == pid);
        let parent = match pid {
            4821 | 9001 | 4242 => Some(1001),
            _ => None,
        };
        Ok(crate::model::ProcessDetails {
            info: info.clone(),
            parent_pid: parent,
            parent_name: parent.map(|_| "systemd".into()),
            cmdline: info.as_ref().map(|p| p.cmd.clone()).unwrap_or_default(),
            cgroup: Some(format!("0::/system.slice/demo-{pid}.scope")),
            unit: if pid == 9001 {
                Some("nginx.service".into())
            } else {
                None
            },
            fd_count: Some(12),
            read_bytes: Some(1_024_000),
            write_bytes: Some(512_000),
        })
    }
}

struct DemoServices {
    registry: UnitRegistry,
}

fn demo_service_list() -> Vec<ServiceInfo> {
    vec![
        ServiceInfo {
            unit: "ssh.service".into(),
            description: "OpenSSH server daemon".into(),
            load_state: "loaded".into(),
            active_state: "active".into(),
            sub_state: "running".into(),
            unit_path: "/org/freedesktop/systemd1/unit/ssh_2eservice".into(),
            unit_file_state: UnitFileState::Enabled,
            fragment_path: None,
        },
        ServiceInfo {
            unit: "nginx.service".into(),
            description: "A high performance web server".into(),
            load_state: "loaded".into(),
            active_state: "active".into(),
            sub_state: "running".into(),
            unit_path: "/org/freedesktop/systemd1/unit/nginx_2eservice".into(),
            unit_file_state: UnitFileState::Enabled,
            fragment_path: None,
        },
        ServiceInfo {
            unit: "broken-demo.service".into(),
            description: "Intentionally failed demo unit".into(),
            load_state: "loaded".into(),
            active_state: "failed".into(),
            sub_state: "failed".into(),
            unit_path: "/org/freedesktop/systemd1/unit/broken_2ddemo_2eservice".into(),
            unit_file_state: UnitFileState::Disabled,
            fragment_path: None,
        },
        ServiceInfo {
            unit: "cron.service".into(),
            description: "Regular background program processing daemon".into(),
            load_state: "loaded".into(),
            active_state: "inactive".into(),
            sub_state: "dead".into(),
            unit_path: "/org/freedesktop/systemd1/unit/cron_2eservice".into(),
            unit_file_state: UnitFileState::Static,
            fragment_path: None,
        },
    ]
}

#[async_trait]
impl ServiceProvider for DemoServices {
    async fn list_services(&self) -> Result<Vec<ServiceInfo>, AppError> {
        let list = demo_service_list();
        self.registry
            .replace_all(list.iter().map(|s| s.unit.clone()));
        Ok(list)
    }

    async fn details(&self, unit: &str) -> Result<ServiceInfo, AppError> {
        let mut list = demo_service_list();
        let Some(mut info) = list.iter().find(|s| s.unit == unit).cloned() else {
            return Err(AppError::Systemd(format!("unknown unit {unit}")));
        };
        info.fragment_path = Some(format!("/lib/systemd/system/{unit}"));
        // silence unused mut warning if we only assign fragment
        let _ = &mut list;
        Ok(info)
    }

    async fn is_available(&self) -> bool {
        true
    }
}

struct DemoLogs;

#[async_trait]
impl LogProvider for DemoLogs {
    async fn recent(&self, unit: Option<&str>, lines: usize) -> Result<Vec<LogEntry>, AppError> {
        self.recent_preset(unit, lines, LogPreset::All).await
    }

    async fn recent_preset(
        &self,
        unit: Option<&str>,
        lines: usize,
        preset: LogPreset,
    ) -> Result<Vec<LogEntry>, AppError> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let unit_name = unit.unwrap_or("system").to_string();
        let mut out = Vec::with_capacity(lines.min(50));
        for i in 0..lines.min(50) {
            let prio = if i % 7 == 0 {
                LogPriority::Err
            } else if i % 5 == 0 {
                LogPriority::Warning
            } else {
                LogPriority::Info
            };
            let uname = if matches!(preset, LogPreset::Kernel) && i % 3 == 0 {
                "kernel".into()
            } else {
                unit_name.clone()
            };
            // Store realtime-ish µs for LastHour filter compatibility.
            let ts_us = now.saturating_sub((lines - i) as u64) * 1_000_000;
            out.push(LogEntry {
                timestamp: format!("{ts_us}"),
                unit: uname,
                pid: Some(1000 + i as u32),
                priority: prio,
                message: format!("Demo log line {i} for {unit_name}"),
            });
        }
        Ok(out)
    }

    async fn is_available(&self) -> bool {
        true
    }
}

struct DemoStorage;

#[async_trait]
impl StorageProvider for DemoStorage {
    async fn scan(
        &self,
        root: PathBuf,
        _stay_on_fs: bool,
        _follow_symlinks: bool,
        cancel: CancellationToken,
        progress: tokio::sync::mpsc::Sender<StorageProgress>,
    ) -> Result<StorageTree, AppError> {
        for step in 0..8u64 {
            if cancel.is_cancelled() {
                return Err(AppError::Storage("scan cancelled".into()));
            }
            let _ = progress
                .send(StorageProgress {
                    root: root.clone(),
                    files: step * 10,
                    dirs: step,
                    bytes: step * 100_000,
                    current: root.join(format!("demo-{step}")),
                    errors: 0,
                })
                .await;
            tokio::time::sleep(std::time::Duration::from_millis(80)).await;
        }

        let mut tree_root = StorageNode {
            name: root
                .file_name()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_else(|| root.display().to_string()),
            path: root.clone(),
            is_dir: true,
            size: 0,
            apparent_size: 0,
            children: vec![
                file_node(&root, "documents", true, 5_000_000),
                file_node(&root, "logs", true, 1_200_000),
                file_node(&root, "readme.txt", false, 4096),
            ],
            inaccessible: false,
            error: None,
        };
        if let Some(docs) = tree_root
            .children
            .iter_mut()
            .find(|c| c.name == "documents")
        {
            docs.children = vec![
                file_node(&docs.path, "report.pdf", false, 2_000_000),
                file_node(&docs.path, "notes.md", false, 12_000),
            ];
        }
        aggregate_sizes(&mut tree_root);
        Ok(StorageTree {
            root: tree_root,
            files: 4,
            dirs: 3,
            errors: 0,
            excluded: default_exclusions(),
            truncated: false,
        })
    }
}

fn file_node(parent: &Path, name: &str, is_dir: bool, size: u64) -> StorageNode {
    StorageNode {
        name: name.into(),
        path: parent.join(name),
        is_dir,
        size,
        apparent_size: size,
        children: vec![],
        inaccessible: false,
        error: None,
    }
}

struct DemoAdmin {
    read_only: bool,
    registry: UnitRegistry,
}

#[async_trait]
impl AdministrativeExecutor for DemoAdmin {
    async fn send_signal(
        &self,
        pid: u32,
        signal: ProcessSignal,
        _expected_start_time: Option<u64>,
    ) -> Result<(), AppError> {
        if self.read_only {
            return Err(AppError::Permission(
                "read-only mode: signals disabled".into(),
            ));
        }
        if pid == 0 || pid == 1 {
            return Err(AppError::Process("protected PID".into()));
        }
        if pid == 1001 {
            return Err(AppError::Permission(format!(
                "demo denied {} for PID {pid}",
                signal.label()
            )));
        }
        Ok(())
    }

    async fn service_action(&self, unit: &str, action: ServiceActionKind) -> Result<(), AppError> {
        if self.read_only {
            return Err(AppError::Permission(
                "read-only mode: systemd actions disabled".into(),
            ));
        }
        if !self.registry.contains(unit) {
            return Err(AppError::Systemd(format!(
                "refusing action on unregistered unit {unit}"
            )));
        }
        if unit == "broken-demo.service" && action == ServiceActionKind::Start {
            return Err(AppError::Systemd(
                "demo: start rejected for broken-demo.service".into(),
            ));
        }
        Ok(())
    }

    fn read_only(&self) -> bool {
        self.read_only
    }
}

struct DemoDiagnostics;

#[async_trait]
impl DiagnosticProbeProvider for DemoDiagnostics {
    async fn probe(&self, deep: bool, enable_smart: bool) -> Result<DiagnosticSnapshot, AppError> {
        let _ = (deep, enable_smart);
        // Cycle datasets slowly so demo UI shows variety.
        let dataset = DemoDataset::from_tick(tick() / 3);
        Ok(dataset.build())
    }
}
