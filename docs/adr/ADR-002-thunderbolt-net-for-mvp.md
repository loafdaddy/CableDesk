# ADR-002: Use `thunderbolt-net`, not `thunderbolt-stream`/USB4STREAM, for the MVP transport

## Status

Accepted (research-backed; not yet implemented — Phase 2).

## Context

Two transport options exist for getting data across a direct USB4/
Thunderbolt cable on Linux: the established `thunderbolt-net` (USB4NET)
kernel driver, which exposes an ordinary `net` device (e.g. `thunderbolt0`)
and lets the normal IP stack run over it; and the newer `thunderbolt-stream`
(USB4STREAM) driver, which exposes raw `/dev/tbstreamX` character devices
for direct application-to-application data transfer, bypassing the network
stack entirely. See `docs/NETWORKING.md` §1–2 for full research.

Sunshine (host) and Moonlight (client) are both built around the assumption
of an IP transport — they speak their streaming protocol over UDP/TCP
sockets to a discovered IP address. Neither has any support for a raw
character-device transport.

## Decision

Use `thunderbolt-net` for the MVP. Document `thunderbolt-stream`/USB4STREAM
in `docs/NETWORKING.md` §2 for future reference, but do not use it.

## Consequences

- CableDesk gets a real IP link with zero changes needed to Sunshine or
  Moonlight — the entire integration surface is "make an IP address appear
  on the right interface, discoverable and scoped correctly," not "teach
  two upstream projects a new transport."
- `thunderbolt-net` ships as a loadable module on most distro kernels
  (confirmed for the research pass's target), not built-in, and
  Linux-to-Linux pairing needs at least one side to `modprobe
  thunderbolt-net` — Windows/macOS peers get auto-load, Linux peers don't
  (see `docs/NETWORKING.md` §1). CableDesk's host-side setup must do this
  explicitly rather than assume auto-load.
- Interface naming is kernel-assigned (`thunderbolt0`, `thunderbolt1`, ...),
  not systemd-predictable-named, and multi-controller/hot-plug numbering
  stability is unverified (`docs/OPEN_QUESTIONS.md` #2) — CableDesk must
  detect the interface by driver name, never hardcode `thunderbolt0`
  (already reflected in `cabledesk-platform-fedora::direct_link`, which
  matches on the driver symlink target rather than a fixed name).
- If a future CableDesk version wants lower latency than an IP stack can
  provide, USB4STREAM is the thing to revisit — but only once it and its
  tooling have matured well past this ADR's research date, and only with a
  plan for the custom framing/transport work that would require on both
  the Sunshine and Moonlight sides (or a CableDesk-native replacement for
  both, a much larger undertaking explicitly out of this project's current
  scope).
