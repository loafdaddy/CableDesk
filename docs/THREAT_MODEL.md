# CableDesk Threat Model

Status: threat model for the target architecture (most of which is not
implemented yet — see `docs/ROADMAP.md`). This document exists now, before
pairing/networking/streaming are built, so those features get designed
against it rather than reviewed for security after the fact.

## Assets to protect

1. **Control of the host computer** — keyboard/mouse input, screen contents.
2. **The host's long-term device identity** (private key, once
   `cabledesk-pairing` exists) and the trusted-peer list.
3. **Confidentiality/integrity of the video/audio stream** between paired
   devices.
4. **The user's existing network configuration** — Wi-Fi, Ethernet, default
   route, DNS — must not be affected by CableDesk at all.
5. **Clipboard text**, once clipboard sync exists (Phase 5).

## Explicitly out of scope

Per `docs/adr` and the project's non-goals: internet remote access, GDM
login-screen control, multi-host/multi-controller fan-out, file transfer.
These are not defended against because they don't exist as attack surface —
CableDesk cannot be attacked over the internet if it never listens on the
internet.

## Threat actors

| Actor | Capability | In scope? |
|---|---|---|
| Someone with physical access to the cable/ports, but not to either logged-in session | Can plug in a cable, cannot touch keyboard/screen of either machine | Yes — this is the primary threat CableDesk's pairing model defends against |
| A process on the same machine as controller or host | Local code execution as the user (not root) | Partially — D-Bus/Polkit boundaries matter here |
| Another device on the same Wi-Fi/LAN as the host or controller | Network access, not physical cable access | Yes — must not be able to reach CableDesk's control channel or stream at all, since both are direct-interface-only |
| A malicious/compromised peer that was legitimately paired once | Holds a previously-trusted identity | Partially — out of scope for MVP (no revocation-detection beyond "Forget"), tracked as a gap below |
| An attacker who can run arbitrary code as root on either machine | Full control of that machine | Out of scope — a fully-compromised endpoint cannot be defended against by an application running on it |

## Key threats and mitigations

### T1 — Physical cable access without session access implies control

**Threat:** someone plugs a cable into an unattended (but logged-in) host
and expects to control it without any prior relationship.

**Mitigation:** engineering rule "no physical-cable-equals-trust
assumption." Every connection requires either (a) completing the six-digit
verification-code pairing ceremony with confirmation on both devices, or
(b) being an already-trusted device — and even then, `require_confirmation`
(per-device, see `cabledesk_core::device::TrustedDevice`) can force
confirmation on every connection, not just the first. **Not implemented
yet** — `cabledesk-pairing` is an empty placeholder crate this milestone.

### T2 — Untrusted/spoofed peer identity

**Threat:** a device on the direct link claims to be a previously-trusted
peer.

**Mitigation:** mutual authentication using long-term public keys generated
at first launch (a maintained Rust cryptography library — not custom
primitives, per engineering rule). The six-digit pairing code is *only* a
human-verification step during initial pairing, never the long-term
credential — see `cabledesk_core::device::PairingCode`'s doc comment.
`CableDeskError::UntrustedPeerIdentity` already exists in the shared error
model with a direct user-facing mapping ("its security identity did not
match the trusted device... blocked"), even though nothing raises it yet.
**Not implemented yet.**

### T3 — Silent fallback to a non-direct network

**Threat:** if the direct interface drops, CableDesk (or Sunshine/Moonlight
underneath it) could keep the session alive over Wi-Fi/Ethernet without the
user noticing, defeating the entire "direct cable only" security posture.

**Mitigation:** engineering rule "no silent fallback to Wi-Fi." Before
starting a stream: verify the peer address belongs to the direct interface,
the route resolves through that interface, and the trusted device identity
matches. If the cable disappears, stop the stream immediately rather than
letting the underlying transport quietly re-route — see
`ConnectionState::Streaming`'s only non-error transitions in
`cabledesk-core/src/state.rs` (`Suspended`, `Disconnecting`,
`WaitingForCable` — never a "still streaming, just slower" state).
**Enforcement not implemented yet** (Phase 2/4).

### T4 — mDNS discovery treated as authentication

**Threat:** trusting mDNS-advertised device info as proof of identity.

**Mitigation:** engineering rule "no secrets in mDNS," and mDNS is
explicitly documented (`docs/NETWORKING.md` §5) as a discovery-only
mechanism — advertised fields (device ID, friendly name, role, pairing
state, port) are non-secret and are never treated as authenticating a peer.
Authentication happens over the pairing protocol's own encrypted channel.

### T5 — Compromised or malicious privileged helper input

**Threat:** the UI or agent (unprivileged, user-controlled) tricks the
privileged helper into doing something dangerous — arbitrary command
execution, arbitrary file writes, capability misuse.

**Mitigation:** `cabledesk-helper`'s D-Bus interface (see
`data/dbus-1/interfaces/org.cabledesk.Helper1.xml`) is narrow and typed —
no method accepts a shell command or an arbitrary file path. Every future
privileged method must perform its own
`org.freedesktop.PolicyKit1.Authority.CheckAuthorization` call (action IDs
already reserved in `data/polkit-1/actions/org.cabledesk.Helper1.policy`)
before doing anything. This milestone's helper implements no privileged
actions at all, so there is currently no such call to audit — tracked as
the first thing to get right in Phase 2.

### T6 — Secrets leaking via logs, process arguments, or diagnostics

**Threat:** private keys, pairing secrets, or clipboard contents ending up
in `journalctl` output, `ps` output, or an exported diagnostics bundle.

**Mitigation:** `cabledesk_core::error::UserFacingError` already enforces
that only a curated headline/detail pair reaches the UI, never raw
`Debug`/`Display` formatting of an error that might embed sensitive
context (unit-tested: `error::tests::user_facing_never_leaks_debug_
formatting`). `cabledesk-helper`'s `CollectDiagnosticsJson` only ever
serializes `PlatformDiagnostics`, which structurally cannot contain a
private key (the type doesn't have one) — see `docs/SECURITY.md`, "Never
include," for the full list once secrets exist to leak.

### T7 — A previously-trusted peer is later compromised

**Threat:** a peer that was legitimately paired is later compromised (stolen
laptop, malware) and still holds valid trust credentials.

**Mitigation:** the `Forget` action (`cabledeskctl forget`,
`TrustedDevice`) removes a peer's trust unilaterally from the other side.
**Gap:** there is no revocation broadcast or expiry — if a device is stolen,
the *other* side must proactively `Forget` it; the compromised device
itself could still hold the shared trust record until then. This is a
known, accepted limitation for a single-user workstation feature (see
non-goals) rather than an oversight, but is recorded here so it's a
conscious tradeoff, not a silent gap. Revisit if CableDesk is ever used in
a context where stolen-device risk is higher.

## Summary table

| ID | Threat | Status |
|---|---|---|
| T1 | Physical cable ≠ trust | Designed for, not implemented |
| T2 | Spoofed peer identity | Designed for (error type exists), not implemented |
| T3 | Silent Wi-Fi fallback | Designed for (state machine shape), not enforced |
| T4 | mDNS as auth | Avoided by design (mDNS carries no secrets) |
| T5 | Helper input validation | Interface is narrow/typed; no privileged actions to audit yet |
| T6 | Secret leakage via logs/diagnostics | Enforced today for the error/diagnostics paths that exist |
| T7 | Compromised trusted peer | Accepted limitation (manual `Forget`, no revocation broadcast) |

See `docs/SECURITY.md` for the corresponding design commitments and
`docs/OPEN_QUESTIONS.md` for what's still unresolved.
