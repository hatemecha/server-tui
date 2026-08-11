# Testing

## Required validation (before claiming done)

```bash
cargo fmt --all --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all
cargo build --release
```

## Manual smoke

```bash
cargo run -- --demo
cargo run -- --read-only
cargo run -- --demo --no-color
cargo run -- --demo --ascii
cargo run -- doctor --demo
cargo run -- doctor --demo --report
cargo run -- doctor --demo --json
cargo run -- --scan-path "$HOME"
```

## What tests cover

- Process filter/sort/`visible_processes` + selection after sort change
- Service filters + `UnitFileState` parsing
- Sanitization including `sanitize_path_display`
- Every confirm dialog defaults to Cancel; `y` is explicit Yes, Enter honors focus, and `n`/Esc cancel
- Demo providers never require root
- Diagnostic evaluator fixtures (OOM, temp wording, no false panic, health Unknown without required probes)
- Shareable/full report policy, free-text identity/IP reduction, Markdown injection resistance, valid JSON
- Atomic `0600` writes preserve user-parent modes, use collision-safe temps, and leave no temp files; explicit app-owned dirs are `0700`
- Deterministic stale-result ordering for storage, logs/follow, diagnostics/SMART persistence, service/process details, preview and PPID maps
- Custom config path typed save boundary; profile derivation/order and history resize
- XDG state.toml atomic roundtrip (incl. SMART CRC counters)
- Safe file preview (symlink refuse, binary detect, 64KiB cap)
- Console unit DryRunFs install/remove (no host writes)
- Trusted sudo argv FakeCommandRunner; ExecutionPolicy timeout/output-cap tests
- Viewport selection clamping
- Footer hint compaction
- TestBackend render smoke + semantic checks (READ ONLY, finding title, tiny fallback)
- Architecture invariants (app↛ui, diagnostics purity, no bare Command::new binaries)

## What tests must never do

- Kill real host processes
- Restart real systemd units
- Real `sudo`, SMART self-tests, or writing `/etc` units
- Scan `/`
- Require root
- Mix Demo and Linux providers in one bundle
- Require an interactive TTY for unit tests

Also validate packaging:

```bash
cargo package --locked --allow-dirty   # or --locked when clean
```

Manual smoke never runs real install/remove host mutation.

## Diagnostic fixtures

Pure evaluator tests live in `src/diagnostics/evaluator.rs` with synthetic `DiagnosticSnapshot` values (no I/O). Demo datasets A–H cycle in the interactive TUI; `doctor --demo` / `support --demo` use fixed **HMixed** so CLI demos always show findings.
