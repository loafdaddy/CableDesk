# CableDesk Networking Research

This document covers the transport and discovery layer research for CableDesk:
how a direct USB4/Thunderbolt cable between two Linux hosts becomes an IP link
that Sunshine/Moonlight can use, how CableDesk should manage that link via
NetworkManager, and how service discovery should be scoped to it.

Research was done against upstream kernel documentation, the NetworkManager
and Avahi D-Bus API references, ArchWiki/Fedora community docs, and
crates.io/docs.rs for the Rust D-Bus ecosystem. Every claim below is cited;
anything that could not be verified against a primary source is called out
in **Open Questions** instead of asserted as fact.

---

## 1. `thunderbolt-net` / USB4NET

### What it is

The Linux kernel has built-in support for tunneling network traffic between
two Thunderbolt/USB4-connected hosts. The kernel's own documentation refers
to this feature as "Networking over Thunderbolt cable" and, in recent
kernel docs, explicitly aliases it as **USB4NET**. It implements Apple's
**ThunderboltIP** protocol, which is also what Windows and macOS use, so a
Linux host can network over Thunderbolt with a Windows or Mac host as well as
another Linux host. [Thunderbolt admin-guide docs (kernel.org)](https://docs.kernel.org/admin-guide/thunderbolt.html), [thunderbolt.rst source](https://www.kernel.org/doc/Documentation/admin-guide/thunderbolt.rst)

Under the hood, the connection is negotiated using the Thunderbolt
**XDomain discovery protocol**, which runs automatically over the control
channel once two Thunderbolt-capable hosts are cabled together; XDomain
properties advertise which services (e.g. networking) each side supports,
and once negotiated, high-speed DMA rings in the host controller are used to
carry the actual traffic. [LWN: Thunderbolt networking](https://lwn.net/Articles/735235/), [XDomain discovery patch](https://lkml.iu.edu/hypermail/linux/kernel/1709.2/01069.html)

### Module name and loading behavior

The kernel module is documented as `thunderbolt-net` (loaded via
`modprobe thunderbolt-net`; the compiled module/source is `drivers/thunderbolt/net.c`,
built into the `thunderbolt` driver family — module aliasing means
`thunderbolt-net` and `thunderbolt_net` are interchangeable at the modprobe
command line). Per the kernel doc:

- If the module is **built into the kernel image**, nothing needs to be done — it activates automatically as soon as XDomain discovery completes.
- If it is a **loadable module** and the *other* host is Windows or macOS, the module loads automatically when the cable is connected.
- If the *other* host is also Linux, you need to `modprobe thunderbolt-net` manually on **one** of the two hosts; doing so triggers module load on the peer automatically (it does not matter which side loads it first).

[Thunderbolt admin-guide docs](https://docs.kernel.org/admin-guide/thunderbolt.html)

**CableDesk implication:** most current distro kernels (Fedora included) ship
`thunderbolt-net` as a loadable module, not built-in. CableDesk's host-side
setup step should explicitly `modprobe thunderbolt-net` (idempotent) rather
than assume auto-load, since a Linux-to-Linux pairing requires at least one
side to trigger it.

### Interface naming

The driver "will create one virtual ethernet interface per Thunderbolt port
which are named like `thunderbolt0` and so on" — i.e. one interface per
physical Thunderbolt/USB4 port on the host controller, not one global
interface. [thunderbolt.rst](https://www.kernel.org/doc/Documentation/admin-guide/thunderbolt.rst)

This is a kernel-driver-assigned name (`thunderbolt%d`), not one derived from
PCI topology, so it falls outside the cases systemd's predictable-naming
scheme (`enp*`, `eno*`, `enx*`) is designed for (on-board index, PCI
slot/path, or MAC-address-derived names for physical/USB Ethernet hardware).
systemd's default `.link` policy only renames an interface when it can derive
a predictable name from `ID_NET_NAME_*` hwdb/udev properties; there is no
public udev/hwdb rule confirmed to produce such a name for
`thunderbolt-net` devices, so in practice the interface should keep the
kernel's own `thunderbolt0`-style name. **This could not be independently
confirmed against a live udev rule and should be verified on real hardware**
(see Open Questions). [systemd Predictable Network Interface Names](https://github.com/systemd/systemd/blob/main/docs/PREDICTABLE_INTERFACE_NAMES.md)

### sysfs / udev signals

Thunderbolt devices (including domains/host controllers and the automatically
formed XDomain link to the peer host) enumerate under
`/sys/bus/thunderbolt/devices/`. Documented attributes include:

- `/sys/bus/thunderbolt/devices/domainX/security` — current security level for that domain (host controller)
- `/sys/bus/thunderbolt/devices/<id>/authorized` — `0`/`1`/`2`, gates whether the device is authorized (see security levels below)
- `/sys/bus/thunderbolt/devices/<id>/device`, `device_name`, `vendor`, `vendor_name`, `unique_id` — identification attributes
- `/sys/bus/thunderbolt/devices/<id>/key` — 32-byte key used for `secure` mode challenge-response

The driver also emits `KOBJ_CHANGE` uevents for **PCIe tunnel** state with
`TUNNEL_EVENT=activated|changed|deactivated|low bandwidth|insufficient bandwidth`
and a `TUNNEL_DETAILS=...` string — useful signal patterns in general, though
these are documented in the context of PCIe tunnels (docks/eGPUs), not
explicitly for the XDomain/networking link.
[thunderbolt.rst](https://www.kernel.org/doc/Documentation/admin-guide/thunderbolt.rst), [ArchWiki: Thunderbolt](https://wiki.archlinux.org/title/Thunderbolt)

The resulting net device itself shows up as an ordinary `SUBSYSTEM=="net"`
device to udev/netlink, with its parent chain rooted under
`/sys/bus/thunderbolt/`; community udev-rule examples match on
`SUBSYSTEM=="net", DRIVER=="thunderbolt-net"` (or similar) to apply custom
naming/link policy. [Thunderbolt Networking Setup gist](https://gist.github.com/scyto/67fdc9a517faefa68f730f82d7fa3570), [Dell TB16 udev gist](https://gist.github.com/madchap/b5e6f9baf489d127b5ca2dbcd71adf63)

**CableDesk implication:** CableDesk's interface-detection logic can watch
for `add`/`remove` events on `SUBSYSTEM=net` where the parent device path
contains `/sys/bus/thunderbolt/` (or, more robustly, where `DRIVER` is a
thunderbolt-net-family driver), rather than hardcoding the name
`thunderbolt0` — this covers multi-controller laptops/hot-plug more safely.

### Security levels and the authorization model

Thunderbolt/USB4 defines hardware+firmware security levels, readable at
`/sys/bus/thunderbolt/devices/domainX/security`:

| Level | Behavior |
|---|---|
| `none` | All devices connect automatically; no user approval ("Legacy mode" in BIOS) |
| `user` | User must approve every new device connection (approval is then remembered) |
| `secure` | Like `user`, plus a stored cryptographic key verifies the device's identity on reconnect |
| `dponly` | Only DisplayPort/USB tunnels are auto-created; no PCIe tunneling |
| `usbonly` | Only USB controller + DisplayPort tunnels (common for docks) |
| `nopcie` | PCIe tunneling disabled entirely at the BIOS level |

[thunderbolt.rst](https://www.kernel.org/doc/Documentation/admin-guide/thunderbolt.rst), [ArchWiki: Thunderbolt](https://wiki.archlinux.org/title/Thunderbolt), [Introducing bolt (Christian Kellner)](https://christian.kellner.me/2017/12/14/introducing-bolt-thunderbolt-3-security-levels-for-gnulinux/)

**`boltd`** is the userspace daemon (Fedora/most distros ship it, package
`bolt`) that performs the actual authorization when the level is `user` or
`secure`. It exposes a D-Bus API (`org.freedesktop.bolt`) to list, enroll
(authorize + remember), and forget devices, and ships the `boltctl` CLI.
Desktop environments (documented for GNOME Shell) auto-enroll new devices
*only if* the current session is an unlocked administrator session — this is
the client-visible "unlock a popup / new Thunderbolt device connected" prompt
users see. On IOMMU-protected systems, boltd can automatically authorize
devices with an "iommu" or "auto" enrollment policy without user interaction,
since kernel IOMMU already blocks arbitrary DMA. [boltd man page (Arch)](https://man.archlinux.org/man/extra/bolt/boltd.8.en), [Fedora bolt package](https://packages.fedoraproject.org/pkgs/bolt/bolt/), [Managing Thunderbolt security on Fedora](https://blog.wains.be/2022/2022-02-10-thunderbolt-security-management/)

**Does the net interface appear before authorization?** Sources are
consistent that boltd's job is to authorize "thunderbolt peripherals" for
**PCIe tunnels**, and that the XDomain link (used for `thunderbolt-net`)
forms automatically over the control channel once two hosts are cabled
together — one kernel commit description states "Security levels and NVM
firmware upgrade continue to work as before with XDomain connections,"
implying XDomain-based networking is *also* gated by the domain's security
level rather than being exempt from it. However, no source consulted gives
an unambiguous, explicit statement of whether an unauthorized (`user`/`secure`
level, not-yet-approved) cable connection blocks `thunderbolt0` from
appearing at all, or whether the interface appears immediately while only
PCIe-tunneled peripherals wait on `authorized`. **This is flagged as an open
question requiring real two-machine hardware testing** — see Open Questions.
[LWN: Thunderbolt networking](https://lwn.net/Articles/735235/), [thunderbolt.rst](https://www.kernel.org/doc/Documentation/admin-guide/thunderbolt.rst)

**CableDesk implication:** CableDesk should not assume the interface is
immediately usable on first cable connection on a `user`/`secure`-level
system. The MVP setup flow should detect the "awaiting authorization" state
(e.g. poll/watch `authorized` under the relevant `/sys/bus/thunderbolt/devices/`
entry, or watch for `boltd`'s D-Bus enrollment signals) and prompt the user
to approve the connection (or run `boltctl authorize`/enroll) before
expecting `thunderbolt0` to be network-ready. Documenting this UX step is
part of MVP scope even though the underlying kernel behavior needs
hardware confirmation.

---

## 2. `thunderbolt-stream` / USB4STREAM (context only — not used in MVP)

USB4STREAM is a newer, separate kernel feature (driver `thunderbolt_stream`,
targeted at the Linux 7.2 kernel per Intel Thunderbolt maintainer Mika
Westerberg's patches) that lets two USB4/Thunderbolt-connected hosts
exchange **raw data streams directly**, bypassing the network stack
entirely. It's configured via ConfigFS
(`/sys/kernel/config/thunderbolt/stream/...`) and exposes `/dev/tbstreamX`
character devices that any app can `read()`/`write()` like a file, with
HopID allocation (including automatic allocation with `-1`). Documented use
cases include disk backup over a rescue initramfs, filesystem transfer via
`tar`/`gzip` pipes, and sharing a camera feed between hosts via GStreamer.
[Phoronix: Intel Introducing USB4STREAM](https://www.phoronix.com/news/Intel-Linux-USB4STREAM), [kernel.org thunderbolt.rst](https://www.kernel.org/doc/Documentation/admin-guide/thunderbolt.rst)

**Why CableDesk's MVP does not use it:**

1. **No IP stack.** Sunshine (host) and Moonlight (client) are built around
   the assumption of an IP transport (they speak their streaming protocol
   over UDP/TCP sockets to a discovered IP address). USB4STREAM deliberately
   bypasses the network stack, so Sunshine/Moonlight cannot use it without
   custom transport-layer work neither project currently does.
2. **Kernel maturity.** As of the sources checked, USB4STREAM only recently
   merged (Linux 7.2 kernel cycle) and is far newer/less battle-tested than
   `thunderbolt-net`, which has shipped since roughly the 4.x kernel series.
   Distro/LTS kernel availability for USB4STREAM will lag for some time.
3. **Scope.** `thunderbolt-net` already solves CableDesk's actual need (get
   an IP link up between the two hosts); USB4STREAM's raw-stream model would
   require CableDesk to build and maintain its own framing/transport
   protocol on top, which is out of scope for the MVP.

This is documented here for future reference only; USB4STREAM should be
revisited if a future CableDesk version wants a lower-latency
non-IP transport, once the driver/tooling matures.

---

## 3. NetworkManager D-Bus API

### Creating and activating a connection profile programmatically

NetworkManager exposes two D-Bus services relevant here, both under the
well-known name `org.freedesktop.NetworkManager`:

- **`org.freedesktop.NetworkManager.Settings`** (object path
  `/org/freedesktop/NetworkManager/Settings`) — manages stored connection
  profiles. Key methods:
  - `AddConnection(a{sa{sv}} connection) -> (o path)` — the "classic" call;
    takes the full nested settings dictionary (section name → property name
    → variant) and returns the new profile's object path.
  - `AddConnection2(a{sa{sv}} settings, u flags, a{sv} args) -> (o path, a{sv} result)`
    — newer, extensible form; `flags` is a bitmask including `0x1`
    (persist to disk), `0x2` (in-memory only, never written to disk), and
    `0x20` (block auto-connect on creation). `args` allows extra options
    (e.g. `plugin`).
  [NetworkManager.Settings D-Bus reference](https://networkmanager.pages.freedesktop.org/NetworkManager/NetworkManager/gdbus-org.freedesktop.NetworkManager.Settings.html)

- **`org.freedesktop.NetworkManager`** (root object path
  `/org/freedesktop/NetworkManager`) — the main manager. Key methods:
  - `ActivateConnection(o connection, o device, o specific_object) -> (o active_connection)`
    — activates an existing (already-added) profile on a given device;
    `connection` may be `"/"` to let NM pick the best match for the device.
  - `AddAndActivateConnection(a{sa{sv}} connection, o device, o specific_object) -> (o path, o active_connection)`
    — atomically creates (in-memory, auto-completing any settings the
    device can infer) and activates a profile in one call — likely the most
    convenient single call for CableDesk's "plug in cable, bring up link"
    flow.
  [NetworkManager D-Bus reference](https://networkmanager.pages.freedesktop.org/NetworkManager/NetworkManager/gdbus-org.freedesktop.NetworkManager.html)

A full worked Python example (`Settings.AddConnection` + `ActivateConnection`)
exists in NetworkManager's own example tree.
[NetworkManager examples: wifi-hotspot.py](https://github.com/NetworkManager/NetworkManager/blob/main/examples/python/dbus/wifi-hotspot.py)

### Restricting the profile so it never becomes the default route

Two `ipv4` connection settings are relevant and should both be set on
CableDesk's profile:

- **`ipv4.never-default`** (boolean): "If TRUE, this connection will never be
  the default connection for this IP type, meaning it will never be assigned
  the default route by NetworkManager." Setting this to `yes` is the primary
  mechanism to guarantee the cable link never hijacks the user's normal
  internet default route.
- **`ipv4.route-metric`** (integer, default `-1` = automatic-by-device-type):
  as a defense-in-depth measure, CableDesk can also set an explicit
  high-numbered metric so that even non-default routes from this profile
  are deprioritized versus the user's normal Wi-Fi/Ethernet routes.

[NetworkManager ipv4 settings reference](https://networkmanager.dev/docs/api/latest/settings-ipv4.html), [RHEL 8: Managing the default gateway](https://docs.redhat.com/en/documentation/red_hat_enterprise_linux/8/html/configuring_and_managing_networking/managing-the-default-gateway-setting_configuring-and-managing-networking)

### Scoping the profile to one interface

A connection profile is inherently bound to a single device by virtue of how
it's activated (`ActivateConnection`/`AddAndActivateConnection` take a
specific `device` object path), and/or by setting `connection.interface-name`
in the profile itself so NM only ever auto-matches it to that named
interface. This is the standard mechanism and requires no special interface
"reservation" step beyond normal profile authoring.
[NetworkManager D-Bus reference](https://networkmanager.pages.freedesktop.org/NetworkManager/NetworkManager/gdbus-org.freedesktop.NetworkManager.html)

### Rust D-Bus crates: `zbus` vs NetworkManager-specific crates

- **`zbus`** (crates.io: `zbus`, repo now at `github.com/z-galaxy/zbus` — the
  crate appears to have moved/rebranded from its original `dbus2/zbus` home):
  a pure-Rust, no-`libdbus`-dependency D-Bus implementation. At time of
  research it is at version **5.18.0**, with roughly **65 million total
  downloads** and **~17 million recent downloads**, and is a dependency of
  over a thousand other crates — by any measure a mature, widely-adopted,
  actively-maintained crate (multiple 2026 releases). This is the crate
  CableDesk should build on for typed D-Bus access to NetworkManager, Avahi,
  UPower, and bolt. [zbus on crates.io (via API)](https://crates.io/crates/zbus), [zbus docs](https://z-galaxy.github.io/zbus/)
- **NetworkManager-specific Rust crates** are comparatively immature and
  fragmented: the `networkmanager` crate (D-Bus via the older `dbus` crate,
  not `zbus`) is explicitly marked by its own maintainer as under-maintained
  ("doesn't have time to take care of this project currently"), sees ~60K
  total downloads, and last published mid-2025; `rusty_network_manager` and
  community projects like `KevinVoell/network_manager` wrap `zbus` with
  NM-specific typed bindings but are small, low-adoption projects. **None of
  these NM-specific crates showed evidence of broad production adoption.**
  [networkmanager crate](https://crates.io/crates/networkmanager), [rusty_network_manager crate](https://crates.io/crates/rusty_network_manager)

**Recommendation:** use `zbus` directly (optionally generating typed
proxies from NM's D-Bus introspection XML via `zbus`'s codegen macros)
rather than depending on a thin, low-adoption NetworkManager-specific crate.
This avoids taking on an under-maintained dependency for a narrow API
surface CableDesk can bind itself.

### Detecting interface add/remove events: NM D-Bus vs udev/netlink

NetworkManager's manager object emits `DeviceAdded(o device_path)` and
`DeviceRemoved(o device_path)` signals, plus a general `StateChanged(u state)`
signal, letting a D-Bus client watch device lifecycle without polling.
Internally, NetworkManager itself learns about new devices via udev, i.e.
udev/netlink is the lower-level, authoritative source of truth and NM's
D-Bus signals are a derived, higher-level view.
[NetworkManager D-Bus reference](https://networkmanager.pages.freedesktop.org/NetworkManager/NetworkManager/gdbus-org.freedesktop.NetworkManager.html), [FOSDEM 2023: Rust for network management tools](https://archive.fosdem.org/2023/schedule/event/rust_using_rust_for_your_network_management_tools/attachments/slides/5395/export/events/attachments/rust_using_rust_for_your_network_management_tools/slides/5395/Using_rust_for_your_network_management_tools.pdf)

**Recommendation:** Since CableDesk already needs to talk to NetworkManager
over D-Bus to create/activate the profile, it's simplest and sufficient to
also listen to NM's `DeviceAdded`/`DeviceRemoved` signals for the
thunderbolt-net interface's lifecycle, rather than adding a second,
lower-level netlink/udev watcher. Direct netlink (e.g. via `rtnetlink`) would
only be worth the extra complexity if CableDesk needed to work in
environments without NetworkManager at all, which is out of scope for MVP
(NetworkManager is assumed present, per Fedora default).

---

## 4. IPv4 link-local (169.254.0.0/16, RFC 3927) with NetworkManager

NetworkManager has first-class, built-in support for IPv4 link-local
addressing:

- The `ipv4.method` connection setting supports `"link-local"` as a value
  alongside `"auto"`, `"manual"`, and `"disabled"`.
- Since NetworkManager 1.52, a separate `ipv4.link-local` property lets you
  enable/disable link-local addressing *independently* of `ipv4.method`,
  including a `"fallback"` mode where NM only assigns a 169.254.x.y address
  if no other IPv4 address (DHCP/manual) is obtained.
- Address conflict detection is controlled by `ipv4.dad-timeout`
  (milliseconds); `0` disables duplicate-address detection, `-1` uses the
  NM default (documented as ~200ms).

[NetworkManager ipv4 settings reference](https://networkmanager.dev/docs/api/latest/settings-ipv4.html)

The underlying RFC 3927 collision-avoidance mechanism (which
`ipv4.dad-timeout`/NM's IPv4LL logic implements) is ARP probing: the host
sends ARP probe packets with sender IP `0.0.0.0` for a candidate 169.254.x.y
address, and if any peer responds claiming that address, a new candidate is
tried; RFC 3927 requires this collision handling for the entire time an
address is in use, not just at assignment. On Linux, this same IPv4LL logic
is also implemented as a standalone daemon, **`avahi-autoipd`**, which is
part of the Avahi/Zeroconf stack and can be invoked by NetworkManager during
connection activation for link-local connections.
[avahi-autoipd (DeepWiki)](https://deepwiki.com/avahi/avahi/6.1-avahi-autoipd:-ipv4-link-local-addressing), [avahi-autoipd man page](https://linuxcommandlibrary.com/man/avahi-autoipd)

**Could not fully verify:** whether NetworkManager's current
`ipv4.method=link-local`/`ipv4.link-local` implementation is a fully
self-contained internal implementation (an "IPv4LL" module inside
`NetworkManager` itself) versus something that still shells out to or
depends on `avahi-autoipd` being installed. Sources describe both an
internal NM property and NM's historical use of `avahi-autoipd` during
activation, but no single authoritative source was found that definitively
states which is authoritative in current (2026-era) NetworkManager releases.
Flagged in Open Questions.

Typical assignment latency for RFC 3927 probing (3 ARP probes with random
100–200ms delays between them per the base spec, so on the order of a few
hundred ms to ~1s in the absence of collisions) was described generically by
IETF/zeroconf sources but a NetworkManager-specific benchmark/measurement
was not found; this should be measured empirically on real hardware.

**CableDesk implication:** `ipv4.method=link-local` plus
`ipv4.never-default=yes` is the right shape for the cable-link connection
profile: it gets both hosts an address in the same 169.254.0.0/16 subnet
without any DHCP server, requires no manual IP configuration, and RFC
3927's collision handling is exactly the right behavior for a lab full of
CableDesk users cabling machines together.

**Explicitly deferred, not in v1:** NM ≥1.52's `ipv4.link-local=fallback`
mode (`ipv4.method=auto` that only falls back to a 169.254.x.y address if
DHCP doesn't answer) is *not* used. There is no DHCP server on a direct
cable link in any scenario CableDesk targets, so this mode adds a
DHCP-timeout wait on every connection for no benefit — it exists in
NetworkManager for networks that sometimes have DHCP and sometimes don't,
which isn't this link. The direct-link profile always uses plain
`ipv4.method=link-local`, unconditionally. If a future version wants to
support networks where DHCP-vs-link-local ambiguity is real, revisit this
as a deliberate feature addition with its own design, not as a default.

---

## 5. Avahi interface-scoped discovery

Avahi's D-Bus API (`org.freedesktop.Avahi.Server`) is explicitly designed to
allow scoping both publishing and browsing to one specific network
interface, rather than only supporting daemon-wide interface allow/deny
lists:

- **Browsing:** `Server.ServiceBrowserNew(interface, protocol, type, domain, flags)`
  takes an `interface` argument, which is normally passed as
  `AVAHI_IF_UNSPEC` (`-1`) to browse on all interfaces, but can instead be
  passed as a specific Linux interface index (e.g. the index of the
  `thunderbolt0` interface, obtainable via `if_nametoindex()`/netlink) to
  restrict the resulting `ServiceBrowser` object's `ItemNew`/`ItemRemoved`
  signals to services seen only on that one interface.
  [Avahi core-browse-services.c example](https://github.com/avahi/avahi/blob/master/examples/core-browse-services.c), [Avahi client-browse-services.c doxygen](https://avahi.org/doxygen/html/client-browse-services_8c-example.html)
- **Publishing:** the equivalent on the publish side is
  `EntryGroup.AddService(interface, protocol, flags, name, type, domain, host, port, txt)`
  — again, `interface` can be `AVAHI_IF_UNSPEC` or a specific interface
  index to restrict which interface(s) the announced service is visible on.

This means CableDesk does **not** need daemon-wide `avahi-daemon.conf`
interface restriction to keep mDNS scoped to the cable link — the API itself
supports per-call interface scoping, which is the correct level to do this
at since it doesn't affect the user's normal LAN mDNS traffic on their other
interfaces.

`avahi-daemon.conf`'s `[server]` section `allow-interfaces`/`deny-interfaces`
options *do* exist (comma-separated interface lists; `allow-interfaces`
takes precedence over `deny-interfaces`; an empty `allow-interfaces` means
"all interfaces except loopback and point-to-point") but these are
system-wide daemon configuration, edited by an administrator/installer — not
something CableDesk should rely on or modify at runtime, since doing so
would affect the whole system's mDNS behavior on all interfaces, not just
the cable link. [avahi-daemon.conf man page (Debian)](https://manpages.debian.org/unstable/avahi-daemon/avahi-daemon.conf.5.en.html)

**Recommendation:** CableDesk's Sunshine-discovery/advertisement helper
should use the Avahi D-Bus API with an explicit interface index (the cable
interface's index) for both its `ServiceBrowser` and `EntryGroup.AddService`
calls, leaving `avahi-daemon.conf` untouched. This is both simpler (no
system config file edits, no daemon restart) and safer (does not risk
degrading mDNS on the user's other interfaces).

---

## Open Questions

1. **Does an unauthorized (`user`/`secure` security level) cable connection
   block the `thunderbolt0` interface from appearing at all, or does the
   interface appear immediately with only PCIe-tunneled peripherals gated on
   `authorized`?** Sources agree XDomain/networking is subject to the same
   domain security level as other Thunderbolt functionality, but no source
   gives an explicit, unambiguous statement of the net-interface-visibility
   timing relative to `boltd` authorization. **Requires real two-machine
   hardware testing** with the security level set to `user` to observe
   whether `ip link` shows `thunderbolt0` before vs. after running
   `boltctl authorize`/enrolling the peer.

2. **Exact interface-naming behavior across multiple Thunderbolt
   controllers, hot-plug/re-plug, and controller order at boot.** The
   documentation only states interfaces are named "`thunderbolt0` and so
   on," implying `thunderbolt1`, `thunderbolt2`, etc. for additional ports/
   controllers, but does not document numbering stability across reboots,
   unplug/replug cycles, or whether systemd ever does rename these via a
   hwdb/udev rule on some distros. Needs verification on real multi-port
   hardware (e.g. a laptop with two Thunderbolt/USB4 ports).

3. **Whether NetworkManager's link-local IPv4 support (`ipv4.method=link-local`
   / `ipv4.link-local`) is a self-contained internal implementation or
   depends on `avahi-autoipd` being installed/running in current NM
   releases.** Sources describe both, but no single current, authoritative
   source resolved this definitively. Relevant to whether CableDesk needs to
   declare `avahi-autoipd` (part of the `avahi` package) as a hard runtime
   dependency alongside NetworkManager itself.

4. **Actual RFC 3927 link-local assignment latency in practice on a direct
   Thunderbolt/USB4 cable link between two real machines.** Only the
   generic RFC 3927 probing timing (a handful of ARP probes with sub-second
   randomized delays) could be sourced; no NetworkManager-specific or
   Thunderbolt-link-specific measurement was found. Needs to be measured on
   real hardware as part of CableDesk's MVP bring-up testing, since this
   directly affects perceived "plug in cable, stream starts" latency.

5. **`zbus`'s repository having apparently moved/rebranded to
   `github.com/z-galaxy/zbus`** — this was surfaced by crates.io metadata
   during research but wasn't independently corroborated against an
   announcement post; worth double-checking the crate's official channels
   (docs.rs, crates.io owner info) at implementation time in case this
   reflects a fork rather than the canonical upstream moving.
