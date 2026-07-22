//! `cabledesk-agent` — the unprivileged, per-user CableDesk background
//! service (`cabledesk-agent.service`, a systemd `--user` unit).
//!
//! This first milestone only stands up the D-Bus skeleton described in
//! data/dbus-1/interfaces/org.cabledesk.Agent1.xml and reports the (always
//! `Unconfigured`) connection state. Discovery, pairing and streaming
//! orchestration land in later phases — see docs/ROADMAP.md.

use cabledesk_core::state::StateMachine;
use std::sync::Mutex;
use zbus::{connection, interface};

const SERVICE_NAME: &str = "org.cabledesk.Agent1";
const OBJECT_PATH: &str = "/org/cabledesk/Agent";

struct Agent {
    state: Mutex<StateMachine>,
}

#[interface(name = "org.cabledesk.Agent1")]
impl Agent {
    #[zbus(property)]
    async fn version(&self) -> String {
        env!("CARGO_PKG_VERSION").to_string()
    }

    /// Returns the current [`cabledesk_core::state::ConnectionState`] as its
    /// `Debug` name (e.g. `"Unconfigured"`, `"Streaming"`).
    async fn get_state(&self) -> String {
        self.state
            .lock()
            .expect("state mutex poisoned")
            .current()
            .to_string()
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

    let agent = Agent {
        state: Mutex::new(StateMachine::new()),
    };

    let _connection = connection::Builder::session()?
        .name(SERVICE_NAME)?
        .serve_at(OBJECT_PATH, agent)?
        .build()
        .await?;

    tracing::info!("cabledesk-agent ready on {SERVICE_NAME} ({OBJECT_PATH})");

    // Skeleton only: no networking, pairing or streaming orchestration yet.
    std::future::pending::<()>().await;
    Ok(())
}
