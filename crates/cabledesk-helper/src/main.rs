//! `cabledesk-helper` — the privileged system service
//! (`cabledesk-helper.service`) described in docs/ARCHITECTURE.md and
//! data/dbus-1/interfaces/org.cabledesk.Helper1.xml.
//!
//! Every privileged operation this service will eventually perform (loading
//! `thunderbolt-net`, creating the NetworkManager profile, installing
//! firewalld rules) MUST be authorised per-call via Polkit — see
//! data/polkit-1/actions and docs/SECURITY.md. This milestone only exposes
//! a narrow, unauthenticated `Ping`/`Version` surface plus a read-only
//! diagnostics call; it performs no privileged actions yet, so no Polkit
//! check is wired up for it. Do not add a privileged method here without
//! also adding its `CheckAuthorization` call — see docs/OPEN_QUESTIONS.md.

use cabledesk_platform::PlatformBackend;
use cabledesk_platform_fedora::FedoraBackend;
use zbus::{connection, interface};

const SERVICE_NAME: &str = "org.cabledesk.Helper1";
const OBJECT_PATH: &str = "/org/cabledesk/Helper";

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

    /// Read-only diagnostics as a JSON string. Every field returned here is
    /// already sanitised by `cabledesk-platform-fedora` (see
    /// docs/SECURITY.md, "Never include").
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

    // Skeleton only: no kernel module loading, NetworkManager profile
    // management or firewalld policy installation yet.
    std::future::pending::<()>().await;
    Ok(())
}
