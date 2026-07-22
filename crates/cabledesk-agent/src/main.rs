//! `cabledesk-agent` — the unprivileged, per-user CableDesk background
//! service (`cabledesk-agent.service`, a systemd `--user` unit).
//!
//! As of Phase 2 (`docs/ROADMAP.md`), the agent watches NetworkManager for
//! the direct-link interface appearing/disappearing
//! (`cabledesk_network::NetworkManagerClient::watch_direct_link_events`)
//! and drives the shared [`cabledesk_core::state::StateMachine`]
//! accordingly, so `GetState` reflects real hotplug activity instead of a
//! hardcoded value. Pairing and streaming orchestration are still not
//! implemented — see `docs/ROADMAP.md`.

use cabledesk_core::state::{ConnectionState, StateMachine};
use cabledesk_network::{DirectLinkEvent, NetworkManagerClient};
use futures_util::StreamExt;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use zbus::{connection, interface};

const SERVICE_NAME: &str = "org.cabledesk.Agent1";
const OBJECT_PATH: &str = "/org/cabledesk/Agent";
const RECONNECT_DELAY: Duration = Duration::from_secs(5);

struct Agent {
    state: Arc<Mutex<StateMachine>>,
}

#[interface(name = "org.cabledesk.Agent1")]
impl Agent {
    #[zbus(property)]
    async fn version(&self) -> String {
        env!("CARGO_PKG_VERSION").to_string()
    }

    /// Returns the current [`cabledesk_core::state::ConnectionState`] as its
    /// `Debug` name (e.g. `"WaitingForCable"`, `"CableDetected"`).
    async fn get_state(&self) -> String {
        self.state
            .lock()
            .expect("state mutex poisoned")
            .current()
            .to_string()
    }
}

/// Applies a transition, logging and discarding it if illegal rather than
/// panicking — a stray or duplicate hotplug event must never crash the
/// agent (see `docs/THREAT_MODEL.md`).
fn try_transition(state: &Mutex<StateMachine>, next: ConnectionState) {
    let mut sm = state.lock().expect("state mutex poisoned");
    let from = sm.current();
    match sm.transition(next) {
        Ok(_) => tracing::info!("state: {from} -> {next:?}"),
        Err(e) => tracing::debug!("ignored illegal transition {from} -> {next:?}: {e}"),
    }
}

/// Watches NetworkManager for the direct-link interface and drives the
/// shared state machine. Reconnects with a fixed delay if the D-Bus
/// connection or subscription fails or ends — this must never give up
/// permanently, since the agent is meant to run for the lifetime of the
/// user session. Cable removal always forces a return to
/// `WaitingForCable`, never a "streaming over something else" state — see
/// `docs/adr/ADR-007-direct-interface-only.md`.
async fn run_hotplug_watcher(state: Arc<Mutex<StateMachine>>) {
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
                    try_transition(&state, ConnectionState::CableDetected);
                }
                DirectLinkEvent::Disappeared { interface_name } => {
                    tracing::info!("direct-link interface disappeared: {interface_name}");
                    try_transition(&state, ConnectionState::WaitingForCable);
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
    // No compatibility gate yet (see docs/ROADMAP.md, Phase 5 diagnostics
    // work) — go straight to waiting for a cable.
    try_transition(&state, ConnectionState::CheckingCompatibility);
    try_transition(&state, ConnectionState::Ready);
    try_transition(&state, ConnectionState::WaitingForCable);

    tokio::spawn(run_hotplug_watcher(state.clone()));

    let agent = Agent { state };

    let _connection = connection::Builder::session()?
        .name(SERVICE_NAME)?
        .serve_at(OBJECT_PATH, agent)?
        .build()
        .await?;

    tracing::info!("cabledesk-agent ready on {SERVICE_NAME} ({OBJECT_PATH})");

    // No pairing or streaming orchestration yet — see docs/ROADMAP.md.
    std::future::pending::<()>().await;
    Ok(())
}
