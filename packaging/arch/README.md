# Arch Linux packaging (not started)

Placeholder for Phase 6 (`docs/ROADMAP.md`). See `packaging/debian/README.md`
for why cross-distribution packaging is deliberately deferred.

When this phase starts, expect a `PKGBUILD` building the same Cargo
workspace with `cargo build --workspace --release --locked`, split into
`cabledesk`, `cabledesk-agent`, `cabledesk-helper` packages (or a single
package with subpackage-equivalent split via `pkgname=(...)` arrays,
Arch's usual approach for multi-binary projects), plus a
`cabledesk-platform-arch` crate for `PlatformBackend`.
