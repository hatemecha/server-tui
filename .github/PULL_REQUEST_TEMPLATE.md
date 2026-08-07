## Summary

Briefly describe **why** this change exists (user-visible or maintainer impact).

## Checklist

- [ ] `cargo fmt --all --check`
- [ ] `cargo clippy --all-targets --all-features -- -D warnings`
- [ ] `cargo test --all`
- [ ] Docs / KEYBINDINGS updated when UX changes
- [ ] No telemetry, listeners, `sh -c`, or destructive FS ops
- [ ] Tests do not kill real processes / restart real services / scan `/`

## Test plan

- [ ] `cargo run -- --demo`
- [ ] Relevant screen/manual smoke if UI changed
