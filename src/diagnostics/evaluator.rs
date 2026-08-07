//! Pure diagnostic evaluation: snapshot → findings. No FS / D-Bus / Command / Tokio.

use crate::model::diagnostics::thresholds;
use crate::model::{
    Category, Confidence, DiagnosticSnapshot, DiagnosticTarget, Evidence, Finding, HealthStatus,
    ScreenTarget, Severity, SuggestedCheck,
};

/// Evaluate findings from an already-collected snapshot. Pure function.
pub fn evaluate(snapshot: &DiagnosticSnapshot) -> Vec<Finding> {
    let mut findings = Vec::new();
    findings.extend(rule_failed_services(snapshot));
    findings.extend(rule_journal_critical(snapshot));
    findings.extend(rule_oom(snapshot));
    findings.extend(rule_previous_boot(snapshot));
    findings.extend(rule_pstore(snapshot));
    findings.extend(rule_coredumps(snapshot));
    findings.extend(rule_psi(snapshot));
    findings.extend(rule_resource_pressure(snapshot));
    findings.extend(rule_temperature(snapshot));
    findings.extend(rule_clock_sync(snapshot));
    findings.extend(rule_etc_meta(snapshot));
    findings.extend(crate::diagnostics::rules::rule_smart(snapshot));
    findings
}

