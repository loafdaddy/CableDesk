# CableDesk Roadmap

Phased plan. Each phase has a concrete success condition; do not start the
next phase until the current one's condition is actually met on real
hardware, not just "compiles."

## Phase 0 — Research and feasibility

**Status: mostly done as documentation, not yet as hands-on hardware
testing.**

Delivered this milestone: research into USB4/Thunderbolt networking,
NetworkManager D-Bus, Avahi discovery, Type-C/UPower power reporting,
Sunshine/Moonlight integration, and Fedora packaging/hardening (see
`docs/NETWORKING.md`, `docs/POWER_DELIVERY.md`,
`docs/UPSTREAM_INTEGRATION.md`, `docs/PACKAGING.md`). A hardware
compatibility CLI exists (`cabledeskctl compatibility`) and was tested on
real Fedora hardware.

**Not yet done:** actually building/running Sunshine and Moonlight-Qt,
pairing them, or streaming a desktop over a real cable between two
machines — this requires two USB4/Thunderbolt-capable machines, which this
milestone's environment did not have. Phase 0's original success
condition — "a Fedora laptop can be controlled from a Fedora desktop
through the direct cable" — is **not met yet**.

## Phase 1 — Secure project foundation

**Status: complete** (software + single-machine verification; no Thunderbolt
hardware on the development machine).

Delivered: Rust workspace, core state model, platform abstraction, Fedora
backend, structured logging (`tracing`/`tracing-journald`), basic GTK
application, user agent skeleton, privileged helper skeleton, D-Bus API
definitions, Polkit action placeholders, compatibility page (CLI + GUI),
basic `cabledeskctl`, RPM spec (not build-tested — see
`docs/OPEN_QUESTIONS.md`).

**Success condition:** "The installed application can accurately report
compatibility, cable state and charging state without manual
configuration." **Met for the machine tested** (see `docs/TEST_PLAN.md`) —
not yet verified on a machine that actually has Thunderbolt/USB4 hardware,
nor on any Tier 2/3 platform.

## Phase 2 — Automated networking

**Complete at the software/single-machine level.** Every deliverable in
the original plan (hotplug detection, the NetworkManager direct-link
profile, mDNS discovery, interface/route validation, firewalld
integration, cable-removal handling, `cabledeskctl repair-network`) now
has a real, working implementation:

