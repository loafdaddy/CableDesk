//! `cabledesk-agent` — unprivileged per-user CableDesk session service.
//!
//! Watches NetworkManager for the direct USB4/Thunderbolt interface, asks
//! `cabledesk-helper` to prepare the cable-only NetworkManager profile
//! (Polkit-gated), publishes/browses `_cabledesk._tcp` via Avahi scoped to
//! that interface, and validates peers with [`validate_cable_peer`].
//! Pairing and streaming are not implemented yet.

use cabledesk_core::config::Role;
use cabledesk_core::state::{ConnectionState, StateMachine};
use cabledesk_discovery::{
    AvahiClient, DiscoveryEvent, PairingState, PublishedService, ServiceRecord, PROTOCOL_VERSION,
};
use cabledesk_network::{
    interface_index, read_interface_driver, require_direct_cable_interface, validate_cable_peer,
    DirectLinkEvent, NetworkManagerClient, PeerClaim, PreparedDirectLink, ValidatedCablePeer,
};
use futures_util::StreamExt;
use serde::Serialize;
use std::net::IpAddr;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::Mutex as AsyncMutex;
use tokio::task::JoinHandle;
use zbus::{connection, interface, proxy, Connection as ZbusConnection};

const SERVICE_NAME: &str = "org.cabledesk.Agent1";
const OBJECT_PATH: &str = "/org/cabledesk/Agent";
const RECONNECT_DELAY: Duration = Duration::from_secs(5);
/// Placeholder control-channel port advertised in mDNS until Phase 3.
const CONTROL_PORT: u16 = 27000;

#[derive(Debug, Clone, Default, Serialize)]
struct SessionSnapshot {
    state: String,
    interface: Option<String>,
    driver: Option<String>,
    device_id: String,
    last_peer_address: Option<String>,
    last_peer_name: Option<String>,
    last_error: Option<String>,
}

struct SessionInner {
    interface_name: Option<String>,
    driver: Option<String>,
    device_id: String,
    friendly_name: String,
    last_peer: Option<ValidatedCablePeer>,
    last_peer_name: Option<String>,
    last_error: Option<String>,
    published: Option<PublishedService>,
    browse_task: Option<JoinHandle<()>>,
}

impl SessionInner {
    fn new() -> Self {
        let hostname = hostname_fallback();
        Self {
            interface_name: None,
            driver: None,
            device_id: local_device_id(&hostname),
            friendly_name: hostname,
            last_peer: None,
            last_peer_name: None,
            last_error: None,
            published: None,
            browse_task: None,
        }
    }

    fn clear_link(&mut self) {
        self.interface_name = None;
        self.driver = None;
        self.last_peer = None;
        self.last_peer_name = None;
        if let Some(task) = self.browse_task.take() {
            task.abort();
        }
    }
}

type SharedSession = Arc<AsyncMutex<SessionInner>>;

struct Agent {
    state: Arc<Mutex<StateMachine>>,
    session: SharedSession,
}

#[interface(name = "org.cabledesk.Agent1")]
impl Agent {
    #[zbus(property)]
    async fn version(&self) -> String {
        env!("CARGO_PKG_VERSION").to_string()
    }

    async fn get_state(&self) -> String {
        self.state
            .lock()
            .expect("state mutex poisoned")
            .current()
            .to_string()
    }

    /// JSON session snapshot for CLI/UI testing (no secrets).
    async fn get_session_json(&self) -> String {
        let state = self
            .state
            .lock()
            .expect("state mutex poisoned")
            .current()
            .to_string();
        let session = self.session.lock().await;
        let snap = SessionSnapshot {
            state,
            interface: session.interface_name.clone(),
            driver: session.driver.clone(),
            device_id: session.device_id.clone(),
            last_peer_address: session.last_peer.as_ref().map(|p| p.address.to_string()),
            last_peer_name: session.last_peer_name.clone(),
            last_error: session.last_error.clone(),
        };
        serde_json::to_string(&snap).unwrap_or_else(|_| "{}".into())
    }
}

