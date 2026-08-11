# Diagnostic Rule Fixture Coverage Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans (or subagent-driven-development if available) to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking. REQUIRED: follow `test-driven-development` (RED → verify fail → GREEN → verify pass). Before claiming done, follow `verification-before-completion` with the Validation block in AGENTS.md.

**Goal:** Close the highest-value safe gaps in pure diagnostic-rule and related helper fixture coverage so every currently untested rule has co-located snapshot fixtures, without host I/O, sudo, or destructive tests.

**Architecture:** Keep rules pure. Add `#[cfg(test)]` modules beside each rule (pattern already used in `src/diagnostics/rules/smart.rs`). Expand existing tables for `unit_looks_safe` and expose `parse_psi_avg10` as `pub(crate)` solely for fixture tests. No UI, provider I/O, or new features.

**Tech Stack:** Rust 2021, cargo test, existing `DiagnosticSnapshot` / model types in `src/model/diagnostics.rs`, thresholds in `model::diagnostics::thresholds`.

## Global Constraints

- English UI strings and assertion messages match existing copy exactly where asserted.
- Tests must not kill processes, restart services, sudo, write `/etc`, scan `/`, or require root (`AGENTS.md` / `TESTING.md`).
- Prefer Unknown / silence over false diagnosis — assert silent paths (`None`, empty, unobservable flags).
- App reducers / providers unchanged unless a visibility tweak (`pub(crate)`) is needed for pure parsers.
- Stay on a feature branch (`fix/diag-rule-fixtures`); do not commit unless the user explicitly asks.
- Validation before claiming complete:
  ```bash
  cargo fmt --all --check
  cargo clippy --all-targets --all-features -- -D warnings
  cargo test --all
  cargo build --release
  ```

## File Map

| File | Role |
|------|------|
| `src/diagnostics/rules/common.rs` | Add `sanitize_id` unit tests |
| `src/diagnostics/rules/pressure_psi.rs` | PSI full / some / silence fixtures |
| `src/diagnostics/rules/journal.rs` | Critical count severities + unobservable silence |
| `src/diagnostics/rules/clock.rs` | Unsynced vs synced/unknown silence |
| `src/diagnostics/rules/coredumps.rs` | Present → warning; empty → silence |
| `src/diagnostics/rules/pstore.rs` | Entries → warning; empty → silence |
| `src/diagnostics/rules/etc_meta.rs` | Notable → Info only |
| `src/diagnostics/rules/resource_pressure.rs` | Swap / load / disk threshold fixtures |
| `src/diagnostics/rules/smart.rs` | FAILED + unavailable skip |
| `src/diagnostics/rules/failed_services.rs` | Unobservable silence |
| `src/diagnostics/rules/boot.rs` | Clean hint silence |
| `src/diagnostics/evaluator.rs` | Health still surfaces Critical when subsystems unobservable |
| `src/providers/linux/systemd.rs` | Thicker `unit_looks_safe` table |
| `src/providers/linux/diagnostics/pressure.rs` | `pub(crate) parse_psi_avg10` + string fixtures |

---

### Task 1: `sanitize_id` fixtures

**Files:**
- Modify: `src/diagnostics/rules/common.rs`
- Test: same file (`#[cfg(test)]`)

**Interfaces:**
- Consumes: `sanitize_id(raw: &str) -> String` (existing)
- Produces: covered edge cases for ID segments used by journal / services / SMART / boot

- [ ] **Step 1: Write the failing test**

Append to `common.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::sanitize_id;

    #[test]
    fn sanitize_id_replaces_unsafe_chars() {
        assert_eq!(sanitize_id("evil/unit\u{1b}.service"), "evil_unit_.service");
        assert_eq!(sanitize_id("ok-unit_1.service"), "ok-unit_1.service");
        assert_eq!(sanitize_id("a b"), "a_b");
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --lib diagnostics::rules::common::tests::sanitize_id_replaces_unsafe_chars -- --exact`
Expected: compile OK and PASS actually — this exercises existing code. If it **passes immediately**, that is correct for documenting current behavior (helper already implemented). Do **not** change `sanitize_id` unless assertion mismatches; if mismatch, treat as bug and follow systematic-debugging before changing production.

