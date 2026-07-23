# CableDesk — Current Status

**Updated:** 2026-07-23  
**Branch:** `main` (merged from `feature/phase2-agent-discovery-wiring`)  
**Graphify:** 636 nodes · 1039 edges (code-only refresh 2026-07-23)

## Where we are

| | |
|--|--|
| **Current phase** | Phase 2 — Automated networking |
| **Last completed (software)** | Phase 2 cable-only orchestration slice |
| **Phase 2 hardware success** | ❌ Not met (no physical USB4/TB two-machine test) |
| **Next feature phase** | Phase 3 — Secure pairing |
| **Next ops step** | Smoke-test agent/helper/UI install; then pairing |

## Phase board

| Phase | Status |
|-------|--------|
| 0 Research | ✅ Docs / 🔌 HW deferred |
| 1 Foundation | ✅ Software complete |
| 2 Networking | ✅ Software wired / 🧪 simulation / 🔌 HW deferred |
| 3 Pairing | ❌ Not started |
| 4 Streaming | ❌ Not started |
| 5 Polish | ❌ Not started |
| 6 Cross-distro | ❌ Not started |
| 7 Release | ❌ Not started |

## Done (testable without Thunderbolt)

```bash
cargo test --workspace --all-features
cargo run -p cabledeskctl --features simulation -- simulate demo
cargo run -p cabledesk-agent          # stays WaitingForCable without TB
cargo run -p cabledeskctl -- session
cargo run -p cabledesk                 # Session panel polls agent
```

With helper installed (Polkit may prompt):

```bash
cargo run -p cabledeskctl -- repair-network
```

Implemented: classify/peer/sysfs, simulation, agent Avahi +
`validate_cable_peer`, helper `PrepareDirectLink` + Polkit, CLI
`status`/`session`/`repair-network`, UI Session panel.

## Left

| Work | Notes |
|------|--------|
| Phase 3 pairing | Crypto, codes, trust store, UI |
| Phase 4 streaming | Sunshine/Moonlight adapters |
| Phase 5 polish | Notifications, clipboard, suspend, … |
| Hardware plan | All items deferred — `HARDWARE_TEST_PLAN.md` |
| Packaging | RPM/`rpmlint` not build-tested |
| Extra helper methods | Load module / install-firewall / repair-profile actions unused |

## Honesty rule

Do **not** claim USB4 networking, charging under load, desktop streaming,
latency, or plug-and-play works until physical cable tests pass.
