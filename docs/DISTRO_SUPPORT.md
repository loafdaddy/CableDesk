# CableDesk Distribution Support

CableDesk supports platforms in tiers, and a distro/desktop combination is
only ever claimed as supported after real integration testing on it — never
merely because the code compiles there. This milestone has real testing on
exactly one system (see below); everything else is a plan.

## Tier 1 — actively developed and tested

- Fedora Workstation, current supported releases
- GNOME, Wayland
- x86_64
- systemd, NetworkManager, PipeWire, firewalld, SELinux enforcing
- USB4 or Thunderbolt host-to-host networking
- Active logged-in graphical session

**Actual testing performed for this milestone:** the whole workspace
(`cabledesk-platform-fedora`, `cabledesk-network`, `cabledesk-discovery`,
`cabledesk-agent`, `cabledeskctl`) was built and its test suite run on a
real Fedora Linux 44 (Workstation Edition) machine — GNOME/Wayland,
SELinux enforcing, firewalld active, NetworkManager active, kernel
7.0.14. Every `CompatibilityCheck` in `cabledeskctl compatibility`'s
output, the NetworkManager/Avahi/firewalld D-Bus code, and the kernel
route lookups were exercised against that machine's real state, not
mocked. That machine has no Thunderbolt/USB4 controller, so the "no
controller detected" path (`CheckStatus::Unsupported`) and
`cabledeskctl repair-network`'s "nothing to repair" path are what's
actually been exercised — the "controller present" path, and everything
involving a second machine, has not been tested on real Thunderbolt/USB4
hardware yet. See `docs/TEST_PLAN.md`.

Implemented so far covers Phase 1 (read-only compatibility detection) and
Phase 2 (NetworkManager profile management, hotplug detection, mDNS
discovery, route validation, firewalld integration) at the software
level. Pairing and streaming are not implemented on any platform yet, and
Phase 2's networking code — despite compiling and passing its own tests —
has not been validated end-to-end on real Thunderbolt/USB4 hardware or
between two machines. See `docs/ROADMAP.md`.

## Tier 2 — planned, not yet implemented

- Ubuntu, Debian, Linux Mint, Pop!_OS
- Arch Linux, EndeavourOS
- openSUSE
- Nobara
- GNOME Wayland and KDE Plasma Wayland as desktop environments

Placeholder packaging directories exist at `packaging/debian/`,
`packaging/arch/`, `packaging/opensuse/` (each a README describing what's
still needed, not working packaging). No `PlatformBackend` implementation
exists for any of these yet — only `cabledesk-platform-fedora`. Per
`docs/ROADMAP.md`, Phase 6 (cross-distribution architecture validation)
does not begin until Fedora behaviour is reliable end-to-end.

## Tier 3 — later, specialised support

- Fedora Silverblue, Bazzite, Bluefin, Kinoite
- NixOS
- Other immutable/declarative distributions
- X11 sessions
- wlroots compositors (Hyprland, Sway)

Not started, not designed for yet. Each of these changes fundamental
assumptions this codebase currently makes (e.g. immutable distros need a
different install/`PlatformBackend` model than dnf-based Fedora; X11
changes input-forwarding and capture-backend assumptions throughout
`docs/UPSTREAM_INTEGRATION.md`).

## Why the tiering, concretely

`cabledesk-platform::PlatformBackend` (see `docs/ARCHITECTURE.md`) is the
entire distribution-independence boundary. Everything in
`cabledesk-platform-fedora` — package names, `dnf`/`rpm` assumptions,
`/sys/class/typec` layout (which is not Fedora-specific but was only tested
on this one machine), Fedora's specific D-Bus service names — must never
leak into `cabledesk-core`, `cabledesk-ui`, `cabledesk-agent`, or
`cabledesk-helper`'s dispatch logic. A `cabledesk-platform-debian` or
`cabledesk-platform-arch` crate implementing the same trait is how Tier 2
gets added later without rewriting the application (see
`docs/adr/ADR-006-platform-backend-abstraction.md`).
