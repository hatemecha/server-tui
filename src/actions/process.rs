//! Process signal execution with hard safety guards.

use async_trait::async_trait;
use nix::sys::signal::kill;
use nix::unistd::Pid;
use sysinfo::{Pid as SysPid, ProcessesToUpdate, System};

use crate::error::AppError;
use crate::model::{is_protected_pid, ProcessSignal, ServiceActionKind, UnitRegistry};
use crate::providers::linux::systemd::{self, unit_looks_safe};
use crate::providers::AdministrativeExecutor;

pub struct LinuxAdminExecutor {
    self_pid: u32,
    registry: UnitRegistry,
}

impl LinuxAdminExecutor {
    pub fn new(registry: UnitRegistry) -> Self {
        Self {
            self_pid: std::process::id(),
            registry,
        }
    }
}

#[async_trait]
impl AdministrativeExecutor for LinuxAdminExecutor {
    async fn send_signal(
        &self,
        pid: u32,
        signal: ProcessSignal,
        expected_start_time: Option<u64>,
    ) -> Result<(), AppError> {
        if is_protected_pid(pid, self.self_pid) {
            return Err(AppError::Process(format!(
                "refusing to send {} to protected PID {pid}",
                signal.label()
            )));
        }

        let result = tokio::task::spawn_blocking(move || {
            if let Some(expected) = expected_start_time {
                match read_process_start_time(pid) {
                    Ok(actual) if actual == expected => {}
                    Ok(_) => {
                        return Err(AppError::Process(format!(
                            "PID {pid} was reused (start time changed); signal aborted"
                        )));
                    }
                    Err(e) => return Err(e),
                }
            }

            let nix_pid = Pid::from_raw(pid as i32);
            // Success = kernel accepted the signal (delivered to the task).
            // Presence of /proc after SIGKILL is not a failure (exit can race).
            kill(nix_pid, signal.as_nix()).map_err(|e| match e {
                nix::errno::Errno::ESRCH => {
                    AppError::Process(format!("process {pid} no longer exists"))
                }
                nix::errno::Errno::EPERM => AppError::Permission(format!(
                    "permission denied sending {} to PID {pid}",
                    signal.label()
                )),
                other => AppError::Process(format!(
                    "failed to send {} to {pid}: {other}",
                    signal.label()
                )),
            })
        })
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        result
    }

    async fn service_action(&self, unit: &str, action: ServiceActionKind) -> Result<(), AppError> {
        if !unit_looks_safe(unit) {
            return Err(AppError::Systemd("refusing unsafe unit name".into()));
        }
        systemd::LinuxServiceProvider::perform_action(unit, action, Some(&self.registry)).await
    }

    fn read_only(&self) -> bool {
        false
    }

    fn unit_registry(&self) -> Option<UnitRegistry> {
        Some(self.registry.clone())
    }
}

fn read_process_start_time(pid: u32) -> Result<u64, AppError> {
    let mut system = System::new();
    let sys_pid = SysPid::from_u32(pid);
    system.refresh_processes(ProcessesToUpdate::Some(&[sys_pid]), true);
    system
        .process(sys_pid)
        .map(|p| p.start_time())
        .ok_or_else(|| AppError::Process(format!("process {pid} no longer exists")))
}

pub struct ReadOnlyExecutor;

#[async_trait]
impl AdministrativeExecutor for ReadOnlyExecutor {
    async fn send_signal(
        &self,
        _pid: u32,
        _signal: ProcessSignal,
        _expected_start_time: Option<u64>,
    ) -> Result<(), AppError> {
        Err(AppError::Permission(
            "READ ONLY: process signals are disabled".into(),
        ))
    }

    async fn service_action(
        &self,
        _unit: &str,
        _action: ServiceActionKind,
    ) -> Result<(), AppError> {
        Err(AppError::Permission(
            "READ ONLY: systemd actions are disabled".into(),
        ))
    }

    fn read_only(&self) -> bool {
        true
    }
}
