//! Per-user CableDesk configuration: role selection and friendly device
//! name. Stored under `$XDG_CONFIG_HOME/cabledesk/device.json` (typically
//! `~/.config/cabledesk/device.json`).

use serde::{Deserialize, Serialize};

/// The current on-disk schema version. Bump this and add a migration path
/// in [`DeviceConfig::migrate`] whenever the shape changes.
pub const CURRENT_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Role {
    Controller,
    Host,
    Both,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeviceConfig {
    pub schema_version: u32,
    pub friendly_name: String,
    pub role: Role,
    /// Whether every connection requires interactive confirmation, even
    /// from an already-trusted, auto-connect-eligible peer.
    pub require_confirmation_always: bool,
}

impl DeviceConfig {
    pub fn new(friendly_name: impl Into<String>, role: Role) -> Self {
        Self {
            schema_version: CURRENT_SCHEMA_VERSION,
            friendly_name: friendly_name.into(),
            role,
            require_confirmation_always: false,
        }
    }

    /// Parses a config, applying forward migrations if it was written by an
    /// older CableDesk version. Returns an error for a schema version newer
    /// than this build understands (downgrade is not supported).
    pub fn migrate(mut self) -> crate::error::Result<Self> {
        if self.schema_version > CURRENT_SCHEMA_VERSION {
            return Err(crate::error::CableDeskError::Config(format!(
                "config schema version {} is newer than supported version {}",
                self.schema_version, CURRENT_SCHEMA_VERSION
            )));
        }
        // No migrations exist yet; schema_version 1 is the only version.
        self.schema_version = CURRENT_SCHEMA_VERSION;
        Ok(self)
    }

    pub fn to_json(&self) -> crate::error::Result<String> {
        Ok(serde_json::to_string_pretty(self)?)
    }

    pub fn from_json(data: &str) -> crate::error::Result<Self> {
        let parsed: Self = serde_json::from_str(data)?;
        parsed.migrate()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_through_json() {
        let config = DeviceConfig::new("Test Host", Role::Host);
        let json = config.to_json().unwrap();
        let parsed = DeviceConfig::from_json(&json).unwrap();
        assert_eq!(config, parsed);
    }

    #[test]
    fn rejects_future_schema_version() {
        let json = r#"{"schema_version":999,"friendly_name":"x","role":"Host","require_confirmation_always":false}"#;
        assert!(DeviceConfig::from_json(json).is_err());
    }

    #[test]
    fn migrates_missing_version_forward() {
        let config = DeviceConfig::new("x", Role::Both);
        let migrated = config.clone().migrate().unwrap();
        assert_eq!(migrated.schema_version, CURRENT_SCHEMA_VERSION);
    }
}
