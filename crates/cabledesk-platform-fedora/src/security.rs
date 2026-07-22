//! SELinux and firewall posture, read-only. CableDesk must work with
//! SELinux enforcing and firewalld enabled — it must never suggest
//! disabling either, so this only ever reports status.

use crate::dbus_util::system_bus_name_active;
use cabledesk_core::error::Result;
use cabledesk_platform::{CheckStatus, CompatibilityCheck, SecurityReport};
use zbus::Connection;

async fn selinux_check() -> CompatibilityCheck {
    match tokio::fs::read_to_string("/sys/fs/selinux/enforce").await {
        Ok(contents) => match contents.trim() {
            "1" => CompatibilityCheck::new(
                "SELinux",
                CheckStatus::Available,
                Some("Enforcing".to_string()),
            ),
            "0" => CompatibilityCheck::new(
                "SELinux",
                CheckStatus::Warning,
                Some(
                    "Permissive. CableDesk supports this but recommends enforcing mode."
                        .to_string(),
                ),
            ),
            other => CompatibilityCheck::new(
                "SELinux",
                CheckStatus::Unknown,
                Some(format!("Unrecognised value: {other}")),
            ),
        },
        Err(_) => CompatibilityCheck::new(
            "SELinux",
            CheckStatus::Unknown,
            Some(
                "This system does not expose /sys/fs/selinux; SELinux may not be in use."
                    .to_string(),
            ),
        ),
    }
}

async fn firewalld_check(connection: Option<&Connection>) -> CompatibilityCheck {
    let Some(connection) = connection else {
        return CompatibilityCheck::new(
            "firewalld",
            CheckStatus::Unknown,
            Some("could not connect to the D-Bus system bus".to_string()),
        );
    };
    if system_bus_name_active(connection, "org.fedoraproject.FirewallD1").await {
        CompatibilityCheck::new(
            "firewalld",
            CheckStatus::Available,
            Some("Active".to_string()),
        )
    } else {
        CompatibilityCheck::new(
            "firewalld",
            CheckStatus::NeedsSetup,
            Some(
                "firewalld is not active. CableDesk requires it to scope direct-link traffic."
                    .to_string(),
            ),
        )
    }
}

pub async fn inspect_security_system() -> Result<SecurityReport> {
    let system_bus = Connection::system().await.ok();
    let checks = vec![
        selinux_check().await,
        firewalld_check(system_bus.as_ref()).await,
    ];
    Ok(SecurityReport { checks })
}
