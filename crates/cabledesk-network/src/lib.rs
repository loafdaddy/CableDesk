//! Direct-link addressing, route validation and interface enforcement.
//!
//! `profile` builds the NetworkManager connection-settings dictionary;
//! `manager` applies it and watches device hotplug events over D-Bus,
//! including a direct-link-filtered stream (`DirectLinkEvent`) that
//! `cabledesk-agent` drives its state machine from; `validate` confirms a
//! peer address actually belongs to the direct-link interface and that
//! the route to it resolves through that interface (see
//! `docs/adr/ADR-007-direct-interface-only.md`).

pub mod manager;
pub mod profile;
pub mod validate;

pub use manager::{DeviceEvent, DirectLinkEvent, NetworkManagerClient};
pub use profile::DirectLinkProfile;
pub use validate::{address_belongs_to_interface, route_resolves_via_interface};
