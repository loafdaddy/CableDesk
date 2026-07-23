# CableDesk Security Model

Status: the design commitments CableDesk is built against, plus an honest
account of what's actually implemented in this milestone (very little of
the model below is enforced yet — see the "This milestone" callouts). For
threat-actor analysis, see `docs/THREAT_MODEL.md`.

## Reporting a vulnerability

This is a pre-release, experimental project with no users yet. Until a
formal process exists (Phase 7, `docs/ROADMAP.md`), open a GitHub issue for
anything that isn't security-sensitive, or contact the maintainers directly
(see repository metadata) for anything that is. There is no bug bounty.

## Design commitments

These are hard constraints, not aspirations — see the engineering rules
they derive from in the project's own planning notes:

- **Physical cable access never implies control.** Every connection
  requires either completed pairing or pre-established trust — see
  `docs/THREAT_MODEL.md` T1.
- **No custom cryptography.** Device identity and the pairing protocol use
  a maintained Rust cryptography library; nothing here gets hand-rolled.
- **No secrets in logs, mDNS, or process arguments.** Verified today by
  `cabledesk_core::error::UserFacingError` (unit-tested) for the error path;
  the rest applies once there are secrets to leak (pairing keys, Phase 3).
- **The GTK UI never runs as root.** Enforced structurally: `cabledesk-ui`
  has no privileged code path at all; every privileged operation lives only
  in `cabledesk-helper`.
- **Privileged code stays small and independently reviewable.** Privileged
  surface is `crates/cabledesk-helper`: read-only `Version` / `Ping` /
  `CollectDiagnosticsJson`, plus mutating `PrepareDirectLink` (Polkit
  `org.cabledesk.helper.prepare-direct-link` on every call; rejects
  non-direct interfaces).
- **No disabling SELinux or firewalld, ever**, including in development
  tooling — see `scripts/dev-setup.sh`, which only ever reports their
  status, never changes it.
- **No silent fallback to Wi-Fi or Ethernet** — see `docs/THREAT_MODEL.md` T3
  and the cable-only invariants in `docs/ARCHITECTURE.md`. Wrong-interface
  peers must fail with `PeerNotOnDirectCable` / “No direct cable connection”.

### Cable-only invariants (enforcement status)

| # | Invariant | Software today |
|---|-----------|----------------|
| 1–4 | Direct USB4/TB interface required; driver evidence; peer on that iface | `classify` / `sysfs` + `PreparedDirectLink` / `validate_cable_peer` |
| 5 | Discovery scoped to direct interface | Agent Avahi publish/browse on direct ifindex |
| 6–8 | Control/Sunshine/Moonlight bound to validated cable endpoint | Not built (Phases 3–4) |
| 9–10 | Cable removal ends session; no Wi-Fi/Ethernet retry | Agent tear-down → `WaitingForCable`; sim proves unplug |
| 11–12 | Trusted-on-LAN / manual IP rejected | Policy + sim; no production manual-IP API |

Full table: `docs/ARCHITECTURE.md` § Cable-only invariants.

## Device identity and pairing (not implemented yet)

Planned design (Phase 3, `docs/ROADMAP.md`; see
`cabledesk_core::device::{DeviceId, PairingCode, TrustedDevice}` for the
non-secret shapes that already exist):

1. First launch generates a random `DeviceId` and a long-term public/private
   key pair via a maintained crate (candidate: `ring` or `rustls`'s
   underlying crypto — to be finalized in an ADR before implementation).
2. Pairing discovers the unpaired peer over the direct cable, establishes an
   encrypted provisional channel, and derives a six-digit
   `PairingCode` for human verification on both devices.
3. The code is **never** the long-term credential — see `PairingCode`'s doc
   comment in `cabledesk-core/src/device.rs`. Long-term trust is the
   exchanged public identity, stored as a `TrustedDevice`.
4. Secrets are stored with owner-only permissions; GNOME Keyring/Secret
   Service is being evaluated as the storage backend (see
   `docs/OPEN_QUESTIONS.md` — not yet decided).

## Connection visibility (not implemented yet)

Planned: a persistent local notification naming the connected controller,
a Disconnect action, an emergency keyboard shortcut, and immediate input
cessation on cable disconnect. Connection start/end gets logged; screen
contents and keystrokes never do (see "Never include" below).

## Privilege separation

Three processes, matching `docs/ARCHITECTURE.md`:

- **`cabledesk` (UI)** — logged-in user, no privileged code path.
- **`cabledesk-agent`** — logged-in user, `systemd --user` service. Holds
  device identity and (eventually) orchestrates pairing/streaming, but
  never runs as root.