fn try_transition(state: &Mutex<StateMachine>, next: ConnectionState) {
    let mut sm = state.lock().expect("state mutex poisoned");
    let from = sm.current();
    match sm.transition(next) {
        Ok(_) => tracing::info!("state: {from} -> {next:?}"),
        Err(e) => tracing::debug!("ignored illegal transition {from} -> {next:?}: {e}"),
    }
}

fn force_waiting_for_cable(state: &Mutex<StateMachine>) {
    let mut sm = state.lock().expect("state mutex poisoned");
    let current = sm.current();
    if current == ConnectionState::WaitingForCable {
        return;
    }
    if current.can_transition_to(ConnectionState::WaitingForCable) {
        let _ = sm.transition(ConnectionState::WaitingForCable);
        tracing::info!("state: {current} -> WaitingForCable");
        return;
    }
    let _ = sm.transition(ConnectionState::Error);
    let _ = sm.transition(ConnectionState::WaitingForCable);
    tracing::info!("state: {current} -> Error -> WaitingForCable (cable lost)");
}

fn hostname_fallback() -> String {
    std::fs::read_to_string("/etc/hostname")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "cabledesk-device".into())
}

fn local_device_id(hostname: &str) -> String {
    let machine = std::fs::read_to_string("/etc/machine-id")
        .unwrap_or_default()
        .trim()
        .chars()
        .take(16)
        .collect::<String>();
    if machine.is_empty() {
        format!("dev-{}", hostname.chars().take(12).collect::<String>())
    } else {
        machine
    }
}

#[proxy(
    interface = "org.cabledesk.Helper1",
    default_service = "org.cabledesk.Helper1",
    default_path = "/org/cabledesk/Helper"
)]
trait Helper1 {
    async fn prepare_direct_link(&self, interface_name: &str) -> zbus::Result<()>;
}

async fn try_prepare_via_helper(interface_name: &str) -> Result<(), String> {
    let conn = ZbusConnection::system()
        .await
        .map_err(|e| format!("system bus: {e}"))?;
    let helper = Helper1Proxy::new(&conn)
        .await
        .map_err(|e| format!("helper proxy: {e}"))?;
    helper
        .prepare_direct_link(interface_name)
        .await
        .map_err(|e| format!("PrepareDirectLink: {e}"))
}

async fn tear_down_session(session: &SharedSession) {
    let mut s = session.lock().await;
    if let Some(published) = s.published.take() {
        if let Err(e) = published.withdraw().await {
            tracing::warn!("failed to withdraw mDNS advertisement: {e}");
        }
    }
    s.clear_link();
}

