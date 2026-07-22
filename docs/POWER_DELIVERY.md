# CableDesk Power Delivery Research

This document covers how CableDesk can read USB-C/USB4 power-role and USB
Power Delivery (PD) status from Linux userspace, and how to read
battery/charging state via UPower — relevant for UI affordances like showing
"this cable is also charging your laptop" or a battery indicator during a
Moonlight session.

Research was done against the Linux kernel's Type-C connector class and
`usb_power_delivery` sysfs ABI documentation, the UCSI driver, the
`power_supply` sysfs ABI, and the UPower D-Bus reference manual. Anything
that could not be verified against a primary source is called out in
**Open Questions** rather than asserted as fact.

---

## 6. Linux Type-C sysfs (`/sys/class/typec/`)

### Port attributes

The kernel's USB Type-C connector class exports one directory per Type-C
port (e.g. `/sys/class/typec/port0/`), giving userspace a unified way to
read connector status and (where supported) request role swaps. Documented
attributes include:

- **`power_role`** — "The supported power roles. This attribute can be used
  to request power role swap on the port." Valid values are `source` and
  `sink`; the currently active role is shown in brackets, e.g.
  `[source] sink`.
- **`data_role`** — "The supported USB data roles. This attribute can be
  used for requesting data role swapping on the port." Valid values are
  `host` and `device`, current role shown the same bracketed way, e.g.
  `[host] device`.
- **`preferred_role`** — lets userspace tell the driver a preferred role
  (`source`, `sink`, or `none` to clear the preference); relevant for
  dual-role-capable ports doing automatic role negotiation.
- **`port_type`** — port's overall capability: `source` (DFP-only), `sink`
  (UFP-only), or `dual` (dual-role capable).
- **`usb_power_delivery_revision`** — "Revision number of the supported USB
  Power Delivery specification, or 0.0 when USB Power Delivery is not
  supported," e.g. `2.0`, `3.0`, `3.1`.

[sysfs-class-typec ABI doc (kernel.org)](https://www.kernel.org/doc/Documentation/ABI/testing/sysfs-class-typec), [USB Type-C connector class driver-api docs](https://docs.kernel.org/next/driver-api/usb/typec.html)

### Partner/cable attributes

When a partner device (the other end of the cable) is connected, it appears
as a child device under the port (e.g.
`/sys/class/typec/port0/port0-partner/`), exposing:

- **`supports_usb_power_delivery`** — whether the connected partner supports
  PD communication at all (`yes`/`no`).
- **`usb_power_delivery_revision`** — the partner's (or cable's) supported PD
  spec revision, same `0.0`-if-unsupported convention as the port attribute.

