# server-tui

**Local Linux observability in your terminal** — dashboard, processes, systemd, journal, storage, and deterministic diagnostics in one binary. No browser, no daemon, no open ports, no telemetry.

Complements resource monitors by also covering systemd, journal, storage, and diagnostics.

[![CI](https://github.com/hatemecha/server-tui/actions/workflows/ci.yml/badge.svg)](https://github.com/hatemecha/server-tui/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/hatemecha/server-tui?display_name=tag)](https://github.com/hatemecha/server-tui/releases/latest)
[![license: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![MSRV](https://img.shields.io/badge/MSRV-1.95-orange.svg)](Cargo.toml)

<!-- Demo GIF: generate with `vhs docs/assets/demo.tape` when vhs is available; do not link a missing gif. -->

<p align="center">
  <img src="docs/assets/social-preview.png" alt="server-tui" width="640" />
</p>

## Highlights

- **Observe first** — CPU/mem/net, processes, systemd units, journal, storage tree, diagnostics findings
- **Typed admin actions** — SIGTERM/KILL/STOP/CONT and systemctl start/stop/restart/reload/enable/disable with confirmations (default Cancel)
- **Safe defaults** — `--read-only`, `--demo`, redacted support reports, no telemetry
- **Old-server friendly** — `--ascii`, `--no-color`, `--terminal-profile tty16`, `--performance-profile low-resource`

## Status

Early **0.3.x** software for local Linux (`Cargo.toml` is the package version source of truth). Best-effort maintenance by a single maintainer. Prefer `--demo` / `--read-only` when evaluating.

## Install

### Option A — GitHub Release binary (x86_64 Linux)

Download the latest `server-tui-v*-x86_64-unknown-linux-gnu.tar.gz` from
[Releases](https://github.com/hatemecha/server-tui/releases), verify `SHA256SUMS`, then:

```bash
tar -xzf server-tui-v0.3.1-x86_64-unknown-linux-gnu.tar.gz
sudo install -m 0755 server-tui-v0.3.1-x86_64-unknown-linux-gnu/server-tui /usr/local/bin/server-tui
server-tui --version
```

Primary supported target is **gnu** (glibc). Musl builds may appear as **experimental** assets when the release job succeeds; prefer gnu for typical distros. Use `~/.local/bin` instead of `/usr/local/bin` if you prefer not to use `sudo`.

### Option B — build from source (`cargo`)

```bash
cargo install --git https://github.com/hatemecha/server-tui --locked --force
```

`--force` updates an existing install. That install step needs network access to GitHub and crates.io. The **running** TUI does not phone home.

Ensure `~/.cargo/bin` is on your `PATH`. Verify: `server-tui --version`.

From a checkout: `cargo build --release` then `./scripts/install-local.sh`. More options: [docs/DEPLOYMENT.md](docs/DEPLOYMENT.md).

## Quick start

```bash
server-tui --demo
server-tui --read-only
server-tui doctor --demo
server-tui support --format markdown
server-tui setup console --status
```

Useful flags: `--wallboard`, `--terminal-profile modern|tty16|high-contrast|monochrome`, `--performance-profile low-resource|balanced|responsive`, `--ascii`, `--no-color`.

## Safety

- No outbound product telemetry, no HTTP listener, no embedded shell, no automatic privilege escalation
- External tools (`journalctl`, `systemctl`, …) use trusted path resolution and bounded execution
- Service actions require validated unit names **and** known-unit registry membership
- Prefer `--read-only` on shared hosts; admin requires confirmation

## Compatibility

- Linux with systemd (journal / units). Demo mode works without privileges.
- MSRV **1.95** (see `Cargo.toml`).
- Old TTYs / SSH: use `--ascii`, `--no-color`, or `--terminal-profile tty16`.

## Documentation

- [docs/DEPLOYMENT.md](docs/DEPLOYMENT.md) · [docs/KEYBINDINGS.md](docs/KEYBINDINGS.md) · [CHANGELOG.md](CHANGELOG.md) · [ROADMAP.md](ROADMAP.md)
- [SECURITY.md](SECURITY.md) · [SUPPORT.md](SUPPORT.md) · [CONTRIBUTING.md](CONTRIBUTING.md) · [ACKNOWLEDGEMENTS.md](ACKNOWLEDGEMENTS.md)
- Architecture: [ARCHITECTURE.md](ARCHITECTURE.md) · maintainer notes: [docs/MAINTAINING.md](docs/MAINTAINING.md) · [TESTING.md](TESTING.md) · [AGENTS.md](AGENTS.md)

## License

MIT — see [LICENSE](LICENSE).
