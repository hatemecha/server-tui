# Security

## Supported versions

| Version | Supported |
|---------|-----------|
| 0.3.x   | yes (best effort) |
| 0.2.x   | best effort (superseded) |
| 0.1.x   | best effort (superseded) |

This is early software. Security fixes are applied on a best-effort basis by the maintainer.

## Threat model

`server-tui` runs as the invoking user on a trusted or semi-trusted host. Primary risks:

1. **Terminal injection** via malicious log/process/unit/file names.
2. **Accidental privilege impact** (wrong signal / wrong unit) when run as root or with admin rights.
3. **Command injection** if external tools were invoked via shell (forbidden by design).
4. **Data exfiltration** if telemetry or network listeners existed (they do not).
5. **Host identity in reports** (`--include-sensitive` is explicit opt-in to full values).

## Safe practice

- Prefer `--read-only` or `--demo` on unfamiliar hosts.
- Prefer an unprivileged account for observation.
- Confirm dialogs carefully (default focus is **Cancel**).
- Do not install this interactive TUI as a long-running systemd service.
- Shareable support reports mask recognized hostname, username, IP and home-path values and omit command arguments. PIDs, findings, and non-identity portions of process/unit names remain diagnostically useful; unit names may identify workloads.
- Treat all reports as host information, and especially reports produced with `--include-sensitive`, according to your sharing policy.

## Untrusted data

Treat as untrusted: journal messages, process command lines, unit descriptions, file names, D-Bus error strings, probe output.

Pipeline: strip ANSI → replace control characters → `sanitize_path_display` for paths → truncate for display where needed.

Support export adds centralized shareable/full redaction before rendering. Markdown additionally escapes structure and HTML; JSON uses serializer escaping rather than Markdown escaping. This is privacy reduction, not a general secret scanner.

## Command execution

Allowed external programs (fixed argv, no shell): `journalctl`, `coredumpctl`, `timedatectl`, optional `smartctl` (read-only `-H -A -l error`), trusted `sudo`/`systemctl` paths for typed elevation. Binaries resolve only under `/usr/bin`, `/bin`, `/usr/sbin`, `/sbin` via `TrustedCommand`. Oneshot execution applies `ExecutionPolicy` (timeout, stdout/stderr caps); over-limit returns `ExternalOutputLimit` (no silent truncate). `journalctl --follow` is streaming-only (kill_on_drop + cancel; no oneshot timeout). Signals via `nix` `kill`. Systemd via zbus.

Unit actions require: lexical `unit_looks_safe` **and** membership in the shared known-unit registry from the last list.

Never signal PID 0, 1, or self. SIGKILL success means the signal was **delivered**, not that `/proc` vanished.

## Privileges

- Observation works without admin rights; probes degrade independently.
- Actions fail closed with permission errors (systemd/D-Bus policy wording).
- `--read-only` disables the administrative executor entirely.
- Running as root amplifies impact; use least privilege.
- If D-Bus denies a **known-unit** systemctl action, the TUI shows **Administrator permission is required** (default Cancel). Only after Yes: leave TUI → interactive `sudo -v` → re-enter → `sudo -n` + absolute `systemctl` + unit. Never silent elevation.

## Non-goals / hard bans

No telemetry, listeners, password storage, setuid helpers, automatic system configuration edits from smoke/tests, auto updates, remote agents, or silent privilege escalation.

Allowed exceptions (user-initiated):

- Delete **only** aged files under the XDG reports directory when retention cleanup runs.
- Library install/remove of a console unit (`ConsoleFsOps` / CLI guidance) — never from automated tests.

## Reporting a vulnerability

1. Use **GitHub Security Advisories** for https://github.com/hatemecha/server-tui, or
2. Contact the maintainer privately: [hatemecha](https://github.com/hatemecha).

Include version (`server-tui --version`), OS, and a minimal reproduction when possible.
