# CableDesk Test Plan

Status: describes the full target testing strategy. Only the "Unit tests"
section below reflects tests that actually exist in this milestone; the
rest is the plan those future tests need to fill in.

## Unit tests (implemented, this milestone)

All in `cargo test --workspace`, plus optional simulation tests:

`cargo test --workspace` (default features) and
`cargo test -p cabledesk-network --features simulation`.

Counts evolve; re-run the commands rather than trusting a fixed number here.
As of 2026-07-23: core + network (with simulation) + discovery +
platform-fedora unit/live-safe tests all pass.

- **State transitions** (`cabledesk-core/src/state.rs`, 5 tests): starts
  `Unconfigured`; a full happy-path walk to `Streaming`; illegal jumps are
  rejected and leave state unchanged; every state can transition to
  `Error`; `Error` recovers back toward `WaitingForCable`; cable removal
  mid-stream returns to `WaitingForCable` rather than any "degraded
  streaming" state (enforces the "no silent Wi-Fi fallback" rule at the
  state-machine level).
- **Error mapping** (`cabledesk-core/src/error.rs`, 2 tests): every
  `CableDeskError` variant maps to a non-empty user-facing headline; a
  `Config` error containing a secret-shaped string does not leak it into
  the user-facing headline (checks the *mechanism*, not full secret
  redaction — see `docs/OPEN_QUESTIONS.md`).
- **Device identity types** (`cabledesk-core/src/device.rs`, 3 tests):
  `DeviceId` hex display, `PairingCode` digit-range validation and display.
- **Config round-trip and migration** (`cabledesk-core/src/config.rs`, 3
  tests): JSON round-trip, rejection of a future schema version, forward
  migration of the current version.
- **os-release parsing** (`cabledesk-platform-fedora/src/system.rs`, 2
  tests): quoted/unquoted values, blank/comment lines ignored.
- **Dependency path scanning** (`cabledesk-platform-fedora/src/dependencies.rs`,
  2 tests): a known-present binary (`sh`) is found; a fabricated binary
  name is not.
- **Fedora backend integration** (`cabledesk-platform-fedora/src/lib.rs`
  and `src/firewall.rs`, 3 tests, `#[tokio::test]`, run against the real
  host — not mocked): `collect_diagnostics` succeeds and returns
  non-empty checks in every category; `install_firewall_policy` (read-only
  — just checks the zone is loaded) reports the CableDesk zone honestly as
  not-installed on a dev machine that hasn't run the system-level part of
  `dev-install.sh`; `firewall::can_query_installed_zones` confirms the
  firewalld D-Bus query itself succeeds. **`prepare_direct_link` has no
  automated test at all** — it is real, mutating code (creates a
  persistent NetworkManager profile, binds a firewalld zone) and this
  workspace's test suite must never create real system state as a side
  effect of `cargo test` — see "Live-but-safe Phase 2 tests" below for
  what *is* exercised from that code path.
- **`DirectLinkProfile` settings construction** (`cabledesk-network/src/
  profile.rs`, 5 tests): the NetworkManager settings dictionary pins the
  exact interface name given, sets `ipv4.never-default=true`, uses plain
  `ipv4.method=link-local` (and never sets the separate `ipv4.link-local`
  fallback property at all — see `docs/adr/ADR-003-networkmanager-dbus.md`),
  sets `connection.autoconnect=true`, and the connection ID embeds the
  interface name for debuggability.
- **Interface address and route validation** (`cabledesk-network/src/
  validate.rs`, 6 tests, real — not mocked): `127.0.0.1` is confirmed to
  belong to `lo`; a fabricated interface name and an address `lo` doesn't
  have are both correctly rejected; a real kernel FIB lookup (via
  `rtnetlink`, the same `RTM_GETROUTE` mechanism `ip route get` uses)
  confirms `127.0.0.1`'s route actually resolves via `lo`, and correctly
  rejects a fabricated interface name.