async fn start_discovery(
    state: Arc<Mutex<StateMachine>>,
    session: SharedSession,
    interface_name: String,
) {
    let driver = read_interface_driver(&interface_name);
    let classification = match require_direct_cable_interface(&interface_name) {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!("refusing non-direct interface {interface_name}: {e}");
            let mut s = session.lock().await;
            s.last_error = Some(e.to_string());
            force_waiting_for_cable(&state);
            return;
        }
    };

    let idx = match interface_index(&interface_name) {
        Ok(i) => i,
        Err(e) => {
            tracing::warn!("no ifindex for {interface_name}: {e}");
            let mut s = session.lock().await;
            s.last_error = Some(e.to_string());
            force_waiting_for_cable(&state);
            return;
        }
    };

    {
        let mut s = session.lock().await;
        s.interface_name = Some(interface_name.clone());
        s.driver = driver.clone();
        s.last_error = None;
        s.last_peer = None;
        s.last_peer_name = None;
    }

    try_transition(&state, ConnectionState::CableDetected);
    try_transition(&state, ConnectionState::InspectingPower);
    try_transition(&state, ConnectionState::ConfiguringLink);

    match try_prepare_via_helper(&interface_name).await {
        Ok(()) => tracing::info!("direct-link profile prepared via helper for {interface_name}"),
        Err(e) => {
            tracing::warn!(
                "helper prepare failed ({e}); continuing to discovery if the link is already up"
            );
            let mut s = session.lock().await;
            s.last_error = Some(format!("prepare: {e}"));
        }
    }

    let mut link = match PreparedDirectLink::new(interface_name.clone(), classification) {
        Ok(l) => l,
        Err(e) => {
            tracing::warn!("prepared link rejected: {e}");
            force_waiting_for_cable(&state);
            return;
        }
    };
    link.interface_index = Some(idx);

    let (device_id, friendly_name) = {
        let s = session.lock().await;
        (s.device_id.clone(), s.friendly_name.clone())
    };

    let record = ServiceRecord {
        protocol_version: PROTOCOL_VERSION,
        device_id: device_id.clone(),
        friendly_name,
        role: Role::Both,
        pairing_state: PairingState::Unpaired,
        control_port: CONTROL_PORT,
    };

    let avahi = match AvahiClient::connect().await {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!("Avahi unavailable: {e}");
            let mut s = session.lock().await;
            s.last_error = Some(format!("avahi: {e}"));
            return;
        }
    };

    match avahi.publish(idx as i32, &record).await {
        Ok(published) => {
            let mut s = session.lock().await;
            s.published = Some(published);
            tracing::info!("published _cabledesk._tcp on {interface_name} (ifindex {idx})");
        }
        Err(e) => {
            tracing::warn!("mDNS publish failed: {e}");
            let mut s = session.lock().await;
            s.last_error = Some(format!("publish: {e}"));
        }
    }

    try_transition(&state, ConnectionState::DiscoveringPeer);

    let browse = match avahi.browse(idx as i32).await {
        Ok(stream) => stream,
        Err(e) => {
            tracing::warn!("mDNS browse failed: {e}");
            let mut s = session.lock().await;
            s.last_error = Some(format!("browse: {e}"));
            return;
        }
    };

    let state_b = state.clone();
    let session_b = session.clone();
    let iface_b = interface_name.clone();
    let driver_b = driver.clone().unwrap_or_else(|| "thunderbolt-net".into());
    let our_id = device_id;

    let handle = tokio::spawn(async move {
        tokio::pin!(browse);
        while let Some(event) = browse.next().await {
            match event {
                DiscoveryEvent::PeerAppeared { name } => {
                    if name == our_id {
                        continue;
                    }
                    tracing::info!("peer advertised on direct link: {name}");
                    match avahi.resolve(idx as i32, &name).await {
                        Ok(resolved) => {
                            let Ok(addr): Result<IpAddr, _> = resolved.address.parse() else {
                                tracing::warn!(
                                    "peer {name} address unparsable: {}",
                                    resolved.address
                                );
                                continue;
                            };
                            if let Some(rec) = &resolved.record {
                                if rec.device_id == our_id {
                                    continue;
                                }
                            }
                            let claim = PeerClaim {
                                address: addr,
                                discovered_on_interface: iface_b.clone(),
                                discovered_on_driver: Some(driver_b.clone()),
                            };
                            match validate_cable_peer(&link, &claim).await {
                                Ok(peer) => {
                                    tracing::info!(
                                        "validated cable peer {} on {}",
                                        peer.address,
                                        peer.interface_name
                                    );
                                    let mut s = session_b.lock().await;
                                    s.last_peer = Some(peer);
                                    s.last_peer_name = Some(name.clone());
                                    s.last_error = None;
                                    drop(s);
                                    try_transition(&state_b, ConnectionState::PairingRequired);
                                }
                                Err(e) => {
                                    tracing::warn!("peer {name} rejected (cable-only policy): {e}");
                                    let mut s = session_b.lock().await;
                                    s.last_error = Some(format!("peer rejected: {e}"));
                                }
                            }
                        }
                        Err(e) => tracing::warn!("resolve {name} failed: {e}"),
                    }
                }
                DiscoveryEvent::PeerDisappeared { name } => {
                    tracing::info!("peer left direct link: {name}");
                    let mut s = session_b.lock().await;
                    if s.last_peer_name.as_deref() == Some(name.as_str()) {
                        s.last_peer = None;
                        s.last_peer_name = None;
                    }
                }
            }
        }
    });

    session.lock().await.browse_task = Some(handle);
}

