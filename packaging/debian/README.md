# Debian/Ubuntu packaging (not started)

Placeholder for Phase 6 (`docs/ROADMAP.md`). Fedora is the only supported
build target until Fedora behaviour is reliable end-to-end (pairing,
networking and streaming all working on real hardware), per the project
plan's explicit instruction not to begin cross-distribution packaging
before then.

When this phase starts, expect:

- A `debian/` directory with `control`, `rules` (likely a thin
  `dh`-based wrapper around the same Cargo workspace), `changelog`,
  `copyright` (SPDX-style, matching `LICENSE`), and `*.install` files per
  binary package (mirroring the Fedora subpackage split: `cabledesk`,
  `cabledesk-agent`, `cabledesk-helper`).
- Investigation of whether Debian/Ubuntu's Rust packaging team
  (`debcargo`) has an equivalent per-crate-vs-vendored tradeoff to
  Fedora's `rust2rpm` (see `docs/PACKAGING.md`) — not yet researched.
- A separate `cabledesk-platform-debian` (or shared Debian/Ubuntu) crate
  implementing `PlatformBackend`, following `docs/adr/ADR-006-platform-backend-abstraction.md`.
