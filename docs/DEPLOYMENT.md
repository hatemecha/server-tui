# Deployment

## Local

```bash
cargo build --release
./scripts/install-local.sh
# equivalent:
sudo install -m 0755 target/release/server-tui /usr/local/bin/server-tui
```

First run after install:

```bash
server-tui --read-only
```

## Remote Linux host

```bash
cargo build --release   # prefer building on the target OS or Ubuntu CI
./scripts/install-remote.sh user@hostname
ssh user@hostname -- server-tui --read-only
```

Manual:

```bash
scp target/release/server-tui user@hostname:/tmp/server-tui
ssh user@hostname \
  'sudo install -m 0755 /tmp/server-tui /usr/local/bin/server-tui &&
   rm -f /tmp/server-tui &&
   server-tui --version'
```

## glibc / musl notes

A binary built on Fedora with a newer glibc may fail on older Ubuntu Server.

Validated approach for distribution: **build on the target Ubuntu host or Ubuntu CI (`x86_64-unknown-linux-gnu`)**.

Planned (not claimed until tested):

- `x86_64-unknown-linux-musl`
- `aarch64-unknown-linux-gnu`

Do **not** install `server-tui` as a systemd service — it is an interactive TUI.
