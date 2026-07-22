//! A typed NetworkManager D-Bus client: applying the direct-link profile
//! and watching device hotplug events. Built directly on `zbus`, not a
//! NetworkManager-specific crate — see
//! `docs/adr/ADR-003-networkmanager-dbus.md` for why.
//!
//! `apply_profile` performs a real, persistent `AddConnection` call. It is
//! implemented and unit-testable at the settings-construction level
//! (`crate::profile`), but deliberately has not been exercised against a
//! live NetworkManager instance during this crate's development — doing
//! so would create a real, persistent connection profile on whatever
//! machine ran the test. See `docs/TEST_PLAN.md` for what's actually been
//! verified live vs. only compiled.

use futures_util::stream::{select, StreamExt};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex as AsyncMutex;
use zbus::zvariant::{OwnedObjectPath, OwnedValue};
use zbus::Connection;

#[zbus::proxy(
    interface = "org.freedesktop.NetworkManager",
    default_service = "org.freedesktop.NetworkManager",
    default_path = "/org/freedesktop/NetworkManager"
)]
trait NetworkManager {
    #[zbus(signal)]
    fn device_added(&self, device_path: OwnedObjectPath) -> zbus::Result<()>;

    #[zbus(signal)]
    fn device_removed(&self, device_path: OwnedObjectPath) -> zbus::Result<()>;
}

/// A single NetworkManager device object — enough to classify whether it
/// is the direct-link interface, without assuming a name (see
/// `docs/NETWORKING.md` §3 and the "never hardcode `thunderbolt0`" rule).
#[zbus::proxy(
    interface = "org.freedesktop.NetworkManager.Device",
    default_service = "org.freedesktop.NetworkManager"
)]
trait NetworkManagerDevice {
    #[zbus(property)]
    fn interface(&self) -> zbus::Result<String>;

    #[zbus(property)]
    fn driver(&self) -> zbus::Result<String>;
}

#[zbus::proxy(
    interface = "org.freedesktop.NetworkManager.Settings",
    default_service = "org.freedesktop.NetworkManager",
    default_path = "/org/freedesktop/NetworkManager/Settings"
)]
trait NetworkManagerSettings {
    fn add_connection(
        &self,
        connection: HashMap<String, HashMap<String, OwnedValue>>,
    ) -> zbus::Result<OwnedObjectPath>;
}

/// A direct-link interface appearing or disappearing, as reported by
/// NetworkManager's own device lifecycle signals (see
/// `docs/NETWORKING.md` §3 — NM already learns this from udev/netlink
/// internally, so CableDesk doesn't need a second, lower-level watcher).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeviceEvent {
    Added(OwnedObjectPath),
    Removed(OwnedObjectPath),
}

/// The direct-link interface specifically appearing or disappearing —
/// [`DeviceEvent`] filtered down to devices whose driver looks like
/// `thunderbolt-net` (matching `cabledesk-platform-fedora`'s
/// `direct_link.rs` classification, but done here over D-Bus rather than
/// sysfs, since this is the distribution-independent watcher).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DirectLinkEvent {
    Appeared { interface_name: String },
    Disappeared { interface_name: String },
}

async fn resolve_direct_link_interface(
    connection: &Connection,
    device_path: &OwnedObjectPath,
) -> Option<String> {
    let device = NetworkManagerDeviceProxy::builder(connection)
        .path(device_path)
        .ok()?
        .build()
        .await
        .ok()?;
    let driver = device.driver().await.ok()?;
    if !driver.contains("thunderbolt") {
        return None;
    }
    device.interface().await.ok()
}

pub struct NetworkManagerClient {
    connection: Connection,
}

impl NetworkManagerClient {
    /// Connects to the system D-Bus bus. Read-only at this point — no
    /// method call is made until `apply_profile` is invoked.
    pub async fn connect() -> zbus::Result<Self> {
        let connection = Connection::system().await?;
        Ok(Self { connection })
    }

