# AGENTS.md — canonical guide for AI assistants

This file is the **source of truth** for future AI work on `server-tui`.

## Objective

Build and maintain a local Linux TUI (`server-tui`) that feels like “Cockpit when you are already on the server”: dashboard, processes, systemd services, journal logs, storage scanning, and **diagnostics** — as a single binary, no web server, no daemon, no open ports.

## Architecture (must preserve)

```
UI (ratatui) → typed AppAction → update → SideEffect
                                      ↓
                         Providers / AdministrativeExecutor
                                      ↓
                         Linux (sysinfo, zbus, journalctl, jwalk, probes)
                         or Demo (synthetic)
```

Diagnostics: `DiagnosticProbeProvider` → `DiagnosticSnapshot` → **pure** `evaluate` → `Finding`s. Evaluators must not touch FS/D-Bus/Command/Tokio/network.

Rules:

1. UI never calls OS APIs directly.
2. Traits live in `src/providers/traits.rs`.
3. Real Linux code lives under `src/providers/linux/`.
4. Demo mode uses **only** `DemoProviders` — never mix with Linux providers.
5. Slow work runs off the render path (`tokio` tasks / `spawn_blocking`) via `TaskTracker`.
6. All external process args use `Command::arg` / `args` — never `sh -c`.

Primary modules:

| Path | Role |
|------|------|
| `src/app/` | State, events, actions, update + inspect_actions/menus |
| `src/ui/` | Rendering only (+ viewport/selection/status/settings/dialogs) |
| `src/providers/` | Metrics/process/service/log/storage/diagnostics traits + impls |
| `src/diagnostics/` | Pure evaluator + rules + report formatting |
| `src/actions/` | Admin executors (signals, systemd, trusted cmds) |
| `src/preview.rs` | Safe file preview + largest-files from storage tree |
| `src/support.rs` | Redacted support reports |
| `src/setup.rs` | Console unit generator + DryRunFs/RealFs (tests never RealFs) |
| `src/glossary.rs` | Glossary overlay data + render |
| `src/doctor.rs` | `doctor` CLI |
| `src/persist.rs` | XDG state.toml |
| `src/sanitize.rs` | Strip ANSI / control chars / path display |
| `src/terminal.rs` | RAII alternate screen + panic restore |
| `src/lib.rs` | `APP_NAME` / `BINARY_NAME` constants |

## MVP boundaries — do not implement yet

Netplan, firewall/UFW, users, packages, apt/dnf updates, Docker/Podman, VMs, RAID/LVM, mount/unmount, file delete/edit, embedded shell, remote agents, HTTP, plugins, telemetry.

Record ideas in `ROADMAP.md` only.

## Security rules

- No telemetry, outbound product beacons, listeners, shell, arbitrary commands, password storage, setuid, auto binary download.
- XDG report cleanup may delete **only** aged files under the reports directory when the user opts in (retention + Settings `C`); never general file delete.
- Optional interactive sudo escalation for typed known-unit systemctl actions is user-confirming and temporary (leave TUI → `sudo -v` → re-enter → `sudo -n`); never silent privilege escalation.
- Sanitize all system-derived strings before paint (`sanitize_path_display` for paths).
- Never signal PID 0, 1, or self.
- Service actions only with validated unit names (`unit_looks_safe`) **and** known-unit registry membership.
- Read-only mode must hard-block admin executor paths.
- Prefer “I don’t know” / Unknown health over false diagnosis.
- Tests must not kill real processes, restart real services, scan `/`, or require root.

## Validation commands (required before claiming done)

```bash
cargo fmt --all --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all
cargo build --release
```

Manual smoke:

```bash
cargo run -- --demo
cargo run -- --read-only
cargo run -- doctor --demo
cargo run -- --scan-path "$HOME"
```

## How to add a screen

1. Extend `Screen` in `src/app/action.rs`.
2. Handle navigation keys in `src/app/update.rs`.
3. Add `src/ui/<screen>.rs` and wire in `src/ui/mod.rs`.
4. Update footer hints in `src/ui/components.rs` and `docs/KEYBINDINGS.md`.

## How to add a provider

1. Add/extend trait in `src/providers/traits.rs`.
2. Implement demo + linux.
3. Wire in `DemoProviders::bundle` / `linux_bundle`.
4. Add unit tests with demo/fake data.

## How to add a diagnostic rule

1. Extend `DiagnosticSnapshot` if needed (model only).
2. Add a pure rule in `src/diagnostics/evaluator.rs` with fixtures.
3. Ensure probe failures leave the rule silent (degradable).
4. Prefer Unknown / skip over false positives (especially boot/panic).

## How to add an administrative action

1. Add typed `AppAction` + confirmation dialog (`ConfirmChoice` default Cancel).
2. Execute only via `AdministrativeExecutor`.
3. Map permission errors to user-visible Spanish/English messages without panicking.
4. Never accept free-form privileged command strings from the UI.

## Rename the product

Change constants in `src/lib.rs` (`APP_NAME`, `BINARY_NAME`, `CONFIG_DIR_NAME`) and Cargo package/bin name.

## What you may change freely

Bug fixes, UX polish within MVP, tests, docs, performance, provider robustness, clippy cleanups.

## What you must not do

- Destructive tests against the host
- Push/publish without user request
- Disable clippy/tests to get green
- Add `unwrap()` in production paths
- Scope-creep past MVP to look complete
