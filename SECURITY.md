# Security

## Supported versions

| Version | Supported |
|---------|-----------|
| 0.1.x   | yes (best effort) |

This is early MVP software. Security fixes are applied on a best-effort basis by the maintainer.

## Threat model

`server-tui` runs as the invoking user on a trusted or semi-trusted host. Primary risks:

1. **Terminal injection** via malicious log/process/unit/file names.
2. **Accidental privilege impact** (wrong signal / wrong unit) when run as root or with admin rights.
3. **Command injection** if external tools were invoked via shell (forbidden by design).
4. **Data exfiltration** if telemetry or network listeners existed (they do not).

## Safe practice

- Prefer `--read-only` or `--demo` on unfamiliar hosts.
- Prefer an unprivileged account for observation.
- Confirm dialogs carefully for signals and systemd actions.
- Do not install this interactive TUI as a long-running systemd service.

## Untrusted data

Treat as untrusted: journal messages, process command lines, unit descriptions, file names, D-Bus error strings.

Pipeline: strip ANSI → replace control characters → truncate for display where needed.

## Command execution

Allowed external program in MVP: `journalctl` only, fixed argv list, unit names validated.

Signals via `nix` `kill` (never `/bin/kill` through a shell).

Systemd actions via zbus; unit names must look like safe `.service` identifiers.

## Privileges

- Observation works without admin rights.
- Actions fail closed with permission errors.
- `--read-only` disables the administrative executor entirely.
- Running as root amplifies impact; use least privilege.

## Non-goals / hard bans

No telemetry, listeners, password storage, setuid helpers, file deletion, automatic system configuration edits, auto updates, or remote agents.

## Reporting a vulnerability

Please **do not** open a public issue with exploit details against production systems.

Preferred path once the repository is on GitHub:

1. Use **GitHub Security Advisories** for the project, or
2. Contact the maintainer privately via their GitHub profile: [hatemecha](https://github.com/hatemecha).

Include version (`server-tui --version`), OS, and a minimal reproduction when possible.
