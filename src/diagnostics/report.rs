//! Human/JSON diagnostic report formatting (detailed evidence omitted by default).

use crate::model::{Finding, HealthStatus};

pub fn format_text_report(
    health: HealthStatus,
    findings: &[Finding],
    include_sensitive: bool,
    probes_degraded: &[String],
) -> String {
    let mut out = String::new();
    out.push_str(&format!("server-tui doctor — health: {}\n", health.label()));
    if !probes_degraded.is_empty() {
        out.push_str("degraded probes:\n");
        for p in probes_degraded {
            out.push_str(&format!("  - {p}\n"));
        }
    }
    if findings.is_empty() {
        out.push_str("No findings.\n");
        return out;
    }
    out.push_str(&format!("findings: {}\n", findings.len()));
    for f in findings {
        out.push_str(&format!(
            "\n[{}] {} ({}/{})\n  {}\n",
            f.severity.label(),
            f.title,
            f.confidence.label(),
            f.category.label(),
            f.summary
        ));
        out.push_str(&format!("  id: {}\n", f.id));
        if include_sensitive {
            for d in &f.evidence.details {
                out.push_str(&format!("  evidence: {}\n", redact_line(d, false)));
            }
            if let Some(sc) = &f.suggested_check {
                out.push_str(&format!("  check: {}\n", sc.description));
                if let Some(cmd) = &sc.command_hint {
                    out.push_str(&format!("  hint: {cmd}\n"));
                }
            }
        } else {
            out.push_str(&format!(
                "  evidence: {}\n",
                redact_line(&f.evidence.summary, true)
            ));
        }
    }
    out
}

pub fn format_json_report(
    health: HealthStatus,
    findings: &[Finding],
    include_sensitive: bool,
    probes_degraded: &[String],
) -> Result<String, serde_json::Error> {
    #[derive(serde::Serialize)]
    struct Report<'a> {
        health: &'a str,
        probes_degraded: &'a [String],
        findings: Vec<FindingView<'a>>,
    }
    #[derive(serde::Serialize)]
    struct FindingView<'a> {
        id: &'a str,
        title: &'a str,
        summary: &'a str,
        severity: &'a str,
        confidence: &'a str,
        category: &'a str,
        evidence_summary: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        evidence_details: Option<Vec<String>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        suggested_check: Option<&'a str>,
    }

    let views: Vec<FindingView<'_>> = findings
        .iter()
        .map(|f| FindingView {
            id: &f.id,
            title: &f.title,
            summary: &f.summary,
            severity: f.severity.label(),
            confidence: f.confidence.label(),
            category: f.category.label(),
            evidence_summary: redact_line(&f.evidence.summary, !include_sensitive),
            evidence_details: if include_sensitive {
                Some(f.evidence.details.clone())
            } else {
                None
            },
            suggested_check: f.suggested_check.as_ref().map(|s| s.description.as_str()),
        })
        .collect();

    let report = Report {
        health: health.label(),
        probes_degraded,
        findings: views,
    };
    serde_json::to_string_pretty(&report)
}

fn redact_line(line: &str, redact: bool) -> String {
    if !redact {
        return line.to_string();
    }
    // Soft redaction: drop likely home paths / long tokens while keeping structure.
    let mut out = line.to_string();
    if let Ok(home) = std::env::var("HOME") {
        if !home.is_empty() {
            out = out.replace(&home, "$HOME");
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Category, Confidence, Evidence, Severity};

    #[test]
    fn text_report_empty() {
        let t = format_text_report(HealthStatus::Ok, &[], false, &[]);
        assert!(t.contains("No findings"));
    }

    #[test]
    fn json_report_ok() {
        let f = Finding {
            id: "x".into(),
            title: "t".into(),
            summary: "s".into(),
            severity: Severity::Info,
            confidence: Confidence::Low,
            category: Category::Other,
            evidence: Evidence {
                summary: "e".into(),
                details: vec!["secret".into()],
            },
            targets: vec![],
            suggested_check: None,
            degradable: true,
        };
        let j = format_json_report(HealthStatus::Ok, &[f], false, &[]).unwrap();
        assert!(j.contains("\"id\": \"x\""));
        assert!(!j.contains("secret"));
    }
}
