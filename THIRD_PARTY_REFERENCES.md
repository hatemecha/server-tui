# Third-party references

This project is original code under MIT. Applications below were studied as **behavioral inspiration only**; their source was not copied.

## Inspiration applications

| Project | License (upstream) | What we studied |
|---------|--------------------|-----------------|
| [btop](https://github.com/aristocratos/btop) | Apache-2.0 | Metric layout, process table ideas, small-terminal behavior |
| [systemctl-tui](https://github.com/rgwood/systemctl-tui) | MIT | Service listing UX, systemd interaction patterns |
| [ncdu](https://dev.yorhel.nl/ncdu) | MIT | Hierarchical size navigation, exclusions, cancelable scan |
| [lnav](https://github.com/tstack/lnav) | BSD-2-Clause | Log follow/search/priority concepts |
| [Cockpit](https://cockpit-project.org/) | LGPL | Conceptual “clear status / observe vs act” messaging |

No modules from these projects were vendored.

## Direct dependencies (see Cargo.lock for pinned versions)

| Crate | Role |
|-------|------|
| ratatui / crossterm | TUI |
| tokio / tokio-util / futures | Async runtime |
| sysinfo | Host metrics & processes |
| zbus | systemd D-Bus |
| clap / serde / toml | CLI & config |
| jwalk | Directory walk |
| nix | Signals |
| strip-ansi-escapes / unicode-width | Sanitization & layout |
| tracing | Internal logging |
| thiserror / anyhow | Errors |

Consult each crate’s license on crates.io / docs.rs before redistribution bundles.

Repository: https://github.com/hatemecha/server-tui

