//! CableDesk GTK4/libadwaita application.
//!
//! This first milestone only renders read-only compatibility information
//! (see docs/ROADMAP.md, Phase 1). It does not yet drive pairing,
//! networking or streaming — the state machine in `cabledesk-core` will
//! back those once the agent exists (Phase 2+).

use cabledesk_platform::{CheckStatus, CompatibilityCheck, PlatformBackend};
use cabledesk_platform_fedora::FedoraBackend;
use gtk4 as gtk;
use gtk4::glib;
use libadwaita as adw;

use adw::prelude::*;

const APP_ID: &str = "org.cabledesk.CableDesk";

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
        .default_height(680)
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
        "An ordinary USB-C connector alone does not guarantee CableDesk compatibility.",
    ));
    subtitle_label.add_css_class("dim-label");
    subtitle_label.set_wrap(true);
    subtitle_label.set_xalign(0.0);
    content.append(&subtitle_label);

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
}

/// Runs [`FedoraBackend::collect_diagnostics`] on a background thread with
/// its own Tokio runtime (the platform backend uses `tokio::fs` and `zbus`,
/// neither of which the GTK main loop drives), then hands the result back to
/// the GLib main context to update widgets. GTK widgets are not `Send`, so
/// this hand-off — not direct cross-thread mutation — is required.
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
