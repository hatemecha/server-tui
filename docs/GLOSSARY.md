# Glossary

The interactive glossary lives **in the app**, not only in this file.

| Key | Action |
|-----|--------|
| `g` | Open glossary overlay |
| `Esc` or `g` | Close |
| `/` | Filter terms / command hints |
| `j` / `k` or arrows | Move selection |

`?` opens **keybindings** (separate overlay). See [KEYBINDINGS.md](KEYBINDINGS.md).

## What you see

1. **Terms** — screens, modes, health/severity/confidence, systemd/journal, signals, diagnostics vocabulary.
2. **Command hints** — external example commands with a risk tag (`ro` / `priv` / `dest`). server-tui **never executes** these rows.

Canonical definitions (and ordering) are in [`src/glossary.rs`](../src/glossary.rs). This document is a short map for contributors and operators.

## Term groups (summary)

| Group | Examples |
|-------|----------|
| UI | screens, panes, search, help vs glossary, confirm |
| Modes / CLI | `--demo`, `--read-only`, `doctor` |
| Screens | Dashboard, Processes, Services, Logs, Storage, Diagnostics |
| Diagnostics | health, severity, confidence, deep link, acknowledge, suggested check |
| Host concepts | unit, journal, SIGTERM/SIGKILL, OOM/PSI/load, coredump/pstore |

## Related docs

- [DIAGNOSTICS.md](DIAGNOSTICS.md) — probe → evaluate → findings
- [KEYBINDINGS.md](KEYBINDINGS.md) — keys per screen
- [SECURITY.md](../SECURITY.md) — threat model and safe practice
