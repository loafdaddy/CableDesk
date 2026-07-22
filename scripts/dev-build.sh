#!/usr/bin/env bash
# dev-build.sh — builds the CableDesk Cargo workspace.
#
# Safe to re-run (cargo build is incremental and idempotent). Does not
# touch anything outside the repository's target/ directory.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${REPO_ROOT}"

PROFILE="dev"
if [[ "${1:-}" == "--release" ]]; then
  PROFILE="release"
fi

log() { printf '==> %s\n' "$1"; }

log "Formatting check (cargo fmt --check)..."
cargo fmt --all -- --check

log "Building workspace (profile: ${PROFILE})..."
if [[ "${PROFILE}" == "release" ]]; then
  cargo build --workspace --release
else
  cargo build --workspace
fi

log "Running clippy..."
cargo clippy --workspace --all-targets -- -D warnings

log "Running tests..."
cargo test --workspace

log "Build complete. Binaries are in target/${PROFILE}/. Next: ./scripts/dev-install.sh"
