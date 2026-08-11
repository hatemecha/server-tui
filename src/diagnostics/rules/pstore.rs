//! Pure diagnostic rule module.

use crate::model::{
    Category, Confidence, DiagnosticSnapshot, DiagnosticTarget, Evidence, Finding, Screen,
    Severity, SuggestedCheck,
};

pub fn rule_pstore(snap: &DiagnosticSnapshot) -> Vec<Finding> {
    if snap.pstore_entries.is_empty() {
        return Vec::new();
    }
    let details: Vec<String> = snap
        .pstore_entries
        .iter()
        .take(10)
        .map(|e| format!("{} ({} bytes)", e.name, e.bytes))
        .collect();
    vec![Finding {
        id: "boot.pstore.present".into(),
        title: "Persistent store (pstore) entries present".into(),
        summary: format!(
            "{} pstore entry/entries observed (read-only listing)",
            snap.pstore_entries.len()
        ),
        severity: Severity::Warning,
        confidence: Confidence::Medium,
        category: Category::Boot,
        evidence: Evidence {
            summary: details.first().cloned().unwrap_or_default(),
            details,
        },
        targets: vec![DiagnosticTarget {
            screen: Screen::Diagnostics,
            search: Some("pstore".into()),
        }],
        suggested_check: Some(SuggestedCheck {
            description: "Inspect /sys/fs/pstore contents as root if further triage is needed"
                .into(),
            command_hint: Some("ls -la /sys/fs/pstore".into()),
        }),
        degradable: true,
    }]
}
