//! Pure diagnostic rule module.

use crate::diagnostics::rules::common::sanitize_id;
use crate::model::{
    Category, Confidence, DiagnosticSnapshot, DiagnosticTarget, Evidence, Finding, Screen,
    Severity, SuggestedCheck,
};

pub fn rule_failed_services(snap: &DiagnosticSnapshot) -> Vec<Finding> {
    if !snap.systemd_observable {
        return Vec::new();
    }
    snap.failed_units
        .iter()
        .map(|u| Finding {
            id: format!("svc.failed.{}", sanitize_id(&u.unit)),
            title: format!("Failed unit: {}", u.unit),
            summary: format!(
                "{} is in failed state ({}/{})",
                u.unit, u.active_state, u.sub_state
            ),
            severity: Severity::Critical,
            confidence: Confidence::High,
            category: Category::Services,
            evidence: Evidence {
                summary: u.description.clone(),
                details: vec![format!(
                    "active={} sub={} desc={}",
                    u.active_state, u.sub_state, u.description
                )],
            },
            targets: vec![DiagnosticTarget {
                screen: Screen::Services,
                search: Some(u.unit.clone()),
            }],
            suggested_check: Some(SuggestedCheck {
                description: "Inspect unit status and recent logs".into(),
                command_hint: Some(format!("systemctl status {}", u.unit)),
            }),
            degradable: true,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::FailedUnitSnapshot;

    #[test]
    fn failed_services_silent_when_systemd_unobservable() {
        let snap = DiagnosticSnapshot {
            systemd_observable: false,
            failed_units: vec![FailedUnitSnapshot {
                unit: "broken.service".into(),
                active_state: "failed".into(),
                sub_state: "failed".into(),
                description: "x".into(),
            }],
            ..DiagnosticSnapshot::default()
        };
        assert!(rule_failed_services(&snap).is_empty());
    }

    #[test]
    fn failed_service_emits_critical_with_sanitized_id() {
        let snap = DiagnosticSnapshot {
            systemd_observable: true,
            failed_units: vec![FailedUnitSnapshot {
                unit: "broken.service".into(),
                active_state: "failed".into(),
                sub_state: "failed".into(),
                description: "x".into(),
            }],
            ..DiagnosticSnapshot::default()
        };
        let f = rule_failed_services(&snap);
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].id, "svc.failed.broken.service");
        assert_eq!(f[0].severity, Severity::Critical);
    }
}
