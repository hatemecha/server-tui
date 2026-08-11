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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::JournalCriticalGroup;

    #[test]
    fn journal_critical_severity_by_count() {
        let mut snap = DiagnosticSnapshot {
            journal_observable: true,
            ..DiagnosticSnapshot::default()
        };
        snap.journal_critical.push(JournalCriticalGroup {
            unit: "ssh.service".into(),
            count: 5,
            sample_message: "boom".into(),
        });
        snap.journal_critical.push(JournalCriticalGroup {
            unit: "cron.service".into(),
            count: 1,
            sample_message: "minor".into(),
        });
        let f = rule_journal_critical(&snap);
        assert_eq!(f.len(), 2);
        let ssh = f
            .iter()
            .find(|x| x.id == "journal.critical.ssh.service")
            .unwrap();
        let cron = f
            .iter()
            .find(|x| x.id == "journal.critical.cron.service")
            .unwrap();
        assert_eq!(ssh.severity, Severity::Critical);
        assert_eq!(cron.severity, Severity::Warning);
    }

    #[test]
    fn journal_unobservable_is_silent() {
        let snap = DiagnosticSnapshot {
            journal_observable: false,
            journal_critical: vec![JournalCriticalGroup {
                unit: "x.service".into(),
                count: 9,
                sample_message: "x".into(),
            }],
            ..DiagnosticSnapshot::default()
        };
        assert!(rule_journal_critical(&snap).is_empty());
    }
}
