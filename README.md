# server-tui

**Local Linux observability in your terminal** — dashboard, processes, systemd, journal, storage, and diagnostics as one binary. No browser, no daemon, no open ports.

[![CI](https://github.com/hatemecha/server-tui/actions/workflows/ci.yml/badge.svg)](https://github.com/hatemecha/server-tui/actions/workflows/ci.yml)
[![license: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![MSRV](https://img.shields.io/badge/MSRV-1.81-orange.svg)](Cargo.toml)
[![version](https://img.shields.io/badge/version-0.3.0-informational.svg)](CHANGELOG.md)

<!-- Demo GIF: generate with `vhs docs/assets/demo.tape` when vhs is available; do not link a missing gif. -->

<p align="center">
  <img src="docs/assets/social-preview.png" alt="server-tui" width="640" />
</p>

## Highlights

- **Observe first** — CPU/mem/net, processes, systemd units, journal, storage tree, diagnostics findings
- **Typed admin actions** — SIGTERM/KILL/STOP/CONT and systemctl start/stop/restart/reload/enable/disable with confirmations (default Cancel)
- **Safe defaults** — `--read-only`, `--demo`, redacted support reports, no telemetry
- **Old-server friendly** — `--ascii`, `--no-color`, `--terminal-profile tty16`, `--performance-profile low-resource`

## Install

Until a tagged GitHub Release ships binaries:

```bash
cargo install --git https://github.com/hatemecha/server-tui --locked
```

Ensure `~/.cargo/bin` is on your `PATH`. Verify: `server-tui --version`.

From a checkout: `cargo build --release` then `./scripts/install-local.sh`.

## Quick start

```bash
server-tui --demo
server-tui --read-only
server-tui doctor --demo
server-tui support --format markdown
server-tui setup console --status
```

Useful flags: `--wallboard`, `--terminal-profile modern|tty16|high-contrast|monochrome`, `--performance-profile low-resource|balanced|responsive`, `--ascii`, `--no-color`.

## Screens

| Key | Screen |
|-----|--------|
| `1` | Dashboard (attention + recent activity; `w` wallboard) |
| `2` | Processes |
| `3` | Services (`s`/`x`/`R`/`u`) |
| `4` | Logs |
| `5` | Storage |
| `6` | Diagnostics |
| `7` | Settings |

See [docs/KEYBINDINGS.md](docs/KEYBINDINGS.md). Glossary: `g`. Help: `?`.

## Safety

- No outbound product telemetry, no HTTP listener, no embedded shell, no automatic privilege escalation
- Service actions require validated unit names **and** known-unit registry membership
- Prefer `--read-only` on shared hosts; admin requires confirmation

## Docs

- [ARCHITECTURE.md](ARCHITECTURE.md) · [SECURITY.md](SECURITY.md) · [TESTING.md](TESTING.md) · [AGENTS.md](AGENTS.md)
- [docs/DEPLOYMENT.md](docs/DEPLOYMENT.md) · [CHANGELOG.md](CHANGELOG.md) · [ROADMAP.md](ROADMAP.md)
- [ACKNOWLEDGEMENTS.md](ACKNOWLEDGEMENTS.md) · [SUPPORT.md](SUPPORT.md)

## Ubuntu Server (install only)

```bash
# on the server
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"
cargo install --git https://github.com/hatemecha/server-tui --locked
server-tui --read-only
```

Do **not** install the startup console unit during validation (`setup console --install` is confirm-gated and refused in smoke). For SSH: run interactively; for a spare TTY, print the unit with `server-tui setup console --print-unit` and review before any manual install.
