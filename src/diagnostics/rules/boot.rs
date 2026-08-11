//! Pure diagnostic rule module.

use crate::diagnostics::rules::common::sanitize_id;
use crate::model::{
    Category, Confidence, DiagnosticSnapshot, DiagnosticTarget, Evidence, Finding, Screen,
    Severity, SuggestedCheck,
};

pub fn rule_previous_boot(snap: &DiagnosticSnapshot) -> Vec<Finding> {
    let Some(prev) = &snap.previous_boot_summary else {
        return Vec::new();
    };
    // Prefer "I don't know" — only emit Info/Warning on unclean_hint, never claim panic.
    if !prev.unclean_hint {
        return Vec::new();
    }
    vec![Finding {
        id: format!("boot.previous.{}", sanitize_id(&prev.boot_id)),
        title: "Previous boot may have ended uncleanly".into(),
        summary: prev.note.clone(),
        severity: Severity::Warning,
        confidence: Confidence::Low,
        category: Category::Boot,
        evidence: Evidence {
            summary: prev.note.clone(),
            details: vec![format!("previous_boot_id={}", prev.boot_id)],
        },
        targets: vec![DiagnosticTarget {
            screen: Screen::Logs,
            search: None,
        }],
        suggested_check: Some(SuggestedCheck {
            description: "Compare previous-boot journal for shutdown vs abrupt end (do not assume kernel panic)".into(),
            command_hint: Some("journalctl -b -1 -n 80".into()),
        }),
        degradable: true,
    }]
}
