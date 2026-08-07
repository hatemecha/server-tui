# Architecture

See also [AGENTS.md](AGENTS.md) and [docs/DIAGNOSTICS.md](docs/DIAGNOSTICS.md).

## Event flow

1. Crossterm `EventStream` + provider pollers enqueue `AppEvent` on an `mpsc` channel.
2. `apply_event` / `apply_action` mutate `AppState` and return `SideEffect`s.
3. `main` spawns tracked tasks for side effects (refresh, scan, follow, admin ops, diagnostics).
4. UI redraws on each loop iteration after event handling.

## State

`AppState` owns screen, focus, **per-screen** `SearchQueries`, selection keys (PID / unit name / finding), metric history, log buffer, storage tree, dialogs (confirm with `ConfirmChoice`, help, glossary, report), health status, findings, and XDG persist state.

Selection is preserved across refreshes when the selected PID/unit still exists. Process navigation always uses `visible_processes` (filter then sort once).

## Providers

| Trait | Linux | Demo |
|-------|-------|------|
| `MetricsProvider` | sysinfo with independent subsystem TTL caches | Synthetic oscillating metrics |
| `ProcessProvider` | sysinfo processes | Fixed sample set |
| `ServiceProvider` | zbus ListUnits + ListUnitFiles; `details()` on demand | Fake units + UnitFileState |
| `LogProvider` | `journalctl --output=json` | Synthetic lines |
| `StorageProvider` | jwalk + tree aggregate | Fake tree with progress |
| `DiagnosticProbeProvider` | journalctl/coredumpctl/timedatectl + FS probes | Datasets A–H |
| `AdministrativeExecutor` | nix signals + systemd D-Bus + unit registry | Simulated success/errors |

Demo mode uses **only** `DemoProviders` — never mixed with Linux providers.

## Diagnostics pipeline

```
DiagnosticProbeProvider.probe() → DiagnosticSnapshot
        ↓
DiagnosticEvaluator (pure: no FS/D-Bus/Command/Tokio)
        ↓
Vec<Finding> + HealthStatus
```

Snapshot may also be assembled from in-memory app lists for failed services when probes run.

## Async tasks

- Metrics / processes / services: periodic intervals from config (metrics respect disk/temp/memory TTLs).
- Storage: `spawn_blocking` walker + cancel token + progress channel.
- Journal follow: child `journalctl --follow` with `kill_on_drop` and explicit kill on cancel.
- Diagnostics / admin: one-shot tracked tasks.
- Shutdown: cancel tokens → `TaskTracker::wait` (2s) → best-effort abort via runtime drop; terminal restore via `TerminalGuard`.

## Rendering

Ratatui frames: shell (header with health / nav / responsive footer) → screen panel → optional dialog/glossary overlay.

Narrow terminals (<100 cols) use horizontal tabs. Footer hints compact below ~80 cols. Tiny terminals show a non-panicking warning.

## Permissions

Permission failures become `AppError::Permission` with systemd/D-Bus policy wording (Polkit mentioned only with evidence). Observation continues.

Optional **typed** elevation for known units: leave TUI → interactive `sudo -v` → re-enter → `sudo -n systemctl <action> <unit>` via trusted argv only. Disabled in `--demo` and `--read-only`.

## Inspectors & exports

On Enter (or ActionMenu), list screens open on-demand inspectors (process `/proc` details, service properties + recent logs, log ±5 context, storage mount/file preview). Export writes redacted reports under the XDG reports directory.

## Viewport

Each list screen owns a `ViewportState`; selection is clamped so it never leaves the visible window (see `src/ui/viewport.rs`).