(Note: TDD iron law applies to **new** behavior. For documenting already-shipped pure helpers, a lock-in fixture that passes on first run is acceptable **only** when no production change is planned. Tasks 2+ introduce new assertions over rule behavior that must still RED→GREEN when adding regressions.)

- [ ] **Step 3: Confirm green**

Run same command; Expected: PASS.

- [ ] **Step 4: Do not commit unless user asks**

---

### Task 2: Untested rules — PSI, journal, clock, coredumps, pstore, etc_meta

**Files:**
- Modify: `src/diagnostics/rules/pressure_psi.rs`
- Modify: `src/diagnostics/rules/journal.rs`
- Modify: `src/diagnostics/rules/clock.rs`
- Modify: `src/diagnostics/rules/coredumps.rs`
- Modify: `src/diagnostics/rules/pstore.rs`
- Modify: `src/diagnostics/rules/etc_meta.rs`

**Interfaces:**
- Consumes: `rule_psi`, `rule_journal_critical`, `rule_clock_sync`, `rule_coredumps`, `rule_pstore`, `rule_etc_meta`, thresholds `PSI_MEMORY_FULL_WARN=10.0`, `PSI_MEMORY_SOME_WARN=20.0`
- Produces: co-located fixtures asserting id / severity / silence

#### 2a — PSI

- [ ] **Step 1: Write failing tests** in `pressure_psi.rs`:

```rust
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
```

- [ ] **Step 2: Run** `cargo test --lib diagnostics::rules::pressure_psi::tests -- --nocapture`
Expected: PASS (locks current thresholds). If fail, fix test to match shipped thresholds before changing rule code.

#### 2b — Journal

- [ ] **Step 3: Write tests** in `journal.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::JournalCriticalGroup;

    #[test]
    fn journal_critical_severity_by_count() {
        let mut snap = DiagnosticSnapshot {
            journal_observable: true,
            ..DiagnosticSnapshot::default()
        };
        snap.journal_critical.push(JournalCriticalGroup {
            unit: "ssh.service".into(),
            count: 5,
            sample_message: "boom".into(),
        });
        snap.journal_critical.push(JournalCriticalGroup {
            unit: "cron.service".into(),
            count: 1,
            sample_message: "minor".into(),
        });
        let f = rule_journal_critical(&snap);
        assert_eq!(f.len(), 2);
        let ssh = f.iter().find(|x| x.id.contains("ssh")).unwrap();
        let cron = f.iter().find(|x| x.id.contains("cron")).unwrap();
        assert_eq!(ssh.severity, Severity::Critical);
        assert_eq!(cron.severity, Severity::Warning);
    }

    #[test]
    fn journal_unobservable_is_silent() {
        let snap = DiagnosticSnapshot {
            journal_observable: false,
            journal_critical: vec![JournalCriticalGroup {
                unit: "x.service".into(),
                count: 9,
                sample_message: "x".into(),
            }],
            ..DiagnosticSnapshot::default()
        };
        assert!(rule_journal_critical(&snap).is_empty());
    }
}
```

- [ ] **Step 4: Run** `cargo test --lib diagnostics::rules::journal::tests`

#### 2c — Clock

- [ ] **Step 5: Write tests** in `clock.rs`:

```rust
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
```

- [ ] **Step 6: Run** `cargo test --lib diagnostics::rules::clock::tests`

#### 2d — Coredumps / pstore / etc_meta

- [ ] **Step 7: Write tests**

`coredumps.rs`:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::CoredumpEntry;

    #[test]
    fn coredumps_present_emits_warning() {
        let snap = DiagnosticSnapshot {
            coredumps: vec![CoredumpEntry {
                exe: "/usr/bin/foo".into(),
                signal: Some("11".into()),
                timestamp: "t".into(),
            }],
            ..DiagnosticSnapshot::default()
        };
        let f = rule_coredumps(&snap);
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].id, "crash.coredump.recent");
        assert_eq!(f[0].severity, Severity::Warning);
    }

    #[test]
    fn coredumps_empty_is_silent() {
        assert!(rule_coredumps(&DiagnosticSnapshot::default()).is_empty());
    }
}
```

`pstore.rs`:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::PstoreEntry;

    #[test]
    fn pstore_entries_emit_warning() {
        let snap = DiagnosticSnapshot {
            pstore_entries: vec![PstoreEntry {
                name: "dmesg-0".into(),
                bytes: 12,
            }],
            ..DiagnosticSnapshot::default()
        };
        let f = rule_pstore(&snap);
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].id, "boot.pstore.present");
        assert_eq!(f[0].severity, Severity::Warning);
    }

    #[test]
    fn pstore_empty_is_silent() {
        assert!(rule_pstore(&DiagnosticSnapshot::default()).is_empty());
    }
}
```

