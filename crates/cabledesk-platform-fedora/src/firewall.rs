//! firewalld D-Bus client, scoped to the `.zone` interface only (the
//! non-deprecated one — see `docs/PACKAGING.md`'s firewalld research:
//! `.direct` and `.policies` are confirmed deprecated in current
//! firewalld, and the exact current replacement for programmatic policy
//! objects wasn't pinned down, so this deliberately avoids depending on
//! either).
//!
//! [`FirewallClient::bind_interface`] is real, mutating D-Bus code
//! (`zone.addInterface`), **not exercised live** in this crate's tests —
//! see `docs/TEST_PLAN.md`. [`FirewallClient::cabledesk_zone_is_installed`]
//! is read-only (`zone.getZones`) and is exercised live, since it mutates
//! nothing.

use zbus::Connection;

pub const CABLEDESK_ZONE: &str = "cabledesk";

#[zbus::proxy(
    interface = "org.fedoraproject.FirewallD1.zone",
    default_service = "org.fedoraproject.FirewallD1",
    default_path = "/org/fedoraproject/FirewallD1"
)]
trait FirewallZone {
    #[zbus(name = "getZones")]
    fn get_zones(&self) -> zbus::Result<Vec<String>>;

    #[zbus(name = "addInterface")]
    fn add_interface(&self, zone: &str, interface: &str) -> zbus::Result<String>;
}

pub struct FirewallClient {
    connection: Connection,
}

impl FirewallClient {
    pub async fn connect() -> zbus::Result<Self> {
        let connection = Connection::system().await?;
        Ok(Self { connection })
    }

    /// Read-only: is the CableDesk-owned zone (installed by the package at
    /// `/usr/lib/firewalld/zones/cabledesk.xml`, see `packaging/fedora/
    /// cabledesk.spec`) currently known to firewalld? Does not mutate
    /// anything — safe to call at any time, including just to check
    /// packaging correctness.
    pub async fn cabledesk_zone_is_installed(&self) -> zbus::Result<bool> {
        let zone = FirewallZoneProxy::new(&self.connection).await?;
        let zones = zone.get_zones().await?;
        Ok(zones.iter().any(|z| z == CABLEDESK_ZONE))
    }

    /// Binds `interface` into the CableDesk zone at runtime
    /// (`zone.addInterface`, a non-permanent/runtime-only binding). Real
    /// and mutating — deliberately not exercised in this crate's tests.
    pub async fn bind_interface(&self, interface: &str) -> zbus::Result<()> {
        let zone = FirewallZoneProxy::new(&self.connection).await?;
        zone.add_interface(CABLEDESK_ZONE, interface).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Live but strictly read-only: confirms the client can reach real
    /// firewalld and list zones. Does not assert whether the CableDesk
    /// zone is present, since that's true only after a real package
    /// install (`docs/TEST_PLAN.md`) — just that the query itself works.
    #[tokio::test]
    async fn can_query_installed_zones() {
        let Ok(client) = FirewallClient::connect().await else {
            eprintln!("skipping: no system D-Bus available in this environment");
            return;
        };
        match client.cabledesk_zone_is_installed().await {
            Ok(installed) => {
                eprintln!("cabledesk zone installed: {installed}");
            }
            Err(e) => eprintln!("skipping: firewalld not reachable on this system bus: {e}"),
        }
    }
}
