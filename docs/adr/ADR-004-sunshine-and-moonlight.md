# ADR-004: Wrap Sunshine and Moonlight rather than building a custom streaming stack

## Status

Accepted (research-backed; not yet implemented — Phase 4).

## Context

CableDesk's core value is automation/integration ("plug in a cable, it
just works"), not novel streaming technology. Building a custom capture/
encode/transport/decode/render pipeline would be a multi-year effort
duplicating what [Sunshine](https://github.com/LizardByte/Sunshine)
(host: capture, encode, audio, input injection) and
[Moonlight](https://github.com/moonlight-stream/moonlight-qt) (client:
decode, present, audio, input forwarding) already do well, and are both
already used over an IP transport, which matches the `thunderbolt-net`
decision in ADR-002.

## Decision

Use CableDesk-managed, native (not Flatpak/Snap/AppImage) builds of
Sunshine and Moonlight-Qt as external processes, driven by a
`StreamingClient` adapter trait that isolates all Moonlight CLI knowledge
in one place (`crates/cabledesk-streaming`, not yet implemented). Do not
fork or embed `moonlight-common-c`, and do not modify Sunshine's own
config/account system — CableDesk gets its own dedicated config and
service (see `docs/UPSTREAM_INTEGRATION.md` for the exact managed-config
approach).

## Consequences

- **Sunshine** already publishes Fedora-installable channels (`dnf copr
  enable lizardbyte/stable`/`beta`, or GitHub-release RPM) and is fully
  configurable via plain-text `sunshine.conf`/`apps.json` with no need to
  touch its web UI — confirmed against current upstream docs/source (see
  `docs/UPSTREAM_INTEGRATION.md` §1, §3).
- **Moonlight-Qt** has no official Fedora RPM and its last tagged release
  is ~19 months stale with only nightly `master` builds continuing —
  CableDesk will likely need to build and pin a `master` commit rather
  than "the latest release" (`docs/OPEN_QUESTIONS.md` #11), with its own
  reproducibility/update-cadence process distinct from Sunshine's.
- Both projects are GPLv3. Redistributing a built Moonlight-Qt binary
  carries source-availability obligations CableDesk must honor (retain
  license, provide corresponding source, attribution) — see
  `docs/UPSTREAM_INTEGRATION.md` §8.
- The Moonlight-Qt CLI's exact current syntax for pairing, listing apps,
  starting a desktop stream, and stopping one, has been verified against
  actual current source (`commandlineparser.cpp`, `quitstream.cpp`,
  `startstream.cpp`), not old forum posts — recorded in
  `docs/UPSTREAM_INTEGRATION.md` §7, per the engineering rule against
  copying unverified commands.
- A concrete design consequence: `moonlight quit` does not signal a
  running Moonlight stream process (it's a separate process calling
  Sunshine's HTTP API) — `StreamingClient::stop` should kill CableDesk's
  own child process directly rather than shell out to `moonlight quit`
  (`docs/OPEN_QUESTIONS.md` #12).
- Capture-backend selection (KMS vs. XDG portal/PipeWire) is deferred to
  ADR-009 — it's a Sunshine-host-side decision independent of this ADR's
  "use Sunshine" decision.
