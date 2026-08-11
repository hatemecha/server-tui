//! Pure diagnostic evaluation: snapshot → findings. No FS / D-Bus / Command / Tokio.

use crate::diagnostics::rules;
use crate::model::{DiagnosticSnapshot, Finding, HealthStatus};

/// Evaluate findings from an already-collected snapshot. Pure function.
pub fn evaluate(snapshot: &DiagnosticSnapshot) -> Vec<Finding> {
    let mut findings = Vec::new();
    findings.extend(rules::rule_failed_services(snapshot));
    findings.extend(rules::rule_journal_critical(snapshot));
    findings.extend(rules::rule_oom(snapshot));
    findings.extend(rules::rule_previous_boot(snapshot));
    findings.extend(rules::rule_pstore(snapshot));
    findings.extend(rules::rule_coredumps(snapshot));
    findings.extend(rules::rule_psi(snapshot));
    findings.extend(rules::rule_resource_pressure(snapshot));
    findings.extend(rules::rule_temperature(snapshot));
    findings.extend(rules::rule_clock_sync(snapshot));
    findings.extend(rules::rule_etc_meta(snapshot));
    findings.extend(rules::rule_smart(snapshot));
    findings
}

/// Aggregate health: never Ok if required subsystems cannot be observed.
pub fn compute_health_status(
    findings: &[Finding],
    systemd_observable: bool,
    journal_observable: bool,
) -> HealthStatus {
    if !systemd_observable || !journal_observable {
        let mut status = HealthStatus::Unknown;
        for f in findings {
            status = status.worse(f.severity.to_health());
        }
        if status == HealthStatus::Ok {
            return HealthStatus::Unknown;
        }
        return status;
    }

    let mut status = HealthStatus::Ok;
    for f in findings {
        status = status.worse(f.severity.to_health());
    }
    status
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{FailedUnitSnapshot, OomEvent, ResourcePressureSnapshot, TempSnapshot};

    fn base() -> DiagnosticSnapshot {
        DiagnosticSnapshot {
            systemd_observable: true,
            journal_observable: true,
            ..DiagnosticSnapshot::default()
        }
    }

    #[test]
    fn failed_service_finding() {
        let mut snap = base();
        snap.failed_units.push(FailedUnitSnapshot {
            unit: "broken.service".into(),
            active_state: "failed".into(),
            sub_state: "failed".into(),
            description: "x".into(),
        });
        let findings = evaluate(&snap);
        assert!(findings.iter().any(|f| f.id.contains("svc.failed")));
    }

    #[test]
    fn temp_does_not_claim_throttling() {
        let mut snap = base();
        snap.temperatures.push(TempSnapshot {
            label: "CPU".into(),
            celsius: 95.0,
        });
        let findings = evaluate(&snap);
        let f = findings
            .iter()
            .find(|f| f.id.starts_with("temp.high"))
            .unwrap();
        assert_eq!(f.title, "High temperature observed");
        assert!(!f.summary.to_lowercase().contains("throttl"));
    }

    #[test]
    fn resource_pressure_conservative() {
        let mut snap = base();
        snap.resource_pressure = Some(ResourcePressureSnapshot {
            mem_used_pct: 96.0,
            swap_used_pct: 10.0,
            load1: 1.0,
            n_cpus: 4,
            disk_max_used_pct: 50.0,
        });
        let findings = evaluate(&snap);
        assert!(findings.iter().any(|f| f.id == "pressure.mem.high"));
    }

    #[test]
    fn health_unknown_without_required() {
        let status = compute_health_status(&[], false, true);
        assert_eq!(status, HealthStatus::Unknown);
    }

    #[test]
    fn oom_finding() {
        let mut snap = base();
        snap.oom_events.push(OomEvent {
            process: "foo".into(),
            message: "Killed process".into(),
        });
        let findings = evaluate(&snap);
        assert!(findings.iter().any(|f| f.id == "mem.oom.recent"));
    }

    #[test]
    fn no_false_kernel_panic_on_previous_boot() {
        let mut snap = base();
        snap.previous_boot_summary = Some(crate::model::PreviousBootSummary {
            boot_id: "abc".into(),
            unclean_hint: true,
            note: "journal ended without clean shutdown marker".into(),
        });
        let findings = evaluate(&snap);
        let f = findings
            .iter()
            .find(|f| f.id.starts_with("boot.previous"))
            .unwrap();
        assert!(!f.title.to_lowercase().contains("panic"));
        assert!(!f.summary.to_lowercase().contains("kernel panic"));
    }

    #[test]
    fn health_surfaces_critical_when_subsystems_unobservable() {
        use crate::model::{Category, Confidence, Evidence, Finding, Severity};
        let findings = vec![Finding {
            id: "x".into(),
            title: "t".into(),
            summary: "s".into(),
            severity: Severity::Critical,
            confidence: Confidence::High,
            category: Category::Services,
            evidence: Evidence {
                summary: "e".into(),
                details: vec![],
            },
            targets: vec![],
            suggested_check: None,
            degradable: true,
        }];
        let status = compute_health_status(&findings, false, false);
        assert_eq!(status, HealthStatus::Critical);
    }
}
