# server-tui

Interactive terminal UI for **observing** (and optionally administering) the Linux host where it runs.

> Cockpit-like visibility when you are already inside the server — no browser, no daemon, no open ports.

**Status:** early MVP (v0.1.0). Maintained by [hatemecha](https://github.com/hatemecha). Expect rough edges; prefer `--read-only` or `--demo` on first use.

```
┌──────────────────────────────────────────────────────────────────────┐
│ server-tui   host: equipo-local   uptime: 4d 11h   mode: READ ONLY   │
├───────────────┬──────────────────────────────────────────────────────┤
│ 1 Dashboard   │  CPU / Memory / Network / Disks                      │
│ 2 Processes   │                                                      │
│ 3 Services    │                                                      │
│ 4 Logs        │                                                      │
│ 5 Storage     │                                                      │
├───────────────┴──────────────────────────────────────────────────────┤
│ Tab focus · / search · r refresh · ? help · q quit                  │
└──────────────────────────────────────────────────────────────────────┘
```

## Safe first run (recommended)

These modes **do not** send signals or change systemd units:

```bash
# Synthetic UI only — never touches your system
cargo run --release -- --demo

# Real metrics/logs, no administrative actions
cargo run --release -- --read-only
```

Default mode (`server-tui` with no flags) can send SIGTERM/SIGKILL and start/stop services **after confirmation**. Do not run it as root until you understand the keybindings. Prefer an unprivileged account for observation.

What will **not** happen by design:

- no file deletion or editing
- no network listeners / outbound product telemetry
- no shell (`sh -c`) or arbitrary command execution
- no automatic scan of `/` on startup (storage defaults to `$HOME`)
- no install as a systemd service by these scripts

## What it is

- Local TUI (Ratatui + Crossterm) for Linux with systemd
- Dashboard, processes, systemd `.service` units, journal logs, disk usage browser
- Single binary; works over SSH, TTY, and tmux

## What it is not

- Not a Cockpit replacement
- Not remote fleet management
- Not a web UI / HTTP server / Docker manager / firewall tool

AI/agent development notes: [AGENTS.md](AGENTS.md).

## Features (MVP)

| Area | Capabilities |
|------|----------------|
| Dashboard | CPU, memory, swap, load, network rates, disks, temps (best-effort) |
| Processes | Filter, sort, details, SIGTERM/SIGKILL **with confirmation** |
| Services | List via D-Bus; start/stop/restart/reload/enable/disable **with confirmation** |
| Logs | `journalctl --output=json`, follow, search, priority filter, sanitization |
| Storage | Background size scan (no delete), cancelable; excludes `/proc` `/sys` `/dev` `/run` |
| Modes | `--demo`, `--read-only`, `--no-color`, `--ascii` |

## Requirements

- Linux with systemd (Fedora / Ubuntu Server oriented; no Windows/macOS support)
- To **build**: Rust 1.81+
- To **run**: D-Bus for systemd actions; `journalctl` for logs (observation degrades gracefully if missing)

## Build

```bash
git clone <your-fork-or-repo-url>
cd server-tui
cargo build --release
./target/release/server-tui --version
```

## Install (optional)

```bash
./scripts/install-local.sh
# equivalent:
sudo install -m 0755 target/release/server-tui /usr/local/bin/server-tui
```

Remote (replace host):

```bash
./scripts/install-remote.sh user@your-server
ssh user@your-server -- server-tui --read-only
```

Build on the **target** distro (or Ubuntu CI) when possible: a Fedora-built glibc binary may not run on older Ubuntu. See [docs/DEPLOYMENT.md](docs/DEPLOYMENT.md).

## Keys

See [docs/KEYBINDINGS.md](docs/KEYBINDINGS.md). Press `?` inside the app. Quit with `q` or `Ctrl+C` (terminal is restored).

## Configuration

Optional file: `~/.config/server-tui/config.toml`  
Example: [config/example.toml](config/example.toml). Missing config is fine (safe defaults).

## Privacy / networking

- No telemetry
- No outbound connections initiated by the app for analytics
- No listening sockets
- Debug logs only when you pass `--debug-log /path/to/file`

## Security

See [SECURITY.md](SECURITY.md). Report vulnerabilities privately via GitHub Security Advisories on the repository once published (or contact the maintainer).

## Limitations

- Early software; test on non-critical hosts first
- Large storage scans can use significant memory; cancel with `Esc`; avoid scanning `/` casually
- Permission errors are expected without admin rights; the UI continues in observation mode
- Out of MVP: Netplan, firewall, users, packages, containers ([ROADMAP.md](ROADMAP.md))

## Development

```bash
cargo fmt --all --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all
cargo build --release
```

More: [TESTING.md](TESTING.md), [ARCHITECTURE.md](ARCHITECTURE.md), [CONTRIBUTING.md](CONTRIBUTING.md), [SUPPORT.md](SUPPORT.md).

## License

MIT — see [LICENSE](LICENSE). Copyright (c) 2026 hatemecha.

Inspiration and crates: [THIRD_PARTY_REFERENCES.md](THIRD_PARTY_REFERENCES.md).
