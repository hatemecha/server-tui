//! Read process details from /proc (on-demand, spawn_blocking).

use std::fs;
use std::path::PathBuf;

use sysinfo::{Pid, ProcessesToUpdate, System};

use crate::error::AppError;
use crate::model::{compute_memory_percentage, ProcessDetails, ProcessInfo};
use crate::sanitize::{sanitize_cell, sanitize_text};

pub fn read_process_details(pid: u32) -> Result<ProcessDetails, AppError> {
    let mut system = System::new();
    system.refresh_memory();
    let sys_pid = Pid::from_u32(pid);
    system.refresh_processes(ProcessesToUpdate::Some(&[sys_pid]), true);
    let total = system.total_memory();

    let info = system.process(sys_pid).map(|proc_| {
        let mem_bytes = proc_.memory();
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
        ProcessInfo {
            pid,
            user: "?".into(),
            name: sanitize_cell(&proc_.name().to_string_lossy()),
            cmd: sanitize_text(&cmd),
            cpu: proc_.cpu_usage(),
            mem_pct: compute_memory_percentage(mem_bytes, total),
            mem_bytes,
            state: sanitize_cell(&format!("{:?}", proc_.status())),
            run_time_secs: proc_.run_time(),
            start_time: proc_.start_time(),
        }
    });

    let cmdline = read_cmdline(pid)
        .unwrap_or_else(|_| info.as_ref().map(|i| i.cmd.clone()).unwrap_or_default());
    let parent_pid = read_ppid(pid).ok().flatten();
    let parent_name = parent_pid.and_then(|pp| {
        let ppid = Pid::from_u32(pp);
        system.refresh_processes(ProcessesToUpdate::Some(&[ppid]), true);
        system
            .process(ppid)
            .map(|p| sanitize_cell(&p.name().to_string_lossy()))
    });
    let cgroup = read_cgroup(pid).ok();
    let unit = cgroup.as_ref().and_then(|cg| extract_unit(cg));
    let fd_count = count_fds(pid).ok();
    let (read_bytes, write_bytes) = read_io(pid).unwrap_or((None, None));

    Ok(ProcessDetails {
        info,
        parent_pid,
        parent_name,
        cmdline: sanitize_text(&cmdline),
        cgroup: cgroup.map(|c| sanitize_text(&c)),
        unit,
        fd_count,
        read_bytes,
        write_bytes,
    })
}

/// Cheap PPID map for tree view (stat field only).
pub fn read_ppid_map(pids: &[u32]) -> Vec<(u32, Option<u32>)> {
    pids.iter()
        .map(|pid| (*pid, read_ppid(*pid).ok().flatten()))
        .collect()
}

fn proc_path(pid: u32, leaf: &str) -> PathBuf {
    PathBuf::from(format!("/proc/{pid}/{leaf}"))
}

fn read_cmdline(pid: u32) -> Result<String, AppError> {
    let raw = fs::read(proc_path(pid, "cmdline")).map_err(|e| AppError::Process(e.to_string()))?;
    let s = raw
        .split(|b| *b == 0)
        .filter(|p| !p.is_empty())
        .map(|p| String::from_utf8_lossy(p))
        .collect::<Vec<_>>()
        .join(" ");
    Ok(s)
}

fn read_ppid(pid: u32) -> Result<Option<u32>, AppError> {
    let stat =
        fs::read_to_string(proc_path(pid, "stat")).map_err(|e| AppError::Process(e.to_string()))?;
    // Format: pid (comm) state ppid ...
    let after_comm = stat
        .rfind(')')
        .map(|i| &stat[i + 1..])
        .ok_or_else(|| AppError::Process("bad stat".into()))?;
    let mut parts = after_comm.split_whitespace();
    let _state = parts.next();
    let ppid = parts
        .next()
        .and_then(|s| s.parse::<u32>().ok())
        .filter(|p| *p > 0);
    Ok(ppid)
}

fn read_cgroup(pid: u32) -> Result<String, AppError> {
    let raw = fs::read_to_string(proc_path(pid, "cgroup"))
        .map_err(|e| AppError::Process(e.to_string()))?;
    Ok(raw.lines().next().unwrap_or("").to_string())
}

fn extract_unit(cgroup_line: &str) -> Option<String> {
    // .service appears in paths like .../system.slice/ssh.service
    for part in cgroup_line.split('/') {
        if part.ends_with(".service") && crate::providers::linux::systemd::unit_looks_safe(part) {
            return Some(part.to_string());
        }
    }
    None
}

fn count_fds(pid: u32) -> Result<usize, AppError> {
    let rd = fs::read_dir(proc_path(pid, "fd")).map_err(|e| AppError::Process(e.to_string()))?;
    Ok(rd.count())
}

fn read_io(pid: u32) -> Result<(Option<u64>, Option<u64>), AppError> {
    let raw =
        fs::read_to_string(proc_path(pid, "io")).map_err(|e| AppError::Process(e.to_string()))?;
    let mut read_b = None;
    let mut write_b = None;
    for line in raw.lines() {
        if let Some(v) = line.strip_prefix("read_bytes: ") {
            read_b = v.trim().parse().ok();
        } else if let Some(v) = line.strip_prefix("write_bytes: ") {
            write_b = v.trim().parse().ok();
        }
    }
    Ok((read_b, write_b))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_self_details() {
        let pid = std::process::id();
        let d = read_process_details(pid).expect("self");
        assert!(d.info.is_some() || !d.cmdline.is_empty() || d.parent_pid.is_some());
    }
}
