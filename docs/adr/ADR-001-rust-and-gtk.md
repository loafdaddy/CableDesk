# ADR-001: Rust for the application, GTK4/libadwaita for the interface

## Status

Accepted, implemented (this milestone).

## Context

CableDesk needs a native Linux application: no Flatpak/Snap/AppImage,
Electron, or Tauri, and no browser-based frontend (project non-goals). It
has a privilege-separated, multi-process architecture (GTK UI, user-session
agent, privileged helper — see `docs/ARCHITECTURE.md`) that needs to talk
to systemd, D-Bus, Polkit, NetworkManager, and eventually manage child
processes (Sunshine, Moonlight) reliably and safely.

## Decision

Use Rust for every component, and direct `gtk4`/`libadwaita` Rust bindings
(gtk4-rs) for the GUI — not a higher-level UI framework — following GNOME
Human Interface Guidelines.

Rationale:

- Memory safety and a strong type system matter for a privileged system
  helper that parses D-Bus messages and will eventually handle untrusted
  peer input during pairing.
- `async`/`await` plus `tokio` gives a natural model for the agent's
  event-driven D-Bus/discovery/streaming orchestration.
- GTK4/libadwaita is the native toolkit for GNOME (Tier 1 target desktop)
  and produces a lightweight, integrated-looking app without a bundled
  runtime, unlike Electron/Tauri.
- gtk4-rs is mature enough that this milestone's `cabledesk-ui` compiled
  and ran against the system's real GTK 4.22/libadwaita 1.9 without any
  binding-version friction (`gtk4 = "0.11"`, `libadwaita = "0.9"` resolved
  and linked cleanly).

## Consequences

- Cross-platform GUI toolkit portability is limited by GTK4/libadwaita
  itself — Tier 2 KDE Plasma support (`docs/DISTRO_SUPPORT.md`) will need
  its own consideration (GTK4 apps run fine under Plasma, but won't look
  native; this is accepted for now, revisit if it matters).
- GTK's main loop does not drive Tokio futures, so any async platform code
  (this milestone: `cabledesk-platform-fedora`'s `tokio::fs`/`zbus` calls)
  needs an explicit hand-off (background thread + its own Tokio runtime +
  `async-channel` back to the GLib main context) — implemented in
  `crates/cabledesk-ui/src/main.rs`, documented in its own comments.
- Every new crate needs to justify its addition against "avoid unnecessary
  frameworks" (project engineering rule) — reflected in
  `Cargo.toml`'s deliberately small `workspace.dependencies` list.
