//! `_cabledesk._tcp` mDNS service record: what gets advertised, and how
//! it's encoded as Avahi TXT records.
//!
//! Per `docs/NETWORKING.md` §5 and the engineering rule "no secrets in
//! mDNS": every field here is safe to broadcast on the local link.
//! Long-term device identity/trust is established over the pairing
//! protocol's own encrypted channel (`cabledesk-pairing`, not this crate)
//! — mDNS is discovery only, never authentication.

use cabledesk_core::config::Role;

pub const SERVICE_TYPE: &str = "_cabledesk._tcp";

/// mDNS/TXT wire format version. Bump when the TXT record shape changes
/// incompatibly, so an older CableDesk build can at least recognise a
/// newer peer's record is unparseable rather than misreading it.
pub const PROTOCOL_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PairingState {
    Unpaired,
    Paired,
}

impl PairingState {
    fn as_str(self) -> &'static str {
        match self {
            PairingState::Unpaired => "unpaired",
            PairingState::Paired => "paired",
        }
    }

    fn from_str(s: &str) -> Option<Self> {
        match s {
            "unpaired" => Some(PairingState::Unpaired),
            "paired" => Some(PairingState::Paired),
            _ => None,
        }
    }
}

fn role_as_str(role: Role) -> &'static str {
    match role {
        Role::Controller => "controller",
        Role::Host => "host",
        Role::Both => "both",
    }
}

fn role_from_str(s: &str) -> Option<Role> {
    match s {
        "controller" => Some(Role::Controller),
        "host" => Some(Role::Host),
        "both" => Some(Role::Both),
        _ => None,
    }
}

/// Everything CableDesk advertises for one device on the direct-link
/// interface. See the module doc comment: nothing here is secret.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceRecord {
    pub protocol_version: u32,
    pub device_id: String,
    pub friendly_name: String,
    pub role: Role,
    pub pairing_state: PairingState,
    pub control_port: u16,
}

impl ServiceRecord {
    /// Encodes this record as Avahi/DNS-SD TXT record entries: each entry
    /// is one `key=value` byte string, per RFC 6763 §6.
    pub fn to_txt_records(&self) -> Vec<Vec<u8>> {
        vec![
            format!("protocol_version={}", self.protocol_version).into_bytes(),
            format!("device_id={}", self.device_id).into_bytes(),
            format!("friendly_name={}", self.friendly_name).into_bytes(),
            format!("role={}", role_as_str(self.role)).into_bytes(),
            format!("pairing_state={}", self.pairing_state.as_str()).into_bytes(),
        ]
    }

    /// Parses TXT records back into a `ServiceRecord`. `control_port` is
    /// not part of the TXT payload — DNS-SD carries it as the SRV
    /// record's port, supplied separately by whatever resolved the
    /// service (`AvahiClient::resolve` in `crate::avahi`).
    pub fn from_txt_records(records: &[Vec<u8>], control_port: u16) -> Option<Self> {
        let mut protocol_version = None;
        let mut device_id = None;
        let mut friendly_name = None;
        let mut role = None;
        let mut pairing_state = None;

        for record in records {
            let text = std::str::from_utf8(record).ok()?;
            let (key, value) = text.split_once('=')?;
            match key {
                "protocol_version" => protocol_version = value.parse::<u32>().ok(),
                "device_id" => device_id = Some(value.to_string()),
                "friendly_name" => friendly_name = Some(value.to_string()),
                "role" => role = role_from_str(value),
                "pairing_state" => pairing_state = PairingState::from_str(value),
                _ => {} // forward-compatible: ignore unknown keys rather than fail
            }
        }

        Some(Self {
            protocol_version: protocol_version?,
            device_id: device_id?,
            friendly_name: friendly_name?,
            role: role?,
            pairing_state: pairing_state?,
            control_port,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> ServiceRecord {
        ServiceRecord {
            protocol_version: PROTOCOL_VERSION,
            device_id: "abcd1234".to_string(),
            friendly_name: "Test Host".to_string(),
            role: Role::Host,
            pairing_state: PairingState::Unpaired,
            control_port: 47989,
        }
    }

    #[test]
    fn round_trips_through_txt_records() {
        let record = sample();
        let txt = record.to_txt_records();
        let parsed = ServiceRecord::from_txt_records(&txt, record.control_port).unwrap();
        assert_eq!(record, parsed);
    }

    #[test]
    fn rejects_missing_required_field() {
        let incomplete = vec![b"protocol_version=1".to_vec()];
        assert!(ServiceRecord::from_txt_records(&incomplete, 47989).is_none());
    }

    #[test]
    fn ignores_unknown_keys_for_forward_compatibility() {
        let record = sample();
        let mut txt = record.to_txt_records();
        txt.push(b"future_field=some-value-from-a-newer-cabledesk".to_vec());
        let parsed = ServiceRecord::from_txt_records(&txt, record.control_port).unwrap();
        assert_eq!(record, parsed);
    }

    #[test]
    fn never_encodes_anything_that_looks_like_a_secret_field() {
        let txt = sample().to_txt_records();
        for entry in &txt {
            let text = String::from_utf8_lossy(entry).to_lowercase();
            assert!(
                !text.contains("key"),
                "TXT record must not carry key material: {text}"
            );
            assert!(!text.contains("secret"));
            assert!(!text.contains("token"));
            assert!(!text.contains("password"));
        }
    }
}
