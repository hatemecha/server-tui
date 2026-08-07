# Roadmap

## Done in 0.2

- Hardening (visible processes, UnitFileState, ListUnitFiles, unit registry, confirm UX, SIGKILL semantics, metrics caching, task shutdown, sanitize paths, per-screen search, SubsystemHealth)
- Diagnostics screen + pure evaluator + probes + doctor CLI
- Glossary overlay
- XDG state.toml acknowledgements

## Next

- Richer systemd unit details / dependencies graph
- Native journald API (sd-journal) without journalctl child
- Configurable process signal set (INT/HUP)
- Export metrics snapshot to file
- Session layout persistence beyond acknowledgements
- musl CI artifacts (after validation)

## Future ideas (MVP out of scope — do not implement yet)

- Netplan read-only view
- Firewall status view (UFW/nftables read-only)
- User session listing (read-only)
- Package update counts (read-only)
- Docker/Podman container list (opt-in)
- Embedded read-only file viewer

## Out of scope

- Replacing Cockpit
- Remote multi-host fleet control
- Web frontend / HTTP API
- Plugin marketplace
- Telemetry / accounts online
- Destructive disk tools (format, LVM reshape)
- Auto privilege escalation helpers
