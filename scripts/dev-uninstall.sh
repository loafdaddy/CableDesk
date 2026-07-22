#!/usr/bin/env bash
# dev-uninstall.sh — removes files placed by dev-install.sh.
#
# Mirrors dev-install.sh exactly: user-level files are removed without
# asking (they're just files in your own home directory), system-level
# files are only removed after explicit confirmation, and only the exact
# files this project's scripts are documented to place are touched —
# nothing else in /etc or /usr is scanned or modified.

set -euo pipefail

log() { printf '==> %s\n' "$1"; }
confirm() {
  local prompt="$1"
  local reply
  read -r -p "${prompt} [y/N] " reply
  [[ "${reply}" =~ ^[Yy]$ ]]
}

USER_BIN="${HOME}/.local/bin"
USER_APPS="${HOME}/.local/share/applications"
USER_ICONS="${HOME}/.local/share/icons/hicolor/scalable/apps"
USER_UNITS="${HOME}/.config/systemd/user"

log "Removing user-level files..."
systemctl --user disable --now cabledesk-agent.service >/dev/null 2>&1 || true
rm -fv \
  "${USER_BIN}/cabledesk" \
  "${USER_BIN}/cabledeskctl" \
  "${USER_BIN}/cabledesk-agent" \
  "${USER_APPS}/org.cabledesk.CableDesk.desktop" \
  "${USER_ICONS}/org.cabledesk.CableDesk.svg" \
  "${USER_UNITS}/cabledesk-agent.service"
systemctl --user daemon-reload

SYSTEM_FILES=(
  /usr/local/libexec/cabledesk-helper
  /etc/systemd/system/cabledesk-helper.service
  /etc/dbus-1/system.d/org.cabledesk.Helper1.conf
  /usr/share/polkit-1/actions/org.cabledesk.Helper1.policy
  /etc/firewalld/zones/cabledesk.xml
  /etc/modules-load.d/cabledesk.conf
  /etc/udev/rules.d/70-cabledesk.rules
)

echo
echo "The following system-level files may exist from a prior dev-install.sh"
echo "run. This would remove exactly these paths, nothing else:"
printf '  %s\n' "${SYSTEM_FILES[@]}"
echo

if confirm "Remove system-level CableDesk files now (requires sudo)?"; then
  sudo systemctl disable --now cabledesk-helper.service >/dev/null 2>&1 || true
  sudo rm -fv "${SYSTEM_FILES[@]}"
  sudo systemctl daemon-reload
  log "System-level uninstall complete."
else
  log "Skipped system-level removal."
fi
