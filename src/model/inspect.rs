//! On-demand inspection models (processes, logs, files, action menus).

use crate::model::{LogEntry, ProcessInfo, ProcessSignal, ServiceActionKind, ServiceInfo};

#[derive(Debug, Clone, Default)]
pub struct ProcessDetails {
    pub info: Option<ProcessInfo>,
    pub parent_pid: Option<u32>,
    pub parent_name: Option<String>,
    pub cmdline: String,
    pub cgroup: Option<String>,
    pub unit: Option<String>,
    pub fd_count: Option<usize>,
    pub read_bytes: Option<u64>,
    pub write_bytes: Option<u64>,
}

impl ProcessDetails {
    pub fn format_body(&self) -> String {
        let info = self.info.as_ref();
        let mut lines = Vec::new();
        if let Some(p) = info {
            lines.push(format!(
                "PID {}  user {}  state {}  cpu {:.1}%  mem {}",
                p.pid,
                p.user,
                p.state,
                p.cpu,
                crate::model::format_mem_pct(p.mem_pct)
            ));
            lines.push(format!("name: {}", p.name));
        }
        lines.push(format!(
            "parent: {} ({})",
            self.parent_pid
                .map(|p| p.to_string())
                .unwrap_or_else(|| "?".into()),
            self.parent_name.as_deref().unwrap_or("?")
        ));
        lines.push(format!("cmdline: {}", self.cmdline));
        lines.push(format!(
            "cgroup: {}",
            self.cgroup.as_deref().unwrap_or("(unknown)")
        ));
        lines.push(format!(
            "unit: {}",
            self.unit.as_deref().unwrap_or("(none)")
        ));
        lines.push(format!(
            "fds: {}",
            self.fd_count
                .map(|n| n.to_string())
                .unwrap_or_else(|| "?".into())
        ));
        lines.push(format!(
            "io read: {}  write: {}",
            self.read_bytes
                .map(crate::model::format_bytes)
                .unwrap_or_else(|| "?".into()),
            self.write_bytes
                .map(crate::model::format_bytes)
                .unwrap_or_else(|| "?".into())
        ));
        lines.join("\n")
    }
}

#[derive(Debug, Clone)]
pub struct ServiceInspectBundle {
    pub service: ServiceInfo,
    pub recent_logs: Vec<LogEntry>,
}

impl ServiceInspectBundle {
    pub fn format_body(&self) -> String {
        let s = &self.service;
        let mut lines = vec![
            format!("unit: {}", s.unit),
            format!(
                "active: {} ({})  load: {}  enabled: {}",
                s.active_state,
                s.sub_state,
                s.load_state,
                s.unit_file_state.label()
            ),
            format!(
                "fragment: {}",
                s.fragment_path.as_deref().unwrap_or("(unknown)")
            ),
            s.description.clone(),
            String::new(),
            "Recent logs:".into(),
        ];
        if self.recent_logs.is_empty() {
            lines.push("  (none)".into());
        } else {
            for e in &self.recent_logs {
                lines.push(format!(
                    "  {} {:>5} {}",
                    e.timestamp,
                    e.priority.label(),
                    e.message
                ));
            }
        }
        lines.join("\n")
    }
}

#[derive(Debug, Clone)]
pub struct LogInspectContext {
    pub index: usize,
    pub before: Vec<LogEntry>,
    pub focus: LogEntry,
    pub after: Vec<LogEntry>,
}

impl LogInspectContext {
    pub fn format_body(&self) -> String {
        let mut lines = Vec::new();
        lines.push("--- before ---".into());
        for e in &self.before {
            lines.push(format_log_line(e));
        }
        lines.push("--- selected ---".into());
        lines.push(format_log_line(&self.focus));
        lines.push("--- after ---".into());
        for e in &self.after {
            lines.push(format_log_line(e));
        }
        lines.join("\n")
    }
}

fn format_log_line(e: &LogEntry) -> String {
    format!(
        "{} {:>5} {:<20} {}",
        e.timestamp,
        e.priority.label(),
        e.unit,
        e.message
    )
}

