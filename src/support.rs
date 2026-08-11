//! Redacted support report generation (CLI + TUI export helper).

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;

use crate::error::AppError;
use crate::model::{Finding, HealthStatus, ProcessInfo, ServiceInfo, SystemMetrics};
use crate::sanitize::{sanitize_path_display, sanitize_text};
use crate::{APP_NAME, APP_VERSION, CONFIG_DIR_NAME};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SupportFormat {
    #[default]
    Text,
    Markdown,
    Json,
}

/// Formal redaction policy for exports / support / doctor reports.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RedactionPolicy {
    /// When false (default), command lines are truncated to binary name.
    pub include_sensitive: bool,
}

impl RedactionPolicy {
    pub fn safe() -> Self {
        Self::default()
    }

    pub fn include_sensitive() -> Self {
        Self {
            include_sensitive: true,
        }
    }

    pub fn redact_cmd(self, cmd: &str) -> String {
        redact_cmd(cmd, self.include_sensitive)
    }
}

impl SupportFormat {
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "text" | "txt" => Some(Self::Text),
            "markdown" | "md" => Some(Self::Markdown),
            "json" => Some(Self::Json),
            _ => None,
        }
    }

    pub fn extension(self) -> &'static str {
        match self {
            Self::Text => "txt",
            Self::Markdown => "md",
            Self::Json => "json",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct SupportReport {
    pub app: String,
    pub version: String,
    pub generated_at_unix: u64,
    pub health: String,
    pub hostname: String,
    pub demo: bool,
    pub read_only: bool,
    pub findings: Vec<SupportFinding>,
    pub top_processes: Vec<SupportProcess>,
    pub failed_services: Vec<String>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SupportFinding {
    pub id: String,
    pub severity: String,
    pub title: String,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SupportProcess {
    pub pid: u32,
    pub name: String,
    pub user: String,
    pub cpu: f32,
    pub mem: String,
    pub cmd_redacted: String,
}

pub fn redact_cmd(cmd: &str, include_sensitive: bool) -> String {
    if include_sensitive {
        return sanitize_text(cmd);
    }
    // Keep argv0 only by default.
    let first = cmd.split_whitespace().next().unwrap_or(cmd);
    sanitize_text(first)
}

#[allow(clippy::too_many_arguments)]
pub fn build_report(
    metrics: &SystemMetrics,
    findings: &[Finding],
    processes: &[ProcessInfo],
    services: &[ServiceInfo],
    health: HealthStatus,
    demo: bool,
    read_only: bool,
    include_sensitive: bool,
) -> SupportReport {
    let mut top: Vec<_> = processes.iter().collect();
    top.sort_by(|a, b| {
        b.cpu
            .partial_cmp(&a.cpu)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    top.truncate(10);

    let failed: Vec<String> = services
        .iter()
        .filter(|s| s.active_state.eq_ignore_ascii_case("failed"))
        .map(|s| s.unit.clone())
        .collect();

    SupportReport {
        app: APP_NAME.into(),
        version: APP_VERSION.into(),
        generated_at_unix: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0),
        health: health.label().into(),
        hostname: sanitize_text(&metrics.hostname),
        demo,
        read_only,
        findings: findings
            .iter()
            .take(50)
            .map(|f| SupportFinding {
                id: f.id.clone(),
                severity: f.severity.label().into(),
                title: sanitize_text(&f.title),
                summary: sanitize_text(&f.summary),
            })
            .collect(),
        top_processes: top
            .into_iter()
            .map(|p| SupportProcess {
                pid: p.pid,
                name: p.name.clone(),
                user: p.user.clone(),
                cpu: p.cpu,
                mem: crate::model::format_mem_pct(p.mem_pct),
                cmd_redacted: redact_cmd(&p.cmd, include_sensitive),
            })
            .collect(),
        failed_services: failed,
        notes: vec![
            "Redacted by default (cmd = argv0). Use --include-sensitive to expand.".into(),
            format!(
                "scan path display policy uses sanitize: {}",
                sanitize_path_display(Path::new("/tmp/example"))
            ),
        ],
    }
}

pub fn render(report: &SupportReport, format: SupportFormat) -> Result<String, AppError> {
    match format {
        SupportFormat::Json => serde_json::to_string_pretty(report)
            .map_err(|e| AppError::Internal(format!("json: {e}"))),
        SupportFormat::Markdown => Ok(render_markdown(report)),
        SupportFormat::Text => Ok(render_text(report)),
    }
}

fn render_text(r: &SupportReport) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "{APP_NAME} support report v{}\nhealth={} host={} demo={} read_only={}\n\n",
        r.version, r.health, r.hostname, r.demo, r.read_only
    ));
    out.push_str("Findings:\n");
    if r.findings.is_empty() {
        out.push_str("- (none)\n");
    } else {
        for f in &r.findings {
            out.push_str(&format!("- [{}] {}: {}\n", f.severity, f.id, f.title));
        }
    }
    out.push_str("\nTop processes:\n");
    if r.top_processes.is_empty() {
        out.push_str("- (none)\n");
    } else {
        for p in &r.top_processes {
            out.push_str(&format!(
                "- pid={} cpu={:.1} mem={} {} ({})\n",
                p.pid, p.cpu, p.mem, p.name, p.cmd_redacted
            ));
        }
    }
    out.push_str("\nFailed services:\n");
    if r.failed_services.is_empty() {
        out.push_str("- (none)\n");
    } else {
        for u in &r.failed_services {
            out.push_str(&format!("- {u}\n"));
        }
    }
    out
}

fn render_markdown(r: &SupportReport) -> String {
    let mut out = format!(
        "# {APP_NAME} support report\n\n- version: `{}`\n- health: **{}**\n- host: `{}`\n- demo: `{}`\n- read_only: `{}`\n\n## Findings\n\n",
        r.version, r.health, r.hostname, r.demo, r.read_only
    );
    if r.findings.is_empty() {
        out.push_str("_none_\n");
    } else {
        for f in &r.findings {
            out.push_str(&format!("- **{}** `{}`: {}\n", f.severity, f.id, f.title));
        }
    }
    out.push_str("\n## Top processes\n\n");
    if r.top_processes.is_empty() {
        out.push_str("_none_\n");
    } else {
        for p in &r.top_processes {
            out.push_str(&format!(
                "- `{}` pid={} cpu={:.1}% mem={} — `{}`\n",
                p.name, p.pid, p.cpu, p.mem, p.cmd_redacted
            ));
        }
    }
    out.push_str("\n## Failed services\n\n");
    if r.failed_services.is_empty() {
        out.push_str("_none_\n");
    } else {
        for u in &r.failed_services {
            out.push_str(&format!("- `{u}`\n"));
        }
    }
    out
}

pub fn default_reports_dir() -> PathBuf {
    dirs::state_dir()
        .or_else(dirs::data_local_dir)
        .unwrap_or_else(|| PathBuf::from("."))
        .join(CONFIG_DIR_NAME)
        .join("reports")
}

pub fn save_report(
    body: &str,
    format: SupportFormat,
    output: Option<&Path>,
) -> Result<PathBuf, AppError> {
    let path = if let Some(p) = output {
        p.to_path_buf()
    } else {
        let dir = default_reports_dir();
        crate::fsutil::ensure_private_dir(&dir)?;
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        dir.join(format!("support-{ts}.{}", format.extension()))
    };
    crate::fsutil::write_private_atomic(&path, body.as_bytes())?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::SystemMetrics;

    #[test]
    fn redacts_cmd_by_default() {
        assert_eq!(redact_cmd("/usr/bin/foo --secret=1", false), "/usr/bin/foo");
    }

    #[test]
    fn builds_and_renders_json() {
        let r = build_report(
            &SystemMetrics::default(),
            &[],
            &[],
            &[],
            HealthStatus::Ok,
            true,
            true,
            false,
        );
        let s = render(&r, SupportFormat::Json).expect("json");
        assert!(s.contains("server-tui"));
    }

    #[test]
    fn markdown_includes_flags_and_empty_placeholders() {
        let r = build_report(
            &SystemMetrics::default(),
            &[],
            &[],
            &[],
            HealthStatus::Ok,
            true,
            true,
            false,
        );
        let md = render(&r, SupportFormat::Markdown).expect("md");
        assert!(md.contains("demo: `true`"));
        assert!(md.contains("read_only: `true`"));
        assert!(md.contains("_none_"));
        assert!(md.contains("## Failed services"));
    }
}