`etc_meta.rs`:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::EtcMetaChange;

    #[test]
    fn etc_meta_is_info_only() {
        let snap = DiagnosticSnapshot {
            etc_mtime_notable: vec![EtcMetaChange {
                path: "/etc/hosts".into(),
                kind: "mtime".into(),
            }],
            ..DiagnosticSnapshot::default()
        };
        let f = rule_etc_meta(&snap);
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].id, "config.etc.recent_meta");
        assert_eq!(f[0].severity, Severity::Info);
        assert_eq!(f[0].confidence, Confidence::Low);
    }

    #[test]
    fn etc_meta_empty_is_silent() {
        assert!(rule_etc_meta(&DiagnosticSnapshot::default()).is_empty());
    }
}
```

- [ ] **Step 8: Run** `cargo test --lib diagnostics::rules::coredumps::tests diagnostics::rules::pstore::tests diagnostics::rules::etc_meta::tests`
Expected: all PASS.

- [ ] **Step 9: Do not commit unless user asks**

---

### Task 3: Resource pressure non-memory branches + SMART FAILED/unavailable

**Files:**
- Modify: `src/diagnostics/rules/resource_pressure.rs`
- Modify: `src/diagnostics/rules/smart.rs`

**Interfaces:**
- Thresholds: `SWAP_USED_PCT_WARN=80`, `LOAD_PER_CPU_WARN=4.0`, `DISK_USED_PCT_WARN=95`
- SMART: `passed == Some(false)` → Critical; `available == false` → skip

- [ ] **Step 1: Write resource_pressure tests**

```rust
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
```

- [ ] **Step 2: Run** `cargo test --lib diagnostics::rules::resource_pressure::tests`

- [ ] **Step 3: Write SMART tests** appending to existing `smart.rs` tests module:

```rust
    #[test]
    fn smart_health_failed_is_critical() {
        let snap = DiagnosticSnapshot {
            smart_disks: vec![SmartDiskSnapshot {
                device: "/dev/sdb".into(),
                available: true,
                passed: Some(false),
                uda_crc_error_count: None,
                prev_uda_crc_error_count: None,
                summary: "FAILED".into(),
                details: vec![],
            }],
            ..DiagnosticSnapshot::default()
        };
        let f = rule_smart(&snap);
        assert!(f.iter().any(|x| {
            x.id == "smart.health.sdb" || x.id.starts_with("smart.health.")
        }));
        assert!(f.iter().any(|x| x.severity == Severity::Critical));
    }

    #[test]
    fn smart_unavailable_disk_skipped() {
        let snap = DiagnosticSnapshot {
            smart_disks: vec![SmartDiskSnapshot {
                device: "/dev/sdc".into(),
                available: false,
                passed: Some(false),
                uda_crc_error_count: Some(99),
                prev_uda_crc_error_count: None,
                summary: "n/a".into(),
                details: vec![],
            }],
            ..DiagnosticSnapshot::default()
        };
        assert!(rule_smart(&snap).is_empty());
    }
```

Note: `sanitize_id("/dev/sdb")` → `_dev_sdb` (slashes → `_`). Assert `id == "smart.health._dev_sdb"`.

- [ ] **Step 4: Run** `cargo test --lib diagnostics::rules::smart::tests`
- [ ] **Step 5: Do not commit unless user asks**

---

### Task 4: Silent/conservative edges — failed services, boot, health aggregation

**Files:**
- Modify: `src/diagnostics/rules/failed_services.rs`
- Modify: `src/diagnostics/rules/boot.rs`
- Modify: `src/diagnostics/evaluator.rs` (extend `tests` module)

- [ ] **Step 1: failed_services silence test**

```rust
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
}
```

- [ ] **Step 2: boot clean silence**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::PreviousBootSummary;

    #[test]
    fn boot_clean_hint_emits_nothing() {
        let snap = DiagnosticSnapshot {
            previous_boot_summary: Some(PreviousBootSummary {
                boot_id: "abc".into(),
                unclean_hint: false,
                note: "clean".into(),
            }),
            ..DiagnosticSnapshot::default()
        };
        assert!(rule_previous_boot(&snap).is_empty());
    }
}
```

