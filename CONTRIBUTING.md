# Contributing to CableDesk

CableDesk is an early-stage, experimental project (see the warning at the
top of `README.md`). Before contributing, please read:

- `docs/ARCHITECTURE.md` — component layout
- `docs/ROADMAP.md` — what phase the project is in and what's next
- `docs/SECURITY.md` and `docs/THREAT_MODEL.md` — security constraints that
  apply to any change touching pairing, networking, or the privileged
  helper
- `docs/OPEN_QUESTIONS.md` — known-unresolved items; check here before
  assuming something is settled

## Ground rules (from the project's own engineering rules)

- No Flatpak, Snap, AppImage, Electron, or Tauri.
- The GTK UI never runs as root; privileged code lives only in
  `cabledesk-helper`, kept small and independently reviewable.
- Never disable SELinux or firewalld, in code, scripts, or documentation.
- Never hardcode `thunderbolt0` — detect the direct interface by driver,
  not by name (see `crates/cabledesk-platform-fedora/src/direct_link.rs`).
- Never assume physical cable access implies trust.
- No custom cryptography — use a maintained Rust crate.
- No secrets in logs, mDNS records, or process arguments.
- No silent fallback to Wi-Fi/Ethernet if the direct link drops.
- Prefer typed D-Bus APIs (`zbus`) over shelling out; shell commands are for
  early proof-of-concept experiments only.
- Don't claim a distro/desktop combination is "supported" without real
  integration testing on it — see `docs/DISTRO_SUPPORT.md`.
- Use Australian/British spelling in user-facing text (authorise,
  synchronise, optimise, customise).

## Development setup

```bash
./scripts/dev-setup.sh      # checks/installs dev dependencies, asks first
./scripts/dev-build.sh      # fmt --check, build, clippy -D warnings, test
./scripts/dev-install.sh    # installs locally; asks before any root/system changes
```

Before opening a PR: `cargo fmt --all`, `cargo clippy --workspace
--all-targets -- -D warnings`, and `cargo test --workspace` must all pass
(`./scripts/dev-build.sh` runs all three).

## Adding a new distribution backend

Implement `cabledesk_platform::PlatformBackend` in a new
`cabledesk-platform-<distro>` crate — see
`docs/adr/ADR-006-platform-backend-abstraction.md`. Do not add
distro-specific branching anywhere outside that crate.

## Pull requests

Keep changes scoped to one phase/concern at a time (see `docs/ROADMAP.md`)
— "don't attempt every phase in one commit" is a project rule, not just
advice. If a change touches the privileged helper, pairing, or networking,
it should reference the relevant section of `docs/SECURITY.md` or
`docs/THREAT_MODEL.md` in the PR description.
