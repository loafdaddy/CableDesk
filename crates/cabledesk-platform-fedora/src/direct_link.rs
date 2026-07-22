//! USB4/Thunderbolt controller and `thunderbolt-net` interface detection.
//!
//! Deliberately does not hardcode the interface name `thunderbolt0`: the
//! direct interface is identified by walking `/sys/class/net/*/device`
//! looking for a driver named `thunderbolt-net` (see docs/NETWORKING.md and
//! engineering rule "no hardcoded thunderbolt0").

use cabledesk_core::error::Result;
use cabledesk_platform::{CheckStatus, CompatibilityCheck};

async fn controller_check() -> CompatibilityCheck {
    match tokio::fs::read_dir("/sys/bus/thunderbolt/devices").await {
        Ok(mut entries) => {
            let mut count = 0usize;
            while let Ok(Some(_)) = entries.next_entry().await {
                count += 1;
            }
            if count > 0 {
                CompatibilityCheck::new(
                    "USB4 or Thunderbolt controller",
                    CheckStatus::Available,
                    Some(format!("{count} Thunderbolt/USB4 device(s) enumerated.")),
                )
            } else {
                CompatibilityCheck::new(
                    "USB4 or Thunderbolt controller",
                    CheckStatus::Unsupported,
                    Some(
                        "No Thunderbolt/USB4 controller was detected. An ordinary USB-C \
                         port does not necessarily include USB4 or Thunderbolt support."
                            .to_string(),
                    ),
                )
            }
        }
        Err(_) => CompatibilityCheck::new(
            "USB4 or Thunderbolt controller",
            CheckStatus::Unsupported,
            Some("This kernel does not expose a Thunderbolt subsystem.".to_string()),
        ),
    }
}

async fn module_check() -> CompatibilityCheck {
    match tokio::fs::read_to_string("/proc/modules").await {
        Ok(modules) => {
            if modules.lines().any(|line| {
                line.split_whitespace()
                    .next()
                    .is_some_and(|name| name == "thunderbolt_net")
            }) {
                CompatibilityCheck::new("thunderbolt-net module", CheckStatus::Available, None)
            } else {
                CompatibilityCheck::new(
                    "thunderbolt-net module",
                    CheckStatus::NeedsSetup,
                    Some(
                        "The thunderbolt-net module is not currently loaded. It is loaded \
                         automatically when a compatible peer is connected."
                            .to_string(),
                    ),
                )
            }
        }
        Err(_) => CompatibilityCheck::new(
            "thunderbolt-net module",
            CheckStatus::Unknown,
            Some("Could not read /proc/modules.".to_string()),
        ),
    }
}

/// Scans `/sys/class/net/*/device/driver` for interfaces whose driver
/// name looks like `thunderbolt-net` — the one detection primitive
/// shared by the compatibility check below and
/// [`detect_direct_link_interface_name`], so there is exactly one place
/// that decides what counts as "the direct interface" (never a hardcoded
/// `thunderbolt0`).
async fn find_direct_link_interfaces() -> std::io::Result<Vec<String>> {
    let mut entries = tokio::fs::read_dir("/sys/class/net").await?;

    let mut matches = Vec::new();
    while let Ok(Some(entry)) = entries.next_entry().await {
        let driver_link = entry.path().join("device/driver");
        if let Ok(target) = tokio::fs::read_link(&driver_link).await {
            if target
                .file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.contains("thunderbolt"))
            {
                matches.push(entry.file_name().to_string_lossy().into_owned());
            }
        }
    }
    Ok(matches)
}

async fn direct_interface_check() -> CompatibilityCheck {
    let Ok(matches) = find_direct_link_interfaces().await else {
        return CompatibilityCheck::new(
            "Direct network interface",
            CheckStatus::Unknown,
            Some("Could not read /sys/class/net.".to_string()),
        );
    };

    if matches.is_empty() {
        CompatibilityCheck::new(
            "Direct network interface",
            CheckStatus::Unsupported,
            Some(
                "No thunderbolt-net interface is currently present. Connect a compatible \
                 USB4 or Thunderbolt cable between two CableDesk devices to create one."
                    .to_string(),
            ),
        )
    } else {
        CompatibilityCheck::new(
            "Direct network interface",
            CheckStatus::Available,
            Some(format!("Found: {}", matches.join(", "))),
        )
    }
}

pub async fn inspect_direct_link() -> Result<Vec<CompatibilityCheck>> {
    Ok(vec![
        controller_check().await,
        module_check().await,
        direct_interface_check().await,
    ])
}

/// The first currently-present direct-link interface name, if any — for
/// callers (like `cabledeskctl repair-network`) that need to act on it,
/// not just report its status. `None` means honestly "no direct-link
/// interface is present right now," not an error.
pub async fn detect_direct_link_interface_name() -> Result<Option<String>> {
    Ok(find_direct_link_interfaces().await?.into_iter().next())
}
