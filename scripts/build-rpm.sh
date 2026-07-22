#!/usr/bin/env bash
# build-rpm.sh — builds the CableDesk RPM from packaging/fedora/cabledesk.spec.
#
# Uses a self-contained rpmbuild tree under ./build/rpmbuild instead of the
# user's ~/rpmbuild, so this script never touches anything outside the
# repository plus whatever rpmbuild itself needs to read (source packages
# it installs as BuildRequires, handled by the normal dnf builddep flow,
# which is confirmed explicitly below).

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

for tool in rpmbuild cargo git; do
  command -v "${tool}" >/dev/null 2>&1 || die "${tool} not found. Run ./scripts/dev-setup.sh first."
done

SPEC="packaging/fedora/cabledesk.spec"
NAME="cabledesk"
VERSION="$(awk '/^Version:/ {print $2}' "${SPEC}")"

BUILD_ROOT="${REPO_ROOT}/build/rpmbuild"
mkdir -p "${BUILD_ROOT}"/{SOURCES,SPECS,BUILD,RPMS,SRPMS}

log "Creating source tarball for ${NAME}-${VERSION}..."
SRC_STAGE="$(mktemp -d)"
trap 'rm -rf "${SRC_STAGE}"' EXIT
git archive --format=tar --prefix="${NAME}-${VERSION}/" HEAD | tar -x -C "${SRC_STAGE}"
tar --zstd -cf "${BUILD_ROOT}/SOURCES/${NAME}-${VERSION}.tar.zst" -C "${SRC_STAGE}" "${NAME}-${VERSION}"

log "Vendoring Cargo dependencies (cargo vendor)..."
VENDOR_STAGE="$(mktemp -d)"
trap 'rm -rf "${SRC_STAGE}" "${VENDOR_STAGE}"' EXIT
( cd "${REPO_ROOT}" && cargo vendor "${VENDOR_STAGE}/vendor" > "${VENDOR_STAGE}/cargo-config-snippet.toml" )
tar --zstd -cf "${BUILD_ROOT}/SOURCES/${NAME}-${VERSION}-vendor.tar.zst" -C "${VENDOR_STAGE}" vendor

cp "${SPEC}" "${BUILD_ROOT}/SPECS/"

if command -v dnf >/dev/null 2>&1; then
  echo "This would run: sudo dnf builddep -y ${SPEC}"
  if confirm "Install build dependencies with dnf builddep now?"; then
    sudo dnf builddep -y "${SPEC}"
  else
    log "Skipped. The rpmbuild step below may fail if dependencies are missing."
  fi
fi

log "Running rpmbuild -ba..."
rpmbuild -ba "${BUILD_ROOT}/SPECS/${NAME}.spec" \
  --define "_topdir ${BUILD_ROOT}" \
  --define "_sourcedir ${BUILD_ROOT}/SOURCES"

log "Build complete. Packages are under ${BUILD_ROOT}/RPMS and ${BUILD_ROOT}/SRPMS."
