# openSUSE packaging (not started)

Placeholder for Phase 6 (`docs/ROADMAP.md`). See `packaging/debian/README.md`
for why cross-distribution packaging is deliberately deferred.

openSUSE also uses RPM, so the Fedora spec in `packaging/fedora/cabledesk.spec`
is the closest starting point — but `rust2rpm`'s `-t opensuse` target and
openSUSE's own Rust packaging macros (distinct from Fedora's
`rust-srpm-macros`/`cargo-rpm-macros`) need their own research pass before
assuming the Fedora spec ports over unchanged. A
`cabledesk-platform-opensuse` crate (likely usable for both openSUSE and
generic zypper-based systems) would back `PlatformBackend` here, following
`docs/adr/ADR-006-platform-backend-abstraction.md`.
