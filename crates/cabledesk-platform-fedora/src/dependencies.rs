//! Runtime dependency presence checks: system services reachable over
//! D-Bus, shared libraries on well-known linker search paths, and
//! executables on `$PATH`. Everything here is read-only and side-effect
//! free (in particular, shared libraries are located by scanning
//! directories, never `dlopen`ed, since loading GTK into a non-GUI process
//! has unwanted side effects).

use crate::dbus_util::system_bus_name_active;
use cabledesk_core::error::Result;
use cabledesk_platform::{CheckStatus, CompatibilityCheck, DependencyReport};
use std::path::Path;
use zbus::Connection;

const LIBRARY_SEARCH_DIRS: &[&str] = &[
    "/usr/lib64",
    "/usr/lib",
    "/usr/lib/x86_64-linux-gnu",
    "/usr/local/lib64",
    "/usr/local/lib",
];

fn library_present(soname_prefix: &str) -> bool {
    LIBRARY_SEARCH_DIRS.iter().any(|dir| {
        std::fs::read_dir(dir)
            .map(|entries| {
                entries.filter_map(|e| e.ok()).any(|entry| {
                    entry
                        .file_name()
                        .to_str()
                        .is_some_and(|name| name.starts_with(soname_prefix))
                })
            })
            .unwrap_or(false)
    })
}

fn executable_on_path(bin_name: &str) -> bool {
    let Some(path_var) = std::env::var_os("PATH") else {
        return false;
    };
    std::env::split_paths(&path_var).any(|dir| dir.join(bin_name).is_file())
}

fn pipewire_socket_present() -> bool {
    std::env::var("XDG_RUNTIME_DIR")
        .map(|dir| Path::new(&dir).join("pipewire-0").exists())
        .unwrap_or(false)
}

async fn dbus_service_check(
    connection: Option<&Connection>,
    name: &str,
    bus_name: &str,
    setup_hint: &str,
) -> CompatibilityCheck {
    let Some(connection) = connection else {
        return CompatibilityCheck::new(
            name,
            CheckStatus::Unknown,
            Some("could not connect to the D-Bus system bus".to_string()),
        );
    };
    if system_bus_name_active(connection, bus_name).await {
        CompatibilityCheck::new(name, CheckStatus::Available, None)
    } else {
        CompatibilityCheck::new(name, CheckStatus::NeedsSetup, Some(setup_hint.to_string()))
    }
}

pub async fn check_dependencies() -> Result<DependencyReport> {
    let system_bus = Connection::system().await.ok();

    let mut checks = vec![
        dbus_service_check(
            system_bus.as_ref(),
            "NetworkManager",
            "org.freedesktop.NetworkManager",
            "NetworkManager is required to configure the direct-link interface.",
        )
        .await,
        dbus_service_check(
            system_bus.as_ref(),
            "Avahi",
            "org.freedesktop.Avahi",
            "avahi-daemon is required for direct-link device discovery.",
        )
        .await,
        dbus_service_check(
            system_bus.as_ref(),
            "firewalld",
            "org.fedoraproject.FirewallD1",
            "firewalld is required to scope traffic to the direct-link interface.",
        )
        .await,
        dbus_service_check(
            system_bus.as_ref(),
            "UPower",
            "org.freedesktop.UPower",
            "UPower is required to report battery and charging status.",
        )
        .await,
        dbus_service_check(
            system_bus.as_ref(),
            "Polkit",
            "org.freedesktop.PolicyKit1",
            "polkit is required to authorise privileged helper operations.",
        )
        .await,
    ];

    checks.push(if library_present("libgtk-4.so") {
        CompatibilityCheck::new("GTK4", CheckStatus::Available, None)
    } else {
        CompatibilityCheck::new(
            "GTK4",
            CheckStatus::NeedsSetup,
            Some("libgtk-4 was not found on the linker search path.".to_string()),
        )
    });

    checks.push(if library_present("libadwaita-1.so") {
        CompatibilityCheck::new("libadwaita", CheckStatus::Available, None)
    } else {
        CompatibilityCheck::new(
            "libadwaita",
            CheckStatus::NeedsSetup,
            Some("libadwaita was not found on the linker search path.".to_string()),
        )
    });

    checks.push(if pipewire_socket_present() {
        CompatibilityCheck::new("PipeWire", CheckStatus::Available, None)
    } else {
        CompatibilityCheck::new(
            "PipeWire",
            CheckStatus::Unknown,
            Some("No PipeWire session socket found for the current user session.".to_string()),
        )
    });

    checks.push(if executable_on_path("sunshine") {
        CompatibilityCheck::new("Sunshine runtime", CheckStatus::Available, None)
    } else {
        CompatibilityCheck::new(
            "Sunshine runtime",
            CheckStatus::NeedsSetup,
            Some(
                "The CableDesk-managed Sunshine runtime is not yet bundled (see \
                 docs/UPSTREAM_INTEGRATION.md)."
                    .to_string(),
            ),
        )
    });

    checks.push(
        if executable_on_path("moonlight-qt") || executable_on_path("moonlight") {
            CompatibilityCheck::new("Moonlight runtime", CheckStatus::Available, None)
        } else {
            CompatibilityCheck::new(
                "Moonlight runtime",
                CheckStatus::NeedsSetup,
                Some(
                    "The CableDesk-managed Moonlight runtime is not yet bundled (see \
                 docs/UPSTREAM_INTEGRATION.md)."
                        .to_string(),
                ),
            )
        },
    );

    Ok(DependencyReport { checks })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn path_scan_finds_a_binary_known_to_exist() {
        // `sh` is guaranteed present on any Linux system CableDesk targets.
        assert!(executable_on_path("sh"));
    }

    #[test]
    fn path_scan_rejects_a_binary_that_cannot_exist() {
        assert!(!executable_on_path(
            "cabledesk-definitely-not-a-real-binary-xyz"
        ));
    }
}
