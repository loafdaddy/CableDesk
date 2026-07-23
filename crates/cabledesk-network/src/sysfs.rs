//! Sysfs helpers for direct-link interface evidence.
//!
//! Never trust an interface name alone — see
//! `docs/adr/ADR-007-direct-interface-only.md`.

use crate::classify::{classify_from_driver_name, InterfaceClassification};
use cabledesk_core::error::{CableDeskError, Result};
use std::path::Path;

/// Basename of `/sys/class/net/<iface>/device/driver`, when present.
pub fn read_interface_driver(interface_name: &str) -> Option<String> {
    let path = Path::new("/sys/class/net")
        .join(interface_name)
        .join("device/driver");
    std::fs::read_link(path)
        .ok()
        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
}

/// Kernel interface index (`if_nametoindex`), or an error if missing.
pub fn interface_index(interface_name: &str) -> Result<u32> {
    nix::net::if_::if_nametoindex(interface_name).map_err(|_| {
        CableDeskError::Detection(format!("interface `{interface_name}` has no kernel index"))
    })
}

/// Classify an interface using sysfs driver evidence (preferred) or name.
pub fn classify_interface(interface_name: &str) -> InterfaceClassification {
    match read_interface_driver(interface_name) {
        Some(driver) => classify_from_driver_name(&driver),
        None => crate::classify::classify_from_interface_name(interface_name),
    }
}

/// Reject anything that is not a USB4/Thunderbolt-derived direct interface.
pub fn require_direct_cable_interface(interface_name: &str) -> Result<InterfaceClassification> {
    let class = classify_interface(interface_name);
    if class.is_direct_cable() {
        Ok(class)
    } else {
        Err(CableDeskError::NoDirectCableConnection)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loopback_is_not_direct_cable() {
        assert!(matches!(
            require_direct_cable_interface("lo"),
            Err(CableDeskError::NoDirectCableConnection)
        ));
    }

    #[test]
    fn loopback_has_an_index() {
        assert!(interface_index("lo").unwrap() > 0);
    }
}
