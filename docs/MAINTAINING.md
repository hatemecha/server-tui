# Maintaining server-tui

Practical notes for humans and agents working on this repository.

## Build / validate

Required before claiming done:

```bash
cargo fmt --all --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all
cargo build --release
cargo package --locked
cargo deny check   # if cargo-deny is installed
```

Safe smoke (no sudo, no real service mutations):

```bash
cargo run -- --demo
cargo run -- --demo --no-color
cargo run -- --demo --ascii
cargo run -- doctor --demo
cargo run -- support --demo --format markdown
```

## Layout (ownership)

| Layer | Path | May call |
|-------|------|----------|
| UI | `src/ui/` | app state (read), theme/status/viewport |
| App | `src/app/` | model, config, profile, status, viewport, keymap — **not** `ui/` |
| Runtime | `src/runtime/` (`app_loop`, `effects`, `privilege`); `main` is CLI bootstrap only | providers, side effects, terminal |
| Providers | `src/providers/` (+ `linux/diagnostics/*` probes) | Linux/Demo APIs — **not** AppState; **no** `state.toml` writes |
| Diagnostics | `src/diagnostics/` (`evaluator` + `rules/*`) | pure evaluate on snapshots only |
| Actions | `src/actions/` | typed admin + trusted argv |

## How-tos

- **Add a screen**: extend `Screen` in `src/app/action.rs`, keymap in `src/app/update/keymap.rs` (+ reducer/events as needed), screen substate in `src/app/substates.rs`, draw in `src/ui/`, footer + `docs/KEYBINDINGS.md`.
- **Add a provider**: trait in `src/providers/traits.rs`, linux + demo, wire bundles, unit tests with synthetic data.
- **Add a diagnostic rule**: model field if needed → pure rule under `src/diagnostics/rules/` → wire in `evaluate()` → fixture tests; silent on probe failure. Probe I/O goes in `providers/linux/diagnostics/`, never in rules.
- **Add admin action**: typed `AppAction` + confirm dialog (default Cancel) → `AdministrativeExecutor` only.
- **Bump version**: PATCH for fixes/hardening; MINOR for user-visible features. Update `Cargo.toml`, `CHANGELOG.md` `[Unreleased]` → version section when tagging.

## Release

Tags `v*` trigger `.github/workflows/release.yml`. GNU builds must succeed; musl may be experimental. Tarball should include binary (+ docs when packaging locally). Verify `--version`, `--help`, `doctor --demo`.

## Persistence invariant

Only the runtime/app path writes `$XDG_STATE_HOME/server-tui/state.toml`. Providers return observations (e.g. SMART CRC); runtime merges and saves.

## Security reminders

No telemetry, shell (`sh -c`), arbitrary commands, silent privilege elevation, or destructive tests against the host.
