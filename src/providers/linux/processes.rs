//! Process listing via sysinfo.

use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use sysinfo::{ProcessesToUpdate, System, Users};

use crate::error::AppError;
use crate::model::{compute_memory_percentage, ProcessDetails, ProcessInfo};
use crate::providers::linux::proc_details::{read_ppid_map, read_process_details};
use crate::providers::ProcessProvider;
use crate::sanitize::{sanitize_cell, sanitize_text};

pub struct LinuxProcessProvider {
    system: Arc<Mutex<System>>,
    users: Arc<Mutex<Users>>,
}

impl Default for LinuxProcessProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl LinuxProcessProvider {
    pub fn new() -> Self {
        Self {
            system: Arc::new(Mutex::new(System::new())),
            users: Arc::new(Mutex::new(Users::new_with_refreshed_list())),
        }
    }
}

#[async_trait]
impl ProcessProvider for LinuxProcessProvider {
    async fn list(&self) -> Result<Vec<ProcessInfo>, AppError> {
        let system = Arc::clone(&self.system);
        let users = Arc::clone(&self.users);
        tokio::task::spawn_blocking(move || list_blocking(&system, &users))
            .await
            .map_err(|e| AppError::Internal(format!("process list join: {e}")))?
    }

    async fn details(&self, pid: u32) -> Result<ProcessDetails, AppError> {
        tokio::task::spawn_blocking(move || read_process_details(pid))
            .await
            .map_err(|e| AppError::Internal(format!("process details join: {e}")))?
    }
}

pub async fn ppid_map(pids: Vec<u32>) -> Result<Vec<(u32, Option<u32>)>, AppError> {
    tokio::task::spawn_blocking(move || read_ppid_map(&pids))
        .await
        .map_err(|e| AppError::Internal(format!("ppid map join: {e}")))
}

fn list_blocking(
    system: &Mutex<System>,
    users: &Mutex<Users>,
) -> Result<Vec<ProcessInfo>, AppError> {
    let mut system = system
        .lock()
        .map_err(|_| AppError::Internal("process lock poisoned".into()))?;
    let users = users
        .lock()
        .map_err(|_| AppError::Internal("users lock poisoned".into()))?;

    // Refresh RAM totals first — process %.MEM requires a known total.
    system.refresh_memory();
    system.refresh_processes(ProcessesToUpdate::All, true);
    let total_mem = system.total_memory();

    let mut out = Vec::new();
    for (pid, proc_) in system.processes() {
        let uid = proc_.user_id();
        let user = uid
            .and_then(|u| users.get_user_by_id(u))
            .map(|u| u.name().to_string())
            .unwrap_or_else(|| uid.map(|u| u.to_string()).unwrap_or_else(|| "?".into()));
        let mem_bytes = proc_.memory();
        let mem_pct = compute_memory_percentage(mem_bytes, total_mem);
        let cmd = if proc_.cmd().is_empty() {
            proc_.name().to_string_lossy().into_owned()
        } else {
            proc_
                .cmd()
                .iter()
                .map(|s| s.to_string_lossy())
                .collect::<Vec<_>>()
                .join(" ")
        };
        out.push(ProcessInfo {
            pid: pid.as_u32(),
            user: sanitize_cell(&user),
            name: sanitize_cell(&proc_.name().to_string_lossy()),
            cmd: sanitize_text(&cmd),
            cpu: proc_.cpu_usage(),
            mem_pct,
            mem_bytes,
            state: sanitize_cell(&format!("{:?}", proc_.status())),
            run_time_secs: proc_.run_time(),
            start_time: proc_.start_time(),
        });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use crate::model::compute_memory_percentage;

    #[test]
    fn mem_pct_unknown_without_total() {
        assert!(compute_memory_percentage(4096, 0).is_none());
    }
}