[sysfs-class-typec ABI doc](https://www.kernel.org/doc/Documentation/ABI/testing/sysfs-class-typec)

### USB Power Delivery detail: `/sys/class/usb_power_delivery/`

For actual PD contract detail (negotiated voltages/currents, not just "is PD
supported"), there is a separate sysfs class,
`/sys/class/usb_power_delivery/`, introduced specifically to give PD Power
Data Objects (PDOs) their own directory structure (split out from
`/sys/class/typec/` per a dedicated kernel patch series). Each PD object
exposes:

- **`revision`** — the USB PD Specification Revision in use for that
  capability set, with an optional `version` file for the more specific
  revision version (frequently absent/unpopulated).
- **`source-capabilities/`** and **`sink-capabilities/`** — directories of
  PDOs named `<position>:<type>` (e.g. `1:fixed_supply`, `2:variable_supply`,
  etc.):
  - The mandatory first PDO, `1:fixed_supply`, represents the baseline
    vSafe5V capability and carries USB-specific flags:
    `dual_role_power`, `dual_role_data`, `usb_communication_capable`,
    `usb_suspend_supported`, `unconstrained_power`,
    `unchunked_extended_messages_supported`.
  - All fixed-supply PDOs report `voltage` (mV); **source**-side fixed
    supplies additionally report `maximum_current`, while **sink**-side
    fixed supplies report `operational_current` and
    `fast_role_swap_current`.
  - Variable, battery, and programmable/adjustable-voltage supply PDO types
    expose the corresponding voltage/current range and power fields for
    their type.

[sysfs-class-usb_power_delivery ABI doc (kernel.org)](https://www.kernel.org/doc/Documentation/ABI/testing/sysfs-class-usb_power_delivery), [PATCH: Separate sysfs directory for USB PD objects](https://lkml.iu.edu/hypermail/linux/kernel/2202.0/02998.html)

**Not verified from documentation alone:** the precise cross-reference
mechanism by which a given `/sys/class/usb_power_delivery/pdX` object is
linked back to its owning `/sys/class/typec/portY` (and, where relevant, its
partner) — e.g. whether it's a symlink, a common parent device, or another
attribute. This should be confirmed by inspecting a real UCSI-driven laptop's
sysfs tree. See Open Questions.

### `power_supply` class and `usb_type`

Independent of (but complementary to) the Type-C class, the
`power_supply` sysfs ABI exposes a per-supply `usb_type` attribute at
`/sys/class/power_supply/<supply_name>/usb_type`, documented since ~March
2018. Valid reported values include: `Unknown`, `SDP`, `DCP`, `CDP`, `ACA`,
`C`, `PD`, `PD_DRP`, `PD_PPS`, `BrickID`. For most battery-charger power
supplies this is **read-only** (it reports what the hardware detected); it
is writable only for power supplies that themselves act as a USB power
*source* (the doc's example is the `UCS1002` USB port power controller
chip), which is not the CableDesk laptop-charging scenario.
[sysfs-class-power ABI doc (kernel.org)](https://www.kernel.org/doc/Documentation/ABI/testing/sysfs-class-power)

### UCSI driver specifics

Modern laptops (most from roughly the last several years) implement Type-C
port/PD control through embedded controller firmware speaking **UCSI**
(USB Type-C Connector System Software Interface) rather than a
vendor-specific in-kernel PD state machine. The kernel's `TYPEC_UCSI` driver
family (with the ACPI-based transport in `ucsi_acpi.c`, present since
roughly kernel 4.13) implements the standard Type-C class interfaces
described above by talking UCSI to the embedded controller/PMIC, rather than
CableDesk (or any userspace tool) needing UCSI-specific knowledge — from
sysfs's point of view a UCSI-backed port looks like any other
`/sys/class/typec/portN`. Kernel development is actively extending UCSI's
`power_supply` integration (recent patch series add PD/PD-DRP distinction
and charge-control-limit-max, allowing a power-role-swap request to be
issued from the `power_supply` sysfs side as well as from
`/sys/class/typec/`). [UCSI ACPI driver source](https://github.com/torvalds/linux/blob/master/drivers/usb/typec/ucsi/ucsi_acpi.c), [USB Type-C Class and driver for UCSI (LWN)](https://lwn.net/Articles/674944/), [UCSI power supply expansion patch](https://lkml.iu.edu/hypermail/linux/kernel/2407.2/01245.html)

**CableDesk implication:** to show "USB4 cable is delivering power" /
"charging via cable" UI, CableDesk should read, per relevant Type-C port:
`power_role` (is this port currently a `sink`, i.e. receiving power),
`usb_power_delivery_revision` (is PD active at all vs. plain USB power), and
cross-reference `/sys/class/power_supply/*/usb_type` (look for `PD`/`PD_DRP`/
`PD_PPS`) plus that supply's standard `online`/`status` attributes for the
simple charging/not-charging boolean. This avoids needing to parse PDOs
directly for the MVP's UI needs — full PDO detail from
`/sys/class/usb_power_delivery/` is "nice to have" (e.g. showing negotiated
wattage) rather than required.

---

## 7. UPower D-Bus API for charging info

UPower (`org.freedesktop.UPower`, system bus) is the standard
desktop-independent daemon for battery/power-supply status, and is what
GNOME/KDE's own battery indicators use.

### Enumerating devices vs. the display device

- **`EnumerateDevices()`** returns an array of object paths, one per
  individual power-supply device UPower knows about (each individual
  battery, each AC/USB power source, etc.) — use this for granular detail
  (e.g. a multi-battery system, or distinguishing the AC adapter from the
  battery).
- **`GetDisplayDevice()`** returns a single, synthesized composite device
  object (guaranteed at the fixed path
  `/org/freedesktop/UPower/devices/DisplayDevice`) representing "the" status
  icon a desktop environment would show — UPower internally merges/picks the
  most relevant real device(s) into this one composite view. This is the
  simpler, recommended choice for CableDesk's likely use case (single laptop
  battery status in a small UI indicator), rather than reasoning about
  multiple raw devices from `EnumerateDevices`.

[UPower.html D-Bus reference](https://upower.freedesktop.org/docs/UPower.html)

### `org.freedesktop.UPower.Device` properties

Once you have a device object path (from either call above), the properties
relevant to charging/battery UI are on the `org.freedesktop.UPower.Device`
interface:

| Property | Type | Meaning |
|---|---|---|
| `Type` | `u` (enum) | Kind of power source (value `2` = Battery per the UPower enum) |
| `State` | `u` (enum) | `0`=Unknown, `1`=Charging, `2`=Discharging, `3`=Empty, `4`=Fully charged, `5`=Pending charge, `6`=Pending discharge |
| `Percentage` | `d` | Energy remaining, 0–100 |
| `EnergyRate` | `d` | Watts; **positive = discharging, negative = charging** — note the sign convention |
| `Energy` | `d` | Current energy in the source, Wh |
| `EnergyFull` | `d` | Energy considered "full" for this source, Wh |
| `TimeToEmpty` | `x`/`t` (seconds) | Seconds until empty; `0` if unknown |
| `TimeToFull` | `x`/`t` (seconds) | Seconds until full; `0` if unknown |
| `IsPresent` | `b` | Whether the power source is physically present (relevant for hot-removable batteries) |
| `IconName` | `s` | Icon name per the freedesktop Icon Naming Specification |
| `WarningLevel` | `u` (enum) | Ranges from Unknown through to Critical/Action-required |

[UPower Device.html D-Bus reference](https://upower.freedesktop.org/docs/Device.html)

**CableDesk implication:** for a "battery/charging" indicator alongside a
Moonlight session, call `GetDisplayDevice()` once, then read `State`
(specifically checking for `1` = Charging, to correlate with "cable is
powering the laptop") and `Percentage` for the headline number, and
optionally `TimeToFull`/`TimeToEmpty` for a more detailed tooltip. Subscribe
to the standard D-Bus `PropertiesChanged` signal on that device object
rather than polling, to update the UI live as the battery state changes.

---

## Open Questions

1. **How exactly a `/sys/class/usb_power_delivery/pdX` PD-capability object
   is cross-referenced back to its owning `/sys/class/typec/portY` (and to
   the relevant `/sys/class/power_supply/*` entry) in sysfs** — symlink,
   shared parent device, or another mechanism. The ABI docs describe each
   class's own attributes but not the explicit linkage between the three
   sysfs trees. **Needs verification on real UCSI hardware** by walking a
   live sysfs tree (e.g. `udevadm info -a` on a real Type-C port) rather
   than documentation alone.

2. **Whether every relevant modern laptop actually surfaces
   `/sys/class/usb_power_delivery/` at all**, versus only exposing the
   simpler `/sys/class/typec/` and `/sys/class/power_supply/usb_type`
   attributes. The `usb_power_delivery` class is comparatively new
   (introduced to split PD detail out of the Type-C class); its presence on
   a given kernel/hardware combination should be treated as optional by
   CableDesk and probed for, not assumed.

3. **Real-hardware confirmation that a USB4/Thunderbolt cable used for
   CableDesk's networking link is the same physical port/connection whose
   Type-C power-role attributes are being read** — i.e. that
   `power_role`/`usb_power_delivery_revision` for the *same* port used for
   `thunderbolt-net` is straightforward to identify from userspace (matching
   a `/sys/class/typec/portN` to the Thunderbolt domain/interface in use).
   The sysfs hierarchies for `/sys/bus/thunderbolt/` and
   `/sys/class/typec/` were not confirmed to share an obvious, documented
   cross-reference; this needs to be checked on real hardware, since
   CableDesk's UI wants to say "this specific cable is charging," not just
   "some USB-C port somewhere is charging."

4. **Exact behavior/latency of UPower's `PropertiesChanged` signal
   emission cadence** (e.g. does it emit on every small percentage tick, or
   only on state transitions/larger changes) — not addressed in the D-Bus
   reference docs consulted. Relevant to whether CableDesk should also
   poll periodically as a fallback. Should be measured empirically.
