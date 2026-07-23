//! Polkit `CheckAuthorization` for privileged helper methods.

use std::collections::HashMap;
use zbus::message::Header;
use zbus::zvariant::{OwnedValue, Str, Value};
use zbus::Connection;

const AUTHORITY_DEST: &str = "org.freedesktop.PolicyKit1";
const AUTHORITY_PATH: &str = "/org/freedesktop/PolicyKit1/Authority";

#[zbus::proxy(
    interface = "org.freedesktop.PolicyKit1.Authority",
    default_service = "org.freedesktop.PolicyKit1",
    default_path = "/org/freedesktop/PolicyKit1/Authority"
)]
trait Authority {
    #[allow(clippy::type_complexity)]
    fn check_authorization(
        &self,
        subject: &(String, HashMap<&str, Value<'_>>),
        action_id: &str,
        details: &HashMap<&str, &str>,
        flags: u32,
        cancellation_id: &str,
    ) -> zbus::Result<(
        bool,                        // is_authorized
        bool,                        // is_challenge
        HashMap<String, OwnedValue>, // details
    )>;
}

/// `CheckAuthorizationFlags::AllowUserInteraction`
const ALLOW_USER_INTERACTION: u32 = 0x1;

/// Authorise the D-Bus caller identified by `header` for `action_id`.
pub async fn check_authorization(
    connection: &Connection,
    header: &Header<'_>,
    action_id: &str,
) -> zbus::fdo::Result<()> {
    let sender = header
        .sender()
        .ok_or_else(|| zbus::fdo::Error::Failed("missing D-Bus sender for Polkit check".into()))?;

    let mut subject_details: HashMap<&str, Value<'_>> = HashMap::new();
    let name = Str::from(sender.as_str());
    subject_details.insert("name", Value::Str(name));
    let subject = ("system-bus-name".to_string(), subject_details);

    let authority = AuthorityProxy::builder(connection)
        .destination(AUTHORITY_DEST)?
        .path(AUTHORITY_PATH)?
        .build()
        .await
        .map_err(|e| zbus::fdo::Error::Failed(format!("Polkit authority unavailable: {e}")))?;

    let details: HashMap<&str, &str> = HashMap::new();
    let (authorized, _challenge, _details) = authority
        .check_authorization(&subject, action_id, &details, ALLOW_USER_INTERACTION, "")
        .await
        .map_err(|e| zbus::fdo::Error::Failed(format!("Polkit CheckAuthorization failed: {e}")))?;

    if authorized {
        Ok(())
    } else {
        Err(zbus::fdo::Error::AccessDenied(format!(
            "Polkit denied action {action_id}"
        )))
    }
}
