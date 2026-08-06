# Testing

See [AGENTS.md](AGENTS.md) for AI constraints.

## Automated

```bash
cargo test --all
```

Coverage includes: filtering/sorting, selection preservation, storage aggregation, sanitization, config parse/clamp, demo/read-only mode, confirmations, circular buffers, journal JSON parsing (unit), navigation actions.

Guarantees:

- No real process kills
- No real systemd mutations
- No scan of `/`
- No root required

## Demo mode

```bash
cargo run -- --demo
```

Use for UI screenshots and navigation without touching the host.

## Read-only mode

```bash
cargo run -- --read-only
```

Verify signals/services show permission denial / status messaging and never mutate.

## Optional demo systemd unit

```bash
./scripts/create-demo-service.sh install
# exercise start/stop/restart manually in the TUI
./scripts/create-demo-service.sh uninstall
```

## Manual checklist

1. Screen navigation 1–5
2. Resize / 80×24
3. Search `/`
4. Storage cancel `Esc`
5. Quit `q` and `Ctrl+C` restore terminal
6. Permission error path (non-root restart of protected unit)
7. `--no-color` / `--ascii`
8. Invalid config file shows path+field error
9. Over SSH/tmux when available

## Fedora / Ubuntu

Prefer validating release builds on the target distro. Document which toolchain produced a given binary in deployment notes.
