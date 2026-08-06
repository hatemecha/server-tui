#!/usr/bin/env bash
# Optional helper to install an innocuous demo systemd user/system unit for manual tests.
# Does NOT run automatically. Shows exact changes and asks for confirmation.
set -euo pipefail

UNIT_NAME="server-tui-demo.service"
UNIT_PATH="/etc/systemd/system/${UNIT_NAME}"

usage() {
  cat <<EOF
Usage: $0 install|uninstall|status

Creates a harmless oneshot-like looping sleep service for manual
start/stop/restart testing with server-tui. Requires sudo for install/uninstall.
EOF
}

cmd="${1:-}"
case "$cmd" in
  install)
    echo "Will write: $UNIT_PATH"
    echo "Contents:"
    cat <<'UNIT'
[Unit]
Description=server-tui harmless demo service
After=network.target

[Service]
Type=simple
ExecStart=/bin/sleep infinity
Restart=no

[Install]
WantedBy=multi-user.target
UNIT
    read -r -p "Install unit with sudo? [y/N] " ans
    [[ "$ans" == "y" || "$ans" == "Y" ]] || exit 0
    sudo tee "$UNIT_PATH" >/dev/null <<'UNIT'
[Unit]
Description=server-tui harmless demo service
After=network.target

[Service]
Type=simple
ExecStart=/bin/sleep infinity
Restart=no

[Install]
WantedBy=multi-user.target
UNIT
    sudo systemctl daemon-reload
    echo "installed. Try: sudo systemctl start ${UNIT_NAME}"
    ;;
  uninstall)
    echo "Will stop/disable and remove $UNIT_PATH"
    read -r -p "Proceed? [y/N] " ans
    [[ "$ans" == "y" || "$ans" == "Y" ]] || exit 0
    sudo systemctl stop "$UNIT_NAME" 2>/dev/null || true
    sudo systemctl disable "$UNIT_NAME" 2>/dev/null || true
    sudo rm -f "$UNIT_PATH"
    sudo systemctl daemon-reload
    echo "removed"
    ;;
  status)
    systemctl status "$UNIT_NAME" --no-pager || true
    ;;
  *)
    usage
    exit 1
    ;;
esac
