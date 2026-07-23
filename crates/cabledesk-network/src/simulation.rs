//! Development-only simulation of the direct-cable lifecycle.
//!
//! Enabled only with the `simulation` Cargo feature. Production builds must
//! not depend on this module. The privileged helper must never expose these
//! events.

use crate::classify::classify_from_driver_name;
use crate::peer::{reject_if_discovery_not_direct_cable, PeerClaim, PreparedDirectLink};
use cabledesk_core::error::{CableDeskError, Result};
use cabledesk_core::state::{ConnectionState, StateMachine};
use serde::{Deserialize, Serialize};
use std::net::{IpAddr, Ipv4Addr};

/// A simulated world CableDesk can drive without Thunderbolt hardware.
#[derive(Debug, Clone, Default)]
pub struct SimulatedWorld {
    pub usb4_controller: bool,
    pub thunderbolt_controller: bool,
    pub cable_inserted: bool,
    pub interface_name: Option<String>,
    pub interface_driver: Option<String>,
    pub peer: Option<SimulatedPeer>,
    pub charging: bool,
    pub discharging: bool,
    pub mock_stream_active: bool,
    pub state: StateMachine,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SimulatedPeer {
    pub address: IpAddr,
    pub discovered_on_interface: String,
    pub discovered_on_driver: String,
    pub trusted: bool,
}

/// Serializable snapshot for chaining `cabledeskctl simulate` invocations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimSnapshot {
    pub usb4_controller: bool,
    pub thunderbolt_controller: bool,
    pub cable_inserted: bool,
    pub interface_name: Option<String>,
    pub interface_driver: Option<String>,
    pub peer: Option<SimulatedPeer>,
    pub charging: bool,
    pub discharging: bool,
    pub mock_stream_active: bool,
    pub state: ConnectionState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SimulateCommand {
    Usb4Controller,
    CableInserted,
    CableRemoved,
    DirectInterfaceAdded,
    DirectInterfaceRemoved,
    PeerDiscovered,
    PeerOnWifi,
    PeerOnEthernet,
    PeerTrusted,
    IdentityMismatch,
    Charging,
    Discharging,
    StreamStarted,
    StreamFailed,
}

impl SimulatedWorld {
    pub fn new() -> Self {
        let mut world = Self::default();
        let _ = world
            .state
            .transition(ConnectionState::CheckingCompatibility);
        let _ = world.state.transition(ConnectionState::Ready);
        let _ = world.state.transition(ConnectionState::WaitingForCable);
        world
    }

    pub fn from_snapshot(snap: SimSnapshot) -> Self {
        Self {
            usb4_controller: snap.usb4_controller,
            thunderbolt_controller: snap.thunderbolt_controller,
            cable_inserted: snap.cable_inserted,
            interface_name: snap.interface_name,
            interface_driver: snap.interface_driver,
            peer: snap.peer,
            charging: snap.charging,
            discharging: snap.discharging,
            mock_stream_active: snap.mock_stream_active,
            state: StateMachine::restored(snap.state),
        }
    }

    pub fn to_snapshot(&self) -> SimSnapshot {
        SimSnapshot {
            usb4_controller: self.usb4_controller,
            thunderbolt_controller: self.thunderbolt_controller,
            cable_inserted: self.cable_inserted,
            interface_name: self.interface_name.clone(),
            interface_driver: self.interface_driver.clone(),
            peer: self.peer.clone(),
            charging: self.charging,
            discharging: self.discharging,
            mock_stream_active: self.mock_stream_active,
            state: self.state.current(),
        }
    }

