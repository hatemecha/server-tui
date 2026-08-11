# Roadmap

Future work only. Completed work lives in [CHANGELOG.md](CHANGELOG.md).

## Near term

- Further journal follow lifecycle hardening from real-server use (still journalctl backend)
- UX polish driven by physical / SSH / old-TTY feedback
- Optional process signal customization beyond TERM/KILL/STOP/CONT

## Later

- Native journald API (sd-journal) as an optional backend — only with evidence that journalctl is a bottleneck or reliability problem (linking/musl/packaging impact)
- Optional metrics snapshot export (evaluate Prometheus textfile compatibility)
- Networking / firewall read-only views
- Package / container visibility (Docker/Podman)
- Session layout persistence beyond acknowledgements / onboarding
- Richer systemd unit dependency graph
- Mouse support / configurable keymaps (only if there is clear user demand; keyboard remains primary on old TTYs and SSH)
- Remote multi-host fleet / HTTP API / plugins
- Internationalization (i18n) beyond English base copy

## Non-goals

- Replacing Cockpit or becoming a web panel
- Auto privilege escalation helpers
- Automatic repair / AI remediation
- Embedded shell, arbitrary commands, telemetry, or silent elevation
- Netplan / firewall / users / packages / containers UIs in the current MVP scope (record ideas here; do not implement yet)
