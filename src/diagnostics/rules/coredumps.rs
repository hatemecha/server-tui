//! Pure diagnostic rule module.

use crate::model::{
    Category, Confidence, DiagnosticSnapshot, DiagnosticTarget, Evidence, Finding, Screen,
    Severity, SuggestedCheck,
};

pub fn rule_coredumps(snap: &DiagnosticSnapshot) -> Vec<Finding> {
    if snap.coredumps.is_empty() {
        return Vec::new();
    }
    let details: Vec<String> = snap
        .coredumps
        .iter()
        .take(8)
        .map(|c| {
            format!(
                "{} signal={} @ {}",
                c.exe,
                c.signal.as_deref().unwrap_or("?"),
                c.timestamp
            )
        })
        .collect();
    vec![Finding {
        id: "crash.coredump.recent".into(),
        title: "Recent coredumps recorded".into(),
        summary: format!("{} coredump(s) listed by coredumpctl", snap.coredumps.len()),
        severity: Severity::Warning,
        confidence: Confidence::High,
        category: Category::CoreDump,
        evidence: Evidence {
            summary: details.first().cloned().unwrap_or_default(),
            details,
        },
        targets: vec![DiagnosticTarget {
            screen: Screen::Diagnostics,
            search: Some("coredump".into()),
        }],
        suggested_check: Some(SuggestedCheck {
            description: "List recent coredumps".into(),
            command_hint: Some("coredumpctl list".into()),
        }),
        degradable: true,
    }]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::CoredumpEntry;

    #[test]
    fn coredumps_present_emits_warning() {
        let snap = DiagnosticSnapshot {
            coredumps: vec![CoredumpEntry {
                exe: "/usr/bin/foo".into(),
                signal: Some("11".into()),
                timestamp: "t".into(),
            }],
            ..DiagnosticSnapshot::default()
        };
        let f = rule_coredumps(&snap);
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].id, "crash.coredump.recent");
        assert_eq!(f[0].severity, Severity::Warning);
    }

    #[test]
    fn coredumps_empty_is_silent() {
        assert!(rule_coredumps(&DiagnosticSnapshot::default()).is_empty());
    }
}
