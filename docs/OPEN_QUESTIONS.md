# Open Questions

Consolidated from every research document plus implementation-level
decisions deferred during this milestone. Each item notes what's needed to
resolve it. Detailed sourcing/citations live in the linked document — this
page is the index, not a duplicate.

## Networking (full detail: `docs/NETWORKING.md`)

1. **Does an unauthorized (`user`/`secure` Thunderbolt security level) cable
   connection block the `thunderbolt0` interface from appearing at all, or
   does it appear immediately while only PCIe-tunneled peripherals wait on
   `boltd` authorization?** No source gives an unambiguous answer. *Needs:*
   real two-machine hardware test with security level set to `user`,
   observing `ip link` before/after `boltctl authorize`.
2. **Interface-naming stability** (`thunderbolt0`, `thunderbolt1`, ...)
   across multiple controllers, hot-plug/re-plug, and reboot ordering.
   *Needs:* real multi-port hardware.
3. **Whether NetworkManager's `ipv4.method=link-local` is fully
   self-contained or depends on `avahi-autoipd` being installed.** Affects
   whether `avahi-autoipd` is a hard runtime dependency. *Needs:* a current
   NetworkManager source read or a real link-local activation trace.
4. **Actual RFC 3927 link-local assignment latency on a real Thunderbolt/
   USB4 link.** Only generic RFC timing was sourced. *Needs:* measurement
   on real hardware — directly affects perceived "plug in, it just works"
   latency.
5. **`zbus`'s repository apparently moved to `github.com/z-galaxy/zbus`** —
   surfaced via crates.io metadata, not independently corroborated. *Needs:*
   a check against docs.rs/crates.io owner info before treating this as
   settled (does not block using the crate, which is unambiguously the
   right dependency either way — see `docs/NETWORKING.md` §"Recommendation").

## Power delivery (full detail: `docs/POWER_DELIVERY.md`)

6. **How a `/sys/class/usb_power_delivery/pdX` object cross-references back
   to its owning `/sys/class/typec/portY` and `/sys/class/power_supply/*`
   entry.** *Needs:* walking a live sysfs tree (`udevadm info -a`) on real
   UCSI hardware — the ABI docs describe each class separately, not the
   linkage.
7. **Whether `/sys/class/usb_power_delivery/` exists on all relevant
   hardware**, or only the older `/sys/class/typec/` + `power_supply.
   usb_type` attributes. *Needs:* treat as optional and probe for it —
   already reflected in `cabledesk-platform-fedora::power`, which only
   reads `power_role`/`usb_power_delivery_revision` from the `typec` class
   today, not the newer PD class.
8. **Whether the Type-C port used for CableDesk's `thunderbolt-net` link is
   straightforwardly identifiable as the same port whose PD attributes are
   being read** — i.e. can CableDesk say "*this cable* is charging you" vs.
   just "*a* USB-C port is." *Needs:* real hardware with both a
   Thunderbolt/USB4 controller and UCSI power reporting (the machine this
   milestone was built on has UCSI power reporting but no Thunderbolt
   controller, so this couldn't even be probed).
9. **UPower `PropertiesChanged` emission cadence** (every tick vs. only on
   state transitions) — affects whether CableDesk should poll as a
   fallback. *Needs:* empirical measurement.

## Sunshine/Moonlight integration (full detail: `docs/UPSTREAM_INTEGRATION.md`)

10. **No hands-on build/run verification was possible** — everything in
    that document is sourced from docs/source reading, not from actually
    building Sunshine/Moonlight-Qt or streaming. Specific unresolved items:
    - Whether the GNOME portal capture permission prompt can be
      pre-approved out-of-band (KDE has a documented bypass; no GNOME
      equivalent was found — may not exist, or may be undocumented).
    - Whether Sunshine's `prep-cmd` `undo` hooks run reliably whether
      CableDesk kills its Moonlight child process directly vs. going
      through `moonlight quit` (which itself doesn't signal the running
      stream process — see finding below).
    - Whether `qmake6 moonlight-qt.pro && make release` on current Fedora
      actually produces a working binary, given Moonlight-Qt's tagged
      release is ~19 months stale and `master` may have drifted.
    - Exact current `dnf copr enable lizardbyte/stable` availability for
      whichever Fedora version CableDesk targets at implementation time.
11. **Moonlight-Qt release cadence vs. Sunshine's** — Sunshine ships
    regular stable releases; Moonlight-Qt's last tag is from 2024-09-17
    with only nightly `master` builds since. *Implication:* CableDesk's
    Moonlight-Qt packaging likely needs to pin a `master` commit, not "the
    latest release," with different reproducibility/update implications
    than Sunshine's packaging.
12. **`moonlight quit` does not signal a running stream process** — it's a
    separate process that calls Sunshine's HTTP API. CableDesk should kill
    its own child process directly rather than shelling out to `moonlight
    quit`. This is a finding, not an open question, but is recorded here
    because it changes the planned `StreamingClient::stop` implementation
    from what a naive CLI reading would suggest.
13. **`encoder = vulkan` / `capture = kwin` are recent Sunshine additions**
    not present in older doc snapshots. *Needs:* verify against whichever
    exact Sunshine version CableDesk ends up pinning, not against
    current/latest docs.

## Packaging and hardening (full detail: `docs/PACKAGING.md`)

14. **`docs.fedoraproject.org` Rust/Packaging Guidelines pages were blocked
    by anti-bot protection during automated research** — claims rest on
    search-result snippets and secondary sources. *Needs:* a human (or a
    non-bot-blocked fetch) load the canonical pages directly and diff
    against `docs/PACKAGING.md` §1 before treating it as final.
15. **No explicit current Fedora policy document states a clear rule for
    `%caps()` vs. Polkit-mediated D-Bus actions.** CableDesk's choice
    (Polkit, per `docs/SECURITY.md`) is reasoned, not citation-backed by a
    single authoritative page.
16. **firewalld's current D-Bus interface for policy objects (as distinct
    from zones) is only partially disambiguated** — `.direct` and
    `.policies` are confirmed deprecated, but the exact replacement wasn't
    pinned down. *Needs:* `busctl introspect org.fedoraproject.FirewallD1
    /org/fedoraproject/FirewallD1` on real current firewalld.
