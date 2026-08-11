//! Pure diagnostic rule module.

use crate::model::diagnostics::thresholds;
use crate::model::{
    Category, Confidence, DiagnosticSnapshot, DiagnosticTarget, Evidence, Finding, Screen, Severity,
};

pub fn rule_psi(snap: &DiagnosticSnapshot) -> Vec<Finding> {
    let Some(psi) = &snap.psi else {
        return Vec::new();
    };
    let mut findings = Vec::new();
    if psi.memory_full_avg10 >= thresholds::PSI_MEMORY_FULL_WARN {
        findings.push(Finding {
            id: "pressure.psi.memory_full".into(),
            title: "Memory PSI full pressure elevated".into(),
            summary: format!("memory full avg10={:.1}%", psi.memory_full_avg10),
            severity: Severity::Warning,
            confidence: Confidence::Medium,
            category: Category::Pressure,
            evidence: Evidence {
                summary: format!(
                    "mem_some={:.1} mem_full={:.1} cpu_some={:.1} io_some={:.1}",
                    psi.memory_some_avg10,
                    psi.memory_full_avg10,
                    psi.cpu_some_avg10,
                    psi.io_some_avg10
                ),
                details: vec![],
            },
            targets: vec![DiagnosticTarget {
                screen: Screen::Dashboard,
                search: None,
            }],
            suggested_check: None,
            degradable: true,
        });
    } else if psi.memory_some_avg10 >= thresholds::PSI_MEMORY_SOME_WARN {
        findings.push(Finding {
            id: "pressure.psi.memory_some".into(),
            title: "Memory PSI some pressure elevated".into(),
            summary: format!("memory some avg10={:.1}%", psi.memory_some_avg10),
            severity: Severity::Info,
            confidence: Confidence::Medium,
            category: Category::Pressure,
            evidence: Evidence {
                summary: format!("mem_some={:.1}", psi.memory_some_avg10),
                details: vec![],
            },
            targets: vec![DiagnosticTarget {
                screen: Screen::Dashboard,
                search: None,
            }],
            suggested_check: None,
            degradable: true,
        });
    }
    findings
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::PsiSnapshot;

    #[test]
    fn psi_memory_full_emits_warning() {
        let snap = DiagnosticSnapshot {
            psi: Some(PsiSnapshot {
                memory_some_avg10: 50.0,
                memory_full_avg10: 10.0,
                cpu_some_avg10: 0.0,
                io_some_avg10: 0.0,
            }),
            ..DiagnosticSnapshot::default()
        };
        let f = rule_psi(&snap);
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].id, "pressure.psi.memory_full");
        assert_eq!(f[0].severity, Severity::Warning);
    }

    #[test]
    fn psi_memory_some_is_info_not_full() {
        let snap = DiagnosticSnapshot {
            psi: Some(PsiSnapshot {
                memory_some_avg10: 20.0,
                memory_full_avg10: 0.0,
                cpu_some_avg10: 0.0,
                io_some_avg10: 0.0,
            }),
            ..DiagnosticSnapshot::default()
        };
        let f = rule_psi(&snap);
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].id, "pressure.psi.memory_some");
        assert_eq!(f[0].severity, Severity::Info);
    }

    #[test]
    fn psi_absent_is_silent() {
        let snap = DiagnosticSnapshot::default();
        assert!(rule_psi(&snap).is_empty());
    }
}
