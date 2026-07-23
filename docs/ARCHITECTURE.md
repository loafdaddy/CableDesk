# CableDesk Architecture

Status: describes the architecture as implemented through Phase 2 of
`docs/ROADMAP.md` (Phase 1 read-only compatibility detection, plus Phase 2
NetworkManager/Avahi/firewalld D-Bus code) plus the target architecture
it is built toward. Where something is not implemented yet, that is
called out explicitly.

## Component overview

```text
┌──────────────── Controller computer ────────────────┐
│ cabledesk (GTK4/libadwaita UI)                       │
│ cabledesk-agent (user-session D-Bus service)         │
│ Managed native Moonlight runtime      [not built yet]│
│ Hardware video decoding               [not built yet]│
└───────────────────────┬─────────────────────────────┘
                        │
     USB4/Thunderbolt networking [code exists, unverified on real hardware]
                        │
┌───────────────────────┴─────────────────────────────┐
│ Host computer                                        │
│ cabledesk-agent (user-session D-Bus service)         │
│ cabledesk-helper (privileged system service)         │
│ Managed native Sunshine runtime        [not built yet]│
│ Hardware video encoding, capture, input [not built yet]│
└──────────────────────────────────────────────────────┘
```

The same package is installed on both machines; role (Controller/Host/Both)
is a per-user configuration choice (`cabledesk_core::config::Role`), not a
build-time distinction.

## Processes and privilege separation

Three separate binaries, matching `docs/SECURITY.md`'s privilege-separation
requirement — the GTK UI never runs as root, and nothing that needs root
runs more code than it has to:

| Binary | Runs as | Bus | Status this milestone |
|---|---|---|---|
| `cabledesk` (`crates/cabledesk-ui`) | logged-in user | session client of `Agent1` | Compatibility via in-process `PlatformBackend`; Session panel polls `GetSessionJson` every 2s |
| `cabledeskctl` (`crates/cabledeskctl`) | logged-in user | session + system (client) | `compatibility`, `diagnostics`, `power`, `links`, `status`, `session`, `repair-network` (helper); simulation behind Cargo feature; other subcommands stubbed |
| `cabledesk-agent` (`crates/cabledesk-agent`) | logged-in user, `systemd --user` | session, `org.cabledesk.Agent1` | Hotplug → helper prepare → Avahi → `validate_cable_peer` → `PairingRequired`; cable loss → `WaitingForCable`. D-Bus: `GetState`, `GetSessionJson` |
| `cabledesk-helper` (`crates/cabledesk-helper`) | root, system `systemd` service | system, `org.cabledesk.Helper1` | `Version`, `Ping`, `CollectDiagnosticsJson`; Polkit-gated `PrepareDirectLink` (rejects non-direct ifaces, then NM + firewalld bind) |

The GTK UI already reads live agent session state over D-Bus. Compatibility
checks still call `PlatformBackend` in-process (cheap, no agent required).

## Shared crates

