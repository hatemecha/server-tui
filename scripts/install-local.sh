#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BIN="$ROOT/target/release/server-tui"
DEST="/usr/local/bin/server-tui"

if [[ ! -f "$BIN" ]]; then
  echo "error: missing $BIN — run: cargo build --release" >&2
  exit 1
fi

echo "Install:"
echo "  source: $BIN"
echo "  dest:   $DEST"
echo
echo "Command to run:"
echo "  sudo install -m 0755 \"$BIN\" \"$DEST\""
echo
read -r -p "Proceed with sudo install? [y/N] " ans
if [[ "${ans}" != "y" && "${ans}" != "Y" ]]; then
  echo "aborted"
  exit 0
fi

sudo install -m 0755 "$BIN" "$DEST"
echo "installed: $(command -v server-tui)"
server-tui --version
