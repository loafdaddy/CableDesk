//! Builds the NetworkManager connection-settings dictionary for
//! CableDesk's direct-link profile.
//!
//! Deliberately minimal: plain `ipv4.method=link-local` only. NetworkManager's
//! `ipv4.link-local=fallback` mode (DHCP-first, link-local only if DHCP
//! doesn't answer) is not used — there is no DHCP server on a direct cable
//! link in any scenario CableDesk targets, so that mode would only add a
//! DHCP-timeout wait for no benefit. See `docs/NETWORKING.md` §4 and
//! `docs/adr/ADR-003-networkmanager-dbus.md`.

use std::collections::HashMap;
use zbus::zvariant::{OwnedValue, Value};

/// A never-default, link-local-only NetworkManager profile pinned to one
/// specific interface name (never a hardcoded `thunderbolt0` — the caller
/// supplies whatever name `cabledesk-platform`'s direct-link detection
/// actually found).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirectLinkProfile {
    pub interface_name: String,
}

impl DirectLinkProfile {
    pub fn new(interface_name: impl Into<String>) -> Self {
        Self {
            interface_name: interface_name.into(),
        }
    }

    pub fn connection_id(&self) -> String {
        format!("CableDesk Direct Link ({})", self.interface_name)
    }

    /// The nested `a{sa{sv}}` settings dictionary expected by
    /// `org.freedesktop.NetworkManager.Settings.AddConnection`. Every
    /// value here directly reflects a design commitment from
    /// `docs/NETWORKING.md`/`docs/THREAT_MODEL.md` T3, not a default left
    /// unexamined:
    ///
    /// - `connection.interface-name` pins the profile to this exact
    ///   device, so NetworkManager can never auto-match it elsewhere.
    /// - `connection.autoconnect = true` so the profile activates as soon
    ///   as the direct interface appears, without user action.
    /// - `ipv4.method = "link-local"` and `ipv4.never-default = true`
    ///   together guarantee this connection can supply an address but can
    ///   never become the machine's default route (see ADR-007).
    /// - `ipv6.method = "disabled"` — this link has one job (IPv4
    ///   link-local for Sunshine/Moonlight); an unconfigured IPv6 stack on
    ///   the direct interface is unnecessary surface area, not a feature
    ///   gap.
    pub fn to_nm_settings(&self) -> HashMap<String, HashMap<String, OwnedValue>> {
        let mut connection = HashMap::new();
        connection.insert("id".to_string(), owned(self.connection_id()));
        connection.insert("type".to_string(), owned("802-3-ethernet"));
        connection.insert(
            "interface-name".to_string(),
            owned(self.interface_name.clone()),
        );
        connection.insert("autoconnect".to_string(), owned(true));

        let mut ipv4 = HashMap::new();
        ipv4.insert("method".to_string(), owned("link-local"));
        ipv4.insert("never-default".to_string(), owned(true));

        let mut ipv6 = HashMap::new();
        ipv6.insert("method".to_string(), owned("disabled"));

        let mut settings = HashMap::new();
        settings.insert("connection".to_string(), connection);
        settings.insert("ipv4".to_string(), ipv4);
        settings.insert("ipv6".to_string(), ipv6);
        settings
    }
}

fn owned<'a>(value: impl Into<Value<'a>>) -> OwnedValue {
    OwnedValue::try_from(value.into()).expect("primitive Value conversions never fail")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn get<'a>(
        settings: &'a HashMap<String, HashMap<String, OwnedValue>>,
        section: &str,
        key: &str,
    ) -> &'a OwnedValue {
        settings
            .get(section)
            .unwrap_or_else(|| panic!("missing section {section}"))
            .get(key)
            .unwrap_or_else(|| panic!("missing key {section}.{key}"))
    }

    #[test]
    fn pins_the_exact_interface_name_supplied() {
        let profile = DirectLinkProfile::new("thunderbolt0");
        let settings = profile.to_nm_settings();
        let name: String = get(&settings, "connection", "interface-name")
            .try_clone()
            .unwrap()
            .try_into()
            .unwrap();
        assert_eq!(name, "thunderbolt0");
    }

    #[test]
    fn never_becomes_the_default_route() {
        let settings = DirectLinkProfile::new("thunderbolt0").to_nm_settings();
        let never_default: bool = get(&settings, "ipv4", "never-default")
            .try_clone()
            .unwrap()
            .try_into()
            .unwrap();
        assert!(never_default);
    }

    #[test]
    fn uses_plain_link_local_not_dhcp_fallback_mode() {
        let settings = DirectLinkProfile::new("thunderbolt0").to_nm_settings();
        let method: String = get(&settings, "ipv4", "method")
            .try_clone()
            .unwrap()
            .try_into()
            .unwrap();
        assert_eq!(method, "link-local");
        // ipv4.link-local (the separate NM>=1.52 property enabling
        // "fallback" mode) is intentionally never set at all — plain
        // ipv4.method=link-local is sufficient and is all v1 supports.
        assert!(!settings["ipv4"].contains_key("link-local"));
    }

    #[test]
    fn autoconnects_so_plugging_in_is_enough() {
        let settings = DirectLinkProfile::new("thunderbolt0").to_nm_settings();
        let autoconnect: bool = get(&settings, "connection", "autoconnect")
            .try_clone()
            .unwrap()
            .try_into()
            .unwrap();
        assert!(autoconnect);
    }

    #[test]
    fn connection_id_embeds_interface_name_for_debuggability() {
        let profile = DirectLinkProfile::new("thunderbolt1");
        assert!(profile.connection_id().contains("thunderbolt1"));
    }
}
