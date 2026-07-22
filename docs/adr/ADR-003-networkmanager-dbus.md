# ADR-003: Drive NetworkManager via its D-Bus API using `zbus`, not shell commands or a dedicated NM crate

## Status

Accepted (research-backed; not yet implemented — Phase 2).

## Context

CableDesk needs to programmatically create/activate a dedicated,
never-default-route NetworkManager profile for the direct-link interface,
and to react to that interface appearing/disappearing. Three approaches
exist: shelling out to `nmcli`, using a NetworkManager-specific Rust crate,
or talking to NetworkManager's D-Bus API directly. See
`docs/NETWORKING.md` §3 for full research.

Research found the NetworkManager-specific Rust crates (`networkmanager`,
`rusty_network_manager`, and similar) to be small, low-adoption, and in at
least one case explicitly marked under-maintained by its own author. `zbus`
itself, by contrast, is mature (v5.18.0 at research time, ~65M total
downloads, actively released) and is what this milestone already uses
successfully for NetworkManager/Avahi/firewalld/UPower/Polkit presence
checks in `cabledesk-platform-fedora::dependencies`.

## Decision

Use `zbus` directly against NetworkManager's documented D-Bus API
(`org.freedesktop.NetworkManager.Settings.AddConnection`/`AddConnection2`,
`org.freedesktop.NetworkManager.AddAndActivateConnection`,
`DeviceAdded`/`DeviceRemoved` signals) rather than shelling out to `nmcli`
or depending on a thin NetworkManager-specific crate. This also follows the
project's own engineering rule: "prefer typed D-Bus APIs over shell
commands; use shell commands only for early proof-of-concept experiments."

## Consequences

- No new dependency beyond `zbus`, already in the workspace.
- The profile is created with `ipv4.method=link-local` (or `ipv4.method=
  auto` + `ipv4.link-local=fallback` on NetworkManager ≥1.52) and
  `ipv4.never-default=yes`, plus `connection.interface-name` scoping it to
  the specific direct-link device — see `docs/NETWORKING.md` §3–4 for the
  exact settings.
- Interface lifecycle detection can reuse the same D-Bus connection
  already needed for profile management (`DeviceAdded`/`DeviceRemoved`
  signals) rather than adding a second, lower-level netlink/udev watcher —
  acceptable because NetworkManager is an assumed-present dependency for
  CableDesk (Fedora default), so there's no "what if NM isn't running"
  case to design around for MVP.
- Whether `ipv4.link-local` needs `avahi-autoipd` as a hard dependency
  alongside NetworkManager itself is still open — see
  `docs/OPEN_QUESTIONS.md` #3.
- `zbus`'s apparent repository move to `github.com/z-galaxy/zbus` should be
  double-checked before pinning a specific fork/source in `Cargo.toml` at
  Phase 2 implementation time (`docs/OPEN_QUESTIONS.md` #5) — this does not
  change the decision to use `zbus`, only which exact source/registry entry
  to depend on.