- [ ] **Step 3: health still surfaces Critical when probes missing**

In `evaluator.rs` tests:

```rust
    #[test]
    fn health_unknown_still_surfaces_critical_findings() {
        use crate::model::{
            Category, Confidence, Evidence, Finding, Severity,
        };
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
```

- [ ] **Step 4: Run**
```bash
cargo test --lib diagnostics::rules::failed_services::tests \
  diagnostics::rules::boot::tests \
  diagnostics::evaluator::tests::health_unknown_still_surfaces_critical_findings -- --exact
```
Expected: PASS.

- [ ] **Step 5: Do not commit unless user asks**

---

### Task 5: `unit_looks_safe` matrix + pure PSI parser fixtures

**Files:**
- Modify: `src/providers/linux/systemd.rs` (extend `unit_safe` or add table test)
- Modify: `src/providers/linux/diagnostics/pressure.rs` — change `fn parse_psi_avg10` → `pub(crate) fn parse_psi_avg10` and add tests

- [ ] **Step 1: Expand unit_looks_safe tests**

```rust
    #[test]
    fn unit_looks_safe_rejects_empty_null_path_and_non_service() {
        assert!(!unit_looks_safe(""));
        assert!(!unit_looks_safe("ssh.timer"));
        assert!(!unit_looks_safe("ssh.socket"));
        assert!(!unit_looks_safe("a\0b.service"));
        assert!(!unit_looks_safe("/abs.service"));
        assert!(unit_looks_safe("user@1000.service"));
        assert!(unit_looks_safe("foo:bar.service"));
        assert!(!unit_looks_safe("ünicode.service")); // non-ascii rejected
    }
```

- [ ] **Step 2: Run** `cargo test --lib providers::linux::systemd::tests::unit_looks_safe_rejects_empty_null_path_and_non_service -- --exact`

- [ ] **Step 3: RED — add PSI parser test against private fn (expect compile fail)**

First add test using `parse_psi_avg10` while still private — confirm compile error naming the private item.

```rust
#[cfg(test)]
mod tests {
    use super::parse_psi_avg10;

    #[test]
    fn parse_psi_avg10_reads_some_and_full() {
        let sample = "some avg10=1.50 avg60=0.00 avg300=0.00 total=0\n\
full avg10=2.25 avg60=0.00 avg300=0.00 total=0\n";
        assert_eq!(parse_psi_avg10(Some(sample), "some"), Some(1.5));
        assert_eq!(parse_psi_avg10(Some(sample), "full"), Some(2.25));
        assert_eq!(parse_psi_avg10(None, "some"), None);
        assert_eq!(parse_psi_avg10(Some("garbage"), "some"), None);
    }
}
```

- [ ] **Step 4: GREEN — change signature to `pub(crate) fn parse_psi_avg10`**

- [ ] **Step 5: Run** `cargo test --lib providers::linux::diagnostics::pressure::tests::parse_psi_avg10_reads_some_and_full -- --exact`
Expected: PASS.

- [ ] **Step 6: Do not commit unless user asks**

---

### Task 6: Full verification gate

**Files:** none (commands only)

- [ ] **Step 1: Run Validation from AGENTS.md**

```bash
cargo fmt --all --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all
cargo build --release
```

- [ ] **Step 2: Using verification-before-completion — only claim success if all four exit 0; paste/summarize counts**
- [ ] **Step 3: Report summary of new tests added (count + modules)**

---

## Self-Review

1. **Spec coverage:** User asked for validation gaps — plan covers untested rules, resource-pressure branches, SMART FAILED/unavailable, silent edges, sanitize_id, unit_looks_safe, PSI parser. Skips soft `$HOME` redact (env-coupled, lower value).
2. **Placeholder scan:** No TBD/TODO steps; concrete assertions included.
3. **Type consistency:** Uses real model structs (`PsiSnapshot`, `JournalCriticalGroup`, `SmartDiskSnapshot`, …) and real finding IDs from production rules.
4. **Branch:** Implement on `fix/diag-rule-fixtures`, not `main`, unless user overrides.
