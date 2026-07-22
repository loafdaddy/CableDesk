//! Address/interface validation — the runtime check behind
//! `docs/adr/ADR-007-direct-interface-only.md`'s "never fall back to
//! Wi-Fi" rule. Before CableDesk trusts a peer address for streaming, it
//! confirms that address is actually assigned to the direct-link
//! interface, not just resolvable in general.
//!
//! Also covers the stronger check: whether the kernel's routing table
//! would actually send traffic to a peer address out through the
//! direct-link interface, via `rtnetlink` (a typed netlink client,
//! preferred over shelling out to `ip route get` per the engineering rule
//! "prefer typed D-Bus/netlink APIs over shell commands").

use futures_util::TryStreamExt;
use nix::ifaddrs::getifaddrs;
use std::net::{IpAddr, Ipv4Addr};

/// Returns `true` if `addr` is currently assigned to the network interface
/// named `interface_name`, per the kernel's live address list (not a
/// cached or assumed value).
pub fn address_belongs_to_interface(addr: IpAddr, interface_name: &str) -> bool {
    let Ok(iter) = getifaddrs() else {
        return false;
    };

    iter.filter(|ifaddr| ifaddr.interface_name == interface_name)
        .filter_map(|ifaddr| ifaddr.address)
        .filter_map(|sockaddr| {
            sockaddr
                .as_sockaddr_in()
                .map(|v4| IpAddr::V4((*v4).ip()))
                .or_else(|| sockaddr.as_sockaddr_in6().map(|v6| IpAddr::V6(v6.ip())))
        })
        .any(|assigned| assigned == addr)
}

/// Returns `true` if the kernel's routing table would actually send a
/// packet to `addr` out through `interface_name` — asks the kernel to do
/// the real FIB lookup (the same `RTM_GETROUTE` mechanism `ip route get`
/// uses), rather than re-implementing longest-prefix-match ourselves. An
/// address can be *assigned* to the direct-link interface while a
/// *different* interface's route still wins for a given destination;
/// this catches that case, which `address_belongs_to_interface` alone
/// cannot.
///
/// IPv6 is not covered — the direct link is IPv4 link-local only in v1
/// (see `docs/adr/ADR-003-networkmanager-dbus.md`).
pub async fn route_resolves_via_interface(addr: Ipv4Addr, interface_name: &str) -> bool {
    let Ok(target_index) = nix::net::if_::if_nametoindex(interface_name) else {
        return false;
    };

    let Ok((connection, handle, _)) = rtnetlink::new_connection() else {
        return false;
    };
    tokio::spawn(connection);

    let query = rtnetlink::RouteMessageBuilder::<Ipv4Addr>::new()
        .destination_prefix(addr, 32)
        .build();

    let mut routes = handle.route().get(query).execute();
    let Ok(Some(route)) = routes.try_next().await else {
        return false;
    };

    route_oif(&route) == Some(target_index)
}

fn route_oif(route: &rtnetlink::packet_route::route::RouteMessage) -> Option<u32> {
    use rtnetlink::packet_route::route::RouteAttribute;
    route.attributes.iter().find_map(|attr| match attr {
        RouteAttribute::Oif(index) => Some(*index),
        _ => None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr};

    #[test]
    fn loopback_address_belongs_to_lo() {
        // `lo` with 127.0.0.1 exists on every Linux system CableDesk
        // targets, so this is a safe, real (not mocked) check.
        let loopback = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
        assert!(address_belongs_to_interface(loopback, "lo"));
    }

    #[test]
    fn loopback_address_does_not_belong_to_a_fabricated_interface() {
        let loopback = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
        assert!(!address_belongs_to_interface(
            loopback,
            "cabledesk-definitely-not-a-real-iface"
        ));
    }

    #[test]
    fn an_address_lo_does_not_have_is_rejected() {
        // 169.254.x.x is never assigned to `lo`.
        let link_local = IpAddr::V4(Ipv4Addr::new(169, 254, 1, 2));
        assert!(!address_belongs_to_interface(link_local, "lo"));
    }

    /// Real kernel FIB lookup, not mocked: 127.0.0.1 must always resolve
    /// via `lo` on any Linux system CableDesk targets.
    #[tokio::test]
    async fn loopback_route_resolves_via_lo() {
        let loopback = Ipv4Addr::new(127, 0, 0, 1);
        assert!(route_resolves_via_interface(loopback, "lo").await);
    }

    #[tokio::test]
    async fn loopback_route_does_not_resolve_via_a_fabricated_interface() {
        let loopback = Ipv4Addr::new(127, 0, 0, 1);
        assert!(
            !route_resolves_via_interface(loopback, "cabledesk-definitely-not-a-real-iface").await
        );
    }
}
