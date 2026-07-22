# ADR-008: Power delivery reporting is read-only in the MVP; no automatic power-role changes

## Status

Accepted, implemented for the read-only reporting half (this milestone).
The "never write" half is a permanent constraint, not a phase to
graduate out of.

## Context

USB-C Power Delivery negotiation is genuinely complex (port capability,
cable rating, firmware negotiation, current workload) and getting it wrong
has real-world consequences — forcing a bad power-role swap can, in the
worst case, affect charging behavior or hardware safety margins on
equipment CableDesk doesn't own or fully understand the characteristics of.
The project's own rules are explicit: no unsupported claims about charging
wattage, no automatic USB-C power-role changes, no writing to Type-C
power-role sysfs attributes automatically.

## Decision

`cabledesk-platform-fedora::power` only ever reads
`/sys/class/typec/portN/{power_role,data_role,usb_power_delivery_revision}`
and `/sys/class/power_supply/*/{type,online,status,capacity}` — there is no
write path anywhere in this crate, and none is planned. Report status
honestly, including "charging information is not available from this
hardware" as a legitimate outcome, rather than inferring charging behavior
CableDesk cannot actually confirm.

## Consequences

- Verified on real hardware this milestone: the dev machine's UCSI-backed
  Type-C ports report `power_role`/`usb_power_delivery_revision` correctly
  (PD 2.0 on both ports), and battery capacity/status (48%, Charging) reads
  correctly from `/sys/class/power_supply/BAT1` — see
  `docs/TEST_PLAN.md`.
- Whether a specific Type-C port's PD attributes can be confidently tied
  back to the *specific* cable/port CableDesk is using for `thunderbolt-
  net` is unresolved (`docs/OPEN_QUESTIONS.md` #8) — until this is
  answered on hardware that actually has both a Thunderbolt controller and
  UCSI power reporting, CableDesk's UI should say "the laptop's battery
  status is X" rather than implying "this specific cable is delivering Y
  watts," which it cannot actually measure or confirm.
- A later, clearly-labeled experimental feature may offer an *explicit*,
  user-initiated power-role request — but only after real hardware safety
  research, and never as an automatic/default behavior. This ADR does not
  authorize that feature; it would need its own ADR.
- The optional "reduce stream quality while discharging" behavior
  (project plan §15) must default off, since it depends on power reporting
  this ADR treats as best-effort/sometimes-unavailable, not guaranteed.
