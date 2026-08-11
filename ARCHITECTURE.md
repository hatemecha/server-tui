# Architecture

See also [AGENTS.md](AGENTS.md), [docs/MAINTAINING.md](docs/MAINTAINING.md), and [docs/adr/](docs/adr/).

## Event flow

1. Crossterm `EventStream` + provider pollers enqueue `AppEvent` on an `mpsc` channel.
2. `apply_event` / `apply_action` mutate `AppState` and return `SideEffect`s.
3. Runtime (`src/runtime/`: `app_loop`, `effects`, `privilege`) spawns tracked tasks for side effects (refresh, scan, follow, admin ops, diagnostics, report save). `main` only parses CLI → bootstraps → `run_loop` → exit.
4. UI redraws on each loop iteration after event handling.
5. Pollers subscribe to a `watch::Receiver<Config>` so refresh intervals follow Settings without duplicate tasks.

## State

`AppState` shell owns navigation/runtime overlays plus coherent screen substates (`ProcessState`, `ServiceState`, `LogState`, `StorageState`, `DiagnosticState`, `SettingsState` in `src/app/substates.rs`), **per-screen** `SearchQueries`, dialogs (confirm with `ConfirmChoice` default Cancel, elevation confirm, help, glossary, report), metric history, and XDG persist state.

Update path is split under `src/app/update/` (`keymap`, `reducer`, `events`, `navigation`, `side_effect`). Keymap resolves digit screen navigation before screen-locals so `1`–`7` always change screens.

Viewport / toast / terminal+performance profiles live outside `ui/` (`src/viewport.rs`, `src/status.rs`, `src/profile.rs`) so `app` does not depend on `ui`.

## Providers

| Trait | Linux | Demo |
|-------|-------|------|
| `MetricsProvider` | sysinfo with independent subsystem TTL caches | Synthetic oscillating metrics |
| `ProcessProvider` | sysinfo processes | Fixed sample set |
| `ServiceProvider` | zbus ListUnits + ListUnitFiles; details cache with TTL | Fake units + UnitFileState |
| `LogProvider` | `journalctl --output=json` | Synthetic lines |
| `StorageProvider` | jwalk + tree aggregate (entry budget) | Fake tree with progress |
| `DiagnosticProbeProvider` | `providers/linux/diagnostics/*` (journal, boot, pstore, coredump, pressure, thermal, clock, config_changes, smart) | Datasets A–H |
| `AdministrativeExecutor` | nix signals + systemd D-Bus + unit registry | Simulated success/errors |

Demo mode uses **only** `DemoProviders` — never mixed with Linux providers.

**Persistence:** providers must not write `state.toml`. Observations (e.g. SMART CRC) flow to the runtime, which is the single writer.

## Diagnostics pipeline

```
DiagnosticProbeProvider.probe() → DiagnosticSnapshot
        ↓
runtime injects prior CRC / metadata
        ↓
evaluate() composes pure rules in diagnostics/rules/*
        ↓
Vec<Finding> + HealthStatus
```

## Permissions

Permission failures that look like D-Bus access denial open **ConfirmElevation** (“Administrator permission is required”, default Cancel). Only on Yes: leave TUI → interactive `sudo -v` → re-enter (RAII `TerminalSuspension`) → `sudo -n` + absolute `systemctl` + unit. Disabled in `--demo` and `--read-only`.

## Viewport

Each list screen owns a `ViewportState`; `viewport_rows` tracks content height from terminal size (PageUp/Down use real pages).
