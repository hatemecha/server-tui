//! Pure diagnostic rule module.

use crate::diagnostics::rules::common::sanitize_id;
use crate::model::{
    Category, Confidence, DiagnosticSnapshot, DiagnosticTarget, Evidence, Finding, Screen,
    Severity, SuggestedCheck,
};

pub fn rule_journal_critical(snap: &DiagnosticSnapshot) -> Vec<Finding> {
    if !snap.journal_observable {
        return Vec::new();
    }
    snap.journal_critical
        .iter()
        .filter(|g| g.count > 0)
        .map(|g| Finding {
            id: format!("journal.critical.{}", sanitize_id(&g.unit)),
            title: format!("Critical journal messages: {}", g.unit),
            summary: format!("{} critical/alert messages for {}", g.count, g.unit),
            severity: if g.count >= 5 {
                Severity::Critical
            } else {
                Severity::Warning
            },
            confidence: Confidence::Medium,
            category: Category::Journal,
            evidence: Evidence {
                summary: g.sample_message.clone(),
                details: vec![format!("count={} sample={}", g.count, g.sample_message)],
            },
            targets: vec![DiagnosticTarget {
                screen: Screen::Logs,
                search: Some(g.unit.clone()),
            }],
            suggested_check: Some(SuggestedCheck {
                description: "Review recent critical journal entries for this unit".into(),
                command_hint: Some(format!("journalctl -p 0..2 -u {} -n 50", g.unit)),
            }),
            degradable: true,
        })
        .collect()
}
