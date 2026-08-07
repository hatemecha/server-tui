# server-tui

Interactive terminal UI for **observing** (and optionally administering) the Linux host where it runs.

> Cockpit-like visibility when you are already inside the server — no browser, no daemon, no open ports.

**Status:** v0.2.0 (early). Maintained by [hatemecha](https://github.com/hatemecha). Prefer `--read-only` or `--demo` on first use.

```
┌──────────────────────────────────────────────────────────────────────┐
│ server-tui   host: equipo-local   uptime: 4d 11h   health: ok       │
├───────────────┬──────────────────────────────────────────────────────┤
│ 1 Dashboard   │  CPU / Memory / Network / Disks                      │
│ 2 Processes   │                                                      │
│ 3 Services    │                                                      │
│ 4 Logs        │                                                      │
│ 5 Storage     │                                                      │
│ 6 Diagnostics │  Findings + health (degradable probes)               │
├───────────────┴──────────────────────────────────────────────────────┤
│ Tab · / · r · g glossary · ? help · q quit                          │
└──────────────────────────────────────────────────────────────────────┘
```

## Install (no git clone required)

### Recommended: `cargo install --git`

Needs a Rust toolchain on the install machine ([rustup](https://rustup.rs/)):

```bash
cargo install --git https://github.com/hatemecha/server-tui --locked
```

This puts the binary in `~/.cargo/bin`. Make sure that directory is on your `PATH`:

```bash
# bash (~/.bashrc) or zsh (~/.zshrc)
export PATH="$HOME/.cargo/bin:$PATH"

# fish (~/.config/fish/config.fish)
fish_add_path $HOME/.cargo/bin
```

Verify:

```bash
server-tui --version
```

### From a source checkout → PATH

```bash
git clone https://github.com/hatemecha/server-tui
cd server-tui
cargo build --release
./scripts/install-local.sh          # ~/.local/bin (default; no sudo)
# ./scripts/install-local.sh --system   # /usr/local/bin (may need sudo)
```

If `~/.local/bin` is missing from `PATH`:

```bash
# bash / zsh
export PATH="$HOME/.local/bin:$PATH"

# fish
fish_add_path $HOME/.local/bin
```

### Releases and crates.io

- **GitHub Releases binaries:** not published yet. After the first tagged release with artifacts, prefer verified downloads if you do not want to build.
- **crates.io:** not published yet. A future `cargo install server-tui` may appear; until then use `--git` or a local build.

More detail: [docs/DEPLOYMENT.md](docs/DEPLOYMENT.md).

## Day-to-day use on a server

This is an **interactive TUI**. It should **not** run as a headless systemd daemon at boot.

Recommended loop:

1. Install once (above) so `server-tui` is on `PATH`.
2. SSH in (or use a console / tmux).
3. Run:
   ```bash
   server-tui --read-only
   server-tui doctor
   ```

Optional convenience: a shell alias in `~/.bashrc` / `~/.zshrc`. A *manual* systemd **user** unit that starts only with a TTY is possible but discouraged and not the default — do not enable a fake “start on boot without a terminal.” See [docs/DEPLOYMENT.md](docs/DEPLOYMENT.md).

## Safe first run

```bash
server-tui --demo
server-tui --read-only
server-tui doctor --demo
```

From a checkout without installing:

```bash
cargo run --release -- --demo
cargo run --release -- --read-only
cargo run --release -- doctor --demo
```

Default mode can send SIGTERM/SIGKILL and start/stop services **after confirmation** (default button is Cancel). Prefer an unprivileged account for observation.

## What it is

- Local TUI (Ratatui + Crossterm) for Linux with systemd
- Dashboard, processes, systemd `.service` units, journal logs, disk usage browser, diagnostics
- Single binary; works over SSH, TTY, and tmux
- `server-tui doctor` for one-shot diagnostics (exit 0/1/2/3)

## What it is not

- Not a Cockpit replacement / remote fleet / web UI / Docker / firewall tool
- Not a background monitoring agent or always-on service

AI/agent notes: [AGENTS.md](AGENTS.md).

## Features (v0.2)

| Area | Capabilities |
|------|----------------|
| Dashboard | CPU, memory, swap, load, network, disks, temps; header health |
| Processes | Filter, sort, details, SIGTERM/SIGKILL with focused confirm |
| Services | ListUnits + ListUnitFiles; on-demand details; enablement states |
| Logs | `journalctl --output=json`, follow, search, priority filter |
| Storage | Background size scan (no delete), cancelable |
| Diagnostics | Probe → pure evaluator → findings; deep links; redacted report |
| Glossary | Global `g` overlay (terms + command risk hints; no execution) |
| Modes | `--demo`, `--read-only`, `--no-color`, `--ascii` |

## Requirements

- Linux with systemd
- Build: Rust **1.81+** (MSRV)
- Run: D-Bus for systemd; `journalctl` for logs (degrades gracefully)

## Build / test (from source)

```bash
git clone https://github.com/hatemecha/server-tui
cd server-tui
cargo fmt --all --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all
cargo build --release
./target/release/server-tui --version
```

## Keys

See [docs/KEYBINDINGS.md](docs/KEYBINDINGS.md). Press `?` (keybindings), `g` (how the app works), and `/` to filter list screens. Quit with `q` or `Ctrl+C`.

## Doctor

```bash
server-tui doctor                 # text report; exit by health
server-tui doctor --report
server-tui doctor --json
server-tui doctor --include-sensitive
server-tui doctor --demo
```

Exit codes: `0` Ok, `1` Warning, `2` Critical, `3` Unknown/probe failure.

## Configuration

`~/.config/server-tui/config.toml` — see [config/example.toml](config/example.toml).  
State (boot id, acknowledged findings): `$XDG_STATE_HOME/server-tui/state.toml`.

## Privacy / security

No telemetry, product beacons, listeners, shell (`sh -c`), or arbitrary command execution. Outbound product networking is not used; the process only talks to local OS facilities (procfs, D-Bus, `journalctl`, optional local tools).  
See [SECURITY.md](SECURITY.md).

## Documentation

- [docs/DEPLOYMENT.md](docs/DEPLOYMENT.md) — install PATH, remote copy, why not a boot daemon
- [docs/DIAGNOSTICS.md](docs/DIAGNOSTICS.md)
- [docs/GLOSSARY.md](docs/GLOSSARY.md)
- [ARCHITECTURE.md](ARCHITECTURE.md)
- [TESTING.md](TESTING.md)
- [ROADMAP.md](ROADMAP.md)
- [CONTRIBUTING.md](CONTRIBUTING.md) · [SUPPORT.md](SUPPORT.md)

## License

MIT — see [LICENSE](LICENSE). Copyright (c) 2026 hatemecha.
