#!/usr/bin/env bash
# dev-setup.sh — prepares a Fedora workstation for CableDesk development.
#
# Safe to re-run: every check below is idempotent (dnf install on an
# already-installed package is a no-op). Never disables SELinux or
# firewalld, never removes packages, never pipes a remote script into a
# root shell, and never runs `sudo` without first printing exactly what it
# is about to do and asking for confirmation.

set -euo pipefail

log() { printf '==> %s\n' "$1"; }
warn() { printf 'WARNING: %s\n' "$1" >&2; }
die() { printf 'ERROR: %s\n' "$1" >&2; exit 1; }

if [[ "${EUID}" -eq 0 ]]; then
  die "Do not run dev-setup.sh as root. It will ask for sudo only for the specific commands that need it."
fi

if ! command -v dnf >/dev/null 2>&1; then
  die "dnf was not found. This script only supports Fedora; see docs/DISTRO_SUPPORT.md."
fi

confirm() {
  local prompt="$1"
  local reply
  read -r -p "${prompt} [y/N] " reply
  [[ "${reply}" =~ ^[Yy]$ ]]
}

log "Checking for the Rust toolchain (rustc, cargo)..."
if command -v rustc >/dev/null 2>&1 && command -v cargo >/dev/null 2>&1; then
  log "Found: $(rustc --version), $(cargo --version)"
else
  warn "rustc/cargo not found on PATH."
  echo "CableDesk expects a Rust toolchain installed via rustup (https://rustup.rs)."
  echo "This script will not silently fetch and run the rustup installer for you."
  echo "Install it yourself, then re-run this script. Never pipe a remote install"
  echo "script into a root shell; run rustup-init as your own user, not root."
  die "Rust toolchain missing."
fi

# Packages this script may offer to install. Every one of these is a
# development header/tool package (safe, reversible, does not touch
# SELinux/firewalld/unrelated config).
DEV_PACKAGES=(
  gtk4-devel
  libadwaita-devel
  dbus-devel
  pkgconf-pkg-config
  systemd-devel
  rpm-build
  rpmlint
  rust-packaging
  desktop-file-utils
  libappstream-glib
)

log "Checking for required development packages..."
MISSING=()
for pkg in "${DEV_PACKAGES[@]}"; do
  if ! rpm -q "${pkg}" >/dev/null 2>&1; then
    MISSING+=("${pkg}")
  fi
done

if [[ "${#MISSING[@]}" -eq 0 ]]; then
  log "All development packages are already installed."
else
  echo
  echo "The following packages are missing:"
  printf '  - %s\n' "${MISSING[@]}"
  echo
  echo "This script would run exactly:"
  echo "  sudo dnf install -y ${MISSING[*]}"
  echo
  if confirm "Run that command now?"; then
    sudo dnf install -y "${MISSING[@]}"
  else
    warn "Skipped. CableDesk will not build until these are installed."
  fi
fi

log "Checking SELinux and firewalld status (informational only — never changed by this script)..."
if command -v getenforce >/dev/null 2>&1; then
  echo "  SELinux: $(getenforce)"
fi
if systemctl is-active --quiet firewalld 2>/dev/null; then
  echo "  firewalld: active"
else
  echo "  firewalld: not active (this script will not enable or disable it)"
fi

log "Development environment check complete. Next: ./scripts/dev-build.sh"
