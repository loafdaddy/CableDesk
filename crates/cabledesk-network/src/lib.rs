//! Direct-link addressing, route validation and interface enforcement.
//!
//! `profile` builds the NetworkManager connection-settings dictionary;
//! `manager` applies it and watches device hotplug events over D-Bus,
//! including a direct-link-filtered stream (`DirectLinkEvent`) that
//! `cabledesk-agent` drives its state machine from; `validate` confirms a
//! peer address actually belongs to the direct-link interface and that
//! the route to it resolves through that interface (see
//! `docs/adr/ADR-007-direct-interface-only.md`).
//!
//! `classify` and `peer` enforce the cable-only transport rule: Wi-Fi,
//! ordinary Ethernet, VPN, and unknown interfaces are never valid session
//! transports.

pub mod classify;
pub mod manager;
pub mod peer;
pub mod profile;
pub mod sysfs;
pub mod validate;

#[cfg(feature = "simulation")]
pub mod simulation;

pub use classify::{
    classify_from_driver_name, classify_from_interface_name, InterfaceClassification,
};
pub use manager::{DeviceEvent, DirectLinkEvent, NetworkManagerClient};
pub use peer::{
    reject_if_discovery_not_direct_cable, validate_cable_peer, validate_cable_peer_claim,
    validate_cable_peer_policy_only, PeerClaim, PreparedDirectLink, ValidatedCablePeer,
};
pub use profile::DirectLinkProfile;
pub use sysfs::{
    classify_interface, interface_index, read_interface_driver, require_direct_cable_interface,
};
pub use validate::{address_belongs_to_interface, route_resolves_via_interface};
