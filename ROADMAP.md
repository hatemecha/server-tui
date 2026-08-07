# Roadmap

## Done in 0.3

- Viewport scrolling, selection markers, toast status, terminal/performance profiles
- Settings + first-run onboarding flag, wallboard, support CLI, console unit generator
- Process MEM% correctness; service hotkey coherence (`r` refresh / `R` restart)
- SMART read-only findings; Attention dashboard panel
- Repo polish (CI matrix, release workflow, deny/audit/dependabot, README)

## Future ideas (MVP out of scope — do not implement yet)

- Richer systemd unit dependency graph
- Native journald API (sd-journal) without journalctl child
- Configurable process signal set beyond TERM/KILL/STOP/CONT
- Session layout persistence beyond acknowledgements / onboarding
- Netplan / firewall / users / packages / containers UIs (read-only proposals)
- Embedded file editor / destructive disk tools
- Remote multi-host fleet / HTTP API / plugins / telemetry

## Out of scope

- Replacing Cockpit
- Auto privilege escalation helpers
- Automatic repair / AI remediation