async fn run_hotplug_watcher(state: Arc<Mutex<StateMachine>>, session: SharedSession) {
    loop {
        let client = match NetworkManagerClient::connect().await {
            Ok(client) => client,
            Err(e) => {
                tracing::warn!(
                    "cannot connect to NetworkManager, retrying in {RECONNECT_DELAY:?}: {e}"
                );
                tokio::time::sleep(RECONNECT_DELAY).await;
                continue;
            }
        };

        let events = match client.watch_direct_link_events().await {
            Ok(events) => events,
            Err(e) => {
                tracing::warn!(
                    "cannot subscribe to NetworkManager device events, retrying in {RECONNECT_DELAY:?}: {e}"
                );
                tokio::time::sleep(RECONNECT_DELAY).await;
                continue;
            }
        };

        tracing::info!("watching for the direct-link interface");
        tokio::pin!(events);
        while let Some(event) = events.next().await {
            match event {
                DirectLinkEvent::Appeared { interface_name } => {
                    tracing::info!("direct-link interface appeared: {interface_name}");
                    tear_down_session(&session).await;
                    start_discovery(state.clone(), session.clone(), interface_name).await;
                }
                DirectLinkEvent::Disappeared { interface_name } => {
                    tracing::info!("direct-link interface disappeared: {interface_name}");
                    tear_down_session(&session).await;
                    force_waiting_for_cable(&state);
                }
            }
        }

        tracing::warn!(
            "NetworkManager device event stream ended, reconnecting in {RECONNECT_DELAY:?}"
        );
        tokio::time::sleep(RECONNECT_DELAY).await;
    }
}

fn init_logging() {
    use tracing_subscriber::prelude::*;

    let journald_layer = tracing_journald::layer().ok();
    let filter =
        tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into());

    tracing_subscriber::registry()
        .with(journald_layer)
        .with(tracing_subscriber::fmt::layer())
        .with(filter)
        .init();
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_logging();
    tracing::info!("starting cabledesk-agent {}", env!("CARGO_PKG_VERSION"));

    let state = Arc::new(Mutex::new(StateMachine::new()));
    let session = Arc::new(AsyncMutex::new(SessionInner::new()));

    try_transition(&state, ConnectionState::CheckingCompatibility);
    try_transition(&state, ConnectionState::Ready);
    try_transition(&state, ConnectionState::WaitingForCable);

    tokio::spawn(run_hotplug_watcher(state.clone(), session.clone()));

    let agent = Agent { state, session };

    let _connection = connection::Builder::session()?
        .name(SERVICE_NAME)?
        .serve_at(OBJECT_PATH, agent)?
        .build()
        .await?;

    tracing::info!("cabledesk-agent ready on {SERVICE_NAME} ({OBJECT_PATH})");
    std::future::pending::<()>().await;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use cabledesk_network::{reject_if_discovery_not_direct_cable, InterfaceClassification};

    #[test]
    fn force_waiting_from_configuring_link_uses_error_bridge() {
        let sm = Mutex::new(StateMachine::new());
        try_transition(&sm, ConnectionState::CheckingCompatibility);
        try_transition(&sm, ConnectionState::Ready);
        try_transition(&sm, ConnectionState::WaitingForCable);
        try_transition(&sm, ConnectionState::CableDetected);
        try_transition(&sm, ConnectionState::InspectingPower);
        try_transition(&sm, ConnectionState::ConfiguringLink);
        force_waiting_for_cable(&sm);
        assert_eq!(
            sm.lock().unwrap().current(),
            ConnectionState::WaitingForCable
        );
    }

    #[test]
    fn wifi_discovery_still_rejected_by_policy() {
        assert!(reject_if_discovery_not_direct_cable(Some("iwlwifi"), "wlan0").is_err());
        assert!(!InterfaceClassification::WiFi.is_direct_cable());
    }
}
