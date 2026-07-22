# CableDesk Packaging Research

Status: research notes for scaffolding the CableDesk RPM packaging. Compiled from
current (2025/2026-era) Fedora, RPM, Polkit, systemd, firewalld, and SELinux
documentation. Every claim below is sourced; anything that could not be
verified against an authoritative, current source is called out explicitly in
the "Open Questions" section instead of being asserted.

CableDesk architecture recap (for context on the recommendations below): an
unprivileged GTK4/libadwaita UI, an unprivileged user-session "agent"
(`systemd --user` service), and a privileged "helper" (system `systemd`
service) that performs narrow, Polkit-gated operations (kernel module loading,
NetworkManager profile creation, firewalld rule installation, USB4/Thunderbolt
inspection). This naturally maps to a multi-subpackage RPM layout (topic 1)
and drives the security-relevant design choices in topics 3-6.

---

## 1. Fedora RPM packaging for a Rust project

### 1.1 Toolchain: rust2rpm and the rust-packaging macros

Fedora's Rust SIG maintains **rust2rpm**, the standard tool for generating RPM
spec files for Rust crates/projects. It ships as a Fedora package and is
developed at `https://pagure.io/fedora-rust/rust2rpm`. The guidance from the
Fedora Rust packagers themselves is: "It is advisable to try `rust2rpm $crate`
first before attempting to write a specfile by hand"
([Rust Packaging Guidelines](https://docs.fedoraproject.org/en-US/packaging-guidelines/Rust/),
[rust2rpm man page](https://www.mankier.com/1/rust2rpm)).

Key mechanics:

- rust2rpm invocation: `rust2rpm [OPTION]... [pkgid]`, where `pkgid` can be a
  crate name, `name@version`, or (run from inside a checkout) infer the name
  from existing files. `-t/--target` selects the distro flavor (fedora,
  mageia, opensuse, plain); `-o` sets an output directory
  ([mankier rust2rpm](https://www.mankier.com/1/rust2rpm)).
- The generated spec relies on three macro files shipped by the
  **rust-srpm-macros** / **cargo-rpm-macros** packages: `macros.rust-srpm`
  (constructing source packages), `macros.rust` (default compiler flags via
  `%build_rustflags`), and `macros.cargo` (building with cargo)
  ([rust-srpm-macros - Fedora Packages](https://packages.fedoraproject.org/pkgs/rust-packaging/rust-srpm-macros/),
  [cargo-rpm-macros](https://codeberg.org/rust2rpm/cargo-rpm-macros)).
- Every Rust package **MUST** carry `BuildRequires: rust-packaging`, which is
  what auto-generates `Requires`/`Provides` metadata from the crate's
  `Cargo.toml`
  ([Rust Packaging Guidelines](https://docs.fedoraproject.org/en-US/packaging-guidelines/Rust/)).
- Crate (library) subpackages **MUST** be named `rust-$crate`; this does not
  apply to the top-level application package name itself (an application like
  CableDesk is named normally, e.g. `cabledesk`), but any of its dependencies
  that Fedora packages separately as libraries follow the `rust-$crate`
  convention.
- A standardized, mandatory set of compiler flags is applied via
  `%build_rustflags`, so a hand-rolled spec that calls `cargo build` directly
  without going through the macros will diverge from Fedora's expected
  hardening/reproducibility flags
  ([Rust Packaging Guidelines](https://docs.fedoraproject.org/en-US/packaging-guidelines/Rust/)).

### 1.2 Library-per-crate model vs. vendoring an application

Fedora's default and strongly preferred model is **one Fedora package per
crate** — every dependency in `Cargo.lock` is expected to already exist (or be
newly packaged) as its own `rust-$crate` RPM, and the application links
against those system-provided crates at build time. A walkthrough of
packaging a Rust program for Fedora confirms this in practice: "all crate
dependencies must be packaged separately as RPM packages," which is presented
as the norm rather than vendoring
([Packaging a Rust Program for Fedora](https://jrfernandez.com/packaging-rust-program-for-fedora/)).

For applications with large/deep dependency trees, this is frequently
impractical, so Fedora also has an official, but explicitly secondary,
**vendoring path**:

- `rust2rpm`'s `-V/--vendor` flag can build a vendor tarball automatically
  (`auto`) or expect a hand-maintained one (`manual`); default is `off`
  ([mankier rust2rpm](https://www.mankier.com/1/rust2rpm)).
- Official support for vendored-dependency builds was added in **version 25**
  of `cargo-rpm-macros`/`rust2rpm`. The `%cargo_prep` macro takes a
  `-v $VENDOR` argument pointing at a directory of vendored crates (typically
  produced by `cargo vendor`), and configures Cargo to build against that
  directory instead of the system crate registry
  ([rust-bundled-packaging / rust2rpm vendoring notes](https://github.com/amazonlinux/rust-bundled-packaging), corroborated by
  [Rust Packaging Guidelines](https://docs.fedoraproject.org/en-US/packaging-guidelines/Rust/)).
- **The Guidelines treat full vendoring as a last resort**: "Building
  exclusively from vendored dependencies... SHOULD only be a last resort,"
  reserved for cases like git-snapshot/forked dependencies that cannot be
  expressed as normal registry dependencies
  ([Rust Packaging Guidelines](https://docs.fedoraproject.org/en-US/packaging-guidelines/Rust/)).
- **Whenever any bundled/vendored crate is used, it MUST be declared** with a
  virtual `Provides: bundled(crate($crate)) = $version` in the subpackage that
  contains it. This exists specifically so Fedora's security/vulnerability
  tooling can find bundled copies of vulnerable crate versions
  ([Rust Packaging Guidelines](https://docs.fedoraproject.org/en-US/packaging-guidelines/Rust/)).
- Ensure a `Cargo.lock` is present in the source tarball (generate one via
  `cargo generate-lockfile` if the upstream project doesn't ship it, and add
  it as an extra `Source`) — required for reproducible dependency resolution
  ([Rust Packaging Guidelines](https://docs.fedoraproject.org/en-US/packaging-guidelines/Rust/)).

**Cargo workspace handling**: for a workspace, the relevant sub-crate is
selected with `cargo generate-rpm -p <path>` style invocation (from the
`cargo-generate-rpm` tool) or, in the Fedora macro world, by pointing
`%cargo_prep`/`%cargo_build`/`%cargo_install` at the right workspace member
directory (per package-level `Cargo.toml`)
([cargo-generate-rpm - crates.io](https://crates.io/crates/cargo-generate-rpm)).
Note `cargo-generate-rpm` is a separate, non-Fedora, self-contained
alternative tool (bundles/vendors by design) more commonly used by upstream
projects that want to ship their own `.rpm` without going through Fedora
proper — useful to know about but distinct from the `rust2rpm`/Fedora path
described above.

**Recommendation for CableDesk**: given three separate Rust binaries (UI,
agent, helper) sharing a workspace and internal crates, treat the *workspace*
as a single Fedora "application" source package (following the model in
§1.2), producing multiple binary subpackages (§1.4) from one `%build`. Prefer
the per-crate/registry model for any third-party dependency that is already
packaged in Fedora, and fall back to a vendor tarball (`%cargo_prep -v`) with
the mandatory `bundled(crate(...))` Provides only for the (hopefully small)
remainder — do not vendor the whole tree by default.

### 1.3 General spec conventions (license, doc, changelog)

- **`%license`**: Fedora requires the actual license text file(s) shipped by
  upstream (e.g. `LICENSE`, `COPYING`) to be marked with the `%license` macro
  in `%files`, not `%doc` — this is a mandatory Fedora Legal requirement, not
  cosmetic
  ([Fedora-packaging: Guidelines about %license](https://packaging.fedoraproject.narkive.com/Klx9pehC/fedora-guidelines-about-license)).
- **`%doc`**: ancillary docs (README, AUTHORS, NEWS, CHANGELOG, TODO) go in
  `%doc`; typical usage is a single line such as
  `%doc AUTHORS ChangeLog NEWS README THANKS TODO` alongside a separate
  `%license COPYING` line
  ([RPM Packaging Guide](https://rpm-packaging-guide.github.io/)).
  Note the two ways of installing into `%{_defaultdocdir}/%{name}-%{version}/`
  (via `%doc` in the spec, or installing directly into the doc dir under
  `$RPM_BUILD_ROOT` in `%install`) are mutually exclusive — mixing them causes
  `%doc` to wipe what was staged
  ([RPM Packaging Guide](https://rpm-packaging-guide.github.io/)).
- **Deprecated/forbidden tags**: current Fedora guidelines say `Copyright:`,
  `Packager:`, `Vendor:`, and `PreReq:` **MUST NOT** be used; `BuildRoot:`,
  `Group:`, and an explicit `%clean` section **SHOULD NOT** be used (modern
  rpmbuild handles these automatically)
  ([Fedora Packaging Guidelines search summary](https://fedoraproject.org/wiki/Archive:Vondruch/Draft_RawhideGuidelines)).
- **Changelog / Release — rpmautospec is now the default recommendation.**
  Rather than hand-maintaining `%changelog` and bumping `Release:` on every
  build, Fedora's current guidance is to set `Release: %autorelease` and put
  `%autochangelog` where `%changelog` used to go. `rpmautospec` then derives
  the release number from the count of git commits since the `Version:` field
  last changed, and synthesizes changelog entries from git commit summaries
  (commits can opt out with `[skip changelog]`). This is now described as the
  default/recommended approach in current Packaging Guidelines and Fedora
  infrastructure
  ([rpmautospec docs](https://fedora-infra.github.io/rpmautospec-docs/),
  [Changes/Rpmautospec by Default](https://fedoraproject.org/wiki/Changes/Rpmautospec_by_Default),
  [Use rpmautospec in Fedora Linux — Fedora Magazine](https://fedoramagazine.org/use-rpmautospec-in-fedora-linux/)).
  This only works cleanly for git-based Fedora dist-git workflows; a
  standalone/upstream spec that isn't built via Fedora's koji+dist-git
  pipeline should keep a conventional hand-written `%changelog` (or adopt
  rpmautospec only once CableDesk has real Fedora dist-git packaging).

### 1.4 Subpackage structure (main / helper / agent / selinux / debuginfo)

Given CableDesk's three-binary, privilege-separated design, a natural RPM
layout is:

- **`cabledesk`** (main package): the GTK4/libadwaita UI binary, `.desktop`
  launcher, icon(s), and AppStream metainfo. Fedora Workstation guidelines
  require GUI apps to ship AppStream metadata: "If a package contains a GUI
  application, then it SHOULD install a `.appdata.xml`/metainfo file into
  `%{_metainfodir}`... MUST follow the AppData specification," validated with
  `appstream-util validate-relax`; the `.desktop` file must be checked with
  `desktop-file-install`/`desktop-file-validate` (`BuildRequires:
  desktop-file-utils`); launcher icons must be 128×128 with an alpha channel
  and a matching high-contrast variant
  ([Fedora AppData/Workstation guidelines search summary](https://fedoraproject.org/wiki/Workstation/Guidelines/Applications_and_Launchers)).
- **`cabledesk-agent`**: the unprivileged user-session agent binary plus its
  `systemd --user` unit (installed under `%{_userunitdir}`), pulled in as a
  `Requires` of the main package (or vice versa, depending on which is
  considered "primary").
- **`cabledesk-helper`**: the privileged system-service binary, its system
  `systemd` unit (`%{_unitdir}`), and the Polkit `.policy` action definition
  (`%{_datadir}/polkit-1/actions/`). This subpackage should carry
  `%post`/`%preun` scriptlets for `systemctl` daemon-reload/enable and for
  Polkit/dbus service activation files as needed.
- **`cabledesk-selinux`**: optional custom SELinux policy module (see §6 in
  the security section below) — kept separate so systems that don't run
  SELinux enforcing (rare on Fedora, but the guideline exists precisely to
  avoid an unconditional `Requires: selinux-policy`) aren't forced to pull it
  in.
- **`cabledesk-debuginfo`** / **`cabledesk-debugsource`**: these are
  **auto-generated by rpmbuild**, not hand-written. Since Fedora 27
  (`Changes/SubpackageAndSourceDebuginfo`), the build toolchain
  (`redhat-rpm-config`'s `/usr/lib/rpm/find-debuginfo.sh`) automatically splits
  debug symbols into a per-subpackage `-debuginfo` package and a separate
  `-debugsource` package, enabling "parallel installable debuginfo" (so
  `cabledesk-helper-debuginfo` and `cabledesk-agent-debuginfo` can coexist and
  be installed independently). The `%_debugsource_packages` macro controls
  debugsource generation if tuning is needed; in the normal case no manual
  spec work is required beyond not disabling the mechanism
  ([Changes/SubpackageAndSourceDebuginfo](https://fedoraproject.org/wiki/Changes/SubpackageAndSourceDebuginfo),
  [Mark Wielaard: Fedora rpm debuginfo improvements](https://gnu.wildebeest.org/blog/mjw/2017/06/30/fedora-rpm-debuginfo-improvements-for-rawhidef27/)).

### 1.5 Reproducible builds

Fedora has been actively pushing package build reproducibility:

- **Fedora 38** (`Changes/ReproducibleBuildsClampMtimes`, 2023): clamps file
  mtimes to `$SOURCE_DATE_EPOCH` (and makes `.pyc` generation respect it too),
  so mtimes packaged in the RPM no longer depend on wall-clock build time
  ([Changes/ReproducibleBuildsClampMtimes](https://fedoraproject.org/wiki/Changes/ReproducibleBuildsClampMtimes)).
- **Fedora 41** (`Changes/ReproduciblePackageBuilds`, 2024): introduces
  `add-determinism`, a Rust-based tool run during builds that normalizes
  further common sources of non-determinism in built binaries/archives
  ([Changes/ReproduciblePackageBuilds](https://fedoraproject.org/wiki/Changes/ReproduciblePackageBuilds),
  [Fedora 41 More Reproducible RPM — Phoronix](https://www.phoronix.com/news/Fedora-41-More-Reproducible-RPM)).
- **RPM 4.20** adds a `%build_mtime_policy` macro with selectable policies
  (clamp to `$SOURCE_DATE_EPOCH` or to build time), superseding/deprecating
  the older `%clamp_mtime_to_source_date_epoch` macro
  ([RPM 4.20.0 Release Notes](https://rpm.org/releases/4.20.0)).
- As of the relevant LWN coverage, Fedora reports roughly **90% of package
  builds are reproducible**, with remaining gaps being package-specific rather
  than systemic
  ([Fedora change aims for 99% package reproducibility — LWN](https://lwn.net/Articles/1014979/)).

Practically for CableDesk: build via the standard Fedora `rpmbuild`/`mock`/
`koji` pipeline (which applies these clamps and `add-determinism`
automatically) rather than a custom non-Fedora build pipeline, to inherit
reproducibility "for free," and avoid embedding build-host-specific paths,
timestamps, or hostnames into any of the three binaries (e.g. avoid `env!()`
build-time timestamps in Rust; prefer git commit hash which is stable per
commit).

---

## 2. RPM file capabilities (`%caps`) vs setuid vs Polkit

### 2.1 Mechanics

RPM has a built-in macro, `%caps()`, used inside a `%files` attribute
specification to assign Linux file capabilities to a binary at package-install
time, instead of (or in addition to) traditional owner/mode via `%attr`.
Example from Fedora's own `iputils` spec (for `ping`):

```
%attr(0755,root,root) %caps(cap_net_raw=ep cap_net_admin=ep) %{_bindir}/ping
```

and from `arping`:

```
%attr(0755,root,root) %caps(cap_net_raw=p) %{_bindir}/arping
```

([rpm.org spec file format docs](https://rpm.org/docs/4.20.x/manual/spec.html),
search-verified against real Fedora specs via
[A crop of new capabilities — LWN](https://lwn.net/Articles/822771/) and
[setcap explainer](https://linux-audit.com/system-administration/commands/setcap/)).

This is RPM's declarative equivalent of running `setcap` by hand after
install; using `%caps()` is preferred because rpmbuild embeds the capability
metadata in the package payload itself (so `rpm -V` can verify it, and the
capability is applied consistently on every install), whereas a `%post`
script calling `setcap` manually is more fragile and invisible to RPM's own
verification. One practical wrinkle: **building** a package that sets file
capabilities generally requires the builder to have `CAP_SETFCAP` — plain
non-root `rpmbuild` runs (e.g. in unprivileged mock chroots without
`fakeroot`-with-capability-support configured, or copr in some
configurations) can fail to apply capabilities correctly, which is a known
practical gotcha documented by third parties
([How to Build an RPM That Sets POSIX File Capabilities — linuxvox.com](https://linuxvox.com/blog/making-an-rpm-which-sets-posix-files-capabilities/)).
The syntax accepts the standard capability-set flags (`e`=effective,
`p`=permitted, `i`=inheritable), e.g. `cap_net_admin=ep`.

### 2.2 When file capabilities are appropriate vs. Polkit-mediated actions

No single current Fedora document was found that states an explicit policy
distinguishing "use `%caps()`" from "use Polkit" as a general rule (flagged
in Open Questions). Reasoning from how Fedora actually uses each mechanism in
its own packages, plus first-principles security tradeoffs:

- **File capabilities (`%caps`)** grant the capability to *any* invocation of
  that binary by *any* local user, unconditionally and persistently (as long
  as the file exists with that capability set) — there is no per-call
  authorization, auditing, or policy decision at call time. This is
  appropriate for small, narrowly-scoped, stateless system utilities that
  legitimately need one specific capability for every invocation regardless
  of caller identity (`ping`/`arping` needing `CAP_NET_RAW` is the canonical
  example: every user is *supposed* to be able to ping).
- **Polkit-mediated actions** are appropriate when the operation should be
  gated by *who* is asking, *session/seat* context, whether the user is
  active/local, and possibly an interactive authentication prompt — and
  where the decision needs to be auditable/configurable by an admin
  (`/etc/polkit-1/rules.d/*.rules` can implement arbitrary logic, e.g. "only
  allow if the user is in the wheel group and this is a local active
  session"). This is a categorically different security model: it's an
  authorization *decision per call*, not a blanket grant baked into the
  file's inode.
- **CableDesk's helper is the Polkit case, not the `%caps` case**: the
  operations described (loading kernel modules, creating NetworkManager
  profiles, installing firewalld rules, inspecting USB4/Thunderbolt hardware)
  are precisely the kind of "any logged-in desktop user should be able to do
  this for their own hardware, but it should go through an auditable,
  admin-overridable authorization check, and ideally prompt for
  authentication for anything destructive" operations Polkit exists for. A
  bare `%caps(cap_sys_module=ep)` on the helper binary would grant *any*
  local user the ability to load arbitrary kernel modules with zero
  authorization check, which is a materially larger blast radius than a
  Polkit-gated D-Bus action — this is why the security section below (topic
  3/4) centers on Polkit + a narrowly capable systemd service, not on file
  capabilities as the primary access-control mechanism. File capabilities may
  still be useful *inside* the helper's own hardening (i.e., what
  `CapabilityBoundingSet=` the systemd unit is allowed to retain), which is
  a related but distinct use of the same underlying kernel mechanism — see
  §4 below.

---

## Recommendation Summary (Topics 1-2)

1. Use the Fedora Rust SIG toolchain (`rust2rpm` + `rust-srpm-macros` /
   `cargo-rpm-macros`) rather than a hand-rolled spec or `cargo-generate-rpm`,
   so CableDesk inherits Fedora's standard Rust build flags, reproducibility
   behavior, and dependency-vulnerability tracking.
2. Package the Cargo workspace as one source package producing
   `cabledesk` (UI), `cabledesk-agent`, `cabledesk-helper`, and an optional
   `cabledesk-selinux` subpackage; let rpmbuild auto-generate
   `-debuginfo`/`-debugsource` packages per binary subpackage.
3. Prefer per-crate Fedora-packaged dependencies; vendor only the minimum
   necessary with `%cargo_prep -v` plus mandatory `bundled(crate(...))`
   Provides, per the Guidelines' "last resort" framing.
4. Use `%license`/`%doc` correctly and adopt `%autorelease`/`%autochangelog`
   once/if CableDesk is built through Fedora's dist-git+koji pipeline;
   otherwise keep a conventional hand-written `%changelog`.
5. Do **not** rely on `%caps()` file capabilities as the access-control
   mechanism for the privileged helper's sensitive operations — those should
   be Polkit-gated D-Bus actions (see security section). Reserve
   `%caps()`/`CapabilityBoundingSet` for constraining what the already-narrow
   helper *process* itself retains after a Polkit check has passed, not as
   a substitute for the authorization check.

---

## Security-Relevant Packaging & Hardening Research (for SECURITY.md)

*(This section is intentionally self-contained research output for another
part of the project to fold into `docs/SECURITY.md`. This file does not
itself modify `SECURITY.md`.)*

### 3. Polkit architecture (current, 2024/2025-era)

**`.pkla`/Local Authority is legacy; JS rules are current.** Polkit moved from
the old "local authority" `.pkla` file format (`.desktop`-like syntax under
`/etc/polkit-1/localauthority/...` or `/var/lib/polkit-1/localauthority/...`)
to a JavaScript-based rules engine. Current guidance (confirmed via Debian's
release-notes bug tracking the same upstream Polkit transition Fedora also
went through) is that administrators override/extend policy by dropping
`.rules` files (JavaScript) into `/etc/polkit-1/rules.d/*.rules`; a
compatibility shim package exists for old `.pkla` files but is being phased
out
([Debian bug #1033511 on the pkla→JS rules transition](https://bugs.debian.org/cgi-bin/bugreport.cgi?bug=1033511),
[pkla-check-authorization legacy tool docs](https://www.linux.org/docs/man8/pkla-check-authorization.html)).
For CableDesk, this means: ship a `polkit.action` policy file defining action
IDs and their default authorization levels (see below) — do **not** ship or
recommend `.pkla` rules for anything.

**Action definitions vs. runtime authorization checks are two different
files/mechanisms:**

1. **Action IDs** are declared in static XML files under
   `/usr/share/polkit-1/actions/*.policy` (installed by the package). Each
   `<action id="...">` declares default authorization results for
   `<allow_any>`, `<allow_inactive>`, `<allow_active>` (none /
   auth_self / auth_admin / auth_self_keep, etc.), plus a human-readable
   description/message and an icon
   ([Writing polkit applications — polkit Reference Manual](https://www.freedesktop.org/software/polkit/docs/latest/polkit-apps.html)).
2. **Runtime authorization checks** are performed by the privileged service
   (CableDesk's helper) at the moment a D-Bus call comes in, via the
   `org.freedesktop.PolicyKit1.Authority` D-Bus interface's
   **`CheckAuthorization`** method, exposed at
   `/org/freedesktop/PolicyKit1/Authority` on the well-known bus name
   `org.freedesktop.PolicyKit1` on the **system bus**. The method signature
   is `CheckAuthorization(Subject subject, String action_id, Dict<String,
   String> details, CheckAuthorizationFlags flags, String cancellation_id)
   -> AuthorizationResult`. `flags` is typically `None` (0) for a
   non-interactive check or `AllowUserInteraction` (1) to permit an
   authentication prompt. The returned `AuthorizationResult` embeds an
   enum-like state: `NotAuthorized` (0), `AuthenticationRequired` (1),
   `AdministratorAuthenticationRequired` (2), `AuthenticationRequiredRetained`
   (3), `AdministratorAuthenticationRequiredRetained` (4), `Authorized` (5)
   ([org.freedesktop.PolicyKit1.Authority Interface — polkit reference](https://www.freedesktop.org/software/polkit/docs/latest/eggdbus-interface-org.freedesktop.PolicyKit1.Authority.html)).
   The `Subject` is typically constructed as a "system-bus-name" subject
   using the D-Bus unique/well-known name of the calling process, which
   polkit resolves to a PID/UID/start-time internally to prevent PID-reuse
   spoofing. `pkcheck(1)` is the CLI wrapper around this same call, useful
   for manual testing during development
   ([DBus and Polkit Introduction blog, 2025](https://u1f383.github.io/linux/2025/05/25/dbus-and-polkit-introduction.html)).

**This is exactly the "D-Bus-activated system service" pattern**, not the
`pkexec` GUI-launch pattern: CableDesk's helper is a long-running system
service (not spawned per-call via `pkexec`), so it should call
`CheckAuthorization` itself for each incoming privileged D-Bus method call
from the agent, rather than relying on `pkexec` to have already gated entry.
This is the standard architecture used by e.g. NetworkManager, firewalld, and
`systemd-hostnamed`/`systemd-timedated` — a persistent system-bus service
that self-checks each request against a polkit action ID.

**Rust crate for this**: `zbus_polkit` (crates.io, MIT-licensed) is a
maintained, `zbus`-based binding specifically for talking to polkit over
D-Bus. At time of research it was at **version 5.0.0** (released
2024-10-21), with a healthy release cadence (16 published versions, a 4.0.0
release earlier in 2024) and substantial usage (~165k downloads/month),
repository at `https://github.com/dbus2/zbus_polkit`
([zbus_polkit on lib.rs](https://lib.rs/crates/zbus_polkit),
[zbus_polkit on crates.io](https://crates.io/crates/zbus_polkit)). `zbus`
itself is a "100% Rust-native" D-Bus implementation and is the general-purpose
D-Bus crate `zbus_polkit` builds on
([zbus on lib.rs](https://lib.rs/crates/zbus)). Given it wraps the exact
`CheckAuthorization` D-Bus call described above with typed Rust structs, it
is preferable to hand-rolling raw `zbus` proxy calls against the
`org.freedesktop.PolicyKit1.Authority` interface, unless a specific typed API
gap is found during implementation.

### 4. systemd service hardening for the privileged helper

The helper needs real privilege (kernel module loading, network
configuration) but should be sandboxed as tightly as those needs allow, and
validated with `systemd-analyze security <unit>`, which scores a unit's
exposure from 0 (fully locked down) to 10 (dangerously open) across every
sandboxing directive
([Systemd service hardening — ruderich.org](https://ruderich.org/simon/notes/systemd-service-hardening);
`systemd-analyze security` behavior corroborated across multiple current
hardening guides, e.g.
[RockyLinux 10 Systemd Units Hardening docs](https://docs.rockylinux.org/10/guides/security/systemd_hardening/)).

Directives relevant to CableDesk's helper, and how the "needs to load kernel
modules and manage NetworkManager profiles" requirement interacts with each:

- **`NoNewPrivileges=yes`**: prevents the process (and children) from gaining
  privileges via setuid/setcap/fscaps at `exec()` time. Safe and recommended
  unconditionally for the helper, since it does not need to exec setuid
  helpers itself — it already runs with the capabilities it needs from
  process start
  ([systemd hardening overview](https://ruderich.org/simon/notes/systemd-service-hardening)).
- **`ProtectSystem=strict`** (+ `ProtectHome=yes` or `read-only`): mounts most
  of the filesystem read-only for the unit. This is compatible with the
  helper's job (module loading and NM/firewalld configuration go through
  kernel syscalls and D-Bus calls to NetworkManager/firewalld, not direct
  filesystem writes to `/etc`), but if the helper needs to write anything
  itself (e.g. a state file, a firewalld zone XML it manages directly rather
  than via D-Bus), that specific path needs an explicit `ReadWritePaths=`
  exception carved out of the otherwise-strict read-only root
  ([Systemd hardening baseline, "highest-value four" being NoNewPrivileges,
  ProtectSystem=strict, PrivateTmp, tight CapabilityBoundingSet](https://ruderich.org/simon/notes/systemd-service-hardening)).
- **`CapabilityBoundingSet=`**: should be pared down to only what's actually
  needed rather than left at the systemd default (full set for a system
  service run as root). For a helper that loads kernel modules, at minimum
  `CAP_SYS_MODULE` is required (there is no finer-grained capability for
  `init_module()`/`finit_module()`); NetworkManager profile creation over
  D-Bus does **not** itself require extra capabilities on the *caller* side
  (NetworkManager, running as its own privileged service, does the actual
  privileged network configuration — the helper is just a D-Bus client to
  NM), so the helper should not need `CAP_NET_ADMIN` merely to talk to NM
  over D-Bus, only if it also directly manipulates network state itself
  (rtnetlink, etc.) beyond what it delegates to NM. A conservative bounding
  set like `CapabilityBoundingSet=CAP_SYS_MODULE` (plus whatever is proven
  necessary for firewalld/USB4 inspection, e.g. none for pure D-Bus/sysfs
  reads) is the target; everything else should be explicitly dropped, e.g.
  via `CapabilityBoundingSet=~CAP_SYS_ADMIN` style exclusions for anything
  found unnecessary during `systemd-analyze security` iteration
  ([CapabilityBoundingSet hardening explainer, incl. the
  `CapabilityBoundingSet=~CAP_SYS_MODULE` example for the *inverse* case of a
  service that must NOT load modules](https://ruderich.org/simon/notes/systemd-service-hardening)).
- **`PrivateNetwork=`**: **must be left unset/false** for this unit — this
  directive gives a service its own private, isolated network namespace with
  no interfaces (only loopback), which would be directly incompatible with a
  helper whose entire purpose is configuring the *host's* real network
  interfaces/NetworkManager profiles and firewalld rules. This is called out
  explicitly because it is one of the highest-value hardening directives for
  most services but is a hard non-starter for CableDesk's helper.
- **`RestrictAddressFamilies=`**: rather than leaving all address families
  open, restrict to what's actually used — e.g. `AF_UNIX` (D-Bus, local
  sockets) and `AF_NETLINK` (needed if the helper talks rtnetlink directly for
  Thunderbolt/USB4 enumeration or any direct network configuration outside of
  NM's D-Bus API); avoid `AF_INET`/`AF_INET6` unless the helper itself opens
  IP sockets (likely not needed if all network config is delegated to
  NetworkManager/firewalld over D-Bus)
  ([RestrictAddressFamilies setting explainer — linux-audit.com](https://linux-audit.com/systemd/settings/units/restrictaddressfamilies/)).
- **`SystemCallFilter=@system-service`**: a reasonable curated baseline
  syscall allow-list for most system services; combine with explicit
  `SystemCallFilter=~@privileged @resources` style *removals* for classes
  the helper doesn't need, tuned iteratively against `systemd-analyze
  security` and real integration testing (module loading requires the
  `init_module`/`finit_module` syscalls, which are part of `@module` in
  systemd's syscall filter groups and would need to remain allowed)
  ([systemd hardening general guidance incl. `@system-service` as baseline](https://ruderich.org/simon/notes/systemd-service-hardening)).
- **`DeviceAllow=`**: default-deny device-node access
  (`DevicePolicy=closed`/`strict` + explicit `DeviceAllow=` entries) for
  anything the helper needs to touch directly for USB4/Thunderbolt inspection
  (if that requires device-node access beyond sysfs reads, which are
  filesystem paths, not device nodes, and thus governed by `ProtectSystem`/
  `ReadOnlyPaths` rather than `DeviceAllow`).
- **Validation loop**: run `systemd-analyze security cabledesk-helper.service`
  after drafting the unit file and iterate — it enumerates every hardening
  knob with a per-directive exposure contribution, which is the practical way
  to find the marginal-value directives worth adding without breaking the
  helper's legitimate function
  ([systemd-analyze security scoring described across multiple current hardening
  guides](https://docs.rockylinux.org/10/guides/security/systemd_hardening/)).

**Bottom line for CableDesk's helper unit**: harden aggressively on
everything *except* `PrivateNetwork` (must stay off), keep
`CapabilityBoundingSet` to the minimum proven set (`CAP_SYS_MODULE` plus
whatever hands-on testing shows is needed beyond delegating to
NetworkManager/firewalld over D-Bus), and treat the Polkit `CheckAuthorization`
call (§3) as the actual authorization boundary — the systemd sandboxing is
defense-in-depth around a process that is *already* supposed to only be
reachable for actions Polkit has approved, not the primary access-control
mechanism itself.

### 5. firewalld: zones vs. policies, and scoping to one interface

**Zones vs. policy objects, and which is current/preferred:** Modern
firewalld has both zones (the original, interface/source-based grouping of
rules) and **policy objects** (a newer, more general primitive for expressing
traffic flow rules *between* zones, e.g. ingress-zone → egress-zone with
independent options like masquerading). However, the firewalld D-Bus
interface itself currently marks both `org.fedoraproject.FirewallD1.direct`
and `org.fedoraproject.FirewallD1.policies` as **deprecated** in favor of the
newer, more structured zone/policy config object model exposed elsewhere in
the same D-Bus service (i.e., don't use the raw legacy "direct" interface or
the original "policies" interface directly; the current D-Bus surface has
since restructured this), confirmed by inspecting firewalld's own D-Bus
policy definition file
([firewalld's org.fedoraproject.FirewallD1 D-Bus interface annotations, GitHub](https://github.com/firewalld/firewalld/blob/main/config/org.fedoraproject.FirewallD1.server.policy.in),
[firewalld.dbus man page](https://firewalld.org/documentation/man-pages/firewalld.dbus.html)).
The exact current recommended top-level D-Bus interface path for
policy-object CRUD (as opposed to the deprecated one found) was not fully
disambiguated by search-only research — flagged in Open Questions for
hands-on verification against a running `firewall-cmd --get-policies`/
`firewall-cmd --policy=...` session and `firewall-cmd --version` on target
Fedora.

**Static zone file vs. D-Bus API for package-installed rules**: firewalld
ships default zone definitions under `/usr/lib/firewalld/zones/` (system
defaults/fallback) and treats `/etc/firewalld/zones/` as the
user/admin-customized overlay — files are only copied into `/etc/...` once
modified
([firewalld.zone man page / zone config docs](https://firewalld.org/documentation/zone/configuration-of-zones)).
For a package like CableDesk that wants a durable, versioned,
survives-firewalld-reload zone definition scoped to one interface, **shipping
a static XML zone file under `/usr/lib/firewalld/zones/cabledesk.xml`** (via
the RPM payload) is the established pattern — this is exactly parallel to how
firewalld ships its own built-in zones (`public.xml`, `home.xml`, etc.) in
that same directory. The zone contains the `<short>`, `<description>`,
`<service>`/`<port>` rules, etc.; interfaces are then bound to it either via
`firewall-cmd --zone=cabledesk --change-interface=<if> --permanent` or, if
NetworkManager manages the interface (which it does under this project's
target environment), NetworkManager can bind the connection profile to the
zone directly (a connection profile's `connection.zone` setting), which is
consistent with "NetworkManager binds interfaces to zones automatically" for
NM-managed interfaces
([firewalld zone practical guide](https://linuxconfig.org/how-to-define-a-custom-firewalld-zone),
corroborating summary from
[Red Hat RHEL 9 firewalld chapter](https://docs.redhat.com/en/documentation/red_hat_enterprise_linux/9/html/configuring_firewalls_and_packet_filters/using-and-configuring-firewalld_firewall-packet-filters)).

**"Only touch one interface, leave the user's existing zones alone" —
best-practice shape**:

1. Ship a dedicated custom zone (e.g. `cabledesk.xml`) under
   `/usr/lib/firewalld/zones/` in the RPM payload rather than mutating
   `public`/`home`/etc. — this avoids ever rewriting a zone the user already
   has configured.
2. Have the **helper** (Polkit-gated, per §3/§4) be the only component that
   ever calls the firewalld D-Bus API to bind a *specific* CableDesk-managed
   interface (e.g. a Thunderbolt/USB4 networking interface it creates) to
   that dedicated zone — never touch the zone assignment of interfaces the
   package didn't create.
3. Prefer the D-Bus API (`org.fedoraproject.FirewallD1`) at runtime for the
   interface-to-zone binding step (since the specific interface name is only
   known at runtime, e.g. a hotplugged Thunderbolt device), while the zone's
   *rule content* itself ships as the static XML file — this splits "what
   traffic is allowed" (static, package-owned, reviewable) from "which
   live interface gets that treatment" (dynamic, done at runtime via D-Bus
   through the helper after a Polkit check), which is the same read as
   firewalld's own doc guidance that `/usr/lib/firewalld/zones` is for
   package/distro-shipped definitions while runtime/admin changes flow
   through `/etc/firewalld/zones` and the D-Bus API
   ([firewalld zone directory precedence](https://firewalld.org/documentation/zone/configuration-of-zones)).

### 6. SELinux application policy packaging

**Current canonical guidance location**: the older
`SELinux_Policy_Modules_Packaging_Draft` wiki page **explicitly says it is
outdated** and redirects to
`https://fedoraproject.org/wiki/SELinux/IndependentPolicy`
([SELinux Policy Modules Packaging Draft, self-marked outdated](https://fedoraproject.org/wiki/SELinux_Policy_Modules_Packaging_Draft)),
so the `IndependentPolicy` page is the current authority used below
([SELinux/IndependentPolicy](https://fedoraproject.org/wiki/SELinux/IndependentPolicy)).

- **Subpackage structure**: a separate `<name>-selinux` subpackage (e.g.
  `cabledesk-selinux`) is the recommended shape — this avoids forcing a
  `Requires: selinux-policy` dependency chain onto users/systems that
  disable SELinux, and lets the module be updated independently
  ([SELinux/IndependentPolicy](https://fedoraproject.org/wiki/SELinux/IndependentPolicy);
  same conclusion echoed by the outdated-but-consistent
  [Packaging Draft](https://fedoraproject.org/wiki/SELinux_Policy_Modules_Packaging_Draft)).
- **BuildRequires**: `selinux-policy-devel` (provides policy-building
  interfaces/macros and the reference-policy headers needed to compile a
  `.pp` module), plus `checkpolicy`
  ([selinux-policy-devel package description](https://packages.fedoraproject.org/pkgs/selinux-policy/selinux-policy-devel/),
  [SELinux Policy Modules Packaging Draft](https://fedoraproject.org/wiki/SELinux_Policy_Modules_Packaging_Draft)).
- **Runtime Requires**: `Requires(post): selinux-policy-%{selinuxtype}` so the
  base policy the module extends is present when the module is loaded.
- **Install/load macros** (current, from `selinux-policy-devel`'s shipped
  macros, `selinux-policy-macros`): use
  `%selinux_modules_install -s %{selinuxtype} <module>.pp.bz2` in `%post` and
  the paired `%selinux_modules_uninstall` in `%postun`, rather than hand-rolled
  `semodule -i`/`-r` loops — these macros embed the correct, **fixed module
  priority of 200** (custom/local policy overriding the distribution's
  priority-100 modules) so packagers should not attempt to override that
  priority themselves
  ([SELinux/IndependentPolicy](https://fedoraproject.org/wiki/SELinux/IndependentPolicy);
  macro source at
  [selinux-policy-macros/macros.selinux-policy](https://github.com/fedora-selinux/selinux-policy-macros/blob/master/macros.selinux-policy)).
- **Relabeling**: for a daemon that needs its file contexts fixed up
  immediately (not deferred to the next full relabel), current guidance is
  to call `%selinux_relabel_post` in `%post` (and the paired
  `%selinux_relabel_pre` ahead of removal in `%preun`), rather than invoking
  `restorecon`/`fixfiles` directly by hand
  ([SELinux/IndependentPolicy](https://fedoraproject.org/wiki/SELinux/IndependentPolicy)).
  `%ghost` is the standard RPM mechanism for files whose contents are
  created at runtime, not packaged (e.g. a state/log file this policy needs
  to label) — the `-selinux` subpackage's `%files` would `%ghost` any such
  runtime-created paths that need a non-default context so ownership is
  tracked without RPM needing actual file content at build time. (No current
  Fedora doc was found spelling out a CableDesk-specific `%ghost` example;
  this is the general RPM mechanism, not something SELinux-specific — flagged
  for confirmation during actual implementation.)
- **Custom policy vs. generic domains**: current guidance explicitly leans
  toward writing a real custom policy module over reaching for a permissive
  generic domain or flipping a broad boolean: "additional access in the
  custom policy module is preferred to switching a boolean that impacts
  other policy modules"
  ([SELinux/IndependentPolicy](https://fedoraproject.org/wiki/SELinux/IndependentPolicy)).
  This directly discourages just running the helper as `unconfined_service_t`
  (which would give it essentially unrestricted SELinux access, defeating
  the point of enforcing) as a long-term answer, in favor of a small,
  purpose-built domain.
- **Realistic scope for CableDesk's helper**: given the helper already talks
  to NetworkManager and firewalld exclusively over D-Bus (rather than
  manipulating their state files directly) and only needs `CAP_SYS_MODULE`
  for module loading, the custom policy module's job is mostly to: (a) define
  a dedicated domain (e.g. `cabledesk_helper_t`) transitioning from
  `init_t`/`systemd` at service start; (b) grant that domain the narrow
  D-Bus send/receive permissions needed to talk to
  `org.freedesktop.NetworkManager`, `org.fedoraproject.FirewallD1`, and
  `org.freedesktop.PolicyKit1` (SELinux mediates D-Bus messages when the
  system bus policy is configured for it, via existing
  `dbus_send_msg`-style interfaces already defined in the reference policy
  for NetworkManager/firewalld peers); (c) grant `sys_module` capability and
  the specific `kernel_module_t` or `insmod_exec_t`-equivalent transition
  needed for loading only the specific driver module(s) CableDesk needs; and
  (d) grant sysfs/USB4-debugfs read access as needed for Thunderbolt
  enumeration. This is very likely small relative to a full custom policy
  (tens, not hundreds, of allow rules) *if* it can reuse existing
  NetworkManager/firewalld SELinux interfaces (`nm_dbus_chat()`-style
  reference-policy interface macros) rather than writing raw `allow` rules
  against those services' domains from scratch — but the exact interface
  names available in current `selinux-policy` reference policy, and whether
  they're granular enough for a third-party (non-NetworkManager-authored)
  domain to use, requires hands-on inspection of the installed
  `selinux-policy-devel` interface files on real Fedora hardware (flagged in
  Open Questions).

---

## Open Questions

The following could not be fully verified via documentation search alone and
require either a more authoritative primary source or hands-on testing on
real Fedora hardware with SELinux enforcing and firewalld enabled:

1. **`docs.fedoraproject.org` Rust and Packaging Guidelines pages were
   inaccessible to automated fetch** (blocked by the site's Anubis
   anti-bot/anti-scraper challenge during this research session — verified
   by two separate fetch attempts). All Rust-guideline claims above were
   reconstructed from search-result snippets of that same page plus
   corroborating secondary sources (mankier, Fedora wiki, third-party blog
   posts, GitHub mirrors of the macro packages), not a direct read of the
   canonical page. **Action item: have a human (or a fetch from a
   non-bot-blocked context) load
   `https://docs.fedoraproject.org/en-US/packaging-guidelines/Rust/` directly
   and diff against the claims in §1** before treating this document as
   final.
2. **No explicit, current Fedora policy document was found stating a clear
   decision rule for "`%caps()` file capabilities vs. Polkit-mediated D-Bus
   action" for a given operation.** §2.2's recommendation is reasoned from
   how each mechanism actually behaves and how Fedora's own packages use
   them, not from a single authoritative "use X when Y" guideline page. Worth
   a targeted follow-up search specifically in Fedora's security/hardening
   docs or a question to Fedora's `#fedora-security`/packaging mailing list
   if a stronger citation is wanted.
3. **firewalld's current D-Bus interface surface for policy objects is only
   partially disambiguated.** We confirmed `org.fedoraproject.FirewallD1.direct`
   and `org.fedoraproject.FirewallD1.policies` are marked deprecated in the
   current source, but did not fully pin down the exact current interface
   name/path that replaces `.policies` for programmatic policy-object CRUD
   (as distinct from zone CRUD, which is on the non-deprecated `.zone`
   interface). **Needs verification**: run `busctl introspect
   org.fedoraproject.FirewallD1 /org/fedoraproject/FirewallD1` on a real
   Fedora Workstation system with a current `firewalld` and read the live
   introspection XML, or check `firewalld`'s CHANGES file for the version
   that deprecated `.policies` and what it says to use instead.
4. **The realistic size/shape of the custom SELinux policy module for the
   helper (§6, last bullet) is an estimate based on general SELinux
   packaging patterns, not a citation of an existing similar package's
   actual `.te` file.** Confirming whether current `selinux-policy-devel`
   ships reusable interfaces for "third-party service talks to
   NetworkManager/firewalld over D-Bus" (vs. requiring raw allow rules)
   requires inspecting the actual interface files (typically under
   `/usr/share/selinux/devel/include/.../networkmanager.if`,
   `firewalld.if` and similar) on a real Fedora installation — this is
   exactly the kind of thing that needs hands-on testing with `sealert`/
   `audit2allow`/`ausearch -m avc` against a real running helper in
   enforcing mode, since AVC denial contents cannot be predicted from
   documentation alone.
5. **Exact `CapabilityBoundingSet` and `SystemCallFilter` minimum sets for
   the helper (§4) are a reasoned starting point, not a validated
   result.** These must be finalized by actually running the helper under
   `systemd-analyze security`, then iteratively tightening while exercising
   every helper code path (module load, NM profile create, firewalld rule
   install, USB4/Thunderbolt inspection) under `strace`/audit to see which
   syscalls and capabilities are truly exercised, on real hardware with
   Thunderbolt/USB4 devices attached.
6. **Whether `PrivateNetwork=false` combined with `RestrictAddressFamilies`
   restricted to `AF_UNIX`/`AF_NETLINK` is sufficient, or whether the helper
   ends up needing `AF_INET`/`AF_INET6` directly** (e.g. if it does any
   direct probing rather than delegating entirely to NetworkManager) is
   implementation-dependent and unverifiable without the actual helper code.
7. **rpmautospec applicability**: whether CableDesk will actually be built
   through Fedora's dist-git + koji pipeline (where `%autorelease`/
   `%autochangelog` make sense) or as a standalone project distributing its
   own RPMs (e.g. via a COPR or self-hosted repo) was not specified in the
   task and changes which changelog convention is actually appropriate —
   flagged rather than assumed.
