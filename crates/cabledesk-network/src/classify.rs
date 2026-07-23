//! Strict interface classification for cable-only enforcement.
//!
//! CableDesk must never treat Wi-Fi, ordinary Ethernet, VPN, or other
//! interfaces as a valid transport. Classification is based on driver /
//! evidence strings — never solely on a hardcoded name like `thunderbolt0`
//! (see `docs/adr/ADR-007-direct-interface-only.md`).

use serde::{Deserialize, Serialize};

/// How CableDesk classifies a network interface for transport decisions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum InterfaceClassification {
    DirectUsb4,
    DirectThunderbolt,
    OrdinaryEthernet,
    WiFi,
    Vpn,
    Bridge,
    Virtual,
    Loopback,
    Container,
    Unknown,
}

impl InterfaceClassification {
    /// Only USB4/Thunderbolt-derived direct links may carry a CableDesk session.
    pub fn is_direct_cable(self) -> bool {
        matches!(self, Self::DirectUsb4 | Self::DirectThunderbolt)
    }
}

/// Classify from a NetworkManager / sysfs driver basename (or similar evidence).
///
/// This is deliberately conservative: unknown drivers are [`Unknown`], not
/// accepted. Misleading names (e.g. an interface *named* `thunderbolt0` with
/// a Wi-Fi driver) follow the **driver**, not the name.
pub fn classify_from_driver_name(driver: &str) -> InterfaceClassification {
    let d = driver.trim().to_ascii_lowercase();
    if d.is_empty() {
        return InterfaceClassification::Unknown;
    }
    if d.contains("thunderbolt") {
        // Kernel module is typically `thunderbolt_net` / sysfs `thunderbolt-net`.
        return InterfaceClassification::DirectThunderbolt;
    }
    if d.contains("usb4") {
        return InterfaceClassification::DirectUsb4;
    }
    if matches!(
        d.as_str(),
        "iwlwifi"
            | "ath9k"
            | "ath10k_pci"
            | "ath11k_pci"
            | "ath12k"
            | "brcmfmac"
            | "mt76"
            | "mt7921e"
            | "rtl8xxxu"
            | "rtw88"
            | "rtw89"
            | "mac80211_hwsim"
    ) || d.contains("wifi")
        || d.contains("wlan")
        || d.starts_with("iwl")
        || d.starts_with("ath1")
        || d.starts_with("mt79")
        || d.starts_with("rtw")
    {
        return InterfaceClassification::WiFi;
    }
    if matches!(
        d.as_str(),
        "wireguard" | "tun" | "tap" | "openvswitch" | "vxlan" | "geneve" | "gre" | "sit" | "ip6tnl"
    ) || d.contains("vpn")
        || d.contains("wireguard")
    {
        return InterfaceClassification::Vpn;
    }
    if d.contains("bridge") || d == "br_netfilter" {
        return InterfaceClassification::Bridge;
    }
    if matches!(d.as_str(), "veth" | "dummy" | "nlmon" | "ifb") || d.contains("veth") {
        return InterfaceClassification::Virtual;
    }
    if d.contains("docker") || d.contains("cni") || d.contains("lxc") || d.contains("podman") {
        return InterfaceClassification::Container;
    }
    if matches!(
        d.as_str(),
        "e1000"
            | "e1000e"
            | "igb"
            | "igc"
            | "ixgbe"
            | "i40e"
            | "r8169"
            | "tg3"
            | "bnx2"
            | "bnxt_en"
            | "virtio_net"
            | "atlantic"
            | "stmmac"
            | "fec"
            | "macb"
    ) || d.contains("ethernet")
        || d.starts_with("r81")
        || d.starts_with("igb")
        || d.starts_with("ixg")
    {
        return InterfaceClassification::OrdinaryEthernet;
    }
    if d == "loopback" || d == "lo" {
        return InterfaceClassification::Loopback;
    }
    InterfaceClassification::Unknown
}

/// Classify from a kernel interface name when no driver evidence is available.
///
/// Names alone are **never** enough to accept a direct cable — at best they
/// yield [`Unknown`] or an explicit non-cable class. A name like
/// `thunderbolt0` without driver evidence stays [`Unknown`] (rejected).
pub fn classify_from_interface_name(name: &str) -> InterfaceClassification {
    let n = name.trim().to_ascii_lowercase();
    if n == "lo" {
        return InterfaceClassification::Loopback;
    }
    if n.starts_with("wl") || n.starts_with("wlan") || n.starts_with("wifi") {
        return InterfaceClassification::WiFi;
    }
    if n.starts_with("en") || n.starts_with("eth") || n.starts_with("em") {
        return InterfaceClassification::OrdinaryEthernet;
    }
    if n.starts_with("br") {
        return InterfaceClassification::Bridge;
    }
    if n.starts_with("veth") || n.starts_with("docker") || n.starts_with("cni") {
        return InterfaceClassification::Container;
    }
    if n.starts_with("wg") || n.starts_with("tun") || n.starts_with("tap") {
        return InterfaceClassification::Vpn;
    }
    InterfaceClassification::Unknown
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thunderbolt_net_driver_is_direct() {
        assert!(classify_from_driver_name("thunderbolt-net").is_direct_cable());
        assert_eq!(
            classify_from_driver_name("thunderbolt_net"),
            InterfaceClassification::DirectThunderbolt
        );
    }

    #[test]
    fn wifi_drivers_are_rejected() {
        assert_eq!(
            classify_from_driver_name("iwlwifi"),
            InterfaceClassification::WiFi
        );
        assert!(!classify_from_driver_name("iwlwifi").is_direct_cable());
    }

    #[test]
    fn ordinary_ethernet_is_rejected() {
        assert_eq!(
            classify_from_driver_name("e1000e"),
            InterfaceClassification::OrdinaryEthernet
        );
        assert!(!classify_from_driver_name("r8169").is_direct_cable());
    }

    #[test]
    fn misleading_thunderbolt_name_without_driver_is_unknown() {
        // Name alone must never authorize a session.
        assert_eq!(
            classify_from_interface_name("thunderbolt0"),
            InterfaceClassification::Unknown
        );
        assert!(!classify_from_interface_name("thunderbolt0").is_direct_cable());
    }

    #[test]
    fn wifi_interface_name_is_rejected() {
        assert_eq!(
            classify_from_interface_name("wlp0s20f3"),
            InterfaceClassification::WiFi
        );
    }

    #[test]
    fn vpn_and_veth_are_rejected() {
        assert_eq!(
            classify_from_driver_name("wireguard"),
            InterfaceClassification::Vpn
        );
        assert_eq!(
            classify_from_driver_name("veth"),
            InterfaceClassification::Virtual
        );
    }
}
