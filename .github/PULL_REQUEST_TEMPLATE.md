name: Pull request

## Summary

<!-- Why this change is needed -->

## Test plan

- [ ] `cargo fmt --all --check`
- [ ] `cargo clippy --all-targets --all-features -- -D warnings`
- [ ] `cargo test --all`
- [ ] Manual: `cargo run -- --demo` (and `--read-only` if relevant)

## Notes

<!-- Risk to production hosts? Prefer no destructive tests. -->
