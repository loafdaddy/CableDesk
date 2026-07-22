# ADR-007: The stream must never fall back to Wi-Fi/Ethernet

## Status

Accepted (design commitment). The two validation primitives
(address-assignment and route-table checks, both in `cabledesk-network`)
are implemented and tested live as of Phase 2. Wiring them into an actual
stream is still Phase 4 (there is no stream yet), and none of this has
been verified against real Thunderbolt/USB4 hardware — see
`docs/THREAT_MODEL.md` T3.

## Context

CableDesk's entire security posture and value proposition depend on the
connection being the direct cable, not the user's normal network. If
Sunshine/Moonlight (or CableDesk's own connection logic) silently
re-resolved to a peer's Wi-Fi/LAN address when the direct link had a
problem, two things would break at once: the "ultra-low-latency direct
wired" performance promise, and the "physical cable access is required"
threat model (`docs/THREAT_MODEL.md` T3) — since a normal LAN address is
reachable by anything else on that LAN, not just someone holding the cable.

## Decision

Before starting a stream, verify: the peer address belongs to the direct
interface, the route to it resolves through that interface, and the peer's
trusted device identity matches. If the cable disappears mid-stream, stop
or suspend the stream immediately rather than letting the transport
quietly re-route. This is enforced with a dedicated firewall zone/policy
per peer where practical, not just application-level address checking.

Reflected today in `cabledesk_core::state::ConnectionState`: from
`Streaming`, the only legal transitions are `Suspended`, `Disconnecting`,
or `WaitingForCable` — there is no "streaming, but over a different
interface now" state, and this is unit-tested
(`state::tests::cable_removed_mid_stream_returns_to_waiting_for_cable`).

## Consequences

- Every future networking/streaming implementation must treat "the direct
  interface is the only valid transport" as a hard invariant, checked
  before *and* continuously during a stream — not just at connection
  start.
- Verifying this end-to-end requires traffic capture and route inspection
  on real hardware once streaming exists (`docs/TEST_PLAN.md`, "Verifying
  'direct interface only'") — this cannot be fully validated by unit tests
  alone, since the whole point is that a lower layer (Sunshine, the OS
  routing table) doesn't quietly do the wrong thing.
- This is one reason `PlatformBackend::prepare_direct_link` and `install_
  firewall_policy` are real trait methods, not an afterthought: the
  firewall/route validation work belongs at the platform-backend layer,
  reusable across future distro backends, rather than duplicated inside
  `cabledesk-streaming` per platform.
