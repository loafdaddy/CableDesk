# CableDesk

> **Experimental project — not ready for daily use.**
> This repository currently contains a **read-only compatibility-detection
> foundation only**. Pairing, direct-link networking, and desktop streaming
> are **not implemented yet**. Nothing in this milestone opens a network
> connection, streams a desktop, or changes system configuration. See
> [docs/ROADMAP.md](docs/ROADMAP.md) for what exists today versus what's
> planned.

CableDesk connects two Linux computers over a direct USB4 or Thunderbolt
cable and aims to provide an ultra-low-latency, near-local desktop-control
experience — plug in a cable, and the laptop's desktop opens on the desktop
computer. It is a native GTK4/libadwaita automation and integration layer
around [Sunshine](https://github.com/LizardByte/Sunshine) (host capture and
encoding) and [Moonlight](https://github.com/moonlight-stream/moonlight-qt)
(client decoding and presentation), not a replacement for either.

CableDesk does not stream over the internet, does not use Wi-Fi as a
fallback, and is not a general-purpose remote-desktop tool.

## An ordinary USB-C cable and port are not enough

**USB-C alone does not mean USB4 or Thunderbolt.** A huge number of USB-C
ports and cables only carry USB 2.0/3.x data, charging, or DisplayPort —
none of which CableDesk can use for its direct networking link. Both
computers need a genuine USB4 or Thunderbolt controller, and the cable
itself needs to support USB4/Thunderbolt data (not just charging). CableDesk
will tell you honestly when this isn't the case instead of assuming
compatibility from the connector shape — see `cabledeskctl compatibility`
below.

## Current status (Phase 1 of `docs/ROADMAP.md`)

What exists right now:

- A Cargo workspace with the core crates, error model and connection state
  machine (`crates/cabledesk-core`).
- A distribution-independent `PlatformBackend` trait
  (`crates/cabledesk-platform`) and a first, **read-only** Fedora
  implementation (`crates/cabledesk-platform-fedora`) that reports:
  distribution, desktop environment, session type, USB4/Thunderbolt
  controller presence, `thunderbolt-net` module/interface status, SELinux
  and firewalld state, and USB-C Power Delivery/battery status.
- A minimal libadwaita GUI (`cabledesk`) and diagnostics CLI
  (`cabledeskctl`) that display this information.
- Skeleton D-Bus services for the user-session agent (`cabledesk-agent`)
  and privileged system helper (`cabledesk-helper`) — see
  `docs/ARCHITECTURE.md`. The helper performs **no privileged actions
  yet**.
- Packaging scaffolding (Fedora RPM spec, systemd units, Polkit action
  placeholders, D-Bus policy, firewalld zone) — none of it installs or
  activates automatically; see `scripts/dev-install.sh`.

What does **not** exist yet: device pairing, the direct-link network
profile, mDNS discovery, managed Sunshine/Moonlight runtimes, and the
desktop stream itself.

## Supported systems

Tier 1 (actively developed and tested): **Fedora Workstation, GNOME,
Wayland, x86_64, SELinux enforcing, firewalld enabled.** Other
distributions and desktop environments are planned (see
`docs/DISTRO_SUPPORT.md`) but not yet implemented or tested — do not assume
CableDesk works there.

## Getting started (development)

CableDesk is not packaged for end users yet. To build it from source:

```bash
git clone <repository-url> cabledesk
cd cabledesk
./scripts/dev-setup.sh      # checks/installs development dependencies (asks first)
./scripts/dev-build.sh      # cargo fmt --check, build, clippy, test
./scripts/dev-install.sh    # installs user-level files; asks before any root/system changes
```

Try the compatibility check:

```bash
cabledeskctl compatibility
```

or launch the (currently read-only) GUI:

```bash
cabledesk
```

To remove everything the scripts above installed:

```bash
./scripts/dev-uninstall.sh
```

See `scripts/dev-reset.sh` to clear build artifacts and local
configuration/state, and `scripts/build-rpm.sh` to build the Fedora RPM.

## Documentation

- [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) — component layout and data flow
- [docs/SECURITY.md](docs/SECURITY.md) / [docs/THREAT_MODEL.md](docs/THREAT_MODEL.md) — security model
- [docs/NETWORKING.md](docs/NETWORKING.md) — `thunderbolt-net`, NetworkManager, discovery research
- [docs/POWER_DELIVERY.md](docs/POWER_DELIVERY.md) — USB-C PD and charging research
- [docs/UPSTREAM_INTEGRATION.md](docs/UPSTREAM_INTEGRATION.md) — Sunshine/Moonlight integration research
- [docs/PACKAGING.md](docs/PACKAGING.md) — RPM packaging and hardening research
- [docs/DISTRO_SUPPORT.md](docs/DISTRO_SUPPORT.md) — supported-platform tiers
- [docs/TEST_PLAN.md](docs/TEST_PLAN.md) — testing strategy
- [docs/IMPLEMENTATION_CHECKLIST.md](docs/IMPLEMENTATION_CHECKLIST.md) — this milestone's checklist
- [docs/OPEN_QUESTIONS.md](docs/OPEN_QUESTIONS.md) — everything still unresolved
- [docs/ROADMAP.md](docs/ROADMAP.md) — phased plan
- [docs/adr/](docs/adr/) — architecture decision records

## License

GPL-3.0-or-later. See [LICENSE](LICENSE). CableDesk depends on and manages
Sunshine and Moonlight, both also GPL-licensed — see
`docs/UPSTREAM_INTEGRATION.md` for attribution and source-availability
obligations.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md).
