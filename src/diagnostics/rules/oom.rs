//! Pure diagnostic rule module.

use crate::model::{
    Category, Confidence, DiagnosticSnapshot, DiagnosticTarget, Evidence, Finding, Screen,
    Severity, SuggestedCheck,
};

pub fn rule_oom(snap: &DiagnosticSnapshot) -> Vec<Finding> {
    if snap.oom_events.is_empty() {
        return Vec::new();
    }
    let samples: Vec<String> = snap
        .oom_events
        .iter()
        .take(5)
        .map(|e| format!("{}: {}", e.process, e.message))
        .collect();
    vec![Finding {
        id: "mem.oom.recent".into(),
        title: "Out-of-memory killer activity".into(),
        summary: format!(
            "{} OOM-related event(s) observed in journal",
            snap.oom_events.len()
        ),
        severity: Severity::Critical,
        confidence: Confidence::High,
        category: Category::Memory,
        evidence: Evidence {
            summary: samples.first().cloned().unwrap_or_default(),
            details: samples,
        },
        targets: vec![
            DiagnosticTarget {
                screen: Screen::Logs,
                search: Some("oom".into()),
            },
            DiagnosticTarget {
                screen: Screen::Processes,
                search: None,
            },
        ],
        suggested_check: Some(SuggestedCheck {
            description: "Inspect memory pressure and recent OOM journal lines".into(),
            command_hint: Some("journalctl -k -g oom -n 30".into()),
        }),
        degradable: true,
    }]
}
