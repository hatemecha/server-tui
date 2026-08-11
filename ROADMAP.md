# Roadmap

## Done in 0.3.1

- Settings keybinding precedence; ActiveState exact paint; elevation dialog + RAII terminal suspension
- LogsReplaced/Appended; SaveReport side effect; single-writer persist (SMART)
- Config watch for pollers; details cache TTL; preview O_NOFOLLOW; 0600/0700 reports
- Dependency inversion for viewport/status/profiles; SettingId; locals-before-globals `map_key`
- Storage entry budget; process-tree guards; architecture/render matrix tests; ADRs + MAINTAINING

## Post-0.3.1 maintainability

Shipped in **0.3.1** (see CHANGELOG): English UI errors; sudo path known-unit registry; unified `Screen`; status_line; docs honesty for Releases.

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
- Further AppState screen substates / full `update.rs` module split
- Netplan / firewall / users / packages / containers UIs (read-only proposals)
- Embedded file editor / destructive disk tools
- Remote multi-host fleet / HTTP API / plugins / telemetry
- Internationalization (i18n) beyond English base copy

## Out of scope

- Replacing Cockpit
- Auto privilege escalation helpers
- Automatic repair / AI remediation