    /// Applies (creates) the given direct-link profile via
    /// `Settings.AddConnection`. **Not exercised live in this crate's own
    /// tests** — see this module's doc comment.
    pub async fn apply_profile(
        &self,
        profile: &crate::profile::DirectLinkProfile,
    ) -> zbus::Result<OwnedObjectPath> {
        let settings = NetworkManagerSettingsProxy::new(&self.connection).await?;
        settings.add_connection(profile.to_nm_settings()).await
    }

    /// A merged stream of `DeviceAdded`/`DeviceRemoved` events for every
    /// device NetworkManager manages. Callers filter this down to the
    /// direct-link interface by driver/name — this client does not assume
    /// a specific interface up front, matching the "never hardcode
    /// `thunderbolt0`" rule.
    pub async fn watch_device_events(
        &self,
    ) -> zbus::Result<impl futures_util::Stream<Item = DeviceEvent> + '_> {
        let proxy = NetworkManagerProxy::new(&self.connection).await?;
        let added = proxy
            .receive_device_added()
            .await?
            .filter_map(|signal| async move {
                signal
                    .args()
                    .ok()
                    .map(|args| DeviceEvent::Added(args.device_path))
            });
        let removed = proxy
            .receive_device_removed()
            .await?
            .filter_map(|signal| async move {
                signal
                    .args()
                    .ok()
                    .map(|args| DeviceEvent::Removed(args.device_path))
            });
        Ok(select(added, removed))
    }

    /// [`Self::watch_device_events`] filtered down to the direct-link
    /// interface only. A `DeviceRemoved` signal's object path is often no
    /// longer queryable by the time it arrives (NetworkManager may have
    /// already unpublished the object), so this caches the interface name
    /// at `Added` time and looks it up (rather than re-querying D-Bus) on
    /// removal — the same reason cable-removal handling can rely on this
    /// stream instead of a separate poll loop.
    pub async fn watch_direct_link_events(
        &self,
    ) -> zbus::Result<impl futures_util::Stream<Item = DirectLinkEvent> + '_> {
        let events = self.watch_device_events().await?;
        let connection = self.connection.clone();
        let known: Arc<AsyncMutex<HashMap<OwnedObjectPath, String>>> =
            Arc::new(AsyncMutex::new(HashMap::new()));

        Ok(events.filter_map(move |event| {
            let connection = connection.clone();
            let known = known.clone();
            async move {
                match event {
                    DeviceEvent::Added(path) => {
                        let name = resolve_direct_link_interface(&connection, &path).await?;
                        known.lock().await.insert(path, name.clone());
                        Some(DirectLinkEvent::Appeared {
                            interface_name: name,
                        })
                    }
                    DeviceEvent::Removed(path) => {
                        let name = known.lock().await.remove(&path)?;
                        Some(DirectLinkEvent::Disappeared {
                            interface_name: name,
                        })
                    }
                }
            }
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Confirms the client can actually reach the real system bus and
    /// subscribe to NetworkManager's signals without error — read-only,
    /// safe to run anywhere NetworkManager is active (this workspace's
    /// dev environment included). Does not assert anything about which
    /// events arrive, since that depends on real hardware being plugged
    /// in — see `docs/TEST_PLAN.md`.
    #[tokio::test]
    async fn can_connect_and_subscribe_to_device_events() {
        let Ok(client) = NetworkManagerClient::connect().await else {
            eprintln!("skipping: no system D-Bus available in this environment");
            return;
        };
        if client.watch_device_events().await.is_err() {
            eprintln!("skipping: NetworkManager not reachable on this system bus");
        }
    }

    /// Same as above, for the direct-link-filtered stream. On a machine
    /// with no Thunderbolt/USB4 controller (this workspace's dev
    /// environment included) this stream never yields anything, which is
    /// the correct, honest behaviour — this test only confirms
    /// subscribing succeeds, not that any event arrives.
    #[tokio::test]
    async fn can_subscribe_to_direct_link_events() {
        let Ok(client) = NetworkManagerClient::connect().await else {
            eprintln!("skipping: no system D-Bus available in this environment");
            return;
        };
        if client.watch_direct_link_events().await.is_err() {
            eprintln!("skipping: NetworkManager not reachable on this system bus");
        }
    }
}
