# Architecture

See also [AGENTS.md](AGENTS.md).

## Event flow

1. Crossterm `EventStream` + provider pollers enqueue `AppEvent` on an `mpsc` channel.
2. `apply_event` / `apply_action` mutate `AppState` and return `SideEffect`s.
3. `main` spawns tasks for side effects (refresh, scan, follow, admin ops).
4. UI redraws on each loop iteration after event handling (interval tick prevents stalls).

## State

`AppState` owns screen, focus, filters, selection keys (PID / unit name), metric history (`CircularBuffer`), log buffer, storage tree breadcrumbs, dialogs, and mode flags (`demo`, `read_only`).

Selection is preserved across refreshes when the selected PID/unit still exists.

## Providers

| Trait | Linux | Demo |
|-------|-------|------|
| `MetricsProvider` | sysinfo + `/etc/os-release` + thermal zones | Synthetic oscillating metrics |
| `ProcessProvider` | sysinfo processes | Fixed sample set |
| `ServiceProvider` | zbus `org.freedesktop.systemd1` | Fake units including failed |
| `LogProvider` | `journalctl --output=json` | Synthetic lines |
| `StorageProvider` | jwalk + tree aggregate | Fake tree with progress |
| `AdministrativeExecutor` | nix signals + systemd D-Bus | Simulated success/errors |

## Async tasks

- Metrics / processes / services: periodic intervals from config.
- Storage: `spawn_blocking` walker + cancel token + progress channel.
- Journal follow: child `journalctl --follow` with `kill_on_drop` and explicit kill on cancel.
- Admin ops: one-shot tasks reporting `OperationFinished`.

## Rendering

Ratatui frames composed by `ui::draw` → shell (header/nav/footer) → screen panel → optional dialog overlay.

Narrow terminals (<100 cols) switch side nav to horizontal tabs. Tiny terminals show a non-panicking warning.

## Cancellation

Application `CancellationToken` cancels pollers on quit. Separate tokens cancel active storage scan and journal follow so children are not orphaned.

## Permissions

Permission failures become `AppError::Permission` with a clear user message. Observation continues. No password prompts, no setuid helper in MVP.
