# CableDesk SELinux policy (placeholder)

No custom SELinux policy module exists yet. This directory is a placeholder
for the future `cabledesk-selinux` subpackage described in
`docs/PACKAGING.md`.

Current plan (see `docs/PACKAGING.md`, "Security-Relevant Packaging &
Hardening Research" and `docs/adr/ADR-005-native-packaging.md`):

- Follow the current canonical Fedora page, `SELinux/IndependentPolicy`,
  not the older packaging-draft page (which self-marks as outdated).
- Ship a `.te`/`.fc`/`.if` policy source, built and installed via the
  `selinux-policy-devel` macros (`%selinux_modules_install`,
  `%selinux_relabel_post`) in a dedicated `-selinux` subpackage, so a
  system without SELinux never needs to pull in policy tooling.
- Prefer a small custom domain for `cabledesk-helper` over reusing
  `unconfined_service_t`, once real privileged operations (module loading,
  NetworkManager profile management, firewalld rule installation) exist to
  write a policy against. Writing policy before those operations exist
  would be guessing at AVC denials that don't exist yet.

Until then, `cabledesk-helper` and `cabledesk-agent` run under whatever
default type Fedora assigns their unit files, and every compatibility check
in this milestone is read-only (see `crates/cabledesk-platform-fedora`), so
there is nothing yet for a custom policy to confine. Do not disable or
loosen SELinux to work around denials that appear once privileged
operations land — fix labeling/behaviour or extend policy instead (see
engineering rule "no disabling SELinux").
