//! CableDesk GTK4/libadwaita application.
//!
//! Shows read-only compatibility information and the live agent session
//! state (`GetState` / `GetSessionJson` on the session bus). Pairing and
//! streaming UI are not implemented yet.

use cabledesk_platform::{CheckStatus, CompatibilityCheck, PlatformBackend};
use cabledesk_platform_fedora::FedoraBackend;
use gtk4 as gtk;
use gtk4::glib;
use libadwaita as adw;

use adw::prelude::*;

const APP_ID: &str = "org.cabledesk.CableDesk";

#[zbus::proxy(
    interface = "org.cabledesk.Agent1",
    default_service = "org.cabledesk.Agent1",
    default_path = "/org/cabledesk/Agent"
)]
trait Agent1 {
    async fn get_state(&self) -> zbus::Result<String>;
    async fn get_session_json(&self) -> zbus::Result<String>;
}

fn main() -> glib::ExitCode {
    tracing_subscriber::fmt().init();

    let app = adw::Application::builder().application_id(APP_ID).build();
    app.connect_activate(build_ui);
    app.run()
}

fn build_ui(app: &adw::Application) {
    let window = adw::ApplicationWindow::builder()
        .application(app)
        .title("CableDesk")
        .default_width(480)
        .default_height(720)
        .build();

    let header = adw::HeaderBar::new();

    let content = gtk::Box::new(gtk::Orientation::Vertical, 18);
    content.set_margin_top(24);
    content.set_margin_bottom(24);
    content.set_margin_start(24);
    content.set_margin_end(24);

    let status_label = gtk::Label::new(Some("Checking system compatibility…"));
    status_label.add_css_class("title-2");
    status_label.set_wrap(true);
    status_label.set_xalign(0.0);
    content.append(&status_label);

    let subtitle_label = gtk::Label::new(Some(
        "CableDesk requires a compatible direct USB4 or Thunderbolt cable.",
    ));
    subtitle_label.add_css_class("dim-label");
    subtitle_label.set_wrap(true);
    subtitle_label.set_xalign(0.0);
    content.append(&subtitle_label);

    let session_group = adw::PreferencesGroup::builder()
        .title("Session")
        .description("Live state from cabledesk-agent (cable-only)")
        .build();
    let agent_state_row = adw::ActionRow::builder()
        .title("Agent state")
        .subtitle("Connecting…")
        .build();
    let agent_link_row = adw::ActionRow::builder()
        .title("Direct interface")
        .subtitle("—")
        .build();
    let agent_peer_row = adw::ActionRow::builder()
        .title("Validated peer")
        .subtitle("—")
        .build();
    session_group.add(&agent_state_row);
    session_group.add(&agent_link_row);
    session_group.add(&agent_peer_row);
    content.append(&session_group);

    let system_group = adw::PreferencesGroup::builder()
        .title("System")
        .description("Detected distribution, desktop and session")
        .build();
    let role_row = adw::ActionRow::builder()
        .title("Role")
        .subtitle("Not configured")
        .build();
    system_group.add(&role_row);
    content.append(&system_group);

    let link_group = adw::PreferencesGroup::builder()
        .title("Direct Link")
        .description("USB4 / Thunderbolt hardware and thunderbolt-net status")
        .build();
    content.append(&link_group);

    let power_group = adw::PreferencesGroup::builder()
        .title("Power")
        .description("USB-C Power Delivery and battery status")
        .build();
    content.append(&power_group);

    let scrolled = gtk::ScrolledWindow::builder()
        .child(&content)
        .vexpand(true)
        .build();

    let toolbar_view = adw::ToolbarView::new();
    toolbar_view.add_top_bar(&header);
    toolbar_view.set_content(Some(&scrolled));

    window.set_content(Some(&toolbar_view));
    window.present();

    spawn_compatibility_check(status_label, system_group, link_group, power_group);
    spawn_agent_poll(agent_state_row, agent_link_row, agent_peer_row);
}

