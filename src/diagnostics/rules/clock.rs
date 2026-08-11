//! Pure diagnostic rule module.

use crate::model::{
    Category, Confidence, DiagnosticSnapshot, DiagnosticTarget, Evidence, Finding, ScreenTarget,
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
                screen: ScreenTarget::Diagnostics,
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