- **`cabledesk-helper`** — root, system `systemd` service, D-Bus-activated
  surface only (`org.cabledesk.Helper1`). Every privileged method must
  perform its own `org.freedesktop.PolicyKit1.Authority.CheckAuthorization`
  call against an action ID in `data/polkit-1/actions/` before doing
  anything (see engineering rule: Polkit checks triggered by clear user
  action, never blocking the GTK main thread). **Implemented mutating
  method:** `PrepareDirectLink` (action
  `org.cabledesk.helper.prepare-direct-link`). Read-only methods
  (`Version`, `Ping`, `CollectDiagnosticsJson`) stay unauthenticated.

### PrepareDirectLink authorization (closed for this path)

`cabledeskctl repair-network` and `cabledesk-agent` call helper
`PrepareDirectLink` over the system bus. The helper:

1. Runs Polkit `CheckAuthorization` for
   `org.cabledesk.helper.prepare-direct-link` (AllowUserInteraction).
2. Rejects interface names that are not USB4/Thunderbolt-derived
   (`require_direct_cable_interface`).
3. Then runs `FedoraBackend::prepare_direct_link` (NM profile + firewalld
   zone bind).

Remaining reserved Polkit actions (`load-thunderbolt-module`,
`install-firewall-policy`, `repair-network-profile`) are still unused —
implement or retire deliberately (see `docs/OPEN_QUESTIONS.md` §32).

### A concrete finding from earlier milestones

Getting `cabledesk-helper` to even start on the system bus required adding
`data/dbus-1/system.d/org.cabledesk.Helper1.conf` — verified by hands-on
testing: without it, the default system bus policy refuses
`RequestName("org.cabledesk.Helper1")` with `AccessDenied`, even for a
root-owned process. This file only grants "root may own this bus name" and
"any user may call methods on it" at the **bus** level — it is not an
authorization mechanism and must never be treated as one. Loosening it
can never substitute for a real per-method Polkit `CheckAuthorization` call.
See `docs/PACKAGING.md`'s Polkit research section for why `.pkla`-style
static bus policy is the wrong place to put actual authorization logic.

## Network exposure (not implemented yet)

Planned: CableDesk/Sunshine must not bind to all interfaces by default.
`docs/NETWORKING.md` §5 and the direct-link enforcement rules in
`docs/THREAT_MODEL.md` T3 describe how the direct interface gets isolated;
whether the managed Sunshine instance can bind only to the direct-link
address, or needs firewall-level enforcement instead, is one of the open
questions in `docs/UPSTREAM_INTEGRATION.md` (capture-backend/network
binding was not confirmed against Sunshine's actual source in this
research pass). No internet remote access in any planned phase covered by
this repository.

## Never include (in logs, diagnostics, or mDNS)

- Private keys or any pairing secret
- Sunshine/Moonlight credentials
- Clipboard contents
- Keystrokes
- Screen contents
- Unrelated browser/application data
- Full unrelated network configuration

`PlatformDiagnostics` (`crates/cabledesk-platform/src/report.rs`), the only
diagnostics type that exists today, structurally cannot violate this list —
it has no field capable of holding any of the above. This constraint
becomes load-bearing once `cabledesk-pairing`/`cabledesk-streaming` add
types that *could* hold secrets; any new diagnostics field must be checked
against this list before merging.

## SELinux and firewalld

Must work with SELinux enforcing and firewalld enabled — see
`docs/PACKAGING.md`'s hardening research and `data/selinux/README.md` for
the (not-yet-written) policy module plan. `scripts/dev-setup.sh` only
reports SELinux/firewalld status; it never changes either.

## Known limitations and gaps (honest accounting)

- No pairing protocol exists yet — anything above describing pairing is a
  plan, not a running system.
- No revocation/expiry for a trusted device beyond manual `Forget` — see
  `docs/THREAT_MODEL.md` T7.
- The exact crypto library choice (Phase 3) is not finalized — tracked in
  `docs/OPEN_QUESTIONS.md`.
- Polkit action `allow_active` defaults in
  `data/polkit-1/actions/org.cabledesk.Helper1.policy` are a starting
  point (`auth_admin_keep`), not a reviewed final decision — see that
  file's own comment and `docs/OPEN_QUESTIONS.md`.
- **`PrepareDirectLink` is Polkit-gated;** other reserved helper actions
  remain unused (OPEN_QUESTIONS §32).
- Phase 2 networking/discovery has never run against real
  Thunderbolt/USB4 hardware or a second machine — see
  `docs/HARDWARE_TEST_PLAN.md` and `docs/TEST_PLAN.md`.
