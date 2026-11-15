//! Plugin manifest and permissions.

use serde::{Deserialize, Serialize};

/// Declared plugin capabilities (fail closed if missing).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Permissions {
    /// May read document state.
    #[serde(default)]
    pub doc_read: bool,
    /// Layer names the plugin may write (`"*"` = all).
    #[serde(default)]
    pub doc_write_layers: Vec<String>,
    /// Allowed HTTP hostnames for `http_get`.
    #[serde(default)]
    pub net_hosts: Vec<String>,
}

impl Permissions {
    /// True if writing to `layer` is allowed.
    pub fn can_write_layer(&self, layer: &str) -> bool {
        self.doc_write_layers
            .iter()
            .any(|l| l == "*" || l.eq_ignore_ascii_case(layer))
    }

    /// True if `host` (hostname) is allowed.
    pub fn can_net_host(&self, host: &str) -> bool {
        let host = host.to_ascii_lowercase();
        self.net_hosts.iter().any(|h| {
            let h = h.to_ascii_lowercase();
            h == "*" || h == host || host.ends_with(&format!(".{h}"))
        })
    }
}

/// Plugin package manifest (`plugin.json`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Manifest {
    /// Stable plugin id.
    pub id: String,
    /// Semver-ish version string.
    pub version: String,
    /// Human name.
    #[serde(default)]
    pub name: String,
    /// Permissions.
    #[serde(default)]
    pub permissions: Permissions,
    /// Max hostcalls per invoke (default 64).
    #[serde(default = "default_max_hostcalls")]
    pub max_hostcalls: u32,
}

fn default_max_hostcalls() -> u32 {
    64
}

impl Manifest {
    /// Minimal read-only manifest.
    pub fn read_only(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            version: "0.1.0".into(),
            name: String::new(),
            permissions: Permissions {
                doc_read: true,
                ..Permissions::default()
            },
            max_hostcalls: 64,
        }
    }
}
