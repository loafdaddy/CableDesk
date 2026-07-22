//! Report shapes returned by [`crate::PlatformBackend`]. These are shared
//! across every backend so the UI/CLI can render them without knowing which
//! distribution produced them.

use serde::{Deserialize, Serialize};

/// The status vocabulary used throughout the first-run compatibility check
/// and the Diagnostics view. See docs section "First-run experience".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CheckStatus {
    Available,
    NeedsSetup,
    Warning,
    Unsupported,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompatibilityCheck {
    pub name: String,
    pub status: CheckStatus,
    pub detail: Option<String>,
}

impl CompatibilityCheck {
    pub fn new(name: impl Into<String>, status: CheckStatus, detail: Option<String>) -> Self {
        Self {
            name: name.into(),
            status,
            detail,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SessionType {
    Wayland,
    X11,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SystemInfo {
    pub distro_id: String,
    pub distro_name: String,
    pub distro_version: String,
    pub desktop_environment: Option<String>,
    pub session_type: SessionType,
    pub architecture: String,
    pub kernel_version: String,
    pub hostname: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DependencyReport {
    pub checks: Vec<CompatibilityCheck>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecurityReport {
    pub checks: Vec<CompatibilityCheck>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PowerDeliveryStatus {
    pub checks: Vec<CompatibilityCheck>,
}

/// Configuration for a not-yet-implemented direct-link preparation call.
/// Kept intentionally minimal until Phase 2 (Automated networking) defines
/// the real negotiation/addressing scheme (see docs/adr and NETWORKING.md).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DirectLink {
    pub interface_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlatformDiagnostics {
    pub system: SystemInfo,
    pub dependencies: DependencyReport,
    pub security: SecurityReport,
    pub power: PowerDeliveryStatus,
    pub direct_link: Vec<CompatibilityCheck>,
}
