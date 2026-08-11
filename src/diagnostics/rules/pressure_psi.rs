//! Pure diagnostic rule module.

use crate::model::diagnostics::thresholds;
use crate::model::{
    Category, Confidence, DiagnosticSnapshot, DiagnosticTarget, Evidence, Finding, ScreenTarget,
    Severity,
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
                screen: ScreenTarget::Dashboard,
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
                screen: ScreenTarget::Dashboard,
                search: None,
            }],
            suggested_check: None,
            degradable: true,
        });
    }
    findings
}
