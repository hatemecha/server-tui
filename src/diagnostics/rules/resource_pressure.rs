//! Pure diagnostic rule module.

use crate::model::diagnostics::thresholds;
use crate::model::{
    Category, Confidence, DiagnosticSnapshot, DiagnosticTarget, Evidence, Finding, Screen, Severity,
};

pub fn rule_resource_pressure(snap: &DiagnosticSnapshot) -> Vec<Finding> {
    let Some(r) = &snap.resource_pressure else {
        return Vec::new();
    };
    let mut findings = Vec::new();
    if r.mem_used_pct >= thresholds::MEM_USED_PCT_WARN {
        findings.push(simple_pressure(
            "pressure.mem.high",
            "High memory utilization",
            format!("memory used {:.0}%", r.mem_used_pct),
            Severity::Warning,
        ));
    }
    if r.swap_used_pct >= thresholds::SWAP_USED_PCT_WARN {
        findings.push(simple_pressure(
            "pressure.swap.high",
            "High swap utilization",
            format!("swap used {:.0}%", r.swap_used_pct),
            Severity::Warning,
        ));
    }
    let n = r.n_cpus.max(1) as f64;
    if r.load1 / n >= thresholds::LOAD_PER_CPU_WARN {
        findings.push(simple_pressure(
            "pressure.load.high",
            "High load average relative to CPUs",
            format!("load1={:.2} cpus={}", r.load1, r.n_cpus),
            Severity::Warning,
        ));
    }
    if r.disk_max_used_pct >= thresholds::DISK_USED_PCT_WARN {
        findings.push(simple_pressure(
            "pressure.disk.high",
            "Filesystem nearly full",
            format!("max disk used {:.0}%", r.disk_max_used_pct),
            Severity::Warning,
        ));
    }
    findings
}

fn simple_pressure(id: &str, title: &str, summary: String, severity: Severity) -> Finding {
    Finding {
        id: id.into(),
        title: title.into(),
        summary: summary.clone(),
        severity,
        confidence: Confidence::Medium,
        category: Category::Pressure,
        evidence: Evidence {
            summary,
            details: vec![],
        },
        targets: vec![DiagnosticTarget {
            screen: Screen::Dashboard,
            search: None,
        }],
        suggested_check: None,
        degradable: true,
    }
}
