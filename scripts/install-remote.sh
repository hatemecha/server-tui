#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BIN="$ROOT/target/release/server-tui"

if [[ $# -lt 1 ]]; then
  echo "usage: $0 user@host" >&2
  echo "example: $0 ubuntu@your-server" >&2
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
echo "  remote: $TARGET:$TMP -> /usr/local/bin/server-tui"
echo
echo "No passwords are stored. scp/ssh will prompt if needed."
echo

scp "$BIN" "$TARGET:$TMP"
ssh "$TARGET" "sudo install -m 0755 '$TMP' /usr/local/bin/server-tui && rm -f '$TMP' && server-tui --version"

echo
echo "Recommended first run:"
echo "  ssh $TARGET"
echo "  server-tui --read-only"
