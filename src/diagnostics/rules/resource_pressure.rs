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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::ResourcePressureSnapshot;

    fn base_pressure() -> ResourcePressureSnapshot {
        ResourcePressureSnapshot {
            mem_used_pct: 10.0,
            swap_used_pct: 10.0,
            load1: 1.0,
            n_cpus: 4,
            disk_max_used_pct: 10.0,
        }
    }

    #[test]
    fn pressure_swap_load_disk_thresholds() {
        let mut swap = base_pressure();
        swap.swap_used_pct = 80.0;
        let ids: Vec<_> = rule_resource_pressure(&DiagnosticSnapshot {
            resource_pressure: Some(swap),
            ..DiagnosticSnapshot::default()
        })
        .into_iter()
        .map(|f| f.id)
        .collect();
        assert_eq!(ids, vec!["pressure.swap.high".to_string()]);

        let mut load = base_pressure();
        load.load1 = 16.0; // 16/4 == 4.0
        let ids: Vec<_> = rule_resource_pressure(&DiagnosticSnapshot {
            resource_pressure: Some(load),
            ..DiagnosticSnapshot::default()
        })
        .into_iter()
        .map(|f| f.id)
        .collect();
        assert_eq!(ids, vec!["pressure.load.high".to_string()]);

        let mut disk = base_pressure();
        disk.disk_max_used_pct = 95.0;
        let ids: Vec<_> = rule_resource_pressure(&DiagnosticSnapshot {
            resource_pressure: Some(disk),
            ..DiagnosticSnapshot::default()
        })
        .into_iter()
        .map(|f| f.id)
        .collect();
        assert_eq!(ids, vec!["pressure.disk.high".to_string()]);
    }

    #[test]
    fn pressure_just_below_thresholds_is_silent() {
        let r = ResourcePressureSnapshot {
            mem_used_pct: 94.9,
            swap_used_pct: 79.9,
            load1: 15.9,
            n_cpus: 4,
            disk_max_used_pct: 94.9,
        };
        assert!(rule_resource_pressure(&DiagnosticSnapshot {
            resource_pressure: Some(r),
            ..DiagnosticSnapshot::default()
        })
        .is_empty());
    }
}
