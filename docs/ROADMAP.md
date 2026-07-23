# CableDesk Roadmap

Phased plan. Each phase has a concrete success condition. Do not claim a
phase complete on hardware until it has been verified on a real USB4 or
Thunderbolt cable (see `docs/HARDWARE_TEST_PLAN.md`). Authoritative
snapshot: `docs/CURRENT_STATUS.md`.

---

## Done vs left (quick view)

| Phase | Software | Hardware |
|-------|----------|----------|
| 0 Research | ✅ Docs + probes | 🔌 Deferred |
| 1 Foundation | ✅ Complete | ⚠️ No TB on original test host |
| 2 Networking | ✅ Software orchestration wired; 🧪 simulation | 🔌 Two-machine TB deferred |
| 3 Pairing | ❌ Not started (stops at `PairingRequired`) | — |
| 4 Streaming | ❌ Not started | — |
| 5 Polish | ❌ Not started | — |
| 6 Cross-distro | ❌ Not started | — |
| 7 Release | ❌ Not started | — |

---

## Phase 0 — Research and feasibility

**Status: ✅ documentation complete; 🔌 hands-on cable validation deferred.**

Delivered: research into USB4/Thunderbolt networking, NetworkManager D-Bus,
Avahi discovery, Type-C/UPower, Sunshine/Moonlight, Fedora packaging
(`docs/NETWORKING.md`, `docs/POWER_DELIVERY.md`,
`docs/UPSTREAM_INTEGRATION.md`, `docs/PACKAGING.md`). Compatibility CLI
exists.

**Left:** Phase 0’s original “stream a desktop over a real cable” success
condition — needs two USB4/Thunderbolt machines (covered under Phase 4 +
hardware plan).

## Phase 1 — Secure project foundation

**Status: ✅ complete** (software + single-machine verification).

Delivered: Rust workspace, core state/error/config, platform abstraction,
Fedora backend, logging, GTK compatibility UI, agent/helper skeletons,
D-Bus XML, Polkit action IDs, `cabledeskctl`, RPM spec (not build-tested).

**Success condition (software):** met for the Fedora host tested. Not
verified on TB hardware or Tier 2/3 distros.

## Phase 2 — Automated networking

**Status: ✅ software complete for the cable-only orchestration slice;
🔌 Thunderbolt/two-machine success condition unmet.**

### Done (software)

- **Libraries:** NM `DirectLinkProfile` (`never-default`, link-local),
  hotplug `watch_direct_link_events`, address + FIB route validation,
  interface classification (`classify` / `sysfs`), peer policy
  (`PreparedDirectLink`, `ValidatedCablePeer`, `validate_cable_peer`),
  Avahi publish/browse/resolve scoped by ifindex, firewalld zone bind
  helpers.
- **Simulation:** `cabledesk-network` feature `simulation` +
  `cabledeskctl simulate` (dev builds only; not in production packages).
- **Agent:** on direct-link appear → `CableDetected` → `InspectingPower` →
  `ConfiguringLink` → helper `PrepareDirectLink` → Avahi publish/browse →
  `validate_cable_peer` → `PairingRequired`; on disappear → tear down →
  `WaitingForCable`. D-Bus: `GetState`, `GetSessionJson`.
- **Helper:** `PrepareDirectLink` with Polkit
  `org.cabledesk.helper.prepare-direct-link` + reject non-direct ifaces.
- **CLI:** `status`, `session`, `repair-network` (via helper).
- **UI:** Session group polls agent every 2s.

### Left (Phase 2)

- [ ] Two-machine discovery over a real USB4/Thunderbolt cable
      (**success condition** — deferred, see `HARDWARE_TEST_PLAN.md`)
- [ ] Smoke-test agent + helper + UI install path on a daily-driver machine
- [ ] Remaining helper methods / Polkit actions (`LoadThunderboltNetModule`,
      dedicated `InstallFirewallPolicy`, `RepairNetworkProfile`) — either
      implement or retire (OPEN_QUESTIONS §32)
- [ ] RPM/`rpmlint` build verification

**Explicitly out of scope for v1:** Wi-Fi/Ethernet/LAN fallback, manual IP,
DHCP-first `ipv4.link-local=fallback`, VPN/Tailscale routes.

**Success condition:** connecting the cable causes both CableDesk agents to
discover each other automatically — **not met** until hardware testing.

## Phase 3 — Secure pairing

**Status: ❌ not started.** Agent may enter `PairingRequired` after a
validated peer; no crypto, codes, or trusted-device store yet.

**Left:** device keys, provisional encrypted channel, verification code,
mutual trust, forget-device, pairing UI (`cabledesk-pairing`).

**Success condition:** pair once; authenticate on later cable connections.

## Phase 4 — Managed streaming

**Status: ❌ not started.**

**Left:** managed Sunshine, capture/encoder selection, Moonlight adapter,
validated cable endpoint only, auto launch/stop on cable events
(`cabledesk-streaming`).

**Success condition:** paired cable opens the host desktop.

## Phase 5 — Plug-and-play refinement

**Status: ❌ not started.**

**Left:** auto-open, notifications, presets, clipboard, charging UI,
suspend/logind, recovery, upgrades (`cabledesk-power`,
`cabledesk-diagnostics`).

## Phase 6 — Cross-distribution

**Status: ❌ not started** (must wait until Fedora is reliable end-to-end).

## Phase 7 — Public release

**Status: ❌ not started.**

---

## Immediate next task

1. **Software smoke-test:** run agent + UI + `cabledeskctl session` /
   `repair-network` on a Fedora machine (no TB required to see
   `WaitingForCable` / “No direct cable connection”).
2. **Then Phase 3 pairing** (software; no TB required for unit tests).
3. **When hardware exists:** execute `docs/HARDWARE_TEST_PLAN.md` before
   claiming Phase 2 success or any streaming behaviour.
