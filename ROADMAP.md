# Roadmap

## MVP (this release)

- Dashboard, processes, services, logs, storage
- Demo + read-only modes (recommended for first runs)
- Config/CLI, docs, CI, install scripts
- Tests + release binary
- Public MIT source tree

## After MVP

- Richer systemd unit details / dependencies graph
- Native journald API (sd-journal) without journalctl child
- Configurable process signal set (INT/HUP)
- Export metrics snapshot to file
- Session layout persistence
- musl CI artifacts (after validation)

## Future ideas

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
