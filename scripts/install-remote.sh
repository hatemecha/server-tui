#!/usr/bin/env bash
# Copy a locally built release binary to a remote Linux host.
# Prefer building on a matching glibc (target OS or Ubuntu CI).
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BIN="$ROOT/target/release/server-tui"
REMOTE_DEST="/usr/local/bin/server-tui"

if [[ $# -lt 1 ]]; then
  echo "usage: $0 user@host" >&2
  echo "example: $0 ubuntu@your-server" >&2
  echo >&2
  echo "Builds the binary on THIS machine, then scp + remote install." >&2
  echo "server-tui is an interactive TUI — install once, SSH in, then run it." >&2
  exit 1
fi

TARGET="$1"

if [[ ! -f "$BIN" ]]; then
  echo "error: missing $BIN — run: cargo build --release" >&2
  exit 1
fi

TMP="/tmp/server-tui.$$"

echo "Will copy and install:"
echo "  local:  $BIN"
echo "  remote: $TARGET:$TMP -> $REMOTE_DEST"
echo
echo "No passwords are stored. scp/ssh will prompt if needed."
echo
echo "Note: this does not enable any systemd unit. After install, SSH and run"
echo "\`server-tui\` interactively (prefer --read-only on first use)."
echo

scp "$BIN" "$TARGET:$TMP"
ssh "$TARGET" "sudo install -m 0755 '$TMP' '$REMOTE_DEST' && rm -f '$TMP' && server-tui --version"

echo
echo "Recommended first run:"
echo "  ssh $TARGET"
echo "  server-tui --read-only"
echo "  server-tui doctor"
