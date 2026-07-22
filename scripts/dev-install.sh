#!/usr/bin/env bash
# dev-install.sh — installs a locally-built CableDesk for development use.
#
# Splits cleanly into:
#   1. User-level files (no root): binaries, desktop entry, icon, the
#      systemd --user unit.
#   2. System-level files (root): the privileged helper binary, its
#      systemd system unit, its D-Bus system policy, its Polkit action
#      definitions, the firewalld zone, modules-load.d and udev rules.
#
# Part 2 is entirely optional and requires explicit confirmation — this
# script prints the exact list of files it will place and the exact
# commands it will run as root before doing anything. It never disables
# SELinux or firewalld, never touches unrelated configuration, and is safe
# to re-run (every copy is idempotent).

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${REPO_ROOT}"

log() { printf '==> %s\n' "$1"; }
die() { printf 'ERROR: %s\n' "$1" >&2; exit 1; }

confirm() {
  local prompt="$1"
  local reply
  read -r -p "${prompt} [y/N] " reply
  [[ "${reply}" =~ ^[Yy]$ ]]
}

PROFILE="dev"
BUILD_DIR="target/debug"
if [[ "${1:-}" == "--release" ]]; then
  PROFILE="release"
  BUILD_DIR="target/release"
fi

for bin in cabledesk cabledeskctl cabledesk-agent cabledesk-helper; do
  [[ -x "${BUILD_DIR}/${bin}" ]] || die "${BUILD_DIR}/${bin} not found. Run ./scripts/dev-build.sh${PROFILE:+ [--release]} first."
done

USER_BIN="${HOME}/.local/bin"
USER_APPS="${HOME}/.local/share/applications"
USER_ICONS="${HOME}/.local/share/icons/hicolor/scalable/apps"
USER_UNITS="${HOME}/.config/systemd/user"

log "Installing user-level files (no root required)..."
mkdir -p "${USER_BIN}" "${USER_APPS}" "${USER_ICONS}" "${USER_UNITS}"
install -m0755 "${BUILD_DIR}/cabledesk" "${USER_BIN}/cabledesk"
install -m0755 "${BUILD_DIR}/cabledeskctl" "${USER_BIN}/cabledeskctl"
install -m0755 "${BUILD_DIR}/cabledesk-agent" "${USER_BIN}/cabledesk-agent"
install -m0644 data/applications/org.cabledesk.CableDesk.desktop "${USER_APPS}/org.cabledesk.CableDesk.desktop"
install -m0644 data/icons/hicolor/scalable/apps/org.cabledesk.CableDesk.svg "${USER_ICONS}/org.cabledesk.CableDesk.svg"
install -m0644 data/systemd/user/cabledesk-agent.service "${USER_UNITS}/cabledesk-agent.service"
echo "  ${USER_BIN}/{cabledesk,cabledeskctl,cabledesk-agent}"
echo "  ${USER_APPS}/org.cabledesk.CableDesk.desktop"
echo "  ${USER_ICONS}/org.cabledesk.CableDesk.svg"
echo "  ${USER_UNITS}/cabledesk-agent.service"

if command -v update-desktop-database >/dev/null 2>&1; then
  update-desktop-database "${USER_APPS}" >/dev/null 2>&1 || true
fi
systemctl --user daemon-reload

echo
log "User-level install complete. Run with: ${USER_BIN}/cabledesk"
echo "  (make sure ${USER_BIN} is on your PATH)"
echo

echo "The privileged system helper is optional for this milestone — it has"
echo "no implemented privileged actions yet (see crates/cabledesk-helper)."
echo "Installing it system-wide would run, as root, exactly:"
echo
echo "  install -Dm0755 ${BUILD_DIR}/cabledesk-helper /usr/local/libexec/cabledesk-helper"
echo "  install -Dm0644 data/systemd/system/cabledesk-helper.service /etc/systemd/system/cabledesk-helper.service"
echo "  install -Dm0644 data/dbus-1/system.d/org.cabledesk.Helper1.conf /etc/dbus-1/system.d/org.cabledesk.Helper1.conf"
echo "  install -Dm0644 data/polkit-1/actions/org.cabledesk.Helper1.policy /usr/share/polkit-1/actions/org.cabledesk.Helper1.policy"
echo "  install -Dm0644 data/firewalld/zones/cabledesk.xml /etc/firewalld/zones/cabledesk.xml"
echo "  install -Dm0644 data/modules-load.d/cabledesk.conf /etc/modules-load.d/cabledesk.conf"
echo "  install -Dm0644 data/udev/rules.d/70-cabledesk.rules /etc/udev/rules.d/70-cabledesk.rules"
echo "  systemctl daemon-reload"
echo
echo "These paths (/etc/...) are deliberately the admin-override locations,"
echo "not /usr/lib/..., so they never collide with a future RPM install and"
echo "are trivially removable by ./scripts/dev-uninstall.sh. Note the helper"
echo "service is installed but NOT enabled or started by this script — do"
echo "that yourself once you are ready to test it."
echo

if confirm "Install the system-level helper files now (requires sudo)?"; then
  sudo install -Dm0755 "${BUILD_DIR}/cabledesk-helper" /usr/local/libexec/cabledesk-helper
  sudo install -Dm0644 data/systemd/system/cabledesk-helper.service /etc/systemd/system/cabledesk-helper.service
  sudo install -Dm0644 data/dbus-1/system.d/org.cabledesk.Helper1.conf /etc/dbus-1/system.d/org.cabledesk.Helper1.conf
  sudo install -Dm0644 data/polkit-1/actions/org.cabledesk.Helper1.policy /usr/share/polkit-1/actions/org.cabledesk.Helper1.policy
  sudo install -Dm0644 data/firewalld/zones/cabledesk.xml /etc/firewalld/zones/cabledesk.xml
  sudo install -Dm0644 data/modules-load.d/cabledesk.conf /etc/modules-load.d/cabledesk.conf
  sudo install -Dm0644 data/udev/rules.d/70-cabledesk.rules /etc/udev/rules.d/70-cabledesk.rules
  sudo systemctl daemon-reload
  log "System-level install complete. Enable when ready with:"
  echo "  sudo systemctl enable --now cabledesk-helper.service"
else
  log "Skipped system-level install. cabledesk-helper is not available system-wide."
fi
