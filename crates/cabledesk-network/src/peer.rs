//! Cable-only peer endpoint validation.
//!
//! Trust (device ID) is never enough: the peer must be associated with a
//! verified direct USB4/Thunderbolt interface. See the cable-only invariants
//! in `docs/ARCHITECTURE.md` and `docs/SECURITY.md`.

use crate::classify::{classify_from_driver_name, InterfaceClassification};
use crate::validate::route_resolves_via_interface;
use cabledesk_core::error::{CableDeskError, Result};
use serde::{Deserialize, Serialize};
use std::net::{IpAddr, Ipv4Addr};

/// An unvalidated peer claim — must pass [`validate_cable_peer`] before use.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PeerClaim {
    pub address: IpAddr,
    /// Interface on which the peer was discovered (e.g. Avahi scope).
    pub discovered_on_interface: String,
    /// Driver evidence for that interface, when known.
    pub discovered_on_driver: Option<String>,
}

/// A peer endpoint that has passed cable-only checks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidatedCablePeer {
    pub address: IpAddr,
    pub interface_name: String,
    pub classification: InterfaceClassification,
}

/// The active direct-cable link CableDesk is willing to use.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreparedDirectLink {
    pub interface_name: String,
    pub classification: InterfaceClassification,
    pub interface_index: Option<u32>,
}

impl PreparedDirectLink {
    pub fn new(
        interface_name: impl Into<String>,
        classification: InterfaceClassification,
    ) -> Result<Self> {
        if !classification.is_direct_cable() {
            return Err(CableDeskError::NoDirectCableConnection);
        }
        Ok(Self {
            interface_name: interface_name.into(),
            classification,
            interface_index: None,
        })
    }
}

/// Policy checks only (no live route lookup): discovery interface must be
/// the prepared direct cable link. Used by simulation and by
/// [`validate_cable_peer`] before the FIB check.
pub fn validate_cable_peer_claim(link: &PreparedDirectLink, claim: &PeerClaim) -> Result<()> {
    if !link.classification.is_direct_cable() {
        return Err(CableDeskError::NoDirectCableConnection);
    }

    let discovered_class = match &claim.discovered_on_driver {
        Some(driver) => classify_from_driver_name(driver),
        None => crate::classify::classify_from_interface_name(&claim.discovered_on_interface),
    };

    if !discovered_class.is_direct_cable() {
        return Err(CableDeskError::PeerNotOnDirectCable);
    }

    if claim.discovered_on_interface != link.interface_name {
        return Err(CableDeskError::PeerNotOnDirectCable);
    }

    Ok(())
}

/// Validate that `claim` is on the prepared direct cable link.
///
/// Rejects Wi-Fi / Ethernet / VPN / unknown discovery interfaces, then
/// confirms the kernel would route to the peer via the direct interface
/// (not Wi-Fi/Ethernet). Peer addresses are *not* required to be assigned
/// on this host — they belong to the peer.
pub async fn validate_cable_peer(
    link: &PreparedDirectLink,
    claim: &PeerClaim,
) -> Result<ValidatedCablePeer> {
    validate_cable_peer_claim(link, claim)?;

    let IpAddr::V4(v4) = claim.address else {
        return Err(CableDeskError::PeerNotOnDirectCable);
    };
    if !route_resolves_via_interface(v4, &link.interface_name).await {
        return Err(CableDeskError::PeerNotOnDirectCable);
    }

    Ok(ValidatedCablePeer {
        address: claim.address,
        interface_name: link.interface_name.clone(),
        classification: link.classification,
    })
}

/// Build a [`ValidatedCablePeer`] after policy checks without a route lookup.
///
/// Prefer [`validate_cable_peer`] on the live path. This exists for unit tests
/// and simulation where no real FIB entry exists for a fabricated peer.
pub fn validate_cable_peer_policy_only(
    link: &PreparedDirectLink,
    claim: &PeerClaim,
) -> Result<ValidatedCablePeer> {
    validate_cable_peer_claim(link, claim)?;
    // Reject obviously non-link-local IPv4 peers in policy-only mode too —
    // CableDesk v1 is IPv4 link-local on the direct interface only.
    if let IpAddr::V4(v4) = claim.address {
        if !is_link_local_v4(v4) {
            return Err(CableDeskError::PeerNotOnDirectCable);
        }
    } else {
        return Err(CableDeskError::PeerNotOnDirectCable);
    }
    Ok(ValidatedCablePeer {
        address: claim.address,
        interface_name: link.interface_name.clone(),
        classification: link.classification,
    })
}

fn is_link_local_v4(addr: Ipv4Addr) -> bool {
    addr.octets()[0] == 169 && addr.octets()[1] == 254
}

/// Pure policy check used by simulation and unit tests when no live
/// addresses exist: reject non-cable discovery without requiring the
/// address to be assigned on this host.
pub fn reject_if_discovery_not_direct_cable(
    discovered_on_driver: Option<&str>,
    discovered_on_interface: &str,
) -> Result<()> {
    let class = match discovered_on_driver {
        Some(driver) => classify_from_driver_name(driver),
        None => crate::classify::classify_from_interface_name(discovered_on_interface),
    };
    if class.is_direct_cable() {
        Ok(())
    } else {
        Err(CableDeskError::PeerNotOnDirectCable)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::classify::InterfaceClassification;
    use std::net::{IpAddr, Ipv4Addr};

    #[test]
    fn prepared_link_rejects_wifi_classification() {
        let err = PreparedDirectLink::new("wlp0s20f3", InterfaceClassification::WiFi).unwrap_err();
        assert!(matches!(err, CableDeskError::NoDirectCableConnection));
    }

    #[test]
    fn peer_on_wifi_is_rejected() {
        let link =
            PreparedDirectLink::new("thunderbolt0", InterfaceClassification::DirectThunderbolt)
                .unwrap();
        let claim = PeerClaim {
            address: IpAddr::V4(Ipv4Addr::new(169, 254, 1, 2)),
            discovered_on_interface: "wlp0s20f3".into(),
            discovered_on_driver: Some("iwlwifi".into()),
        };
        let err = validate_cable_peer_policy_only(&link, &claim).unwrap_err();
        assert!(matches!(err, CableDeskError::PeerNotOnDirectCable));
    }

    #[test]
    fn peer_on_ethernet_is_rejected() {
        assert!(matches!(
            reject_if_discovery_not_direct_cable(Some("e1000e"), "enp3s0"),
            Err(CableDeskError::PeerNotOnDirectCable)
        ));
    }

    #[test]
    fn peer_on_thunderbolt_driver_is_accepted_by_policy() {
        assert!(
            reject_if_discovery_not_direct_cable(Some("thunderbolt-net"), "thunderbolt0").is_ok()
        );
    }

    #[test]
    fn trusted_device_on_wrong_interface_is_still_rejected() {
        // Policy layer: even a "trusted" peer advertised on Wi-Fi fails.
        assert!(matches!(
            reject_if_discovery_not_direct_cable(Some("iwlwifi"), "wlan0"),
            Err(CableDeskError::PeerNotOnDirectCable)
        ));
    }

    #[test]
    fn user_facing_message_for_wrong_interface() {
        use cabledesk_core::error::UserFacingError;
        let uf = UserFacingError::from(&CableDeskError::PeerNotOnDirectCable);
        assert!(uf.headline.contains("direct cable"));
        let none = UserFacingError::from(&CableDeskError::NoDirectCableConnection);
        assert_eq!(none.headline, "No direct cable connection");
    }
}