#[derive(Debug, Clone)]
pub struct FilePreview {
    pub path: String,
    pub bytes_read: usize,
    pub truncated: bool,
    pub is_binary: bool,
    pub text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    Text,
    Markdown,
    Json,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuAction {
    ProcessInspect,
    ProcessFollow,
    ProcessTree,
    ProcessLogs,
    ProcessService,
    ProcessDiagnostics,
    ProcessExport,
    ProcessSignal(ProcessSignal),
    ServiceInspect,
    ServiceLogs,
    ServiceDiagnostics,
    ServiceExport,
    ServiceAction(ServiceActionKind),
    LogExportSelected(ExportFormat),
    LogExportVisible(ExportFormat),
    LogExportContext(ExportFormat),
    FindingExport,
    StoragePreview,
    Close,
}

#[derive(Debug, Clone)]
pub struct MenuItem {
    pub label: String,
    pub action: MenuAction,
    pub enabled: bool,
    pub disabled_reason: Option<String>,
}

impl MenuItem {
    pub fn enabled(label: impl Into<String>, action: MenuAction) -> Self {
        Self {
            label: label.into(),
            action,
            enabled: true,
            disabled_reason: None,
        }
    }

    pub fn disabled(
        label: impl Into<String>,
        action: MenuAction,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            label: label.into(),
            action,
            enabled: false,
            disabled_reason: Some(reason.into()),
        }
    }
}

/// Build process tree rows (on-demand; indent by depth). Returns (pid, display name, depth).
/// Cycle/corrupt PPID graphs are guarded with a visited set and max depth.
pub fn build_process_tree(
    procs: &[ProcessInfo],
    details_ppid: &[(u32, Option<u32>)],
) -> Vec<(u32, String, usize)> {
    use std::collections::{HashMap, HashSet};
    const MAX_DEPTH: usize = 64;
    let mut children: HashMap<u32, Vec<u32>> = HashMap::new();
    let mut ppid_of: HashMap<u32, Option<u32>> = HashMap::new();
    let names: HashMap<u32, String> = procs.iter().map(|p| (p.pid, p.name.clone())).collect();
    for (pid, ppid) in details_ppid {
        // Ignore self-parent / nonsense that would loop immediately.
        if ppid == &Some(*pid) {
            ppid_of.insert(*pid, None);
            continue;
        }
        ppid_of.insert(*pid, *ppid);
        if let Some(parent) = ppid {
            children.entry(*parent).or_default().push(*pid);
        }
    }
    for p in procs {
        ppid_of.entry(p.pid).or_insert(None);
    }
    let mut roots: Vec<u32> = procs
        .iter()
        .filter(|p| {
            ppid_of
                .get(&p.pid)
                .copied()
                .flatten()
                .map(|pp| !names.contains_key(&pp))
                .unwrap_or(true)
        })
        .map(|p| p.pid)
        .collect();
    roots.sort_unstable();
    let mut out = Vec::new();
    let mut visited = HashSet::new();
    fn walk(
        pid: u32,
        depth: usize,
        children: &HashMap<u32, Vec<u32>>,
        names: &HashMap<u32, String>,
        visited: &mut HashSet<u32>,
        out: &mut Vec<(u32, String, usize)>,
    ) {
        if depth > MAX_DEPTH || !visited.insert(pid) {
            return;
        }
        let name = names.get(&pid).cloned().unwrap_or_else(|| "?".into());
        out.push((pid, name, depth));
        if let Some(kids) = children.get(&pid) {
            let mut kids = kids.clone();
            kids.sort_unstable();
            for k in kids {
                walk(k, depth + 1, children, names, visited, out);
            }
        }
    }
    for r in roots {
        walk(r, 0, &children, &names, &mut visited, &mut out);
    }
    // Orphans not reached (corrupt graph) — append flat.
    for p in procs {
        if visited.insert(p.pid) {
            out.push((p.pid, p.name.clone(), 0));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tree_orders_children() {
        let procs = vec![
            ProcessInfo {
                pid: 1,
                user: "root".into(),
                name: "init".into(),
                cmd: "init".into(),
                cpu: 0.0,
                mem_pct: Some(0.1),
                mem_bytes: 1,
                state: "S".into(),
                run_time_secs: 1,
                start_time: 1,
            },
            ProcessInfo {
                pid: 10,
                user: "root".into(),
                name: "child".into(),
                cmd: "child".into(),
                cpu: 0.0,
                mem_pct: Some(0.1),
                mem_bytes: 1,
                state: "S".into(),
                run_time_secs: 1,
                start_time: 1,
            },
        ];
        let tree = build_process_tree(&procs, &[(1, None), (10, Some(1))]);
        assert_eq!(tree[0].0, 1);
        assert_eq!(tree[1].0, 10);
        assert_eq!(tree[1].2, 1);
    }
}
