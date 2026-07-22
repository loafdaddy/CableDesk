//! Small helper for checking whether a well-known D-Bus name currently has
//! an owner, used to detect NetworkManager/Avahi/firewalld/UPower without
//! shelling out to `systemctl is-active` (see engineering rule: prefer typed
//! D-Bus APIs over shell commands).

use zbus::Connection;

pub async fn system_bus_name_active(connection: &Connection, name: &str) -> bool {
    let Ok(proxy) = zbus::fdo::DBusProxy::new(connection).await else {
        return false;
    };
    let Ok(bus_name) = name.to_string().try_into() else {
        return false;
    };
    proxy.name_has_owner(bus_name).await.unwrap_or(false)
}
