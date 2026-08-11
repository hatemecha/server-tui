//! Pure diagnostic rule module.

use crate::model::{
    Category, Confidence, DiagnosticSnapshot, DiagnosticTarget, Evidence, Finding, ScreenTarget,
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
            screen: ScreenTarget::Diagnostics,
            search: Some("coredump".into()),
        }],
        suggested_check: Some(SuggestedCheck {
            description: "List recent coredumps".into(),
            command_hint: Some("coredumpctl list".into()),
        }),
        degradable: true,
    }]
}
