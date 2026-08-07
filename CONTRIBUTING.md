# Contributing

Thanks for considering a contribution. This is a **maintainer-led** personal project: proposals are welcome; the maintainer decides what merges. You may fork and redistribute under the MIT License without permission.

## Before you start

1. Read [AGENTS.md](AGENTS.md), [SECURITY.md](SECURITY.md), and the MVP limits in [ROADMAP.md](ROADMAP.md).
2. Prefer small, reviewable diffs.
3. Do not add features outside the MVP without updating the roadmap intentionally.
4. Install for daily use is documented in [README.md](README.md) / [docs/DEPLOYMENT.md](docs/DEPLOYMENT.md) (`cargo install --git` or `scripts/install-local.sh`).

## Development setup

```bash
git clone https://github.com/hatemecha/server-tui
cd server-tui
cargo test --all
cargo clippy --all-targets --all-features -- -D warnings
cargo fmt --all --check
```

No Docker, paid services, or private registries are required.

## Safety rules for tests

Automated tests must **not**:

- kill real processes
- restart real systemd services
- scan `/`
- require root
- modify `/etc`

Use `--demo` providers / fakes. Optional manual demo unit: `scripts/create-demo-service.sh` (asks confirmation).

## Pull requests

- Describe *why*, not only *what*.
- Note how you tested (commands).
- Keep the license MIT; do not add proprietary code.
- Preserve attribution in [THIRD_PARTY_REFERENCES.md](THIRD_PARTY_REFERENCES.md) when relevant.

## License

By contributing, you agree that your contributions are licensed under the MIT License (see [LICENSE](LICENSE)).