- **`_cabledesk._tcp` TXT record encoding** (`cabledesk-discovery/src/
  service.rs`, 4 tests): round-trips through encode/decode; a record
  missing a required field is rejected; an unknown key from a
  hypothetically newer CableDesk version is ignored rather than failing
  (forward compatibility); and — directly enforcing the "no secrets in
  mDNS" engineering rule — the encoded TXT records are asserted to never
  contain anything key/secret/token/password-shaped.

Run with: `cargo test --workspace` (see `scripts/dev-build.sh`).

## Simulation / cable-only lifecycle (development-only)

Enabled only with Cargo feature `simulation` (never in production packages).
Production binaries and the privileged helper must not expose sim events.

```bash
cargo test -p cabledesk-network --features simulation
cargo run -p cabledeskctl --features simulation -- simulate demo
cargo run -p cabledeskctl --features simulation -- simulate peer-on-wifi
```

Covered by unit tests in `cabledesk-network/src/{simulation,classify,peer,sysfs}.rs`
and agent state-bridge tests:

- WaitingForCable → mock USB4 cable → direct iface → peer on cable →
  mock stream → cable removed → WaitingForCable
- Peer discovered on Wi-Fi or ordinary Ethernet is rejected
- No direct interface → “No direct cable connection”
- Cable loss from mid-orchestration states returns to WaitingForCable

## Agent / helper / UI (software, no TB required)

- Agent unit tests: force-waiting bridge; Wi-Fi policy rejection
- Manual: `cargo run -p cabledesk-agent` then `cabledeskctl session`
  (expects `WaitingForCable` without a thunderbolt-net iface)
- Helper `PrepareDirectLink` is exercised only when a direct iface exists
  and Polkit authorizes (not automated in `cargo test`)

## Live-but-safe Phase 2 tests

Several tests and manual checks in this milestone deliberately go beyond
mocks and run real D-Bus/netlink calls (or start real binaries) against
whatever machine executes them, because doing so is safe and reversible:

- `cabledesk-network::manager::can_connect_and_subscribe_to_device_events`
  and `can_subscribe_to_direct_link_events` — connect to the real system
  bus and subscribe to NetworkManager's `DeviceAdded`/`DeviceRemoved`
  signals (raw and direct-link-filtered). Read-only.
- `cabledesk-network::validate::loopback_route_resolves_via_lo` and
  `loopback_route_does_not_resolve_via_a_fabricated_interface` — a real
  kernel FIB lookup via `rtnetlink`. Read-only.
- `cabledesk-discovery::avahi::publish_browse_resolve_then_withdraw` —
  publishes a uniquely-named `_cabledesk._tcp` test service to the real
  `avahi-daemon`, browses for it, **resolves it, and asserts the resolved
  record matches what was published**, then withdraws it. Mutating but
  temporary and fully reversible (standard practice for anything using
  Avahi); does not hard-fail if mDNS propagation doesn't complete within
  5 seconds, since that timing isn't guaranteed in every environment.
- `cabledesk-platform-fedora::firewall::can_query_installed_zones` and
  `tests::install_firewall_policy_reports_zone_not_installed_honestly` —
  query firewalld's currently-loaded zones. Read-only.
- **Manual, not part of `cargo test` (see `docs/ROADMAP.md` Phase 2 for
  the exact commands run):** started `cabledesk-agent` and confirmed
  `GetState` over the real session bus returns `WaitingForCable` (real
  hotplug watcher live, no synthetic state); ran `cabledeskctl
  repair-network` and confirmed it correctly detects no direct-link
  interface and reports "nothing to repair" without ever reaching the
  mutating call; ran `cabledeskctl status` both with and without the
  agent running, getting the correct real state or a clear failure
  message in each case.

**Never made live, in tests or manually, in this milestone:** any call
that would create persistent state — `NetworkManagerClient::apply_profile`
(a real, persistent NetworkManager connection profile) and
`FirewallClient::bind_interface` (a real firewalld zone binding). Both are
real, compiled, working code, reachable via `FedoraBackend::
prepare_direct_link` and `cabledeskctl repair-network`'s mutating branch
— but that branch has literally never executed in this milestone's
environment, since it has no direct-link interface to trigger it. This
is a deliberate scope decision (not an accident of missing hardware
alone) to avoid mutating a development machine's real network
configuration — see `docs/ROADMAP.md` Phase 2 and
`docs/OPEN_QUESTIONS.md`.

