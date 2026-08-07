# Keybindings

## Global

| Key | Action |
|-----|--------|
| `1`–`6` | Screens (Dashboard … Diagnostics) |
| `Tab` / `Shift+Tab` | Cycle focus (nav / content / details) |
| `/` | Filter current list (Processes, Services, Logs, Storage, Diagnostics; also Glossary). Matches relevant row fields. Not advertised on Dashboard. |
| `Esc` | Clear active filter / cancel dialog / cancel storage scan / leave search |
| `r` | Refresh current screen (diagnostics re-probes) |
| `g` | Glossary overlay |
| `?` | Help (keybindings) |
| `q` / `Ctrl+C` | Quit (terminal restored) |

## Confirm dialogs

| Key | Action |
|-----|--------|
| `Left` / `Right` / `Tab` | Move Yes/Cancel focus (default **Cancel**) |
| `Enter` | Activate **focused** button |
| `y` | Confirm |
| `n` / `Esc` | Cancel |

## Processes

| Key | Action |
|-----|--------|
| `j`/`k` / arrows | Move |
| `s` | Cycle sort |
| `c` | Toggle full command |
| `t` | SIGTERM (confirm) |
| `K` | SIGKILL (confirm + warning) |

## Services

| Key | Action |
|-----|--------|
| `s`/`x`/`r`/`R`/`e`/`d` | start/stop/restart/reload/enable/disable |
| `l` | Open logs for selected unit |
| `f` | Toggle failed-only filter |

Confirmations default to Cancel.

## Logs

| Key | Action |
|-----|--------|
| `f` | Toggle follow |
| `n`/`N` | Next/prev search match |
| `p` | Cycle min priority |
| `w` | Toggle wrap |

## Storage

| Key | Action |
|-----|--------|
| `Enter` | Enter directory |
| `Backspace` | Parent |
| `s` | Sort |
| `a` | Apparent vs disk size |
| `x` | Stay on filesystem |
| `Esc` | Cancel running scan |

## Diagnostics

| Key | Action |
|-----|--------|
| `Enter` | Follow deep link (screen + search) |
| `o` | Open redacted report |
| `a` | Acknowledge finding (persisted) |
| `r` | Re-run probes |

## CLI

```bash
server-tui doctor [--report] [--json] [--include-sensitive] [--demo]
```
