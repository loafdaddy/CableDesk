//! Device identity types shared between the agent, pairing and UI layers.
//!
//! Key material itself (private keys, provisional-channel secrets) lives in
//! `cabledesk-pairing`, not here — this module only holds the
//! serializable, non-secret shapes that get stored and displayed.

use serde::{Deserialize, Serialize};
use std::fmt;

/// A random, unique identifier for a CableDesk installation, generated on
/// first launch. Not secret — this is what gets advertised over mDNS.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DeviceId(pub [u8; 16]);

impl fmt::Display for DeviceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in self.0 {
            write!(f, "{byte:02x}")?;
        }
        Ok(())
    }
}

/// The six-digit pairing verification code shown on both devices.
///
/// This is derived from the provisional channel's key material and is only
/// for human verification during pairing — it is never the long-term trust
/// credential (see docs/SECURITY.md, "Pairing protocol").
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PairingCode(pub [u8; 6]);

impl fmt::Display for PairingCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for digit in self.0 {
            write!(f, "{digit}")?;
        }
        Ok(())
    }
}

impl PairingCode {
    /// Builds a code from six digits (0-9 each), validating range.
    pub fn from_digits(digits: [u8; 6]) -> Option<Self> {
        digits.iter().all(|d| *d <= 9).then_some(Self(digits))
    }
}

/// A previously-paired peer, as stored in the trusted-device database.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrustedDevice {
    pub id: DeviceId,
    pub friendly_name: String,
    /// Human-readable fingerprint of the peer's long-term public key, for
    /// display in the Devices view (e.g. a hex/word-grouped digest).
    pub public_key_fingerprint: String,
    pub role: super::config::Role,
    pub auto_connect: bool,
    pub require_confirmation: bool,
    /// RFC 3339 timestamp of the last successful connection, if any.
    pub last_connected: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn device_id_displays_as_lowercase_hex() {
        let id = DeviceId([0xAB; 16]);
        assert_eq!(id.to_string(), "ab".repeat(16));
    }

    #[test]
    fn pairing_code_rejects_out_of_range_digits() {
        assert!(PairingCode::from_digits([1, 2, 3, 4, 5, 10]).is_none());
        assert!(PairingCode::from_digits([0, 0, 0, 0, 0, 0]).is_some());
    }

    #[test]
    fn pairing_code_displays_six_digits() {
        let code = PairingCode::from_digits([1, 2, 3, 4, 5, 6]).unwrap();
        assert_eq!(code.to_string(), "123456");
    }
}
