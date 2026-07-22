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

**Status: this milestone.**

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

**Not started.** Deliverables: hotplug detection, the NetworkManager
direct-link profile (`ipv4.method=link-local` + `ipv4.never-default`, per
`docs/NETWORKING.md`), mDNS discovery scoped to the direct interface,
interface/route validation, firewalld integration, cable-removal handling,
`cabledeskctl repair-network`. This is where
`PlatformBackend::prepare_direct_link` and `install_firewall_policy` get
real implementations (they currently return an explicit "not implemented"
error).

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

Phase 2 cannot be meaningfully tested without two USB4/Thunderbolt-capable
machines connected by a real cable. Before writing Phase 2 networking code,
the highest-value next step is **hands-on Phase 0 hardware validation**:
get `thunderbolt-net` up between two real Linux machines, confirm the
interface-naming and security-level-authorization open questions in
`docs/NETWORKING.md` (items 1–2), and do a manual (non-CableDesk) Sunshine↔
Moonlight stream over that link to validate the capture-backend choice in
`docs/UPSTREAM_INTEGRATION.md` before automating any of it. Writing
`cabledesk-network`/`cabledesk-discovery` against unverified assumptions
about interface-appearance timing risks building the wrong thing.
