//! SMART / disk health rules (pure). Never requests self-tests.

use crate::diagnostics::rules::common::sanitize_id;
use crate::model::{
    Category, Confidence, DiagnosticSnapshot, DiagnosticTarget, Evidence, Finding, ScreenTarget,
    Severity, SuggestedCheck,
};

pub fn rule_smart(snap: &DiagnosticSnapshot) -> Vec<Finding> {
    let mut out = Vec::new();
    for disk in &snap.smart_disks {
        if !disk.available {
            continue;
        }
        if disk.passed == Some(false) {
            out.push(Finding {
                id: format!("smart.health.{}", sanitize_id(&disk.device)),
                title: format!("SMART health failed: {}", disk.device),
                summary: "smartctl -H reported FAILED (read-only check)".into(),
                severity: Severity::Critical,
                confidence: Confidence::High,
                category: Category::Hardware,
                evidence: Evidence {
                    summary: disk.summary.clone(),
                    details: disk.details.clone(),
                },
                targets: vec![DiagnosticTarget {
                    screen: ScreenTarget::Diagnostics,
                    search: Some(disk.device.clone()),
                }],
                suggested_check: Some(SuggestedCheck {
                    description: "Inspect SMART attributes (read-only)".into(),
                    command_hint: Some(format!("smartctl -H -A -l error {}", disk.device)),
                }),
                degradable: true,
            });
        }
        if let Some(crc) = disk.uda_crc_error_count {
            if crc > 0 {
                let increased = disk
                    .prev_uda_crc_error_count
                    .map(|prev| crc > prev)
                    .unwrap_or(false);
                out.push(Finding {
                    id: format!("smart.crc.{}", sanitize_id(&disk.device)),
                    title: format!("SATA CRC errors on {}", disk.device),
                    summary: if increased {
                        format!("CRC error count increased to {crc}")
                    } else {
                        format!("Historic CRC error count is {crc} (non-zero)")
                    },
                    severity: if increased {
                        Severity::Warning
                    } else {
                        Severity::Info
                    },
                    confidence: Confidence::Medium,
                    category: Category::Hardware,
                    evidence: Evidence {
                        summary: format!("UDMA_CRC_Error_Count={crc}"),
                        details: disk.details.clone(),
                    },
                    targets: vec![DiagnosticTarget {
                        screen: ScreenTarget::Storage,
                        search: None,
                    }],
                    suggested_check: Some(SuggestedCheck {
                        description: "Check cabling/link integrity; do not run SMART self-tests from this app".into(),
                        command_hint: Some(format!("smartctl -A {}", disk.device)),
                    }),
                    degradable: true,
                });
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::SmartDiskSnapshot;

    #[test]
    fn crc_historic_is_info() {
        let snap = DiagnosticSnapshot {
            smart_disks: vec![SmartDiskSnapshot {
                device: "/dev/sda".into(),
                available: true,
                passed: Some(true),
                uda_crc_error_count: Some(3),
                prev_uda_crc_error_count: Some(3),
                summary: "ok".into(),
                details: vec![],
            }],
            ..DiagnosticSnapshot::default()
        };
        let f = rule_smart(&snap);
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].severity, Severity::Info);
    }

    #[test]
    fn crc_increase_is_warning() {
        let snap = DiagnosticSnapshot {
            smart_disks: vec![SmartDiskSnapshot {
                device: "/dev/sda".into(),
                available: true,
                passed: Some(true),
                uda_crc_error_count: Some(5),
                prev_uda_crc_error_count: Some(3),
                summary: "ok".into(),
                details: vec![],
            }],
            ..DiagnosticSnapshot::default()
        };
        let f = rule_smart(&snap);
        assert!(f.iter().any(|x| x.severity == Severity::Warning));
    }
}
