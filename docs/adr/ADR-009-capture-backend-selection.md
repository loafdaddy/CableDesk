# ADR-009: Sunshine capture backend selection on Fedora GNOME/Wayland

## Status

**Proposed — not decided.** Research complete (`docs/UPSTREAM_INTEGRATION.md`
§2), hands-on validation not yet performed. This ADR is recorded now,
before Phase 4 implementation, specifically so the decision gets made
deliberately rather than defaulted into.

## Context

Sunshine on Linux supports multiple capture backends; the two relevant to
Fedora GNOME/Wayland (this project's Tier 1 target) are:

- **KMS**: requires `cap_sys_admin` (or root), works without any per-session
  permission prompt, but is incompatible with sandboxed packaging (moot for
  CableDesk per ADR-005) and needs its security implications understood
  before use in a Polkit/capability-scoped helper.
- **XDG desktop portal + PipeWire**: works across compositors, but requires
  a one-time on-screen permission grant that current research could not
  confirm is *permanently* bypassable on GNOME the way KDE documents a
  `flatpak permission-set`-style bypass for. If the grant can be invalidated
  by a crash, monitor hotplug, or DE restart, requiring the user to
  re-approve a portal prompt on every such event directly contradicts the
  "plug in cable, it just works" plug-and-play goal.

Sunshine also supports `wlroots`/KWin-specific capture paths and a newer
`vulkan` encoder + `kwin` capture combination (recent as of the 2026-era
release researched) — not directly relevant to GNOME.

## Decision

**Deferred.** Do not pick a default yet. Before Phase 4 implementation:

1. Do the Phase 0 hands-on comparison the original project plan calls for
   (initial permission prompt behavior, whether permission survives a
   restart/crash/monitor hotplug, cursor capture, multi-monitor behavior,
   behavior across screen lock/suspend) on real Fedora GNOME/Wayland
   hardware — none of this could be performed in this milestone's
   environment (no second machine to actually stream to, and no hands-on
   Sunshine build/run was performed at all — see
   `docs/OPEN_QUESTIONS.md` #10).
2. Specifically resolve whether a GNOME-side permanent-grant bypass exists
   (undocumented or genuinely absent) before assuming XDG/PipeWire can meet
   the plug-and-play requirement.
3. If XDG/PipeWire cannot meet it, KMS becomes the default for GNOME/
   Wayland, with its `cap_sys_admin` requirement designed into
   `cabledesk-helper`'s capability set deliberately (not as an
   afterthought) and documented plainly rather than downplayed — per the
   project's own rule: "if XDG/PipeWire requires a new permission prompt
   for every connection, it may not satisfy plug-and-play behaviour.
   Document this rather than hiding it."

## Consequences (once decided)

- Whichever backend is chosen becomes a `FedoraBackend`-level concern
  (likely surfaced through `cabledesk-streaming`'s Sunshine configuration
  management, Phase 4) — GNOME vs. KDE Plasma may end up with genuinely
  different defaults, which is fine (`docs/DISTRO_SUPPORT.md` already
  treats desktop environment as part of the support matrix, not just the
  distro).
- This decision directly affects `cabledesk-helper`'s required Linux
  capabilities (`data/systemd/system/cabledesk-helper.service`'s
  currently-empty `CapabilityBoundingSet` will need `cap_sys_admin` added
  if KMS is chosen) and its Polkit action set
  (`data/polkit-1/actions/org.cabledesk.Helper1.policy`) — both files are
  intentionally minimal right now specifically because this ADR isn't
  resolved yet.