17. **Realistic scope of the SELinux policy module for `cabledesk-helper`**
    is an estimate, not based on an existing similar package's actual `.te`
    file. *Needs:* inspecting real `selinux-policy-devel` interface files
    and, once privileged actions exist, `sealert`/`audit2allow` against a
    real running helper in enforcing mode.
18. **Exact `CapabilityBoundingSet`/`SystemCallFilter` minimum sets** for
    `cabledesk-helper` (`data/systemd/system/cabledesk-helper.service`
    currently ships an empty bounding set). Phase 2's real actions
    (`prepare_direct_link`, `install_firewall_policy`) now exist in
    `cabledesk-platform-fedora`, and — unlike kernel module loading —
    neither one obviously needs a new Linux capability of its own: both
    are pure D-Bus calls to NetworkManager/firewalld, which do the
    privileged work themselves. This is a reasoned expectation, not a
    verified one — still needs `systemd-analyze security` plus
    `strace`/audit against the helper unit actually running these calls
    (which hasn't happened — see item 26) before treating the empty
    bounding set as confirmed sufficient.
19. **rpmautospec applicability** — whether CableDesk ends up building
    through Fedora's dist-git+koji pipeline (where `%autorelease`/
    `%autochangelog` are the current default) or as a standalone COPR/
    self-hosted project changes the correct changelog convention. The spec
    currently uses a conventional hand-written `%changelog`, deliberately
    not `%autochangelog`, until this is decided — see the spec file's own
    comment.
20. **RPM spec was not build-tested** — `rpmbuild`/`rpmlint` are not
    installed in the environment this milestone was built in, and
    installing them wasn't done without asking first (per the project's
    own rule about confirming system changes). *Needs:* run
    `./scripts/dev-setup.sh` (which will ask before installing
    `rpm-build`/`rpmlint`) then `./scripts/build-rpm.sh` on a real Fedora
    machine.

## Implementation-level decisions deferred this milestone

21. **Cryptography library choice for device identity/pairing** (Phase 3)
    — `docs/SECURITY.md` names `ring`/`rustls`-adjacent crates as
    candidates but this is not finalized. Needs an ADR before
    `cabledesk-pairing` implementation starts.
22. **Secret storage backend** — GNOME Keyring vs. Secret Service vs. a
    plain owner-only-permissions file, for long-term key material (Phase
    3). Not decided.
23. **Polkit `allow_active` defaults** in
    `data/polkit-1/actions/org.cabledesk.Helper1.policy` are currently
    `auth_admin_keep` as a starting point. Whether these single-user
    workstation actions actually warrant `auth_self_keep` instead (the
    user's own password, not an administrator's) should be revisited
    before relying on these actions in production — see that policy
    file's own comment.
