//! The first concrete [`PlatformBackend`] implementation, targeting Fedora
//! Workstation (GNOME, Wayland, systemd, NetworkManager, firewalld,
//! SELinux enforcing — see docs/DISTRO_SUPPORT.md, Tier 1).
//!
//! Every check in this crate is read-only. The mutating methods
//! (`prepare_direct_link`, `install_firewall_policy`) are not implemented
//! yet: they belong to the privileged `cabledesk-helper` and are scoped to
//! Phase 2/3 of docs/ROADMAP.md, not this first read-only foundation.

mod dbus_util;
mod dependencies;
mod direct_link;
mod power;
mod security;
mod system;

use async_trait::async_trait;
use cabledesk_core::error::{CableDeskError, Result};
use cabledesk_platform::{
    CompatibilityCheck, DependencyReport, DirectLink, PlatformBackend, PlatformDiagnostics,
    PowerDeliveryStatus, SecurityReport, SystemInfo,
};

/// Standalone accessor for the direct-link checks, for callers (like
/// `cabledeskctl links`) that want just this subset without a full
/// [`PlatformDiagnostics`] bundle.
pub async fn inspect_direct_link_report() -> Result<Vec<CompatibilityCheck>> {
    direct_link::inspect_direct_link().await
}

#[derive(Debug, Default)]
pub struct FedoraBackend;

impl FedoraBackend {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl PlatformBackend for FedoraBackend {
    async fn detect_system(&self) -> Result<SystemInfo> {
        system::detect_system().await
    }

    async fn check_dependencies(&self) -> Result<DependencyReport> {
        dependencies::check_dependencies().await
    }

    async fn inspect_security_system(&self) -> Result<SecurityReport> {
        security::inspect_security_system().await
    }

    async fn prepare_direct_link(&self, _link: &DirectLink) -> Result<()> {
        Err(CableDeskError::Config(
            "prepare_direct_link is not implemented yet; the Phase 1 foundation only performs \
             read-only compatibility checks (see docs/ROADMAP.md, Phase 2)"
                .to_string(),
        ))
    }

    async fn install_firewall_policy(&self) -> Result<()> {
        Err(CableDeskError::Config(
            "install_firewall_policy is not implemented yet; the Phase 1 foundation only \
             performs read-only compatibility checks (see docs/ROADMAP.md, Phase 2)"
                .to_string(),
        ))
    }

    async fn inspect_power_delivery(&self) -> Result<PowerDeliveryStatus> {
        power::inspect_power_delivery().await
    }

    async fn collect_diagnostics(&self) -> Result<PlatformDiagnostics> {
        let system = self.detect_system().await?;
        let dependencies = self.check_dependencies().await?;
        let security = self.inspect_security_system().await?;
        let power = self.inspect_power_delivery().await?;
        let direct_link = direct_link::inspect_direct_link().await?;

        Ok(PlatformDiagnostics {
            system,
            dependencies,
            security,
            power,
            direct_link,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn collect_diagnostics_succeeds_on_this_host() {
        let backend = FedoraBackend::new();
        let diagnostics = backend.collect_diagnostics().await.unwrap();
        assert!(!diagnostics.system.architecture.is_empty());
        assert!(!diagnostics.dependencies.checks.is_empty());
        assert!(!diagnostics.security.checks.is_empty());
        assert!(!diagnostics.power.checks.is_empty());
        assert!(!diagnostics.direct_link.is_empty());
    }

    #[tokio::test]
    async fn mutating_methods_are_explicitly_unimplemented() {
        let backend = FedoraBackend::new();
        let link = DirectLink {
            interface_name: "thunderbolt0".to_string(),
        };
        assert!(backend.prepare_direct_link(&link).await.is_err());
        assert!(backend.install_firewall_policy().await.is_err());
    }
}