## Mocked integration tests (not implemented yet)

Planned mocks: NetworkManager D-Bus, Avahi, UPower, Polkit, systemd,
Sunshine, Moonlight, udev, firewalld. Planned scenarios: cable insertion/
removal, interface appearance/removal, pairing (success and identity
mismatch), reconnection, untrusted-peer rejection, link-local address
collision, Sunshine/Moonlight startup failure, viewer crash, firewall
failure, power status unavailable, discharging-while-connected, suspend/
resume, upgrade/config migration.

`cabledesk-network` and `cabledesk-discovery` now exist (Phase 2), but
their tests so far are either pure-logic (settings construction, TXT
encoding) or live-but-safe (see above) — not mocked D-Bus. A proper mock
of NetworkManager/Avahi/firewalld would let CableDesk test scenarios that
aren't safe or possible to trigger live (identity mismatch, address
collision, Sunshine/Moonlight startup failure) without touching real
system state; it just hasn't been built yet. `cabledesk-pairing` and
`cabledesk-streaming` remain empty placeholder crates (see
`docs/ARCHITECTURE.md`) — writing their mocks first, against an interface
that doesn't exist, would just be guessing at the interface shape.

## Real hardware tests (not performed yet, beyond what's noted below)

**Performed this milestone:** `cabledesk-platform-fedora`'s full check
suite (`cabledeskctl compatibility`) was run on one real Fedora 44
Workstation machine (see `docs/DISTRO_SUPPORT.md` for exact configuration)
— confirmed accurate for: distro/DE/session detection, NetworkManager/
Avahi/firewalld/UPower/Polkit D-Bus reachability, GTK4/libadwaita presence,
SELinux enforcing detection, USB-C PD port/battery/external-power reporting
(real values: 48% charging, PD 2.0 on both Type-C ports), and — importantly
— the "no Thunderbolt/USB4 controller present" path, since that machine
doesn't have one. The GTK4/libadwaita `cabledesk` binary was launched on
that machine's real GNOME/Wayland session and stayed running without
crashing (visual confirmation was not possible — no screenshot tool was
available in the environment this was built in). `cabledesk-agent` was
started and its `GetState`/`Version` D-Bus methods were queried live over
the session bus successfully. `cabledesk-helper` was started and correctly
failed to acquire its system-bus name until the D-Bus policy file was
added — see `docs/SECURITY.md`.

**Not performed, planned for later phases:** see
`docs/HARDWARE_TEST_PLAN.md` (all items deferred — not failed). The full
real-hardware matrix from the project plan — AMD↔NVIDIA controller/host
pairs, actual USB4/Thunderbolt hardware on both ends, simultaneous
Wi-Fi+Ethernet+direct-link, streaming quality/latency, input forwarding,
cable removal during an active stream, reboot, suspend/resume, charging
under load, and distinguishing USB4 vs charge-only vs ordinary USB-C
cables — remains deferred. None of this is claimed working.

## Cross-distribution CI (not implemented yet)

Planned: Fedora current stable + Fedora next in CI first; Ubuntu LTS,
Debian stable, Arch container, openSUSE Tumbleweed later (Phase 6). No CI
configuration exists in this repository yet. Container CI can validate
compilation and unit tests on multiple distros but, per the project plan,
cannot replace real USB4/Thunderbolt hardware testing — containers don't
have a Thunderbolt controller to detect.

## Verifying "direct interface only" (not implemented yet)

Once streaming exists, packet routing must be verified with traffic
capture and route inspection to confirm the stream uses only the direct
interface — see `docs/THREAT_MODEL.md` T3. This is a hard requirement, not
a nice-to-have, and should become an automated check (e.g. a test that
asserts the active stream's socket is bound to the direct interface's
address) rather than a manual one where possible.