    pub fn apply(&mut self, cmd: SimulateCommand) -> Result<String> {
        match cmd {
            SimulateCommand::Usb4Controller => {
                self.usb4_controller = true;
                self.thunderbolt_controller = true;
                Ok("Simulated USB4/Thunderbolt controller present.".into())
            }
            SimulateCommand::CableInserted => {
                if !(self.usb4_controller || self.thunderbolt_controller) {
                    return Err(CableDeskError::NoDirectLinkHardware);
                }
                self.cable_inserted = true;
                Ok("Simulated compatible cable inserted.".into())
            }
            SimulateCommand::DirectInterfaceAdded => {
                if !self.cable_inserted {
                    return Err(CableDeskError::DirectInterfaceMissing);
                }
                self.interface_name = Some("sim-thunderbolt0".into());
                self.interface_driver = Some("thunderbolt-net".into());
                self.state.transition(ConnectionState::CableDetected)?;
                self.state.transition(ConnectionState::InspectingPower)?;
                self.state.transition(ConnectionState::ConfiguringLink)?;
                Ok("Simulated direct interface added; link configuring.".into())
            }
            SimulateCommand::PeerDiscovered => {
                self.ensure_direct_interface()?;
                self.peer = Some(SimulatedPeer {
                    address: IpAddr::V4(Ipv4Addr::new(169, 254, 10, 20)),
                    discovered_on_interface: "sim-thunderbolt0".into(),
                    discovered_on_driver: "thunderbolt-net".into(),
                    trusted: false,
                });
                self.state.transition(ConnectionState::DiscoveringPeer)?;
                self.state.transition(ConnectionState::PairingRequired)?;
                Ok("Simulated peer discovered on direct cable interface.".into())
            }
            SimulateCommand::PeerOnWifi => {
                self.ensure_direct_interface()?;
                Err(
                    reject_if_discovery_not_direct_cable(Some("iwlwifi"), "wlp0s20f3")
                        .err()
                        .unwrap_or(CableDeskError::PeerNotOnDirectCable),
                )
            }
            SimulateCommand::PeerOnEthernet => {
                self.ensure_direct_interface()?;
                Err(
                    reject_if_discovery_not_direct_cable(Some("e1000e"), "enp3s0")
                        .err()
                        .unwrap_or(CableDeskError::PeerNotOnDirectCable),
                )
            }
            SimulateCommand::PeerTrusted => {
                self.ensure_direct_interface()?;
                let peer = self
                    .peer
                    .as_mut()
                    .ok_or_else(|| CableDeskError::Config("no simulated peer".into()))?;
                peer.trusted = true;
                if self.state.current() == ConnectionState::PairingRequired {
                    self.state.transition(ConnectionState::Pairing)?;
                    self.state.transition(ConnectionState::Authenticating)?;
                    self.state.transition(ConnectionState::PreparingHost)?;
                }
                Ok("Simulated peer marked trusted (no real crypto).".into())
            }
            SimulateCommand::IdentityMismatch => Err(CableDeskError::UntrustedPeerIdentity),
            SimulateCommand::Charging => {
                self.charging = true;
                self.discharging = false;
                Ok("Simulated charging.".into())
            }
            SimulateCommand::Discharging => {
                self.charging = false;
                self.discharging = true;
                Ok("Simulated discharging while connected.".into())
            }
            SimulateCommand::StreamStarted => {
                if self.state.current() == ConnectionState::PreparingHost {
                    self.state.transition(ConnectionState::StartingSunshine)?;
                    self.state.transition(ConnectionState::StartingViewer)?;
                    self.state.transition(ConnectionState::Streaming)?;
                } else if self.state.current() != ConnectionState::Streaming {
                    return Err(CableDeskError::Config(format!(
                        "cannot start mock stream from {:?}",
                        self.state.current()
                    )));
                }
                self.mock_stream_active = true;
                Ok("Mock stream started (no Sunshine/Moonlight).".into())
            }
            SimulateCommand::StreamFailed => {
                self.mock_stream_active = false;
                self.state.transition(ConnectionState::Error)?;
                Ok("Mock stream failed.".into())
            }
            SimulateCommand::DirectInterfaceRemoved | SimulateCommand::CableRemoved => {
                self.invalidate_session_on_cable_loss()
            }
        }
    }

    fn ensure_direct_interface(&self) -> Result<()> {
        match (&self.interface_name, &self.interface_driver) {
            (Some(_), Some(driver)) if classify_from_driver_name(driver).is_direct_cable() => {
                Ok(())
            }
            _ => Err(CableDeskError::NoDirectCableConnection),
        }
    }

    fn invalidate_session_on_cable_loss(&mut self) -> Result<String> {
        self.cable_inserted = false;
        self.interface_name = None;
        self.interface_driver = None;
        self.peer = None;
        self.mock_stream_active = false;
        let current = self.state.current();
        if current == ConnectionState::WaitingForCable {
            return Ok("Already waiting for cable.".into());
        }
        if current.can_transition_to(ConnectionState::WaitingForCable) {
            self.state.transition(ConnectionState::WaitingForCable)?;
        } else {
            self.state.transition(ConnectionState::Error)?;
            self.state.transition(ConnectionState::WaitingForCable)?;
        }
        Ok("Cable removed — session stopped. No Wi-Fi/Ethernet fallback.".into())
    }

    pub fn prepared_link(&self) -> Result<PreparedDirectLink> {
        let name = self
            .interface_name
            .clone()
            .ok_or(CableDeskError::NoDirectCableConnection)?;
        let driver = self
            .interface_driver
            .as_deref()
            .ok_or(CableDeskError::NoDirectCableConnection)?;
        PreparedDirectLink::new(name, classify_from_driver_name(driver))
    }

