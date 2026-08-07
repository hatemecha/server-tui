# Diagnostics

## Pipeline

1. **`DiagnosticProbeProvider`** collects a `DiagnosticSnapshot` (Linux or Demo). It does **not** receive `AppState`.
2. **`evaluate(snapshot)`** is a pure function → `Vec<Finding>`.
3. **`compute_health_status`** aggregates findings with required subsystem observability (systemd + journal). Health is never `Ok` when required subsystems cannot be observed.

## Finding fields

Stable `id`, `title`, `summary`, `Severity`, `Confidence`, `Category`, `Evidence`, `targets` (`DiagnosticTarget` deep links), optional `SuggestedCheck` (hint only — never executed).

## Rules (all degradable)

| Rule | Notes |
|------|-------|
| Failed services | From ListUnits-derived snapshot |
| Journal critical | Grouped by unit; priority 0..2 |
| OOM | Journal/kernel lines |
| Previous boot | Unclean **hint** only — never claims kernel panic |
| pstore | Read-only listing of `/sys/fs/pstore` |
| Coredumps | `coredumpctl` when available |
| PSI | `/proc/pressure/*` |
| Resource pressure | Conservative named thresholds |
| Temperature | Title: “High temperature observed” (not throttling) |
| Clock sync | `timedatectl status` |
| `/etc` metadata | Info only (recent mtime) |

Probe failures are independent and recorded in `probes_degraded`.

## Doctor CLI

```bash
server-tui doctor
server-tui doctor --report
server-tui doctor --json
server-tui doctor --include-sensitive
server-tui doctor --demo
```

Exit: `0` Ok · `1` Warning · `2` Critical · `3` Unknown / probe failure.

## Persistence

`$XDG_STATE_HOME/server-tui/state.toml` (atomic write): `last_seen_boot_id`, `last_diagnostic_at`, `acknowledged_finding_ids`.

## External commands

Fixed argv only: `journalctl`, `coredumpctl`, `timedatectl`. Typed `JournalQuery` → `OsString` args. No shell.
