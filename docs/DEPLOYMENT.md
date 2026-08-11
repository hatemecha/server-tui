# Deployment

`server-tui` is an **interactive TUI**. Install the binary once, then run it after SSH login (or on a local console). It is **not** a headless daemon and should **not** be enabled as a boot-time systemd service without a TTY.

## Recommended workflow (server)

1. Install the binary onto PATH (see below).
2. SSH in (or sit at a console / tmux).
3. Run:
   ```bash
   server-tui --read-only   # safe first use
   server-tui               # admin actions only after confirmation
   server-tui doctor        # one-shot text/JSON report
   ```

## Install without cloning (day-to-day use)

### Option A — `cargo install --git` (recommended)

Requires Rust (`rustup`) on the machine where you install:

```bash
cargo install --git https://github.com/hatemecha/server-tui --locked
```

Binary lands in `~/.cargo/bin/server-tui`. Ensure that directory is on `PATH`:

```bash
# bash (~/.bashrc) or zsh (~/.zshrc)
export PATH="$HOME/.cargo/bin:$PATH"

# fish (~/.config/fish/config.fish)
fish_add_path $HOME/.cargo/bin
```

Then:

```bash
server-tui --version
server-tui --read-only
```

### Option B — build in a checkout, then install to PATH

```bash
git clone https://github.com/hatemecha/server-tui
cd server-tui
cargo build --release
./scripts/install-local.sh          # ~/.local/bin (default, no sudo)
./scripts/install-local.sh --system # /usr/local/bin (may need sudo)
```

If `~/.local/bin` is missing from `PATH`, add:

```bash
# bash / zsh
export PATH="$HOME/.local/bin:$PATH"

# fish
fish_add_path $HOME/.local/bin
```

### Option C — GitHub Releases binaries

Published for tags matching `v*` (e.g. `v0.3.1`). Prefer the **gnu** asset for typical glibc distros:

- `server-tui-<tag>-x86_64-unknown-linux-gnu.tar.gz` (+ `SHA256SUMS`)

Optional experimental musl assets may appear when the musl CI job succeeds; treat them as best-effort.

```bash
# after downloading the gnu tarball beside SHA256SUMS
sha256sum -c SHA256SUMS --ignore-missing
tar -xzf server-tui-v0.3.1-x86_64-unknown-linux-gnu.tar.gz
install -m 0755 server-tui-v0.3.1-x86_64-unknown-linux-gnu/server-tui ~/.local/bin/server-tui
# or: sudo install -m 0755 … /usr/local/bin/server-tui
server-tui --version
```

Release page: https://github.com/hatemecha/server-tui/releases

### crates.io

**Not published yet.** A future `cargo install server-tui` from crates.io is optional; today use `--git` or a local build.

## Remote Linux host (copy a release binary)

Build on a matching glibc (prefer the target OS or Ubuntu CI), then:

```bash
cargo build --release
./scripts/install-remote.sh user@hostname
ssh user@hostname -- server-tui --read-only
```

Manual equivalent:

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

## Optional convenience (not required)

- **Shell alias** (after PATH is set):
  ```bash
  alias stui='server-tui --read-only'
  ```
- **Login-session launch:** someone who always wants the TUI on a dedicated TTY *could* write a *manual* systemd **user** unit that starts only when that session has a TTY. This is discouraged for most operators and is **not** “start on boot without a terminal.” Do not ship or enable such a unit by default. Prefer: install once → SSH → type `server-tui`.

## Do not

- Enable `server-tui` as a headless `WantedBy=multi-user.target` boot daemon.
- Expect it to listen on a port or serve a web UI (it does neither).
