# Changelog

## 0.2.0 — 2026-08-07

Hardening + diagnostics release:

- Canonical `visible_processes` (filter→sort once); selection stable across refresh/sort
- Explicit `UnitFileState`; ListUnits + ListUnitFiles (no N× GetUnitFileState); details on demand
- Shared known-unit registry; admin refuses unregistered units
- Confirm dialogs: default Cancel; Left/Right/Tab focus; Enter activates; y/n/Esc
- SIGKILL success = signal delivered (no false failure if `/proc` lingers); PID reuse guard kept
- Independent metrics refresh caching (CPU/net, memory, disk, temperature, capabilities)
- Structured task shutdown via `TaskTracker` (cancel → wait → timeout)
- `sanitize_path_display`; per-screen `SearchQueries`; `SubsystemHealth`
- Diagnostics screen (6), pure evaluator + probes, demo datasets, deep links, redacted reports
- `server-tui doctor` (`--report` / `--json` / `--include-sensitive`); exit 0/1/2/3
- Glossary (`g`): scannable term list + definition/commands; dense product copy; searchable (`/`)
- List search (`/`): live multi-field filter on list screens + glossary; Esc clears; not on Dashboard
- Dashboard: CPU/mem/swap/load gauges, history + net RX/TX gauges/sparklines; `--ascii` / `--no-color` safe
- Repository URL fixed to https://github.com/hatemecha/server-tui; MSRV 1.81
- Install docs: `cargo install --git`, PATH-aware `install-local.sh`, honest Releases/crates.io status; glossary doc maps to in-app `g`

## 0.1.0 — 2026-08-06

Initial public MVP:

- Dashboard, processes, systemd services, journal logs, storage browser
- `--demo`, `--read-only`, `--no-color`, `--ascii`, optional TOML config
- Confirmations for signals and service actions; terminal restore on quit/signals
- Docs, CI, local/remote install scripts
- MIT license
