# AGENTS.md — canonical guide for AI assistants

This file is the **source of truth** for future AI work on `server-tui`.

Deeper how-tos: [docs/MAINTAINING.md](docs/MAINTAINING.md). Architecture: [ARCHITECTURE.md](ARCHITECTURE.md). ADRs: [docs/adr/](docs/adr/).

## Objective

Local Linux TUI (`server-tui`): dashboard, processes, systemd services, journal logs, storage scanning, and diagnostics — single binary, no web server, no daemon, no open ports.

## Architecture (must preserve)

```
UI (ratatui) → typed AppAction → update → SideEffect
                                      ↓
                         runtime → Providers / AdministrativeExecutor
                                      ↓
                         Linux (sysinfo, zbus, journalctl, jwalk, probes)
                         or Demo (synthetic)
```

Diagnostics: Probe → Snapshot → **pure** `evaluate` → Findings. Evaluators must not touch FS/D-Bus/Command/Tokio/network/persist.

Rules:

1. UI never calls OS APIs directly; `app` must not import `ui`.
2. Traits live in `src/providers/traits.rs`; Linux under `src/providers/linux/`.
3. Demo mode uses **only** `DemoProviders`.
4. Slow work runs off the render path via `TaskTracker`.
5. External process args use `Command::arg` / `args` — never `sh -c`.
6. Providers never write `state.toml` (single-writer runtime).
7. Runtime lives in `src/runtime/` (`app_loop`, `effects`, `privilege`); `main` stays CLI → bootstrap → run → exit.
8. Screen state lives in `src/app/substates.rs`; update path in `src/app/update/` (keymap locals before globals). `Screen` enum lives in `src/model/screen.rs` (re-exported from `app::action`).
9. Probe I/O under `providers/linux/diagnostics/`; pure rules under `diagnostics/rules/`.

## MVP boundaries — do not implement yet

Netplan, firewall/UFW, users, packages, apt/dnf, Docker/Podman, VMs, RAID/LVM, mount/unmount, file delete/edit, embedded shell, remote agents, HTTP, plugins, telemetry, i18n framework.

Record ideas in `ROADMAP.md` only.

## Security rules

- No telemetry, listeners, shell, arbitrary commands, password storage, setuid, auto binary download, silent privilege elevation.
- Report cleanup deletes **only** aged files under the reports directory when opted in.
- Elevate only via ConfirmElevation → leave TUI → `sudo -v` → typed `sudo -n systemctl …`.
- Sanitize host-derived strings before paint; never signal PID 0/1/self.
- Service actions: `unit_looks_safe` **and** known-unit registry; read-only hard-blocks admin paths.
- Prefer Unknown health over false diagnosis.
- Tests must not kill real processes, restart real services, sudo, write `/etc`, scan `/`, or require root.

## Validation (required)

```bash
cargo fmt --all --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all
cargo build --release
```

## Product language

English is the base product language for UI, errors, and docs. No i18n system in MVP.

## What you must not do

- Destructive tests against the host
- Push/publish without user request
- Disable clippy/tests to get green / blanket `#[allow]`
- Add `unwrap()` in production paths
- Scope-creep past MVP
