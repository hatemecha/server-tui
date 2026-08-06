//! Process models and filtering/sorting helpers.

use std::cmp::Ordering;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessSignal {
    Term,
    Kill,
}

impl ProcessSignal {
    pub fn as_nix(self) -> nix::sys::signal::Signal {
        match self {
            Self::Term => nix::sys::signal::Signal::SIGTERM,
            Self::Kill => nix::sys::signal::Signal::SIGKILL,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Term => "SIGTERM",
            Self::Kill => "SIGKILL",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ProcessSort {
    #[default]
    Cpu,
    Memory,
    Pid,
    Name,
}

impl ProcessSort {
    pub fn next(self) -> Self {
        match self {
            Self::Cpu => Self::Memory,
            Self::Memory => Self::Pid,
            Self::Pid => Self::Name,
            Self::Name => Self::Cpu,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Cpu => "cpu",
            Self::Memory => "mem",
            Self::Pid => "pid",
            Self::Name => "name",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ProcessInfo {
    pub pid: u32,
    pub user: String,
    pub name: String,
    pub cmd: String,
    pub cpu: f32,
    pub mem_pct: f32,
    pub mem_bytes: u64,
    pub state: String,
    pub run_time_secs: u64,
    /// Process start time (unix seconds). Used to resist PID reuse on signal confirm.
    pub start_time: u64,
}

pub fn filter_processes<'a>(items: &'a [ProcessInfo], query: &str) -> Vec<&'a ProcessInfo> {
    if query.is_empty() {
        return items.iter().collect();
    }
    let q = query.to_lowercase();
    items
        .iter()
        .filter(|p| {
            p.name.to_lowercase().contains(&q)
                || p.cmd.to_lowercase().contains(&q)
                || p.user.to_lowercase().contains(&q)
                || p.pid.to_string().contains(&q)
        })
        .collect()
}

pub fn sort_processes(items: &mut [&ProcessInfo], sort: ProcessSort) {
    items.sort_by(|a, b| match sort {
        ProcessSort::Cpu => b
            .cpu
            .partial_cmp(&a.cpu)
            .unwrap_or(Ordering::Equal)
            .then_with(|| a.pid.cmp(&b.pid)),
        ProcessSort::Memory => b
            .mem_pct
            .partial_cmp(&a.mem_pct)
            .unwrap_or(Ordering::Equal)
            .then_with(|| a.pid.cmp(&b.pid)),
        ProcessSort::Pid => a.pid.cmp(&b.pid),
        ProcessSort::Name => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
    });
}

/// Preserve selection by PID across refreshes. Returns the new index or 0.
pub fn preserve_selection(filtered: &[&ProcessInfo], selected_pid: Option<u32>) -> usize {
    if filtered.is_empty() {
        return 0;
    }
    if let Some(pid) = selected_pid {
        if let Some(idx) = filtered.iter().position(|p| p.pid == pid) {
            return idx;
        }
    }
    0
}

pub fn is_protected_pid(pid: u32, self_pid: u32) -> bool {
    pid == 0 || pid == 1 || pid == self_pid
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Vec<ProcessInfo> {
        vec![
            ProcessInfo {
                pid: 10,
                user: "root".into(),
                name: "alpha".into(),
                cmd: "/usr/bin/alpha".into(),
                cpu: 1.0,
                mem_pct: 5.0,
                mem_bytes: 1000,
                state: "S".into(),
                run_time_secs: 10,
                start_time: 1000,
            },
            ProcessInfo {
                pid: 20,
                user: "alex".into(),
                name: "beta".into(),
                cmd: "beta --flag".into(),
                cpu: 20.0,
                mem_pct: 1.0,
                mem_bytes: 500,
                state: "R".into(),
                run_time_secs: 5,
                start_time: 2000,
            },
        ]
    }

    #[test]
    fn filters_by_user_and_pid() {
        let items = sample();
        assert_eq!(filter_processes(&items, "alex").len(), 1);
        assert_eq!(filter_processes(&items, "20").len(), 1);
    }

    #[test]
    fn sorts_by_cpu() {
        let items = sample();
        let mut refs: Vec<_> = items.iter().collect();
        sort_processes(&mut refs, ProcessSort::Cpu);
        assert_eq!(refs[0].pid, 20);
    }

    #[test]
    fn preserves_pid_selection() {
        let items = sample();
        let refs: Vec<_> = items.iter().collect();
        assert_eq!(preserve_selection(&refs, Some(20)), 1);
        assert_eq!(preserve_selection(&refs, Some(999)), 0);
    }

    #[test]
    fn protects_special_pids() {
        assert!(is_protected_pid(0, 100));
        assert!(is_protected_pid(1, 100));
        assert!(is_protected_pid(100, 100));
        assert!(!is_protected_pid(42, 100));
    }
}
