//! Pure diagnostic rule module.

use crate::model::{
    Category, Confidence, DiagnosticSnapshot, DiagnosticTarget, Evidence, Finding, ScreenTarget,
    Severity,
};

pub fn rule_etc_meta(snap: &DiagnosticSnapshot) -> Vec<Finding> {
    if snap.etc_mtime_notable.is_empty() {
        return Vec::new();
    }
    let details: Vec<String> = snap
        .etc_mtime_notable
        .iter()
        .take(12)
        .map(|e| format!("{} ({})", e.path, e.kind))
        .collect();
    vec![Finding {
        id: "config.etc.recent_meta".into(),
        title: "Notable /etc metadata changes".into(),
        summary: format!(
            "{} path(s) under /etc with recent metadata changes (info only)",
            snap.etc_mtime_notable.len()
        ),
        severity: Severity::Info,
        confidence: Confidence::Low,
        category: Category::Config,
        evidence: Evidence {
            summary: details.first().cloned().unwrap_or_default(),
            details,
        },
        targets: vec![DiagnosticTarget {
            screen: ScreenTarget::Diagnostics,
            search: Some("etc".into()),
        }],
        suggested_check: None,
        degradable: true,
    }]
}
