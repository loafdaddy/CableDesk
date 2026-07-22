# CableDesk.

<p align="center">
  <img src="data/brand/cabledesk-lockup.svg" alt="CableDesk." width="520"/>
</p>

<p align="center">
  <strong>Ultra-low-latency desktop control over a USB4 / Thunderbolt cable</strong><br/>
  GTK4 · libadwaita · Rust · Sunshine · Moonlight · Fedora Tier 1
</p>

<p align="center">
  <a href="LICENSE"><img src="https://img.shields.io/badge/License-GPL--3.0--or--later-E8A45C" alt="GPL-3.0-or-later"/></a>
  <a href="https://www.rust-lang.org/"><img src="https://img.shields.io/badge/Rust-1.82+-000000?logo=rust&logoColor=white" alt="Rust"/></a>
  <a href="https://www.gtk.org/"><img src="https://img.shields.io/badge/GTK-4-7C3AED?logo=gtk&logoColor=white" alt="GTK4"/></a>
  <a href="docs/ROADMAP.md"><img src="https://img.shields.io/badge/status-Phase%201%20foundation-2A1C10" alt="Phase 1 foundation"/></a>
</p>

<p align="center">
  <a href="docs/ROADMAP.md">Roadmap</a>
  ·
  <a href="docs/ARCHITECTURE.md">Architecture</a>
  ·
  <a href="CONTRIBUTING.md">Contributing</a>
  ·
  <a href="docs/OPEN_QUESTIONS.md">Open questions</a>
  ·
  <a href="data/brand/README.md">Brand</a>
</p>

> **Experimental — not ready for daily use.**
> This repository currently contains a **read-only compatibility-detection
> foundation only**. Pairing, direct-link networking, and desktop streaming
> are **not implemented yet**. Nothing in this milestone opens a network
> connection, streams a desktop, or changes system configuration. See
> [docs/ROADMAP.md](docs/ROADMAP.md) for what exists today versus what's
> planned.

CableDesk connects two Linux computers over a direct USB4 or Thunderbolt
cable and aims to provide a near-local desktop-control experience — plug in
a cable, and the laptop's desktop opens on the desktop computer. It is a
native GTK4/libadwaita automation layer around
[Sunshine](https://github.com/LizardByte/Sunshine) (host capture/encoding)
and [Moonlight](https://github.com/moonlight-stream/moonlight-qt) (client
decode/presentation), not a replacement for either.

CableDesk does not stream over the internet, does not use Wi-Fi as a
fallback, and is not a general-purpose remote-desktop tool.

## Why CableDesk?

Most remote-desktop tools assume the network already exists. CableDesk starts
from the cable: detect real USB4/Thunderbolt hardware honestly, prepare a
dedicated direct link, then hand capture and presentation to Sunshine and
Moonlight. Built to feel like it could ship with Fedora Workstation —
Wayland-first, Polkit/SELinux-aware, no Electron.

> An ordinary USB-C cable and port are **not** enough. USB-C alone does not
> mean USB4 or Thunderbolt. CableDesk will tell you when the hardware isn't
> compatible instead of assuming it from the connector shape.

## Current status (Phase 1)

What exists right now:

- Cargo workspace with core types, error model, and connection state machine
- Distro-independent `PlatformBackend` + read-only Fedora backend
  (controllers, `thunderbolt-net`, SELinux/firewalld, USB-C PD/battery)
- Minimal libadwaita GUI (`cabledesk`) and diagnostics CLI (`cabledeskctl`)
- Skeleton D-Bus agent/helper — **no privileged actions yet**
- Packaging scaffolding (RPM spec, systemd, Polkit, firewalld zone)

What does **not** exist yet: pairing, direct-link networking, mDNS discovery,
managed Sunshine/Moonlight runtimes, or the desktop stream itself.

## Supported systems

**Tier 1** (actively developed): **Fedora Workstation, GNOME, Wayland,
x86_64, SELinux enforcing, firewalld enabled.**

Other distributions are planned — see [docs/DISTRO_SUPPORT.md](docs/DISTRO_SUPPORT.md).
Do not assume CableDesk works there yet.

## Getting started (development)

CableDesk is not packaged for end users yet.

```bash
git clone https://github.com/loafdaddy/CableDesk.git
cd CableDesk
./scripts/dev-setup.sh      # checks/installs development dependencies (asks first)
./scripts/dev-build.sh      # cargo fmt --check, build, clippy, test
./scripts/dev-install.sh    # installs user-level files; asks before any root/system changes
```

```bash
cabledeskctl compatibility   # hardware/compatibility report
cabledesk                    # read-only GTK UI
./scripts/dev-uninstall.sh   # remove what the scripts installed
```

## Documentation

| Doc | What it covers |
|-----|----------------|
| [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) | Component layout and data flow |
| [docs/SECURITY.md](docs/SECURITY.md) / [docs/THREAT_MODEL.md](docs/THREAT_MODEL.md) | Security model |
| [docs/NETWORKING.md](docs/NETWORKING.md) | `thunderbolt-net`, NetworkManager, discovery |
| [docs/POWER_DELIVERY.md](docs/POWER_DELIVERY.md) | USB-C PD and charging research |
| [docs/UPSTREAM_INTEGRATION.md](docs/UPSTREAM_INTEGRATION.md) | Sunshine / Moonlight integration |
| [docs/PACKAGING.md](docs/PACKAGING.md) | RPM packaging and hardening |
| [docs/ROADMAP.md](docs/ROADMAP.md) | Phased plan |
| [docs/OPEN_QUESTIONS.md](docs/OPEN_QUESTIONS.md) | Unresolved upstream questions |
| [docs/adr/](docs/adr/) | Architecture decision records |
| [data/brand/README.md](data/brand/README.md) | Lockup, mark, palette |

## License

GPL-3.0-or-later. See [LICENSE](LICENSE). CableDesk depends on and manages
Sunshine and Moonlight, both also GPL-licensed — see
[docs/UPSTREAM_INTEGRATION.md](docs/UPSTREAM_INTEGRATION.md).

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md). Contributors are welcome — research,
Rust, GTK, packaging, docs, and hardware testing all help.
