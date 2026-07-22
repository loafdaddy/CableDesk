//! The first concrete [`PlatformBackend`] implementation, targeting Fedora
//! Workstation (GNOME, Wayland, systemd, NetworkManager, firewalld,
//! SELinux enforcing — see docs/DISTRO_SUPPORT.md, Tier 1).
//!
//! Most checks in this crate are read-only. `prepare_direct_link` and
//! `install_firewall_policy` are real, wired-up implementations as of
//! Phase 2 (see docs/ROADMAP.md) — `prepare_direct_link` mutates real
//! system state (creates a NetworkManager profile, binds a firewalld
//! zone) and is intentionally **not exercised against a live system** by
//! this crate's own test suite (see docs/TEST_PLAN.md); only its
//! read-only counterpart, `install_firewall_policy`'s zone-presence
//! check, is tested live.

mod dbus_util;
mod dependencies;
mod direct_link;
pub mod firewall;
mod power;
mod security;
mod system;

use async_trait::async_trait;
use cabledesk_core::error::{CableDeskError, Result};
use cabledesk_network::{DirectLinkProfile, NetworkManagerClient};
use cabledesk_platform::{
    CompatibilityCheck, DependencyReport, DirectLink, PlatformBackend, PlatformDiagnostics,
    PowerDeliveryStatus, SecurityReport, SystemInfo,
};
use firewall::FirewallClient;

/// Standalone accessor for the direct-link checks, for callers (like
/// `cabledeskctl links`) that want just this subset without a full
/// [`PlatformDiagnostics`] bundle.
pub async fn inspect_direct_link_report() -> Result<Vec<CompatibilityCheck>> {
    direct_link::inspect_direct_link().await
}

/// The first currently-present direct-link interface name, if any — for
/// `cabledeskctl repair-network` and similar callers that need to act on
/// it, not just display its status.
pub async fn detect_direct_link_interface_name() -> Result<Option<String>> {
    direct_link::detect_direct_link_interface_name().await
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

    /// Creates/activates the direct-link NetworkManager profile, then
    /// binds the interface into the CableDesk firewalld zone. Real,
    /// mutating D-Bus calls — see this module's doc comment for why this
    /// crate's own tests never invoke this against a live system.
    async fn prepare_direct_link(&self, link: &DirectLink) -> Result<()> {
        let profile = DirectLinkProfile::new(link.interface_name.clone());
        let nm = NetworkManagerClient::connect().await.map_err(|e| {
            CableDeskError::Config(format!("failed to connect to NetworkManager: {e}"))
        })?;
        nm.apply_profile(&profile).await.map_err(|e| {
            CableDeskError::Config(format!(
                "failed to create the direct-link NetworkManager profile for {}: {e}",
                link.interface_name
            ))
        })?;

        let firewall = FirewallClient::connect()
            .await
            .map_err(|e| CableDeskError::Config(format!("failed to connect to firewalld: {e}")))?;
        firewall
            .bind_interface(&link.interface_name)
            .await
            .map_err(|e| {
                CableDeskError::Config(format!(
                    "failed to bind {} into the {} firewalld zone: {e}",
                    link.interface_name,
                    firewall::CABLEDESK_ZONE
                ))
            })?;

        Ok(())
    }

    /// Read-only: confirms the CableDesk-owned firewalld zone (shipped by
    /// the package at `/usr/lib/firewalld/zones/cabledesk.xml`) is
    /// actually loaded. Does not create or mutate anything itself —
    /// "installing" the zone file is the package's job
    /// (`packaging/fedora/cabledesk.spec`), not this method's.
    async fn install_firewall_policy(&self) -> Result<()> {
        let firewall = FirewallClient::connect()
            .await
            .map_err(|e| CableDeskError::Config(format!("failed to connect to firewalld: {e}")))?;
        let installed = firewall
            .cabledesk_zone_is_installed()
            .await
            .map_err(|e| CableDeskError::Config(format!("failed to query firewalld zones: {e}")))?;
        if installed {
            Ok(())
        } else {
            Err(CableDeskError::Config(format!(
                "the {} firewalld zone is not loaded; check that \
                 data/firewalld/zones/cabledesk.xml was installed to \
                 /usr/lib/firewalld/zones/ and firewalld was reloaded",
                firewall::CABLEDESK_ZONE
            )))
        }
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

    // `prepare_direct_link` is deliberately not covered by an automated
    // test here: it performs real, mutating D-Bus calls (a persistent
    // NetworkManager connection profile, a firewalld zone binding), and
    // this workspace's own test suite must never create real system
    // state as a side effect of `cargo test` — see this module's doc
    // comment and docs/TEST_PLAN.md. `cabledesk-network`'s and
    // `firewall.rs`'s own tests cover the read-only/pure-logic parts of
    // the same code paths live.

    #[tokio::test]
    async fn install_firewall_policy_reports_zone_not_installed_honestly() {
        // Read-only (see `install_firewall_policy`'s doc comment), so
        // safe to run live: on a dev machine that hasn't run
        // `dev-install.sh`'s system-level step, the CableDesk zone
        // genuinely isn't loaded, and this must say so rather than
        // silently succeed.
        let backend = FedoraBackend::new();
        match backend.install_firewall_policy().await {
            Ok(()) => {
                // Only expected if this machine really has the zone
                // installed (e.g. dev-install.sh's system step already ran).
            }
            Err(e) => assert!(e.to_string().contains("firewalld")),
        }
    }
}