    pub fn validate_current_peer_policy(&self) -> Result<()> {
        let peer = self
            .peer
            .as_ref()
            .ok_or_else(|| CableDeskError::Config("no simulated peer".into()))?;
        reject_if_discovery_not_direct_cable(
            Some(&peer.discovered_on_driver),
            &peer.discovered_on_interface,
        )?;
        let link = self.prepared_link()?;
        if peer.discovered_on_interface != link.interface_name {
            return Err(CableDeskError::PeerNotOnDirectCable);
        }
        Ok(())
    }
}

pub fn wifi_peer_claim() -> PeerClaim {
    PeerClaim {
        address: IpAddr::V4(Ipv4Addr::new(192, 168, 1, 50)),
        discovered_on_interface: "wlp0s20f3".into(),
        discovered_on_driver: Some("iwlwifi".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::classify::InterfaceClassification;
    use crate::peer::ValidatedCablePeer;
    use cabledesk_core::error::UserFacingError;

    #[test]
    fn full_simulated_lifecycle_rejects_wifi_and_stops_on_unplug() {
        let mut world = SimulatedWorld::new();
        assert_eq!(world.state.current(), ConnectionState::WaitingForCable);

        world.apply(SimulateCommand::Usb4Controller).unwrap();
        world.apply(SimulateCommand::CableInserted).unwrap();
        world.apply(SimulateCommand::DirectInterfaceAdded).unwrap();
        assert_eq!(world.state.current(), ConnectionState::ConfiguringLink);

        world.apply(SimulateCommand::PeerDiscovered).unwrap();
        world.validate_current_peer_policy().unwrap();

        let wifi_err = world.apply(SimulateCommand::PeerOnWifi).unwrap_err();
        assert!(matches!(wifi_err, CableDeskError::PeerNotOnDirectCable));
        let uf = UserFacingError::from(&wifi_err);
        assert!(uf.headline.contains("direct cable"));

        world.apply(SimulateCommand::PeerTrusted).unwrap();
        world.apply(SimulateCommand::StreamStarted).unwrap();
        assert!(world.mock_stream_active);
        assert_eq!(world.state.current(), ConnectionState::Streaming);

        world.apply(SimulateCommand::CableRemoved).unwrap();
        assert!(!world.mock_stream_active);
        assert!(world.interface_name.is_none());
        assert_eq!(world.state.current(), ConnectionState::WaitingForCable);
    }

    #[test]
    fn ordinary_ethernet_peer_is_rejected() {
        let mut world = SimulatedWorld::new();
        world.apply(SimulateCommand::Usb4Controller).unwrap();
        world.apply(SimulateCommand::CableInserted).unwrap();
        world.apply(SimulateCommand::DirectInterfaceAdded).unwrap();
        let err = world.apply(SimulateCommand::PeerOnEthernet).unwrap_err();
        assert!(matches!(err, CableDeskError::PeerNotOnDirectCable));
    }

    #[test]
    fn no_cable_message() {
        let world = SimulatedWorld::new();
        let err = world.prepared_link().unwrap_err();
        assert!(matches!(err, CableDeskError::NoDirectCableConnection));
        assert_eq!(
            UserFacingError::from(&err).headline,
            "No direct cable connection"
        );
    }

    #[test]
    fn wifi_peer_cannot_become_validated_against_direct_link() {
        let link = PreparedDirectLink::new(
            "sim-thunderbolt0",
            InterfaceClassification::DirectThunderbolt,
        )
        .unwrap();
        let err =
            crate::peer::validate_cable_peer_policy_only(&link, &wifi_peer_claim()).unwrap_err();
        assert!(matches!(err, CableDeskError::PeerNotOnDirectCable));
        let _unused: Option<ValidatedCablePeer> = None;
    }

    #[test]
    fn snapshot_round_trip_preserves_state() {
        let mut world = SimulatedWorld::new();
        world.apply(SimulateCommand::Usb4Controller).unwrap();
        world.apply(SimulateCommand::CableInserted).unwrap();
        world.apply(SimulateCommand::DirectInterfaceAdded).unwrap();
        let restored = SimulatedWorld::from_snapshot(world.to_snapshot());
        assert_eq!(restored.state.current(), ConnectionState::ConfiguringLink);
        assert_eq!(restored.interface_name.as_deref(), Some("sim-thunderbolt0"));
    }
}
