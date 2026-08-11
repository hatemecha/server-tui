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
| App | `src/app/` | model, config, profile, status, viewport — **not** `ui/` |
| Runtime | `src/runtime/` (`app_loop`, `effects`, `privilege`); `main` is CLI bootstrap only | providers, side effects, terminal |
| Providers | `src/providers/` (+ `linux/diagnostics/*` probes) | Linux/Demo APIs — **not** AppState; **no** `state.toml` writes |
| Diagnostics | `src/diagnostics/` (`evaluator` + `rules/*`) | pure evaluate on snapshots only |
| Actions | `src/actions/` | typed admin + trusted argv |

Root `theme.rs` / `viewport.rs` / `status.rs` hold shared models so `app` can use them without importing `ui/`. `ui/theme.rs` and `ui/viewport.rs` are re-exports for paint code.

Authoritative key map is `src/app/update/keymap.rs` (`map_key`). There is no parallel binding registry.

## How-tos

- **Add a screen** (full surface):
  1. Extend `Screen` in `src/model/screen.rs` (re-exported as `app::action::Screen`) — digits, labels, `supports_search`, serde slug.
  2. Keymap locals in `src/app/update/keymap.rs` (locals before globals).
  3. Screen substate in `src/app/substates.rs` + field on `AppState` if needed.
  4. `SearchQueries` in `src/app/state.rs` when the screen is searchable.
  5. Reducer `ChangeScreen` / refresh arms in `src/app/update/reducer.rs`; list length / viewport in `src/app/update/navigation.rs`.
  6. Draw dispatch in `src/ui/mod.rs`, screen widget, footer hints in `src/ui/components.rs`, help/glossary as needed.
  7. Update `docs/KEYBINDINGS.md`.
  8. Deep-links from findings use the same `Screen` in `DiagnosticTarget` — no separate target enum.
- **Add a provider**: trait in `src/providers/traits.rs`, linux + demo, wire bundles, unit tests with synthetic data.
- **Add a diagnostic rule**: model field if needed → pure rule under `src/diagnostics/rules/` → wire in `evaluate()` → fixture tests; silent on probe failure. Probe I/O goes in `providers/linux/diagnostics/`, never in rules.
- **Add admin action**: typed `AppAction` + confirm dialog (default Cancel) → `AdministrativeExecutor` only. Unit actions require `unit_looks_safe` **and** known-unit registry on both D-Bus and sudo elevation paths.
- **Bump version**: PATCH for fixes/hardening; MINOR for user-visible features. Update `Cargo.toml`, `CHANGELOG.md` `[Unreleased]` → version section when tagging.

## Release

Tags `v*` trigger `.github/workflows/release.yml`.

- **gnu (`x86_64-unknown-linux-gnu`)**: supported — build/smoke failure fails the job.
- **musl (`x86_64-unknown-linux-musl`)**: **experimental** — failure skips that asset only; do not promise musl in install docs as primary.

Tarball includes binary + LICENSE + README. Verify `--version`, `--help`, `doctor --demo`. GitHub Actions use major version tags (`@v4`) intentionally after prior full-SHA pins proved unverifiable; Dependabot still watches `github-actions`.

## External commands

All allowlisted tools go through `src/actions/trusted.rs` (`TrustedCommand` + `ExecutionPolicy` + `run_oneshot` / `CommandRunner`). Do not add `Command::new("journalctl")`-style PATH lookups.

## Persistence invariant

Only runtime side effects write configuration or `$XDG_STATE_HOME/server-tui/state.toml`. Providers return observations (e.g. SMART CRC); the reducer accepts only the current diagnostic generation, merges it, then asks runtime to save.

## Async freshness and filesystem ownership

- Give replaceable/context-sensitive async work a request generation at intent creation. Scope failures as well as successes. Cancellation is not freshness validation.
- Use `ensure_private_app_dir` only for directories owned by server-tui. `write_private_atomic` creates a unique `0600` sibling, syncs contents, renames, then syncs the parent on Linux, but never chmods an existing parent.
- A custom `--config` path and explicit support `--output` are user-managed. Preserve the exact path and its parent permissions.

## Effective runtime policy

Precedence is persisted base config, then CLI overrides (including `--refresh-ms`), then profile derivation. Every profile switch derives from base values, publishes one effective config to existing pollers/provider caches/tick timing, and resizes history without discarding retained newest samples. Wallboard affects presentation only; combine `--wallboard --performance-profile low-resource` on older hosts.

## Security reminders

No telemetry, shell (`sh -c`), arbitrary commands, silent privilege elevation, or destructive tests against the host.
