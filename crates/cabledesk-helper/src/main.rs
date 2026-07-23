//! `cabledesk-helper` — privileged system service (Polkit-gated).
//!
//! Mutating methods (`PrepareDirectLink`) perform a per-call
//! `CheckAuthorization` against `data/polkit-1/actions` before acting.
//! Read-only methods (`Ping`, `Version`, `CollectDiagnosticsJson`) stay
//! unauthenticated.

mod polkit;

use cabledesk_network::require_direct_cable_interface;
use cabledesk_platform::{DirectLink, PlatformBackend};
use cabledesk_platform_fedora::FedoraBackend;
use zbus::message::Header;
use zbus::{connection, interface};

const SERVICE_NAME: &str = "org.cabledesk.Helper1";
const OBJECT_PATH: &str = "/org/cabledesk/Helper";
const ACTION_PREPARE_DIRECT_LINK: &str = "org.cabledesk.helper.prepare-direct-link";

struct Helper;

#[interface(name = "org.cabledesk.Helper1")]
impl Helper {
    #[zbus(property)]
    async fn version(&self) -> String {
        env!("CARGO_PKG_VERSION").to_string()
    }

    async fn ping(&self) -> String {
        "pong".to_string()
    }

    /// Read-only diagnostics as a JSON string. Already sanitised by the
    /// Fedora backend (see docs/SECURITY.md, "Never include").
    async fn collect_diagnostics_json(&self) -> String {
        let backend = FedoraBackend::new();
        match backend.collect_diagnostics().await {
            Ok(diagnostics) => {
                serde_json::to_string(&diagnostics).unwrap_or_else(|_| "{}".to_string())
            }
            Err(err) => {
                tracing::warn!("diagnostics collection failed: {err}");
                "{}".to_string()
            }
        }
    }

    /// Create/repair the CableDesk NetworkManager profile and bind the
    /// interface into the CableDesk firewalld zone. Cable-only: rejects
    /// Wi-Fi / ordinary Ethernet / unknown interfaces before mutating.
    async fn prepare_direct_link(
        &self,
        interface_name: String,
        #[zbus(header)] header: Header<'_>,
    ) -> zbus::fdo::Result<()> {
        let conn = zbus::Connection::system()
            .await
            .map_err(|e| zbus::fdo::Error::Failed(format!("system bus: {e}")))?;

        polkit::check_authorization(&conn, &header, ACTION_PREPARE_DIRECT_LINK).await?;

        if let Err(e) = require_direct_cable_interface(&interface_name) {
            tracing::warn!("PrepareDirectLink rejected non-direct interface {interface_name}: {e}");
            return Err(zbus::fdo::Error::InvalidArgs(format!(
                "interface `{interface_name}` is not a direct USB4/Thunderbolt link"
            )));
        }

        let backend = FedoraBackend::new();
        let link = DirectLink {
            interface_name: interface_name.clone(),
        };
        backend.prepare_direct_link(&link).await.map_err(|e| {
            tracing::error!("prepare_direct_link({interface_name}) failed: {e}");
            zbus::fdo::Error::Failed(e.to_string())
        })?;

        tracing::info!("prepared direct-link profile for {interface_name}");
        Ok(())
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
    tracing::info!("starting cabledesk-helper {}", env!("CARGO_PKG_VERSION"));

    let helper = Helper;

    let _connection = connection::Builder::system()?
        .name(SERVICE_NAME)?
        .serve_at(OBJECT_PATH, helper)?
        .build()
        .await?;

    tracing::info!("cabledesk-helper ready on {SERVICE_NAME} ({OBJECT_PATH})");
    std::future::pending::<()>().await;
    Ok(())
}
