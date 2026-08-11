//! Pure diagnostic rule module.

use crate::model::{
    Category, Confidence, DiagnosticSnapshot, DiagnosticTarget, Evidence, Finding, Screen,
    Severity, SuggestedCheck,
};

pub fn rule_clock_sync(snap: &DiagnosticSnapshot) -> Vec<Finding> {
    let Some(clock) = &snap.clock_sync else {
        return Vec::new();
    };
    match clock.ntp_synchronized {
        Some(false) => vec![Finding {
            id: "clock.ntp.unsynced".into(),
            title: "System clock not reported as NTP-synchronized".into(),
            summary: clock.system_time_status.clone(),
            severity: Severity::Warning,
            confidence: Confidence::Medium,
            category: Category::Clock,
            evidence: Evidence {
                summary: clock.system_time_status.clone(),
                details: vec![],
            },
            targets: vec![DiagnosticTarget {
                screen: Screen::Diagnostics,
                search: Some("clock".into()),
            }],
            suggested_check: Some(SuggestedCheck {
                description: "Check timedatectl synchronization status".into(),
                command_hint: Some("timedatectl status".into()),
            }),
            degradable: true,
        }],
        Some(true) | None => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::ClockSyncSnapshot;

    #[test]
    fn clock_unsynced_emits_warning() {
        let snap = DiagnosticSnapshot {
            clock_sync: Some(ClockSyncSnapshot {
                ntp_synchronized: Some(false),
                system_time_status: "NTP not synchronized".into(),
            }),
            ..DiagnosticSnapshot::default()
        };
        let f = rule_clock_sync(&snap);
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].id, "clock.ntp.unsynced");
        assert_eq!(f[0].severity, Severity::Warning);
    }

    #[test]
    fn clock_synced_or_unknown_is_silent() {
        for ntp in [Some(true), None] {
            let snap = DiagnosticSnapshot {
                clock_sync: Some(ClockSyncSnapshot {
                    ntp_synchronized: ntp,
                    system_time_status: "ok".into(),
                }),
                ..DiagnosticSnapshot::default()
            };
            assert!(rule_clock_sync(&snap).is_empty());
        }
        assert!(rule_clock_sync(&DiagnosticSnapshot::default()).is_empty());
    }
}
