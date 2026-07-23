# Implementation Checklist — Milestone: Phase 1 Foundation

Tracks the specific deliverables requested for this milestone (the initial
scaffolding task). See `docs/ROADMAP.md` for what comes after.

- [x] Inspect the repository (was an empty git init, no prior work).
- [x] Determine whether it was empty or contained existing work (empty).
- [x] Research current upstream documentation/source for: `thunderbolt-net`/
      USB4NET, USB4STREAM, NetworkManager D-Bus, IPv4 link-local, Avahi
      interface-scoped discovery, Type-C sysfs, UPower, Sunshine Fedora
      packaging/capture backends/config/input permissions, Moonlight-Qt
      build/CLI/pairing, Fedora RPM packaging, RPM file capabilities,
      Polkit, systemd hardening, firewalld, SELinux policy packaging. See
      `docs/NETWORKING.md`, `docs/POWER_DELIVERY.md`,
      `docs/UPSTREAM_INTEGRATION.md`, `docs/PACKAGING.md`.
- [x] Did not rely on old/unverified commands — every CLI claim in
      `docs/UPSTREAM_INTEGRATION.md` is sourced from the actual current
      upstream source, not forum posts (see that doc's citations).
- [x] Recorded findings in the documentation files listed in the project
      plan.
- [x] Put unresolved upstream behaviour in `docs/OPEN_QUESTIONS.md`.
- [x] Scaffolded the complete Rust workspace (`Cargo.toml` + 13 crates under
      `crates/`, matching the planned layout).
- [x] Implemented shared configuration and error types
      (`cabledesk-core::config`, `cabledesk-core::error`).
- [x] Implemented a minimal platform-detection abstraction
      (`cabledesk-platform::PlatformBackend`).
- [x] Implemented the initial Fedora backend with read-only checks
      (`cabledesk-platform-fedora`, tested on real hardware — see
      `docs/TEST_PLAN.md`).
- [x] Created a basic libadwaita application displaying: CableDesk name,
      device hostname, distribution, desktop environment, session type,
      role not configured, USB4 support status, direct interface status,
      charging information status (`crates/cabledesk-ui`; ran successfully
      on real GNOME/Wayland without crashing — see `docs/TEST_PLAN.md`).
- [x] Created `cabledeskctl compatibility` (plus `diagnostics`, `power`,
      `links`, and stubs for the remaining planned subcommands).
- [x] Created placeholder systemd units (`data/systemd/user/cabledesk-
      agent.service`, `data/systemd/system/cabledesk-helper.service`).
- [x] Created versioned D-Bus interface definitions
      (`data/dbus-1/interfaces/org.cabledesk.{Agent1,Helper1}.xml`, plus a
      D-Bus system policy file discovered necessary by hands-on testing).
- [x] Created Polkit policy placeholders
      (`data/polkit-1/actions/org.cabledesk.Helper1.policy`).
- [x] Created the Fedora RPM spec (`packaging/fedora/cabledesk.spec`).
- [x] Created the development scripts (`scripts/dev-setup.sh`,
      `dev-build.sh`, `dev-install.sh`, `dev-uninstall.sh`, `dev-reset.sh`,
      `build-rpm.sh`).
- [x] Did not bundle Sunshine or Moonlight — `docs/UPSTREAM_INTEGRATION.md`
      documents packaging/CLI/licensing, but no runtime is bundled;
      `cabledeskctl compatibility` correctly reports both as "needs setup."
- [x] Added an experimental-project warning to the README.
- [x] Added a clear warning that ordinary USB-C is not necessarily
      compatible (README, GTK UI subtitle, `docs/DISTRO_SUPPORT.md`).
- [x] Ran Rust formatting, Clippy, unit tests (see `docs/TEST_PLAN.md` and
      the final report for exact commands/results).
- [ ] RPM spec validation — `rpmbuild`/`rpmlint` are not installed in this
      environment; running `sudo dnf install rpm-build rpmlint` was not
      done without asking first, per the project's own rule that dev
      tooling installation must be confirmed. The spec was written from
      verified Fedora packaging research (`docs/PACKAGING.md`) but not
      build-tested. See `docs/OPEN_QUESTIONS.md`.
- [x] Reported: architecture decisions, research findings, files created,
      build commands, test results, open questions, current security
      limitations, recommended next task (see final report and
      `docs/adr/`, `docs/OPEN_QUESTIONS.md`, `docs/SECURITY.md`,
      `docs/ROADMAP.md`).

## Explicitly not claimed as working (Phase 1 scope)

At Phase 1 completion, streaming, pairing, and plug-and-play behaviour were
not implemented and were not claimed to work — Phase 1 itself was read-only
compatibility detection, tested on one real machine that has no
Thunderbolt/USB4 hardware.

**Update (2026-07-23):** Phase 2 software orchestration is wired — agent
Avahi + peer validation, helper `PrepareDirectLink` + Polkit, UI Session
panel, simulation. Authoritative status: `docs/CURRENT_STATUS.md` and
`docs/ROADMAP.md`. This file remains primarily a Phase 1 historical
checklist.

### Phase 2+ checklist (2026-07-23)

- [x] Cable-only interface classification (`InterfaceClassification`)
- [x] Peer policy types (`PreparedDirectLink`, `ValidatedCablePeer`)
- [x] `simulation` feature + `cabledeskctl simulate` (dev builds only)
- [x] Simulated E2E lifecycle tests (Wi-Fi/Ethernet reject, cable unplug)
- [x] `docs/CURRENT_STATUS.md`, `docs/HARDWARE_TEST_PLAN.md`,
      `docs/PROJECT_PLAN.md`
- [x] Agent wires Avahi + `validate_cable_peer` on live hotplug path
- [x] Helper D-Bus `PrepareDirectLink` + Polkit CheckAuthorization
- [x] UI polls agent session state
- [x] `cabledeskctl session` + `repair-network` via helper
- [ ] Physical USB4/Thunderbolt two-machine validation (deferred)
- [ ] RPM/`rpmlint` build verification
- [ ] Phase 3 pairing