24. **`UserFacingError`'s secret-redaction depth** — the current unit test
    (`error::tests::user_facing_never_leaks_debug_formatting`) only checks
    that one specific error variant's headline doesn't contain a specific
    test string; it is not a general guarantee that no future
    `CableDeskError` variant could leak sensitive detail through its
    `Display`/mapping. Worth a stronger invariant (e.g. a lint or trait
    bound) once more error variants with genuinely sensitive payloads
    exist (Phase 3+).
## Phase 2 implementation (networking/discovery)

26. **`prepare_direct_link` and `install_firewall_policy` (`cabledesk-
    platform-fedora`) have never been invoked against a live system** —
    they're real, compiled, working D-Bus code (and `cabledesk-agent` now
    really does drive its state machine from live NetworkManager hotplug
    events, and `cabledeskctl repair-network` really does call
    `prepare_direct_link` when a direct-link interface is detected), but
    the *mutating* branch of that call path has never actually executed —
    this milestone's hardware has no direct-link interface, so
    `repair-network` always takes the honest "nothing to repair" path
    instead. Also, neither method is reachable from `cabledesk-helper`'s
    own D-Bus surface yet — only from a separate process calling
    `FedoraBackend` directly (`cabledeskctl`). *Needs:* real
    Thunderbolt/USB4 hardware, a decision on how the helper exposes these
    as D-Bus methods (with Polkit gating — see `data/polkit-1/actions/
    org.cabledesk.Helper1.policy`), and then an actual end-to-end test.
27. **Resolved:** route-table validation is now implemented
    (`cabledesk_network::validate::route_resolves_via_interface`, via
    `rtnetlink`'s kernel FIB lookup) and tested live (127.0.0.1 resolves
    via `lo`). Not yet exercised for an actual direct-link interface,
    since none exists in this milestone's environment — real hardware
    would confirm the same code path against `thunderbolt0`-style names.
28. **Resolved:** `ResolveService` is now implemented
    (`AvahiClient::resolve`), its exact D-Bus signature confirmed against
    Avahi's own upstream interface XML, and exercised live end-to-end
    (publish → browse → resolve → assert the resolved record matches →
    withdraw).
29. **NetworkManager's `AddConnection` reply/error shape under real
    failure conditions is unverified** — `docs/NETWORKING.md` documents
    the method signature from NetworkManager's own D-Bus reference, but
    this milestone never called it against a live bus (see item 26), so
    behavior on a duplicate profile, an invalid interface name, or a
    permissions failure is understood from documentation only, not
    observed.
30. **`cabledesk-agent`'s hotplug watcher has never seen a real
    direct-link interface appear or disappear** — the `DirectLinkEvent`
    stream, the `CableDetected`/`WaitingForCable` transitions it drives,
    and the reconnect-on-failure loop are all real code, verified to
    start up correctly and report `WaitingForCable` on a hardware-less
    machine, but the actual "cable plugged in, state changes" path is
    unverified — same underlying gap as item 26, just from the
    `cabledesk-agent` side rather than `cabledeskctl repair-network`'s.

## Other deferred decisions

25. **Whether `cabledesk-agent` should be D-Bus-activated** (started
    on-demand by dbus-daemon) **rather than started directly by systemd
    with `Type=dbus`** (the current choice, see
    `data/systemd/user/cabledesk-agent.service`) — both are valid; this
    milestone picked the simpler always-running-service model since the
    agent is meant to be always available once a session starts. Revisit
    if startup-time/resource-usage data suggests otherwise.
31. **`cabledeskctl status`'s D-Bus client is hand-written locally in
    `crates/cabledeskctl/src/main.rs` rather than sharing a proxy
    definition with `cabledesk-agent`** — fine at one caller, but if a
    second caller needs the same `org.cabledesk.Agent1` proxy (e.g. the
    GTK UI, per item 25's cross-reference), this should move to a shared
    location (perhaps `cabledesk-core` or a new thin crate) rather than
    being copy-pasted.
32. **CableDesk's own reserved Polkit actions
    (`org.cabledesk.helper.prepare-direct-link`,
    `org.cabledesk.helper.install-firewall-policy`) are not wired up or
    enforced anywhere.** `cabledeskctl repair-network` calls
    `prepare_direct_link`/`install_firewall_policy` directly as the
    logged-in user, not through `cabledesk-helper`, so authorization for
    these mutating operations currently comes entirely from
    NetworkManager's and firewalld's own Polkit policies, not
    CableDesk's — see `docs/SECURITY.md`, "An architectural gap this
    milestone surfaced." *Needs a decision*: either route these calls
    through `cabledesk-helper` with a real `CheckAuthorization` call
    before Phase 3, or explicitly retire these two reserved actions as
    redundant with NM's/firewalld's own gating — but decide deliberately,
    don't leave it implicit.
