# Support

**Maintainer:** [hatemecha](https://github.com/hatemecha)

This is a personal project. Support is **best effort** and not a paid SLA.

## How to get help

1. Read [README.md](README.md) (especially **Install** and **Safe first run**).
2. If `server-tui` is “not found”, check that `~/.cargo/bin` or `~/.local/bin` is on `PATH` ([docs/DEPLOYMENT.md](docs/DEPLOYMENT.md)).
3. Check [TESTING.md](TESTING.md) and [docs/KEYBINDINGS.md](docs/KEYBINDINGS.md).
4. Open a GitHub issue with: OS/distro, `server-tui --version`, command line used, and what you expected vs observed.

## What is in scope

- Bugs that crash the TUI or leave the terminal broken
- False permissions handling / `--read-only` bypass
- Build/CI failures on Ubuntu/Fedora with documented Rust versions

## What is out of scope

- Customizing production fleets or remote orchestration
- Guaranteeing binary glibc compatibility across all distros
- Feature requests that belong in [ROADMAP.md](ROADMAP.md) “out of scope”

## Security issues

See [SECURITY.md](SECURITY.md).
