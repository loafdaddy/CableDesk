#!/usr/bin/env bash
# dev-reset.sh — resets the local development environment.
#
# This is a *developer convenience* script (clears build artifacts and this
# user's CableDesk config/state), distinct from `cabledeskctl reset`, which
# resets an installed CableDesk's own application configuration. Nothing is
# removed without a listed, explicit confirmation.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${REPO_ROOT}"

log() { printf '==> %s\n' "$1"; }
confirm() {
  local prompt="$1"
  local reply
  read -r -p "${prompt} [y/N] " reply
  [[ "${reply}" =~ ^[Yy]$ ]]
}

if [[ -d target ]]; then
  echo "This would run: cargo clean (removes ${REPO_ROOT}/target)"
  if confirm "Proceed?"; then
    cargo clean
    log "Removed build artifacts."
  else
    log "Skipped cargo clean."
  fi
else
  log "No target/ directory to clean."
fi

CONFIG_DIR="${HOME}/.config/cabledesk"
STATE_DIR="${HOME}/.local/state/cabledesk"

if [[ -d "${CONFIG_DIR}" || -d "${STATE_DIR}" ]]; then
  echo
  echo "This would remove local CableDesk configuration/state:"
  [[ -d "${CONFIG_DIR}" ]] && echo "  ${CONFIG_DIR}"
  [[ -d "${STATE_DIR}" ]] && echo "  ${STATE_DIR}"
  echo "This deletes any locally-stored device identity and trusted-device list."
  if confirm "Proceed?"; then
    rm -rfv "${CONFIG_DIR}" "${STATE_DIR}"
    log "Removed local configuration/state."
  else
    log "Skipped configuration/state removal."
  fi
else
  log "No local CableDesk configuration/state found."
fi

log "Reset complete. Next: ./scripts/dev-build.sh"
