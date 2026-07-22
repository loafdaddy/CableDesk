# CableDesk Test Plan

Status: describes the full target testing strategy. Only the "Unit tests"
section below reflects tests that actually exist in this milestone; the
rest is the plan those future tests need to fill in.

## Unit tests (implemented, this milestone)

All in `cargo test --workspace`, 20 tests passing as of this milestone:

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
- **Fedora backend integration** (`cabledesk-platform-fedora/src/lib.rs`, 2
  tests, `#[tokio::test]`, run against the real host — not mocked):
  `collect_diagnostics` succeeds and returns non-empty checks in every
  category on whatever machine runs the test suite; the two mutating
  methods (`prepare_direct_link`, `install_firewall_policy`) return an
  explicit error rather than `Ok(())`.

Run with: `cargo test --workspace` (see `scripts/dev-build.sh`).

## Mocked integration tests (not implemented yet)

Planned mocks: NetworkManager D-Bus, Avahi, UPower, Polkit, systemd,
Sunshine, Moonlight, udev, firewalld. Planned scenarios: cable insertion/
removal, interface appearance/removal, pairing (success and identity
mismatch), reconnection, untrusted-peer rejection, link-local address
collision, Sunshine/Moonlight startup failure, viewer crash, firewall
failure, power status unavailable, discharging-while-connected, suspend/
resume, upgrade/config migration.

None of this can be written yet because the code it would test
(`cabledesk-network`, `cabledesk-discovery`, `cabledesk-pairing`,
`cabledesk-streaming`) doesn't exist — these are empty placeholder crates
(see `docs/ARCHITECTURE.md`). Writing the mocks first, against an
interface that doesn't exist, would just be guessing at the interface
shape.

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

**Not performed, planned for later phases:** the full real-hardware matrix
from the project plan — AMD↔NVIDIA controller/host pairs, actual USB4/
Thunderbolt hardware on both ends, simultaneous Wi-Fi+Ethernet+direct-link,
1080p60/native-resolution/120fps streaming, H.264/HEVC/AV1, keyboard/mouse/
Alt+Tab forwarding, cable removal during an active stream, reboot,
suspend/resume, charging vs. discharging under a real stream, and
distinguishing "USB4 cable" vs. "charge-only cable" vs. "ordinary USB-C
cable" behaviour. None of this is possible to test yet because streaming
doesn't exist. Every item here needs two physical machines connected by a
real cable, which this milestone's environment did not have.

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
