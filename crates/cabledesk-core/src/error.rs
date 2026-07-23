//! Error model shared by every CableDesk component.
//!
//! [`CableDeskError`] is what code returns internally. [`UserFacingError`] is
//! the *only* thing the GTK interface is allowed to render directly — raw
//! `Debug`/`Display` output of a [`CableDeskError`] must stay in diagnostics,
//! never in the primary error surface (see docs/OPEN_QUESTIONS.md and
//! engineering rule "never display raw Rust debug output as the main error").

use thiserror::Error;

pub type Result<T> = std::result::Result<T, CableDeskError>;

#[derive(Debug, Error)]
pub enum CableDeskError {
    #[error("no compatible direct-link hardware was detected")]
    NoDirectLinkHardware,

    #[error("a direct interface did not appear after cable connection")]
    DirectInterfaceMissing,

    #[error("peer device identity did not match the trusted record")]
    UntrustedPeerIdentity,

    #[error("the direct cable connection was lost")]
    LinkLost,

    #[error("no direct cable connection")]
    NoDirectCableConnection,

    #[error("peer was not discovered or reachable through the direct cable interface")]
    PeerNotOnDirectCable,

    #[error("hardware video encoding is unavailable")]
    NoHardwareEncoder,

    #[error("required platform dependency is missing: {0}")]
    MissingDependency(String),

    #[error("platform detection failed: {0}")]
    Detection(String),

    #[error("configuration error: {0}")]
    Config(String),

    #[error("state transition error: {0}")]
    Transition(#[from] crate::state::TransitionError),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("(de)serialization error: {0}")]
    Serde(#[from] serde_json::Error),
}

/// A message CableDesk is allowed to show a user directly: a short,
/// plain-language `headline` plus an optional `detail` line. The underlying
/// [`CableDeskError`] is preserved separately for diagnostics/logs only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserFacingError {
    pub headline: String,
    pub detail: Option<String>,
}

impl UserFacingError {
    pub fn new(headline: impl Into<String>) -> Self {
        Self {
            headline: headline.into(),
            detail: None,
        }
    }

    pub fn with_detail(headline: impl Into<String>, detail: impl Into<String>) -> Self {
        Self {
            headline: headline.into(),
            detail: Some(detail.into()),
        }
    }
}

impl From<&CableDeskError> for UserFacingError {
    fn from(err: &CableDeskError) -> Self {
        match err {
            CableDeskError::NoDirectLinkHardware => UserFacingError::with_detail(
                "No compatible USB4 or Thunderbolt connection was detected.",
                "This USB-C port may support charging or display output without \
                 supporting host-to-host USB4 networking.",
            ),
            CableDeskError::DirectInterfaceMissing => UserFacingError::with_detail(
                "The cable was detected, but Linux did not create a direct network interface.",
                "Check that the cable supports USB4 or Thunderbolt data.",
            ),
            CableDeskError::UntrustedPeerIdentity => UserFacingError::with_detail(
                "The connected device's security identity did not match the trusted device.",
                "The connection was blocked.",
            ),
            CableDeskError::LinkLost => UserFacingError::with_detail(
                "The direct cable connection was lost.",
                "CableDesk did not fall back to Wi-Fi or Ethernet.",
            ),
            CableDeskError::NoDirectCableConnection => {
                UserFacingError::new("No direct cable connection")
            }
            CableDeskError::PeerNotOnDirectCable => UserFacingError::with_detail(
                "Peer rejected because it was not discovered through the direct cable interface.",
                "CableDesk does not connect over Wi-Fi, Ethernet, or any other route.",
            ),
            CableDeskError::NoHardwareEncoder => UserFacingError::with_detail(
                "Hardware video encoding is unavailable.",
                "CableDesk can try software encoding, but performance may be reduced.",
            ),
            CableDeskError::MissingDependency(dep) => UserFacingError::with_detail(
                "A required component is not installed.",
                format!("Missing: {dep}"),
            ),
            CableDeskError::Detection(_)
            | CableDeskError::Config(_)
            | CableDeskError::Transition(_)
            | CableDeskError::Io(_)
            | CableDeskError::Serde(_) => UserFacingError::with_detail(
                "Something went wrong while setting up the direct connection.",
                "See diagnostics for technical details.",
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_facing_never_leaks_debug_formatting() {
        let err = CableDeskError::Config("secret-token=abc123".to_string());
        let user_facing = UserFacingError::from(&err);
        assert!(!user_facing.headline.contains("secret-token"));
        assert!(user_facing
            .detail
            .as_deref()
            .unwrap_or_default()
            .contains("diagnostics"));
    }

    #[test]
    fn every_variant_maps_to_a_non_empty_headline() {
        let samples = [
            CableDeskError::NoDirectLinkHardware,
            CableDeskError::DirectInterfaceMissing,
            CableDeskError::UntrustedPeerIdentity,
            CableDeskError::LinkLost,
            CableDeskError::NoDirectCableConnection,
            CableDeskError::PeerNotOnDirectCable,
            CableDeskError::NoHardwareEncoder,
            CableDeskError::MissingDependency("sunshine".into()),
        ];
        for err in samples {
            let uf = UserFacingError::from(&err);
            assert!(!uf.headline.is_empty());
        }
    }
}
