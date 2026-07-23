# CableDesk — Project Plan

CableDesk requires a compatible direct USB4 or Thunderbolt cable between a
Linux host and a Linux controller. It does **not** support Wi-Fi, ordinary
Ethernet/LAN, routers, VPNs, Tailscale/ZeroTier, manual IP entry, or any
silent network fallback. If the cable is removed, the session stops.

## Goals

1. Detect USB4/Thunderbolt direct-link capability and report compatibility.
2. Automate the direct-cable network lifecycle (profile, address, discovery,
   route validation, cable-loss handling).
3. Pair devices securely while a direct cable link is active.
4. Manage Sunshine (host) and Moonlight (controller) only over validated
   cable endpoints.
5. Refine plug-and-play behaviour without expanding transport scope.

## Phase status (see `docs/ROADMAP.md` for detail)

| Phase | Focus | Status |
|-------|--------|--------|
| 0 | Research & feasibility | ✅ Docs; 🔌 HW deferred |
| 1 | Secure project foundation | ✅ Software complete |
| 2 | Automated direct-cable networking | ✅ Software orchestration; 🔌 two-machine TB deferred |
| 3 | Secure pairing | ❌ Not started |
| 4 | Managed streaming | ❌ Not started |
| 5 | Plug-and-play refinement | ❌ Not started |
| 6 | Cross-distro backends | ❌ Not started |
| 7 | Public release | ❌ Not started |

Audited snapshot: `docs/CURRENT_STATUS.md`.  
Physical cable tests: `docs/HARDWARE_TEST_PLAN.md` (all deferred).

## Non-goals (v1)

- Any transport other than a verified USB4/Thunderbolt-derived interface
- Internet / remote-access product modes
- Bundling Sunshine or Moonlight binaries
- Disabling SELinux or firewalld
- Running the GTK UI as root

## Rejected or deferred concepts

- Wi-Fi or Ethernet fallback when the cable drops
- Manual host/IP entry in the normal application
- Sunshine/Moonlight discovery across the normal LAN
- DHCP-first / `ipv4.link-local=fallback` on the direct link (v1 uses plain
  `ipv4.method=link-local` — ADR-003)

## Documentation map

| Document | Role |
|----------|------|
| `docs/CURRENT_STATUS.md` | What works now vs what is left |
| `docs/ROADMAP.md` | Phased deliverables and success conditions |
| `docs/ARCHITECTURE.md` | Processes, state machine, cable-only invariants |
| `docs/SECURITY.md` / `THREAT_MODEL.md` | Security commitments |
| `docs/NETWORKING.md` | Direct-link networking research |
| `docs/POWER_DELIVERY.md` | Type-C / UPower research |
| `docs/UPSTREAM_INTEGRATION.md` | Sunshine / Moonlight |
| `docs/PACKAGING.md` / `DISTRO_SUPPORT.md` | Packaging and tiers |
| `docs/TEST_PLAN.md` | Automated and live-safe tests |
| `docs/HARDWARE_TEST_PLAN.md` | Deferred physical tests |
| `docs/OPEN_QUESTIONS.md` | Unresolved decisions |
| `docs/adr/` | Architecture Decision Records |
