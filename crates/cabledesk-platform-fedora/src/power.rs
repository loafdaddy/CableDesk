//! USB-C Power Delivery and battery status, read from `/sys/class/typec`
//! and `/sys/class/power_supply`. Strictly read-only: CableDesk never
//! writes to Type-C power-role sysfs attributes (see docs/POWER_DELIVERY.md
//! and engineering rule "no automatic USB-C power-role changes").

use cabledesk_core::error::Result;
use cabledesk_platform::{CheckStatus, CompatibilityCheck, PowerDeliveryStatus};
use std::path::Path;

async fn read_trimmed(path: impl AsRef<Path>) -> Option<String> {
    tokio::fs::read_to_string(path)
        .await
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

async fn typec_port_check() -> CompatibilityCheck {
    let Ok(mut entries) = tokio::fs::read_dir("/sys/class/typec").await else {
        return CompatibilityCheck::new(
            "USB-C Power Delivery reporting",
            CheckStatus::Unknown,
            Some("No /sys/class/typec on this kernel.".to_string()),
        );
    };

    let mut ports = Vec::new();
    while let Ok(Some(entry)) = entries.next_entry().await {
        let name = entry.file_name().to_string_lossy().into_owned();
        // Skip partner/alt-mode sub-nodes like "port0-partner"; we only want
        // the port objects themselves.
        if name.contains('-') {
            continue;
        }
        ports.push(entry.path());
    }

    if ports.is_empty() {
        return CompatibilityCheck::new(
            "USB-C Power Delivery reporting",
            CheckStatus::Unknown,
            Some("No USB Type-C ports were reported by the kernel.".to_string()),
        );
    }

    let mut details = Vec::new();
    for port in &ports {
        let power_role = read_trimmed(port.join("power_role")).await;
        let pd_revision = read_trimmed(port.join("usb_power_delivery_revision")).await;
        if let Some(role) = &power_role {
            let mut line = format!("{}: {role}", port.file_name().unwrap().to_string_lossy());
            if let Some(rev) = &pd_revision {
                line.push_str(&format!(" (PD {rev})"));
            }
            details.push(line);
        }
    }

    CompatibilityCheck::new(
        "USB-C Power Delivery reporting",
        CheckStatus::Available,
        Some(details.join("; ")),
    )
}

async fn battery_check() -> CompatibilityCheck {
    let Ok(mut entries) = tokio::fs::read_dir("/sys/class/power_supply").await else {
        return CompatibilityCheck::new(
            "Battery status",
            CheckStatus::Unknown,
            Some("No /sys/class/power_supply on this system.".to_string()),
        );
    };

    let mut battery_path = None;
    while let Ok(Some(entry)) = entries.next_entry().await {
        if read_trimmed(entry.path().join("type")).await.as_deref() == Some("Battery") {
            battery_path = Some(entry.path());
            break;
        }
    }

    let Some(battery_path) = battery_path else {
        return CompatibilityCheck::new(
            "Battery status",
            CheckStatus::Unknown,
            Some("No battery present. This is expected on a desktop.".to_string()),
        );
    };

    let capacity = read_trimmed(battery_path.join("capacity")).await;
    let status = read_trimmed(battery_path.join("status")).await;

    match (capacity, status) {
        (Some(capacity), Some(status)) => CompatibilityCheck::new(
            "Battery status",
            CheckStatus::Available,
            Some(format!("{capacity}% ({status})")),
        ),
        _ => CompatibilityCheck::new(
            "Battery status",
            CheckStatus::Unknown,
            Some("Battery present but capacity/status attributes were unreadable.".to_string()),
        ),
    }
}

async fn external_power_check() -> CompatibilityCheck {
    let Ok(mut entries) = tokio::fs::read_dir("/sys/class/power_supply").await else {
        return CompatibilityCheck::new(
            "External power",
            CheckStatus::Unknown,
            Some("No /sys/class/power_supply on this system.".to_string()),
        );
    };

    let mut sources = Vec::new();
    while let Ok(Some(entry)) = entries.next_entry().await {
        let kind = read_trimmed(entry.path().join("type")).await;
        if matches!(kind.as_deref(), Some("Mains") | Some("USB")) {
            let online = read_trimmed(entry.path().join("online")).await;
            if online.as_deref() == Some("1") {
                sources.push(entry.file_name().to_string_lossy().into_owned());
            }
        }
    }

    if sources.is_empty() {
        CompatibilityCheck::new(
            "External power",
            CheckStatus::Warning,
            Some("No external power source currently reports as online.".to_string()),
        )
    } else {
        CompatibilityCheck::new(
            "External power",
            CheckStatus::Available,
            Some(format!("Online: {}", sources.join(", "))),
        )
    }
}

pub async fn inspect_power_delivery() -> Result<PowerDeliveryStatus> {
    let checks = vec![
        typec_port_check().await,
        battery_check().await,
        external_power_check().await,
    ];
    Ok(PowerDeliveryStatus { checks })
}
