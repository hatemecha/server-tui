#!/usr/bin/env bash
# Install a locally built server-tui binary onto PATH.
# Default: ~/.local/bin (no sudo). Optional: --system → /usr/local/bin.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BIN="$ROOT/target/release/server-tui"
MODE="user" # user | system
DEST=""
YES=0

usage() {
  cat <<'EOF'
Usage: scripts/install-local.sh [--user|--system] [-y|--yes] [--dest DIR]

  --user     Install to ~/.local/bin/server-tui (default; no sudo)
  --system   Install to /usr/local/bin/server-tui (requires sudo)
  --dest DIR Install to DIR/server-tui (overrides --user/--system)
  -y, --yes  Skip confirmation prompt

Prerequisites:
  cargo build --release   # produces target/release/server-tui

After install, ensure the destination directory is on your PATH, then run:
  server-tui --version
  server-tui --read-only
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --user) MODE="user"; shift ;;
    --system) MODE="system"; shift ;;
    --dest)
      DEST="${2:-}"
      if [[ -z "$DEST" ]]; then
        echo "error: --dest requires a directory" >&2
        exit 1
      fi
      shift 2
      ;;
    -y|--yes) YES=1; shift ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "error: unknown argument: $1" >&2
      usage >&2
      exit 1
      ;;
  esac
done

if [[ ! -f "$BIN" ]]; then
  echo "error: missing $BIN" >&2
  echo "run:  cargo build --release" >&2
  exit 1
fi

if [[ -z "$DEST" ]]; then
  if [[ "$MODE" == "system" ]]; then
    DEST="/usr/local/bin"
  else
    DEST="${HOME}/.local/bin"
  fi
fi

TARGET="${DEST%/}/server-tui"

echo "Install:"
echo "  source: $BIN"
echo "  dest:   $TARGET"
echo

if [[ "$YES" -ne 1 ]]; then
  read -r -p "Proceed? [y/N] " ans
  if [[ "${ans}" != "y" && "${ans}" != "Y" ]]; then
    echo "aborted"
    exit 0
  fi
fi

mkdir -p "$DEST"

if [[ -w "$DEST" ]]; then
  install -m 0755 "$BIN" "$TARGET"
else
  echo "Destination not writable; trying sudo…"
  sudo install -m 0755 "$BIN" "$TARGET"
fi

echo "installed: $TARGET"

# PATH checks (do not mutate the user's shell config)
path_ok=0
case ":${PATH}:" in
  *":${DEST}:"*) path_ok=1 ;;
esac

if command -v server-tui >/dev/null 2>&1; then
  server-tui --version
  echo
  echo "OK: \`server-tui\` is on PATH."
elif [[ "$path_ok" -eq 1 ]]; then
  echo
  echo "Installed, but \`server-tui\` was not found yet (hash/cache?)."
  echo "Try: hash -r   # bash"
  echo "Or open a new shell, then: server-tui --version"
else
  echo
  echo "Installed, but ${DEST} is not on your PATH."
  echo
  echo "Add it (pick your shell), then open a new terminal:"
  echo
  echo "  # bash (~/.bashrc) or zsh (~/.zshrc)"
  echo "  export PATH=\"\$HOME/.local/bin:\$PATH\""
  echo
  echo "  # fish (~/.config/fish/config.fish)"
  echo "  fish_add_path \$HOME/.local/bin"
  echo
  if [[ "$DEST" == "/usr/local/bin" ]]; then
    echo "  # /usr/local/bin is usually already on PATH; check: echo \$PATH"
  elif [[ "$DEST" != "${HOME}/.local/bin" ]]; then
    echo "  export PATH=\"${DEST}:\$PATH\""
  fi
fi

echo
echo "Recommended first run (interactive TUI; not a daemon):"
echo "  server-tui --read-only"
echo "  server-tui doctor --demo"
