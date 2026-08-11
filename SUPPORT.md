# Support

**Maintainer:** [hatemecha](https://github.com/hatemecha)

This is a personal project. Support is **best effort** and not a paid SLA.

## How to get help

1. Read [README.md](README.md) (**Install**, **Quick start**, **Safety**) and [docs/DEPLOYMENT.md](docs/DEPLOYMENT.md).
2. If `server-tui` is “not found”, check that `~/.cargo/bin` or `~/.local/bin` is on `PATH`.
3. Check [docs/KEYBINDINGS.md](docs/KEYBINDINGS.md) and [TESTING.md](TESTING.md) for expected smoke commands.
4. Open a GitHub issue with: OS/distro, `server-tui --version`, command line used, and what you expected vs observed.

## What is in scope

- Bugs that crash the TUI or leave the terminal broken
- False permissions handling / `--read-only` bypass
- Build/CI failures on platforms CI actually covers (Ubuntu runners in `.github/workflows/ci.yml`, MSRV 1.95)

## What is out of scope

- Customizing production fleets or remote orchestration
- Guaranteeing binary glibc compatibility across all distros
- Feature requests that belong in [ROADMAP.md](ROADMAP.md) “out of scope”

## Security issues

See [SECURITY.md](SECURITY.md).
