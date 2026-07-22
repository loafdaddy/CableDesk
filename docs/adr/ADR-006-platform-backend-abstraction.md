# ADR-006: A single `PlatformBackend` trait, not scattered `#[cfg(distro)]` branching

## Status

Accepted, implemented (this milestone).

## Context

CableDesk targets Fedora first but plans Tier 2 (Ubuntu, Debian, Arch,
openSUSE) and Tier 3 support later (`docs/DISTRO_SUPPORT.md`). Package
names, firewall backends (firewalld vs. UFW vs. raw nftables), SELinux vs.
AppArmor, and installation mechanics differ per distribution. Scattering
`if distro == "fedora"` branches through the core application would make
every future distro addition a cross-cutting, error-prone change.

## Decision

Define one `PlatformBackend` async trait
(`crates/cabledesk-platform/src/lib.rs`) covering everything
distribution-specific: `detect_system`, `check_dependencies`,
`inspect_security_system`, `prepare_direct_link`, `install_firewall_
policy`, `inspect_power_delivery`, `collect_diagnostics`. Implement the
first concrete backend as `FedoraBackend`
(`crates/cabledesk-platform-fedora`). No other crate — `cabledesk-core`,
`cabledesk-ui`, `cabledesk-agent`, `cabledesk-helper`'s dispatch logic —
may import `cabledesk-platform-fedora` directly or branch on distro.

## Consequences

- At least 90% of the application (the project's own target) stays
  distribution-independent by construction: this milestone's `cabledesk-
  core`, `cabledesk-platform`, `cabledesk-ui`, `cabledesk-agent`, and
  `cabledesk-helper` contain zero Fedora-specific logic — every `dnf`/
  `rpm`/Fedora-specific path lives only in `cabledesk-platform-fedora`.
- Adding Tier 2 support later means writing `cabledesk-platform-debian`/
  `cabledesk-platform-arch`/etc. implementing the same trait, not
  rewriting the application — see `docs/DISTRO_SUPPORT.md`.
- The trait's two mutating methods (`prepare_direct_link`,
  `install_firewall_policy`) were part of the trait from the start, even
  before Phase 1's `FedoraBackend` implemented anything beyond an
  explicit "not implemented yet" error for them — settling the interface
  shape before Phase 2 filled it in meant no caller could silently treat
  "not implemented" as "succeeded" in the meantime. As of Phase 2, both
  are real implementations (see `docs/ROADMAP.md`), still behind the same
  trait signature unchanged.
- Report types (`SystemInfo`, `DependencyReport`, `SecurityReport`,
  `PowerDeliveryStatus`, `PlatformDiagnostics`, `CompatibilityCheck`) are
  also defined once in `cabledesk-platform`, shared by every backend — a
  future `cabledesk-platform-debian` produces the same shapes, so
  `cabledesk-ui`/`cabledeskctl` never need per-backend rendering logic.