/// Aggregate health: never Ok if required subsystems cannot be observed.
pub fn compute_health_status(
    findings: &[Finding],
    systemd_observable: bool,
    journal_observable: bool,
) -> HealthStatus {
    if !systemd_observable || !journal_observable {
        // Required observation missing → at best Unknown, escalate with findings.
        let mut status = HealthStatus::Unknown;
        for f in findings {
            status = status.worse(f.severity.to_health());
        }
        // Unknown if we literally cannot see required subsystems, even with no findings.
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

fn rule_failed_services(snap: &DiagnosticSnapshot) -> Vec<Finding> {
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
                screen: ScreenTarget::Services,
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

fn rule_journal_critical(snap: &DiagnosticSnapshot) -> Vec<Finding> {
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
                screen: ScreenTarget::Logs,
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

fn rule_oom(snap: &DiagnosticSnapshot) -> Vec<Finding> {
    if snap.oom_events.is_empty() {
        return Vec::new();
    }
    let samples: Vec<String> = snap
        .oom_events
        .iter()
        .take(5)
        .map(|e| format!("{}: {}", e.process, e.message))
        .collect();
    vec![Finding {
        id: "mem.oom.recent".into(),
        title: "Out-of-memory killer activity".into(),
        summary: format!(
            "{} OOM-related event(s) observed in journal",
            snap.oom_events.len()
        ),
        severity: Severity::Critical,
        confidence: Confidence::High,
        category: Category::Memory,
        evidence: Evidence {
            summary: samples.first().cloned().unwrap_or_default(),
            details: samples,
        },
        targets: vec![
            DiagnosticTarget {
                screen: ScreenTarget::Logs,
                search: Some("oom".into()),
            },
            DiagnosticTarget {
                screen: ScreenTarget::Processes,
                search: None,
            },
        ],
        suggested_check: Some(SuggestedCheck {
            description: "Inspect memory pressure and recent OOM journal lines".into(),
            command_hint: Some("journalctl -k -g oom -n 30".into()),
        }),
        degradable: true,
    }]
}

fn rule_previous_boot(snap: &DiagnosticSnapshot) -> Vec<Finding> {
    let Some(prev) = &snap.previous_boot_summary else {
        return Vec::new();
    };
    // Prefer "I don't know" — only emit Info/Warning on unclean_hint, never claim panic.
    if !prev.unclean_hint {
        return Vec::new();
    }
    vec![Finding {
        id: format!("boot.previous.{}", sanitize_id(&prev.boot_id)),
        title: "Previous boot may have ended uncleanly".into(),
        summary: prev.note.clone(),
        severity: Severity::Warning,
        confidence: Confidence::Low,
        category: Category::Boot,
        evidence: Evidence {
            summary: prev.note.clone(),
            details: vec![format!("previous_boot_id={}", prev.boot_id)],
        },
        targets: vec![DiagnosticTarget {
            screen: ScreenTarget::Logs,
            search: None,
        }],
        suggested_check: Some(SuggestedCheck {
            description: "Compare previous-boot journal for shutdown vs abrupt end (do not assume kernel panic)".into(),
            command_hint: Some("journalctl -b -1 -n 80".into()),
        }),
        degradable: true,
    }]
}

fn rule_pstore(snap: &DiagnosticSnapshot) -> Vec<Finding> {
    if snap.pstore_entries.is_empty() {
        return Vec::new();
    }
    let details: Vec<String> = snap
        .pstore_entries
        .iter()
        .take(10)
        .map(|e| format!("{} ({} bytes)", e.name, e.bytes))
        .collect();
    vec![Finding {
        id: "boot.pstore.present".into(),
        title: "Persistent store (pstore) entries present".into(),
        summary: format!(
            "{} pstore entry/entries observed (read-only listing)",
            snap.pstore_entries.len()
        ),
        severity: Severity::Warning,
        confidence: Confidence::Medium,
        category: Category::Boot,
        evidence: Evidence {
            summary: details.first().cloned().unwrap_or_default(),
            details,
        },
        targets: vec![DiagnosticTarget {
            screen: ScreenTarget::Diagnostics,
            search: Some("pstore".into()),
        }],
        suggested_check: Some(SuggestedCheck {
            description: "Inspect /sys/fs/pstore contents as root if further triage is needed"
                .into(),
            command_hint: Some("ls -la /sys/fs/pstore".into()),
        }),
        degradable: true,
    }]
}

fn rule_coredumps(snap: &DiagnosticSnapshot) -> Vec<Finding> {
    if snap.coredumps.is_empty() {
        return Vec::new();
    }
    let details: Vec<String> = snap
        .coredumps
        .iter()
        .take(8)
        .map(|c| {
            format!(
                "{} signal={} @ {}",
                c.exe,
                c.signal.as_deref().unwrap_or("?"),
                c.timestamp
            )
        })
        .collect();
    vec![Finding {
        id: "crash.coredump.recent".into(),
        title: "Recent coredumps recorded".into(),
        summary: format!("{} coredump(s) listed by coredumpctl", snap.coredumps.len()),
        severity: Severity::Warning,
        confidence: Confidence::High,
        category: Category::CoreDump,
        evidence: Evidence {
            summary: details.first().cloned().unwrap_or_default(),
            details,
        },
        targets: vec![DiagnosticTarget {
            screen: ScreenTarget::Diagnostics,
            search: Some("coredump".into()),
        }],
        suggested_check: Some(SuggestedCheck {
            description: "List recent coredumps".into(),
            command_hint: Some("coredumpctl list".into()),
        }),
        degradable: true,
    }]
}

fn rule_psi(snap: &DiagnosticSnapshot) -> Vec<Finding> {
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

fn rule_resource_pressure(snap: &DiagnosticSnapshot) -> Vec<Finding> {
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
            screen: ScreenTarget::Dashboard,
            search: None,
        }],
        suggested_check: None,
        degradable: true,
    }
}

fn rule_temperature(snap: &DiagnosticSnapshot) -> Vec<Finding> {
    snap.temperatures
        .iter()
        .filter(|t| t.celsius >= thresholds::TEMP_CELSIUS_HIGH)
        .map(|t| Finding {
            id: format!("temp.high.{}", sanitize_id(&t.label)),
            title: "High temperature observed".into(),
            summary: format!("{} at {:.0}°C", t.label, t.celsius),
            severity: Severity::Warning,
            confidence: Confidence::Medium,
            category: Category::Temperature,
            evidence: Evidence {
                summary: format!(
                    "{:.1}°C (threshold {}°C)",
                    t.celsius,
                    thresholds::TEMP_CELSIUS_HIGH
                ),
                details: vec!["Wording intentionally avoids claiming thermal throttling.".into()],
            },
            targets: vec![DiagnosticTarget {
                screen: ScreenTarget::Dashboard,
                search: None,
            }],
            suggested_check: None,
            degradable: true,
        })
        .collect()
}

fn rule_clock_sync(snap: &DiagnosticSnapshot) -> Vec<Finding> {
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

fn rule_etc_meta(snap: &DiagnosticSnapshot) -> Vec<Finding> {
    if snap.etc_mtime_notable.is_empty() {
        return Vec::new();
    }
    let details: Vec<String> = snap
        .etc_mtime_notable
        .iter()
        .take(12)
        .map(|e| format!("{} ({})", e.path, e.kind))
        .collect();
    vec![Finding {
        id: "config.etc.recent_meta".into(),
        title: "Notable /etc metadata changes".into(),
        summary: format!(
            "{} path(s) under /etc with recent metadata changes (info only)",
            snap.etc_mtime_notable.len()
        ),
        severity: Severity::Info,
        confidence: Confidence::Low,
        category: Category::Config,
        evidence: Evidence {
            summary: details.first().cloned().unwrap_or_default(),
            details,
        },
        targets: vec![DiagnosticTarget {
            screen: ScreenTarget::Diagnostics,
            search: Some("etc".into()),
        }],
        suggested_check: None,
        degradable: true,
    }]
}

fn sanitize_id(raw: &str) -> String {
    raw.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' {
                c
            } else {
                '_'
            }
        })
        .collect()
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
}