- **`cabledesk-core`**: distribution-independent shared types used by every
  other crate.
  - `state::{ConnectionState, StateMachine}` — the single source of truth
    for connection progress (see `docs/adr` and engineering rule "no
    scattered boolean flags"). `StateMachine` validates every transition
    against an explicit allow-list and rejects illegal jumps instead of
    silently accepting them (`cabledesk-core/src/state.rs`, unit-tested).
  - `error::{CableDeskError, UserFacingError}` — internal errors vs. the
    sanitised headline/detail pairs the UI is allowed to render (engineering
    rule "never display raw Rust debug output as the main error").
  - `config::{DeviceConfig, Role}` — per-user configuration with an explicit
    `schema_version` field and a `migrate()` path, so future format changes
    don't break existing installs.
  - `device::{DeviceId, PairingCode, TrustedDevice}` — non-secret identity
    shapes; actual key material lives in `cabledesk-pairing` once that
    crate is implemented (Phase 3).
- **`cabledesk-platform`**: the `PlatformBackend` async trait and the report
  types (`SystemInfo`, `DependencyReport`, `SecurityReport`,
  `PowerDeliveryStatus`, `PlatformDiagnostics`, `CompatibilityCheck` with its
  `CheckStatus` vocabulary: `Available` / `NeedsSetup` / `Warning` /
  `Unsupported` / `Unknown`). This is the entire distribution-independence
  boundary — at least 90% of CableDesk should never import
  `cabledesk-platform-fedora` directly.
- **`cabledesk-platform-fedora`**: the first (and only, so far) concrete
  backend. Most inspect methods are read-only; mutating Phase 2 methods
  are called out below:
  - `system.rs` — parses `/etc/os-release`, `$XDG_CURRENT_DESKTOP`,
    `$XDG_SESSION_TYPE`, `/proc/sys/kernel/{osrelease,hostname}`.
  - `dependencies.rs` — NetworkManager/Avahi/firewalld/UPower/Polkit
    reachability via `zbus` system-bus `NameHasOwner` calls (not
    `systemctl is-active` shell-outs, per engineering rule "prefer typed
    D-Bus APIs"); GTK4/libadwaita presence via a linker-search-path scan
    (deliberately not `dlopen`, to avoid loading GTK into a non-GUI
    process); Sunshine/Moonlight/PipeWire presence via `$PATH`/socket
    checks.
  - `security.rs` — SELinux mode from `/sys/fs/selinux/enforce`, firewalld
    active state via D-Bus.
  - `power.rs` — USB-C PD and battery status from `/sys/class/typec/*` and
    `/sys/class/power_supply/*` (see `docs/POWER_DELIVERY.md`).
  - `direct_link.rs` — USB4/Thunderbolt controller, `thunderbolt-net`
    module, and direct network interface detection from
    `/sys/bus/thunderbolt/`, `/proc/modules`, and `/sys/class/net/*/device/
    driver` — **never hardcodes `thunderbolt0`** (engineering rule), instead
    matching on driver name.
  - `firewall.rs` — a firewalld D-Bus client scoped to the non-deprecated
    `.zone` interface (see `docs/PACKAGING.md`'s firewalld research).
  - `prepare_direct_link` and `install_firewall_policy` are now real
    (Phase 2): the former creates the NetworkManager profile (via
    `cabledesk-network`) and binds the interface into the CableDesk
    firewalld zone — mutating, and **deliberately never invoked against a
    live system by this workspace's own tests** (see `docs/TEST_PLAN.md`).
    The latter just confirms the zone is loaded (read-only) and is
    exercised live.
- **`cabledesk-network`** (Phase 2): `profile::DirectLinkProfile` builds the
  NetworkManager link-local/never-default settings dictionary (pure,
  unit-tested); `manager::NetworkManagerClient` wraps `zbus` for
  `Settings.AddConnection`, a merged `DeviceAdded`/`DeviceRemoved` signal
  stream, and `watch_direct_link_events` (the same stream filtered to
  `thunderbolt-net`-driver devices, with an interface-name cache so a
  `Removed` event doesn't need to re-query a possibly-already-gone D-Bus
  object — this is what `cabledesk-agent` drives its state machine from);
  `validate::address_belongs_to_interface` (address-assignment check) and
  `validate::route_resolves_via_interface` (a real kernel FIB lookup via
  `rtnetlink`, the same `RTM_GETROUTE` mechanism `ip route get` uses) —
  together, the full `docs/adr/ADR-007-direct-interface-only.md` check.
- **`cabledesk-discovery`** (Phase 2): `service::ServiceRecord` is the
  `_cabledesk._tcp` record shape and its TXT-record encoding (pure,
  unit-tested — including a test that the encoding never contains
  anything key/secret/token/password-shaped, per "no secrets in mDNS");
  `avahi::AvahiClient` wraps `zbus` for interface-scoped
  `publish`/`browse`/`resolve` against `org.freedesktop.Avahi.Server`
  (`ResolveService`'s signature was confirmed against Avahi's own D-Bus
  interface XML upstream, not assumed).
- **`cabledesk-pairing`, `cabledesk-streaming`, `cabledesk-power`,
  `cabledesk-diagnostics`**: still empty placeholder crates; see each
  crate's doc comment for which `docs/ROADMAP.md` phase builds it out.

## D-Bus surface

Both interfaces are versioned (`org.cabledesk.Agent1`, `org.cabledesk.
Helper1`) so a future incompatible change ships as `...Agent2`/`...Helper2`
rather than silently breaking older clients. Full interface definitions
(including the not-yet-implemented methods, kept as XML comments so
introspection never advertises a call that doesn't exist) live in
`data/dbus-1/interfaces/`.

Getting `cabledesk-helper` to even start required an explicit system D-Bus
policy file (`data/dbus-1/system.d/org.cabledesk.Helper1.conf`) — confirmed
by hands-on testing during this milestone: without it, the default system
bus policy refuses `RequestName` for the service with `AccessDenied`, even
though the systemd unit runs as root. This is a bus-level "may own this
name" grant only; it is not itself an authorization mechanism (see
`docs/SECURITY.md`).

## Data flow (this milestone)

```text
cabledeskctl compatibility  ─┐
cabledesk (GTK4 UI)         ─┼─▶ FedoraBackend::collect_diagnostics()
                             │      (crates/cabledesk-platform-fedora)
                             │        ├─▶ system::detect_system()        (sysfs/env reads)
                             │        ├─▶ dependencies::check_dependencies() (zbus + fs scans)
                             │        ├─▶ security::inspect_security_system() (sysfs + zbus)
                             │        ├─▶ power::inspect_power_delivery()  (sysfs reads)
                             │        └─▶ direct_link::inspect_direct_link() (sysfs reads)
                             └─▶ PlatformDiagnostics (serde, JSON-serializable)
```

The GTK UI runs this on a background thread with its own Tokio runtime
(GTK's main loop doesn't drive `tokio::fs`/`zbus` futures) and hands the
result back via an `async-channel`, since GTK widgets are not `Send` — see
`crates/cabledesk-ui/src/main.rs` for the full rationale in comments.

## Cable-only invariants

CableDesk is a **strictly cable-only** product. The only supported transport is
a direct USB4/Thunderbolt cable between two Linux machines. Wi-Fi, ordinary
Ethernet, router LAN, VPN, Tailscale/ZeroTier, manual IP entry, and any
silent fallback are **out of product scope**.

| # | Invariant |
|---|-----------|
| 1 | No session begins without an active USB4/Thunderbolt-derived interface |
| 2 | Peer endpoint must belong to that direct interface |
| 3 | Kernel route to the peer must resolve through that interface |
| 4 | Direct interface must be associated with USB4/Thunderbolt driver evidence (never name alone) |
| 5 | Discovery must be scoped to the direct interface |
| 6 | Control channel must bind to the direct link |
| 7 | Managed Sunshine must be inaccessible from unrelated interfaces |
| 8 | Moonlight receives only a validated cable endpoint |
| 9 | Cable removal invalidates the session immediately |
| 10 | No retry through Wi-Fi, Ethernet, or another route |
| 11 | A previously paired device on normal LAN is rejected |
| 12 | Manual IP addresses are not accepted by the normal application |

Software enforcement today: `cabledesk-network::{classify, peer, validate,
sysfs}`, agent Avahi scoped to the direct ifindex + `validate_cable_peer`,
agent cable-loss → `WaitingForCable`, helper rejects non-direct ifaces,
NM `never-default` profile. Simulation: `cabledesk-network` feature
`simulation`. Hardware verification: deferred (`docs/HARDWARE_TEST_PLAN.md`).

## What is explicitly *not* built yet

Pairing crypto/UI, managed Sunshine/Moonlight, the streaming session,
clipboard sync, and suspend/logind integration are not built. Phase 2
software orchestration (hotplug → prepare → Avahi → peer validation →
`PairingRequired`) is implemented and unit/simulation-tested, but **has
not** been exercised against real Thunderbolt/USB4 hardware or a second
machine (see `docs/TEST_PLAN.md`, `docs/CURRENT_STATUS.md`,
`docs/HARDWARE_TEST_PLAN.md`).
