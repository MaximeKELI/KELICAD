//! KELICAD plugin host — capability-gated hostcalls (ADR 0009).

#![forbid(unsafe_code)]
#![deny(missing_docs)]

pub mod error;
pub mod guest;
pub mod host;
pub mod macros;
pub mod manifest;
pub mod market;

#[cfg(test)]
mod rigorous;

pub use error::PluginError;
pub use guest::{Guest, NativeGuest, ScriptGuest, ScriptStep};
pub use host::PluginHost;
pub use macros::{
    macro_from_json, macro_to_json, record_macro, replay_macro, replay_ops_json, MacroScript,
    MacroStep,
};
pub use manifest::{Manifest, Permissions};
pub use market::{
    public_key_hex, sign_entry, signing_key_from_seed, verify_catalog, verify_entry, MarketCatalog,
    MarketEntry, DEMO_MARKET_SEED,
};

#[cfg(test)]
mod tests {
    use kelicad_ops::DocSession;
    use serde_json::json;

    use super::*;

    fn layer0_write_manifest() -> Manifest {
        Manifest {
            id: "hello-offset".into(),
            version: "0.1.0".into(),
            name: "Hello Offset".into(),
            permissions: Permissions {
                doc_read: true,
                doc_write_layers: vec!["0".into()],
                net_hosts: vec![],
            },
            max_hostcalls: 32,
        }
    }

    #[test]
    fn allowed_create_line() {
        let mut host = PluginHost::install(layer0_write_manifest(), DocSession::new());
        let guest = NativeGuest::new(|h| {
            let _ = h.hostcall(
                "create_line",
                &json!({ "start": [0.0, 0.0], "end": [1.0, 0.0], "layer": "0" }),
            )?;
            Ok(())
        });
        host.run_guest(&guest).unwrap();
        assert_eq!(host.session.document.entity_count(), 1);
    }

    #[test]
    fn deny_write_other_layer() {
        let mut host = PluginHost::install(layer0_write_manifest(), DocSession::new());
        let guest = NativeGuest::new(|h| {
            h.hostcall(
                "create_line",
                &json!({ "start": [0.0, 0.0], "end": [1.0, 0.0], "layer": "Walls" }),
            )?;
            Ok(())
        });
        let err = host.run_guest(&guest).unwrap_err();
        assert!(matches!(err, PluginError::Denied(_)));
        assert_eq!(host.session.document.entity_count(), 0);
    }

    #[test]
    fn deny_net_without_host() {
        let mut host = PluginHost::install(layer0_write_manifest(), DocSession::new());
        let guest = NativeGuest::new(|h| {
            h.hostcall("http_get", &json!({ "url": "https://example.com/" }))?;
            Ok(())
        });
        let err = host.run_guest(&guest).unwrap_err();
        assert!(matches!(err, PluginError::Denied(_)));
    }

    #[test]
    fn allow_net_when_declared() {
        let mut m = layer0_write_manifest();
        m.permissions.net_hosts = vec!["example.com".into()];
        let mut host = PluginHost::install(m, DocSession::new());
        let guest = NativeGuest::new(|h| {
            let v = h.hostcall("http_get", &json!({ "url": "https://example.com/x" }))?;
            assert_eq!(v["status"], 200);
            Ok(())
        });
        host.run_guest(&guest).unwrap();
    }

    #[test]
    fn deny_read_without_permission() {
        let m = Manifest {
            id: "x".into(),
            version: "0.1.0".into(),
            name: String::new(),
            permissions: Permissions::default(),
            max_hostcalls: 8,
        };
        let mut host = PluginHost::install(m, DocSession::new());
        let err = host.hostcall("list_entities", &json!({})).unwrap_err();
        assert!(matches!(err, PluginError::Denied(_)));
    }

    #[test]
    fn macro_roundtrip() {
        let mut session = DocSession::new();
        let mut host = PluginHost::install(layer0_write_manifest(), DocSession::new());
        let guest = NativeGuest::new(|h| {
            h.hostcall(
                "create_line",
                &json!({ "start": [0.0, 0.0], "end": [2.0, 0.0], "layer": "0" }),
            )?;
            Ok(())
        });
        host.run_guest(&guest).unwrap();
        let script = record_macro(&host.session);
        let json = macro_to_json(&script).unwrap();
        let parsed = macro_from_json(&json).unwrap();
        let n = replay_macro(&mut session, &parsed).unwrap();
        assert_eq!(n, 1);
        assert_eq!(session.document.entity_count(), 1);
    }
}