fn spawn_compatibility_check(
    status_label: gtk::Label,
    system_group: adw::PreferencesGroup,
    link_group: adw::PreferencesGroup,
    power_group: adw::PreferencesGroup,
) {
    let (tx, rx) = async_channel::bounded(1);
    std::thread::spawn(move || {
        let runtime = tokio::runtime::Runtime::new().expect("failed to start tokio runtime");
        let backend = FedoraBackend::new();
        let result = runtime.block_on(backend.collect_diagnostics());
        let _ = tx.send_blocking(result);
    });

    glib::MainContext::default().spawn_local(async move {
        match rx.recv().await {
            Ok(Ok(diagnostics)) => {
                status_label.set_text(&format!(
                    "{}\n{}",
                    diagnostics.system.distro_name, diagnostics.system.hostname
                ));

                let de_row = adw::ActionRow::builder()
                    .title("Desktop Environment")
                    .subtitle(
                        diagnostics
                            .system
                            .desktop_environment
                            .clone()
                            .unwrap_or_else(|| "Unknown".to_string()),
                    )
                    .build();
                system_group.add(&de_row);

                let session_row = adw::ActionRow::builder()
                    .title("Session Type")
                    .subtitle(format!("{:?}", diagnostics.system.session_type))
                    .build();
                system_group.add(&session_row);

                for check in &diagnostics.direct_link {
                    link_group.add(&check_row(check));
                }
                for check in &diagnostics.power.checks {
                    power_group.add(&check_row(check));
                }
            }
            Ok(Err(err)) => {
                status_label.set_text("Could not check compatibility");
                tracing::warn!("compatibility check failed: {err}");
            }
            Err(_) => {
                status_label.set_text("Compatibility check did not complete.");
            }
        }
    });
}

/// Polls the agent every 2s so testers can watch cable insert/remove without
/// a StateChanged signal yet.
fn spawn_agent_poll(state_row: adw::ActionRow, link_row: adw::ActionRow, peer_row: adw::ActionRow) {
    let (tx, rx) = async_channel::bounded(1);
    std::thread::spawn(move || {
        let runtime = tokio::runtime::Runtime::new().expect("failed to start tokio runtime");
        runtime.block_on(async move {
            loop {
                let payload = match query_agent_session().await {
                    Ok(json) => json,
                    Err(e) => serde_json::json!({
                        "state": "Agent unavailable",
                        "last_error": e,
                        "interface": serde_json::Value::Null,
                        "last_peer_address": serde_json::Value::Null,
                    })
                    .to_string(),
                };
                if tx.send(payload).await.is_err() {
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            }
        });
    });

    glib::MainContext::default().spawn_local(async move {
        while let Ok(json) = rx.recv().await {
            let value: serde_json::Value =
                serde_json::from_str(&json).unwrap_or_else(|_| serde_json::json!({}));
            let state = value
                .get("state")
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown");
            state_row.set_subtitle(state);

            let iface = value
                .get("interface")
                .and_then(|v| v.as_str())
                .unwrap_or("No direct cable connection");
            let driver = value.get("driver").and_then(|v| v.as_str()).unwrap_or("");
            if driver.is_empty() {
                link_row.set_subtitle(iface);
            } else {
                let text = format!("{iface} ({driver})");
                link_row.set_subtitle(&text);
            }

            let peer = value
                .get("last_peer_address")
                .and_then(|v| v.as_str())
                .unwrap_or("—");
            let peer_name = value
                .get("last_peer_name")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            if peer_name.is_empty() {
                peer_row.set_subtitle(peer);
            } else {
                let text = format!("{peer} ({peer_name})");
                peer_row.set_subtitle(&text);
            }

            if (state == "WaitingForCable" || state == "Agent unavailable")
                && value.get("interface").and_then(|v| v.as_str()).is_none()
            {
                link_row.set_subtitle("No direct cable connection");
            }
        }
    });
}

async fn query_agent_session() -> Result<String, String> {
    let connection = zbus::Connection::session()
        .await
        .map_err(|e| e.to_string())?;
    let proxy = Agent1Proxy::new(&connection)
        .await
        .map_err(|e| e.to_string())?;
    proxy.get_session_json().await.map_err(|e| e.to_string())
}

fn check_row(check: &CompatibilityCheck) -> adw::ActionRow {
    let row = adw::ActionRow::builder()
        .title(check.name.clone())
        .subtitle(check.detail.clone().unwrap_or_default())
        .build();

    let (icon_name, css_class) = match check.status {
        CheckStatus::Available => ("emblem-ok-symbolic", "success"),
        CheckStatus::NeedsSetup | CheckStatus::Warning => ("dialog-warning-symbolic", "warning"),
        CheckStatus::Unsupported => ("dialog-error-symbolic", "error"),
        CheckStatus::Unknown => ("dialog-question-symbolic", "dim-label"),
    };
    let icon = gtk::Image::from_icon_name(icon_name);
    icon.add_css_class(css_class);
    row.add_suffix(&icon);

    row
}
