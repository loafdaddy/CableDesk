//! Distro / desktop / session detection via `/etc/os-release`, `/proc` and
//! well-known environment variables. Pure reads, no privilege required.

use cabledesk_core::error::Result;
use cabledesk_platform::{SessionType, SystemInfo};
use std::collections::HashMap;

pub async fn detect_system() -> Result<SystemInfo> {
    let os_release = parse_os_release(&tokio::fs::read_to_string("/etc/os-release").await?);

    let distro_id = os_release
        .get("ID")
        .cloned()
        .unwrap_or_else(|| "unknown".to_string());
    let distro_name = os_release
        .get("PRETTY_NAME")
        .or_else(|| os_release.get("NAME"))
        .cloned()
        .unwrap_or_else(|| "Unknown Linux".to_string());
    let distro_version = os_release
        .get("VERSION_ID")
        .cloned()
        .unwrap_or_else(|| "unknown".to_string());

    let desktop_environment = std::env::var("XDG_CURRENT_DESKTOP").ok();
    let session_type = match std::env::var("XDG_SESSION_TYPE").as_deref() {
        Ok("wayland") => SessionType::Wayland,
        Ok("x11") => SessionType::X11,
        _ => SessionType::Unknown,
    };

    let kernel_version = tokio::fs::read_to_string("/proc/sys/kernel/osrelease")
        .await
        .unwrap_or_default()
        .trim()
        .to_string();
    let hostname = tokio::fs::read_to_string("/proc/sys/kernel/hostname")
        .await
        .unwrap_or_default()
        .trim()
        .to_string();

    Ok(SystemInfo {
        distro_id,
        distro_name,
        distro_version,
        desktop_environment,
        session_type,
        architecture: std::env::consts::ARCH.to_string(),
        kernel_version,
        hostname,
    })
}

/// Parses the `KEY=VALUE` / `KEY="VALUE"` shell-subset format of os-release.
fn parse_os_release(contents: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for line in contents.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((key, value)) = line.split_once('=') {
            let value = value.trim().trim_matches('"').to_string();
            map.insert(key.trim().to_string(), value);
        }
    }
    map
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_quoted_and_unquoted_values() {
        let sample = r#"
NAME="Fedora Linux"
ID=fedora
VERSION_ID=44
# a comment
PRETTY_NAME="Fedora Linux 44 (Workstation Edition)"
"#;
        let map = parse_os_release(sample);
        assert_eq!(map.get("ID").unwrap(), "fedora");
        assert_eq!(map.get("VERSION_ID").unwrap(), "44");
        assert_eq!(
            map.get("PRETTY_NAME").unwrap(),
            "Fedora Linux 44 (Workstation Edition)"
        );
    }

    #[test]
    fn ignores_blank_and_comment_lines() {
        let map = parse_os_release("\n# comment\n\nID=fedora\n");
        assert_eq!(map.len(), 1);
    }
}
