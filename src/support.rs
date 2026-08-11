//! Shareable-by-default support report generation (CLI + TUI export helper).

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;

use crate::error::AppError;
use crate::model::{Finding, HealthStatus, ProcessInfo, ServiceInfo, SystemMetrics};
use crate::sanitize::{sanitize_path_display, sanitize_text};
use crate::{APP_NAME, APP_VERSION, CONFIG_DIR_NAME};

static REPORT_NONCE: AtomicU64 = AtomicU64::new(0);

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
    /// Full mode disables privacy reductions; terminal sanitization still applies.
    pub include_sensitive: bool,
}

impl RedactionPolicy {
    pub fn shareable() -> Self {
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

struct Redactor {
    policy: RedactionPolicy,
    hostname: String,
    users: Vec<String>,
    home: Option<String>,
}

impl Redactor {
    fn new(policy: RedactionPolicy, metrics: &SystemMetrics, processes: &[ProcessInfo]) -> Self {
        let mut users: Vec<_> = processes
            .iter()
            .map(|process| sanitize_text(&process.user))
            .filter(|user| !user.is_empty())
            .collect();
        users.sort();
        users.dedup();
        Self {
            policy,
            hostname: sanitize_text(&metrics.hostname),
            users,
            home: dirs::home_dir().map(|path| sanitize_path_display(&path)),
        }
    }

    fn text(&self, value: &str) -> String {
        let mut value = sanitize_text(value);
        if self.policy.include_sensitive {
            return value;
        }
        if let Some(home) = &self.home {
            if !home.is_empty() {
                value = value.replace(home, "$HOME");
            }
        }
        if !self.hostname.is_empty() {
            value = value.replace(&self.hostname, "<host>");
        }
        for user in &self.users {
            value = value.replace(user, "<user>");
        }
        mask_ip_addresses(&value)
    }

    fn hostname(&self) -> String {
        if self.policy.include_sensitive {
            self.text(&self.hostname)
        } else {
            "<host>".into()
        }
    }

    fn user(&self, user: &str) -> String {
        if self.policy.include_sensitive {
            self.text(user)
        } else {
            "<user>".into()
        }
    }
}

fn mask_ip_addresses(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    let mut token = String::new();
    let flush = |token: &mut String, out: &mut String| {
        if !token.is_empty() {
            if token.parse::<std::net::IpAddr>().is_ok() {
                out.push_str("<ip>");
            } else {
                out.push_str(token);
            }
            token.clear();
        }
    };
    for ch in value.chars() {
        if ch.is_ascii_hexdigit() || matches!(ch, '.' | ':' | '%') {
            token.push(ch);
        } else {
            flush(&mut token, &mut out);
            out.push(ch);
        }
    }
    flush(&mut token, &mut out);
    out
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
    let policy = if include_sensitive {
        RedactionPolicy::include_sensitive()
    } else {
        RedactionPolicy::shareable()
    };
    let redactor = Redactor::new(policy, metrics, processes);
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
        .map(|s| redactor.text(&s.unit))
        .collect();

    SupportReport {
        app: APP_NAME.into(),
        version: APP_VERSION.into(),
        generated_at_unix: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0),
        health: health.label().into(),
        hostname: redactor.hostname(),
        demo,
        read_only,
        findings: findings
            .iter()
            .take(50)
            .map(|f| SupportFinding {
                id: redactor.text(&f.id),
                severity: f.severity.label().into(),
                title: redactor.text(&f.title),
                summary: redactor.text(&f.summary),
            })
            .collect(),
        top_processes: top
            .into_iter()
            .map(|p| SupportProcess {
                pid: p.pid,
                name: redactor.text(&p.name),
                user: redactor.user(&p.user),
                cpu: p.cpu,
                mem: crate::model::format_mem_pct(p.mem_pct),
                cmd_redacted: redactor.text(&policy.redact_cmd(&p.cmd)),
            })
            .collect(),
        failed_services: failed,
        notes: vec![
            "Shareable mode masks recognized host, user, IP and home-path values; command arguments are omitted.".into(),
            "PIDs, findings, and non-identity portions of process/systemd unit names remain for troubleshooting; unit names may identify a workload.".into(),
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
            out.push_str(&format!("  {}\n", f.summary));
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

fn markdown_text(value: &str) -> String {
    let value = sanitize_text(value);
    let mut out = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '\\' | '`' | '*' | '_' | '{' | '}' | '[' | ']' | '(' | ')' | '#' | '+' | '-' | '.'
            | '!' | '|' => {
                out.push('\\');
                out.push(ch);
            }
            _ => out.push(ch),
        }
    }
    out
}

fn render_markdown(r: &SupportReport) -> String {
    let mut out = format!(
        "# {APP_NAME} support report\n\n- version: {}\n- health: **{}**\n- host: {}\n- demo: {}\n- read_only: {}\n\n## Findings\n\n",
        markdown_text(&r.version),
        markdown_text(&r.health),
        markdown_text(&r.hostname),
        r.demo,
        r.read_only
    );
    if r.findings.is_empty() {
        out.push_str("_none_\n");
    } else {
        for f in &r.findings {
            out.push_str(&format!(
                "- **{}** {}: {}\n  {}\n",
                markdown_text(&f.severity),
                markdown_text(&f.id),
                markdown_text(&f.title),
                markdown_text(&f.summary)
            ));
        }
    }
    out.push_str("\n## Top processes\n\n");
    if r.top_processes.is_empty() {
        out.push_str("_none_\n");
    } else {
        for p in &r.top_processes {
            out.push_str(&format!(
                "- {} pid={} cpu={:.1}% mem={} — {}\n",
                markdown_text(&p.name),
                p.pid,
                p.cpu,
                markdown_text(&p.mem),
                markdown_text(&p.cmd_redacted)
            ));
        }
    }
    out.push_str("\n## Failed services\n\n");
    if r.failed_services.is_empty() {
        out.push_str("_none_\n");
    } else {
        for u in &r.failed_services {
            out.push_str(&format!("- {}\n", markdown_text(u)));
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
        if let Some(app_dir) = dir.parent() {
            crate::fsutil::ensure_private_app_dir(app_dir)?;
        }
        save_report_path(&dir, format)?
    };
    crate::fsutil::write_private_atomic(&path, body.as_bytes())?;
    Ok(path)
}

fn save_report_path(dir: &Path, format: SupportFormat) -> Result<PathBuf, AppError> {
    crate::fsutil::ensure_private_app_dir(dir)?;
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let nonce = REPORT_NONCE.fetch_add(1, Ordering::Relaxed);
    Ok(dir.join(format!(
        "support-{}-{:09}-{nonce}.{}",
        now.as_secs(),
        now.subsec_nanos(),
        format.extension()
    )))
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
    fn shareable_report_omits_structured_identity_and_command_arguments() {
        let metrics = SystemMetrics {
            hostname: "prod-server-01".into(),
            ..SystemMetrics::default()
        };
        let process = ProcessInfo {
            pid: 7,
            user: "alice".into(),
            name: "foo".into(),
            cmd: "/usr/bin/foo --token=secret --bind=10.1.2.3".into(),
            cpu: 1.0,
            mem_pct: Some(2.0),
            mem_bytes: 1,
            state: "S".into(),
            run_time_secs: 1,
            start_time: 1,
        };
        let report = build_report(
            &metrics,
            &[],
            &[process],
            &[],
            HealthStatus::Ok,
            false,
            true,
            false,
        );
        let json = render(&report, SupportFormat::Json).expect("json");
        for secret in ["prod-server-01", "alice", "--token=secret", "10.1.2.3"] {
            assert!(!json.contains(secret), "leaked {secret}: {json}");
        }
        assert!(json.contains("/usr/bin/foo"));
        assert!(json.contains("\"pid\": 7"));
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
        assert!(md.contains("demo: true"));
        assert!(md.contains("read_only: true"));
        assert!(md.contains("_none_"));
        assert!(md.contains("## Failed services"));
    }

    #[test]
    fn consecutive_default_report_paths_are_unique() {
        let dir = tempfile::tempdir().expect("tempdir");
        let first = save_report_path(dir.path(), SupportFormat::Text).expect("first path");
        crate::fsutil::write_private_atomic(&first, b"first").expect("first write");
        let second = save_report_path(dir.path(), SupportFormat::Text).expect("second path");
        crate::fsutil::write_private_atomic(&second, b"second").expect("second write");

        assert_ne!(first, second);
        assert!(first.is_file() && second.is_file());
    }

    #[test]
    fn shareable_redactor_masks_known_identity_paths_and_ips() {
        let redactor = Redactor {
            policy: RedactionPolicy::shareable(),
            hostname: "prod-server-01".into(),
            users: vec!["alice".into()],
            home: Some("/home/alice".into()),
        };
        let input = "prod-server-01 alice /home/alice/file 10.1.2.3 2001:db8::1";
        let output = redactor.text(input);
        for secret in [
            "prod-server-01",
            "alice",
            "/home/alice",
            "10.1.2.3",
            "2001:db8::1",
        ] {
            assert!(!output.contains(secret), "leaked {secret}: {output}");
        }
        assert!(output.contains("<host>"));
        assert!(output.contains("$HOME"));
        assert!(output.matches("<ip>").count() >= 2);

        let full = Redactor {
            policy: RedactionPolicy::include_sensitive(),
            hostname: "prod-server-01".into(),
            users: vec!["alice".into()],
            home: Some("/home/alice".into()),
        };
        assert_eq!(full.text(input), input);
        assert_eq!(
            full.policy.redact_cmd("/usr/bin/foo --token=secret"),
            "/usr/bin/foo --token=secret"
        );
    }

    #[test]
    fn markdown_escapes_host_derived_structure_and_json_stays_valid() {
        let report = SupportReport {
            app: APP_NAME.into(),
            version: APP_VERSION.into(),
            generated_at_unix: 0,
            health: "ok".into(),
            hostname: "**prod**".into(),
            demo: false,
            read_only: true,
            findings: vec![SupportFinding {
                id: "finding".into(),
                severity: "warning".into(),
                title: "# injected heading".into(),
                summary: "<script>\n[next](https://example.invalid)".into(),
            }],
            top_processes: vec![SupportProcess {
                pid: 7,
                name: "`oops`".into(),
                user: "<user>".into(),
                cpu: 1.0,
                mem: "2.0".into(),
                cmd_redacted: "/usr/bin/foo".into(),
            }],
            failed_services: vec!["[click](https://example.invalid)".into()],
            notes: Vec::new(),
        };
        let markdown = render(&report, SupportFormat::Markdown).expect("markdown");
        assert!(!markdown
            .lines()
            .any(|line| line.starts_with("# injected heading")));
        assert!(!markdown.contains("<script>"));
        assert!(!markdown.contains("[click](https://example.invalid)"));
        assert!(markdown.contains("\\# injected heading"));
        let json = render(&report, SupportFormat::Json).expect("json");
        let _: serde_json::Value = serde_json::from_str(&json).expect("valid json");
        assert!(json.contains("# injected heading"));
    }
}
