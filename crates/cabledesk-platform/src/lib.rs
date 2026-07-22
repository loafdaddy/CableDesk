//! Distribution-independent platform abstraction.
//!
//! At least 90% of CableDesk must not know which distribution it is running
//! on. Everything that does is implemented behind [`PlatformBackend`], with
//! the first concrete implementation in `cabledesk-platform-fedora`.

pub mod report;

pub use report::{
    CheckStatus, CompatibilityCheck, DependencyReport, DirectLink, PlatformDiagnostics,
    PowerDeliveryStatus, SecurityReport, SessionType, SystemInfo,
};

use async_trait::async_trait;
use cabledesk_core::error::Result;

/// Everything CableDesk needs to know from, or do to, the host operating
/// system. Implementations must never assume Fedora-specific paths, package
/// names or service managers outside their own module.
#[async_trait]
pub trait PlatformBackend: Send + Sync {
    /// Distro, desktop environment, session type, kernel, architecture.
    async fn detect_system(&self) -> Result<SystemInfo>;

    /// Presence of runtime dependencies (GTK4, NetworkManager, Avahi,
    /// firewalld, PipeWire, Sunshine/Moonlight runtimes, etc).
    async fn check_dependencies(&self) -> Result<DependencyReport>;

    /// SELinux/AppArmor and firewall state, read-only.
    async fn inspect_security_system(&self) -> Result<SecurityReport>;

    /// Create or repair the dedicated NetworkManager profile for the direct
    /// link. Mutating and privileged — not implemented in the Phase 1
    /// read-only foundation; see docs/OPEN_QUESTIONS.md.
    async fn prepare_direct_link(&self, link: &DirectLink) -> Result<()>;

    /// Install the CableDesk-owned firewalld zone/policy. Mutating and
    /// privileged — not implemented in the Phase 1 read-only foundation.
    async fn install_firewall_policy(&self) -> Result<()>;

    /// USB-C Power Delivery role/negotiation and battery status, read-only.
    async fn inspect_power_delivery(&self) -> Result<PowerDeliveryStatus>;

    /// The full sanitised diagnostics bundle shown in the Diagnostics view
    /// and exported via `cabledeskctl diagnostics`.
    async fn collect_diagnostics(&self) -> Result<PlatformDiagnostics>;
}
