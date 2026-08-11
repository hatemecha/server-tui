# Keybindings

## Global

| Key | Action |
|-----|--------|
| `1`–`7` | Screens (Dashboard … Settings) — always; never overridden by Settings |
| `Tab` / `Shift+Tab` | Enter content / cycle focus (nav ↔ content ↔ details) |
| `/` | Filter current list (not Dashboard/Settings) |
| `Esc` | Clear filter / cancel dialog / cancel scan |
| `r` | **Refresh** (always — never restart) |
| `w` | Toggle wallboard (Dashboard; also `--wallboard`) |
| `g` | Glossary |
| `?` | Help |
| `q` / `Ctrl+C` | Quit |

All confirmation dialogs start on Cancel. Left/Right/Tab moves focus, Enter activates focus, `y` explicitly chooses Yes, and `n`/Esc cancels.

## Processes

| Key | Action |
|-----|--------|
| `j`/`k` · arrows · PgUp/PgDn · Home/End | Move (viewport keeps selection visible) |
| `s` | Cycle sort |
| `c` | Toggle full command |
| `Enter` | Process inspector (parent, cmdline, IO, cgroup, unit, fds) |
| `m` | Action menu (follow, tree, signals, export, …) |
| `F` / `T` | Follow selected PID / tree view (on-demand parents) |
| `t` / `K` / `z` / `Z` | SIGTERM / SIGKILL / SIGSTOP / SIGCONT (confirm) |
| `e` | Export process context (command arguments omitted) |

## Services

| Key | Action |
|-----|--------|
| `Enter` / `m` | Inspector (properties + recent logs) / action menu |
| `s` | Start |
| `x` | Stop |
| `R` | Restart |
| `u` | Reload |
| `e` / `d` | Enable / Disable |
| `E` | Export service context |
| `l` | Logs for unit |
| `f` | Failed-only filter |

Permission-denied D-Bus actions open **Administrator permission is required** (Cancel default). Confirming leaves the TUI for interactive `sudo -v`, then typed `sudo -n` + absolute `systemctl` + unit (not in `--demo` / `--read-only`).

## Logs

| Key | Action |
|-----|--------|
| `f` | Follow |
| `n`/`N` | Next/prev match |
| `p` | Priority floor |
| `w` | Wrap |
| `[` | Cycle preset (important / boot / 1h / kernel / service / all) |
| `Enter` | Event inspector (±5 context) |
| `m` | Export menu (selected / visible / context; text|md|json) |
| `e` | Export visible (text) |

## Storage

| Key | Action |
|-----|--------|
| `t` | Cycle tabs (mounts / directory / largest) |
| `Enter` | Mounts: details · Directory: enter dir · Largest/file: 64KiB preview |
| `Backspace` | Parent (directory tab) |
| `s` / `a` / `x` | Sort / apparent / stay-on-fs |
| `r` | Rescan |
| `Esc` | Cancel scan |
| `m` | Action menu (preview) |

No delete.

## Diagnostics

| Key | Action |
|-----|--------|
| `Enter` | Finding inspector |
| `e` / `m` | Export finding · action menu |
| `o` | Full diagnostic report overlay |
| `a` | Acknowledge |
| `D` | Deep scan (enables SMART when allowed in Settings) |
| `r` | Re-probe |

## Settings (`7`)

| Key | Action |
|-----|--------|
| `Tab` / `Shift+Tab` | Sections menu ↔ options list |
| `j`/`k` · arrows | Move (section or option, depending on focus) |
| `Space` / Enter | Toggle checkbox · cycle value · run action |
| `←` / `→` | Cycle profile/preset (checkbox: off / on) |
| `S` | Save config (also via **Save settings** row) |
| Home / End | First / last item in focused pane |

## CLI

```bash
server-tui doctor [--report] [--json] [--include-sensitive] [--demo]
server-tui support [--format text|markdown|json] [--output PATH] [--include-sensitive] [--demo]
server-tui setup console --status|--print-unit [--tty tty2] [--user NAME]
```

Wallboard does not change resource policy. For persistent display on old hosts, combine `--wallboard --performance-profile low-resource`.

Install/remove of the console unit is library-backed (`ConsoleFsOps`); automated tests/smoke use dry-run only and never write `/etc`.
