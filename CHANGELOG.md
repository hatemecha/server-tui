# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

## [0.3.1] — 2026-08-11

### Fixed

- Settings toggles `1`–`5` no longer intercepted by global screen digits (locals resolve before globals)
- Service status paint: `inactive` is no longer treated as active (exact `ActiveState` enum)
- Explicit elevation dialog (Cancel default) before any `sudo -v` path; RAII terminal suspension always reenters
- Log refresh replaces the buffer; follow appends with basic dedupe (`LogsReplaced` / `LogsAppended`)
- Export I/O moved to `SideEffect::SaveReport` (no reducer filesystem writes)
- SMART probes no longer write `state.toml` (runtime merges CRC observations)
- File preview opens with `O_NOFOLLOW` + fd re-validation
- Report/config/state files created as `0600`, dirs `0700`
- Product UI/error strings English-only (`AppError::user_message`, systemd permission maps)
- Sudo elevation re-checks known-unit registry; typed `ServiceActionKind` on trusted argv
- MSRV raised to **1.95** (lockfile: sysinfo / ratatui / zbus floors)
- CI/release Actions pins switched to resolvable major tags after broken SHA lookups
- `-CI matrix (ubuntu-latest)`, MSRV **1.95**, D-Bus `libdbus-1-dev` on runners; `deny.toml` for current cargo-deny

### Changed

- App no longer depends on `ui/` for viewport/status/profiles (crate-root modules)
- Typed `SettingId` replaces `ToggleConfigBool(&'static str)`
- Config `version` + migrate; removed dead `config_path_override`
- Config watch channel updates poller intervals without respawn
- Service details cache TTL + invalidation on list refresh
- Storage scan entry budget with truncated partial results
- Process tree cycle/depth guards; Top-K largest files via heap
- Trusted sudo argv uses absolute `systemctl` path
- timedatectl uses machine-readable `show --property=NTPSynchronized --value`
- Release tarballs include LICENSE + README; doctor verify no longer ignores failure
- Docs: `docs/MAINTAINING.md`, ADRs 0001–0003; English-primary agent notes
- `AppState` screen fields extracted into substates (`ProcessState`, `ServiceState`, `LogState`, `StorageState`, `DiagnosticState`, `SettingsState`)
- `src/app/update/` split into keymap / reducer / events / navigation (locals-before-globals keymap remains source of truth)
- Event loop, pollers, and side-effect runner moved to `src/runtime/` (`main` is CLI bootstrap only)
- Linux diagnostic probes split under `providers/linux/diagnostics/`; pure rules under `diagnostics/rules/`; `evaluate()` composes rules
- Single `Screen` type for navigation and diagnostic deep-links (`src/model/screen.rs`)
- Status chrome reads toast only (`status_line`); settings actions extracted from inspect handlers
- Docs: README status/install honesty; SUPPORT matches CI; ACKNOWLEDGEMENTS is sole attribution file

### Added

- Architecture invariant / render matrix tests (locals-before-globals, app↛ui, diagnostics purity, providers↛persist)
- `RedactionPolicy` for support/export redaction

### Removed

- Parallel unused keybinding registry (`src/keymap.rs`); dead `execute_service_action` / `LogsUpdated`
- Stub `THIRD_PARTY_REFERENCES.md`

## [0.3.0] — 2026-08-07

### Added

- Settings screen (`7`) with General / Appearance / Performance / Dashboard / Logs / Diagnostics / Safety / Startup
- Terminal profiles (`--terminal-profile`) and performance profiles (`--performance-profile`)
- Wallboard mode (`--wallboard` / `w` on Dashboard)
- Reusable viewport scrolling with `N / M` position labels and scrollbars on list screens
- Selection markers (`>`) + REVERSED/BOLD for `--no-color` / Tty16
- Toast/status kinds (success/warning/error/progress)
- Process signals SIGSTOP/SIGCONT; service hotkeys: `r` refresh, `R` restart, `u` reload
- Support reports: `server-tui support [--format text|markdown|json]`
- Startup console generator: `server-tui setup console --status|--print-unit` (install refused in smoke)
- Trusted command resolution for sudo/systemctl paths; FakeCommandRunner tests
- SMART read-only probe (`smartctl -H -A -l error`) + CRC findings (pure rules)
- Dashboard Attention + Recent activity panels
- Config atomic save; onboarding_completed; first-run settings hint
- CI matrix (ubuntu-22.04 + latest), MSRV 1.81 job, TERM=linux render job, deny/audit/dependabot, release workflow on `v*` tags
- VHS tape at `docs/assets/demo.tape`

### Fixed

- Process MEM%: refresh memory before %; `Option<f32>` with `?` when unknown; no `total_memory.max(1)` hide
- Services `r` no longer conflicts with global refresh

### Changed

- README product-first; ACKNOWLEDGEMENTS.md replaces THIRD_PARTY_REFERENCES.md
- Version 0.3.0

## [0.2.0] — 2026-08-07

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

## [0.1.0] — 2026-08-06

Initial public MVP:

- Dashboard, processes, systemd services, journal logs, storage browser
- `--demo`, `--read-only`, `--no-color`, `--ascii`, optional TOML config
- Confirmations for signals and service actions; terminal restore on quit/signals
- Docs, CI, local/remote install scripts
- MIT license
