//! Avahi/mDNS-based peer discovery scoped to the direct-link interface.
//!
//! `service` defines the `_cabledesk._tcp` record shape and its TXT
//! encoding (no secrets — see the module doc comment). `avahi` is the
//! D-Bus client that publishes/browses/resolves it, scoped to a specific
//! interface index rather than the whole system.

pub mod avahi;
pub mod service;

pub use avahi::{AvahiClient, DiscoveryEvent, PublishedService, ResolvedService, AVAHI_IF_UNSPEC};
pub use service::{PairingState, ServiceRecord, PROTOCOL_VERSION, SERVICE_TYPE};