- `cabledesk-network`: `DirectLinkProfile` (the NetworkManager
  link-local/never-default settings dictionary — pure, unit-tested);
  `NetworkManagerClient` (real `zbus` code for `Settings.AddConnection`,
  a merged `DeviceAdded`/`DeviceRemoved` signal stream, and
  `watch_direct_link_events` — the same stream filtered down to
  interfaces whose driver matches `thunderbolt-net`, caching the name at
  `Added` time so `Removed` doesn't need to re-query a possibly-already-gone
  D-Bus object); `address_belongs_to_interface` (real address-assignment
  validation, tested live against `lo`) and `route_resolves_via_interface`
  (a real kernel FIB lookup via `rtnetlink` — the same `RTM_GETROUTE`
  mechanism `ip route get` uses — tested live: 127.0.0.1 resolves via
  `lo`, a fabricated interface name doesn't).
- `cabledesk-agent`: now actually watches `watch_direct_link_events` and
  drives the shared state machine — `WaitingForCable` on startup,
  `CableDetected` when the interface appears, back to `WaitingForCable`
  the instant it disappears (never a "streaming over something else"
  state — see `docs/adr/ADR-007-direct-interface-only.md`). Verified live:
  started the agent, queried `GetState` over the real session bus, got
  `WaitingForCable` back.
- `cabledesk-discovery`: the `_cabledesk._tcp` `ServiceRecord`/TXT-record
  encoding (pure, unit-tested, including a test that the encoding never
  contains anything key/secret/token/password-shaped); `AvahiClient` for
  interface-scoped `publish`/`browse`/`resolve` (`ResolveService`'s exact
  signature confirmed against Avahi's own D-Bus interface XML upstream,
  not assumed) — exercised live end-to-end: publish a uniquely-named test
  service, browse for it, resolve it, confirm the resolved record matches
  what was published, withdraw it.
- `PlatformBackend::prepare_direct_link` and `install_firewall_policy`
  (in `cabledesk-platform-fedora`) are real implementations: the former
  creates the NetworkManager profile and binds the interface into the
  CableDesk firewalld zone (mutating — deliberately never invoked against
  a live system by this workspace's own tests, see `docs/TEST_PLAN.md`);
  the latter is read-only (just confirms the zone is loaded) and is
  exercised live.
- `cabledeskctl repair-network` detects the current direct-link interface
  and calls `prepare_direct_link` on it, or honestly reports "nothing to
  repair" if none is present (verified live — on this milestone's
  hardware, that's always the no-op path, which is itself a real,
  meaningful test: the code correctly never reaches the mutating call
  when there's nothing to act on). `cabledeskctl status` queries the
  agent's real `GetState` over D-Bus.

**Still not exercised against real Thunderbolt/USB4 hardware or a second
machine** — every "real, live" test above is real D-Bus/netlink code
running against this one machine's NetworkManager/Avahi/firewalld/routing
table, not an end-to-end two-machine test. That remains the actual
Phase 2 success condition below, and the honest state is: the plumbing
works, hardware validation hasn't happened. See `docs/TEST_PLAN.md`.

**Explicitly deferred out of v1** (revisit only as a deliberate future
addition, not a default): NetworkManager's DHCP-first/link-local-fallback
addressing mode (`ipv4.link-local=fallback`) — the direct link never has a
DHCP server, so v1 always uses plain `ipv4.method=link-local` — see
`docs/NETWORKING.md` §4 and `docs/adr/ADR-003-networkmanager-dbus.md`.
More generally, no network-fallback/auto-negotiation behavior beyond exact
direct-link detection is in scope for v1: if the direct interface isn't
there, CableDesk reports that honestly rather than trying alternate
addressing strategies to work around it.

**Success condition:** connecting the cable causes both CableDesk agents to
discover each other automatically.

## Phase 3 — Secure pairing

**Not started.** Deliverables: device keys (crypto library choice pending,
see `docs/OPEN_QUESTIONS.md`), encrypted provisional channel, matching
verification code, mutual trust, trusted-device database, forget-device
flow, pairing UI. Builds out the currently-empty `cabledesk-pairing` crate
and the non-secret shapes already in `cabledesk_core::device`.

**Success condition:** two devices can pair once and securely authenticate
on future cable connections.

## Phase 4 — Managed streaming

**Not started.** Deliverables: managed Sunshine service/configuration,
capture backend selection (KMS vs. XDG portal/PipeWire — see
`docs/UPSTREAM_INTEGRATION.md` §2 for the tradeoffs, not yet decided),
hardware encoder selection, native Moonlight runtime, the `StreamingClient`
adapter (CLI syntax already verified against current source — see
`docs/UPSTREAM_INTEGRATION.md` §7), automated Sunshine/Moonlight pairing,
desktop stream launch, keyboard/mouse/audio, stream termination. Builds out
`cabledesk-streaming`.

**Success condition:** connecting a paired cable automatically opens the
host desktop.

## Phase 5 — Plug-and-play refinement

**Not started.** Deliverables: auto-open setting, notifications, stream
presets, capability negotiation, text clipboard, charging UI, suspend
handling, logind inhibitor, better diagnostics, automatic service recovery,
package upgrades. Builds out `cabledesk-power` and
`cabledesk-diagnostics`.

**Success condition:** a normal Fedora user can install, pair once, and use
CableDesk afterward without terminal configuration.

## Phase 6 — Cross-distribution architecture validation

**Not started, and per the project plan must not start before Fedora is
reliable end-to-end** (i.e. not before Phase 5 is actually done). See
`docs/DISTRO_SUPPORT.md` for the Tier 2 target list and
`packaging/{debian,arch,opensuse}/README.md` for current (empty)
placeholders.

**Success condition:** Debian/Ubuntu and Arch packaging exist and are
integration-tested, with a documented platform capability matrix.

## Phase 7 — Public release

**Not started.** Deliverables: signed package repository, COPR, GitHub
Releases, reproducible builds (Fedora's build pipeline gets this mostly for
free per `docs/PACKAGING.md` §1.5, if CableDesk builds through it rather
than a custom pipeline), licence compliance, update process, a real
security reporting process (see `docs/SECURITY.md`), user documentation,
uninstall/purge documentation.

## Immediate next task recommendation

Phase 2's code is now feature-complete at the software level — hotplug
detection, NetworkManager profile creation, route validation, mDNS
discovery/resolve, firewalld integration, and `cabledeskctl
repair-network` all exist, compile, and run correctly against this
machine's real NetworkManager/Avahi/firewalld/routing table. The agent
watches direct-link hotplug and drives the state machine, but does **not
yet** call `cabledesk-discovery`'s `AvahiClient` to publish/browse peers
(that library is covered by its own live Avahi tests; wiring it into the
agent is the remaining software step before the Phase 2 success
condition can even be attempted). **None of that is the same as
validating Phase 2 end-to-end**, which needs two USB4/Thunderbolt-capable
machines connected by a real cable — this milestone's environment has
never had one. The highest-value next step is **hands-on hardware
validation**: get `thunderbolt-net` up between two real Linux machines,
confirm the interface-naming and security-level-authorization open
questions in `docs/NETWORKING.md` (items 1–2), watch whether
`cabledesk-agent`'s hotplug watcher actually transitions to
`CableDetected` for the real interface appearing, wire Avahi publish/
browse into the agent, and only then exercise
`prepare_direct_link`/`cabledeskctl repair-network`'s mutating path for
real (it has deliberately never been run against a live system so far —
see `docs/TEST_PLAN.md`). Doing a manual (non-CableDesk)
Sunshine↔Moonlight stream over that link to validate the capture-backend
choice in `docs/UPSTREAM_INTEGRATION.md` is still worth doing before
wiring up `cabledesk-streaming` in Phase 4.

If hardware isn't available yet, Phase 3 (pairing) can proceed in
parallel purely in software — it has no hardware dependency of its own.
