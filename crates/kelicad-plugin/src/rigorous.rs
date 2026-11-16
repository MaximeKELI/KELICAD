//! Rigorous marketplace and capability edge cases.

use kelicad_ops::DocSession;
use serde_json::json;

use crate::error::PluginError;
use crate::guest::NativeGuest;
use crate::host::PluginHost;
use crate::manifest::{Manifest, Permissions};
use crate::market::{
    public_key_hex, sign_entry, signing_key_from_seed, verify_catalog, verify_entry, MarketCatalog,
    MarketEntry, DEMO_MARKET_SEED,
};

fn layer0_manifest() -> Manifest {
    Manifest {
        id: "t".into(),
        version: "0.1.0".into(),
        name: "t".into(),
        permissions: Permissions {
            doc_read: true,
            doc_write_layers: vec!["0".into()],
            net_hosts: vec![],
        },
        max_hostcalls: 4,
    }
}

#[test]
fn content_hash_tamper_denied() {
    let sk = signing_key_from_seed(DEMO_MARKET_SEED);
    let mut entry = MarketEntry {
        id: "p".into(),
        version: "1.0.0".into(),
        description: "d".into(),
        path: "x".into(),
        content_hash: String::new(),
        signature: String::new(),
    };
    sign_entry(&mut entry, &sk);
    let catalog = MarketCatalog {
        version: 1,
        public_key: public_key_hex(&sk),
        entries: vec![entry.clone()],
    };
    entry.description = "tampered".into();
    // Hash string still old, but compute_hash differs → Denied
    let err = verify_entry(&catalog, &entry).unwrap_err();
    assert!(matches!(err, PluginError::Denied(_)));
}

#[test]
fn wrong_public_key_denied() {
    let sk = signing_key_from_seed(DEMO_MARKET_SEED);
    let other = signing_key_from_seed([0x42; 32]);
    let mut entry = MarketEntry {
        id: "p".into(),
        version: "1.0.0".into(),
        description: "d".into(),
        path: "x".into(),
        content_hash: String::new(),
        signature: String::new(),
    };
    sign_entry(&mut entry, &sk);
    let catalog = MarketCatalog {
        version: 1,
        public_key: public_key_hex(&other),
        entries: vec![entry.clone()],
    };
    assert!(verify_entry(&catalog, &entry).is_err());
}

#[test]
fn empty_catalog_verifies() {
    let sk = signing_key_from_seed(DEMO_MARKET_SEED);
    let catalog = MarketCatalog {
        version: 1,
        public_key: public_key_hex(&sk),
        entries: vec![],
    };
    assert!(verify_catalog(&catalog).is_ok());
}

#[test]
fn hostcall_budget_enforced() {
    let mut host = PluginHost::install(layer0_manifest(), DocSession::new());
    let guest = NativeGuest::new(|h| {
        for _ in 0..5 {
            h.hostcall(
                "create_line",
                &json!({ "start": [0.0, 0.0], "end": [1.0, 0.0], "layer": "0" }),
            )?;
        }
        Ok(())
    });
    let err = host.run_guest(&guest).unwrap_err();
    assert!(matches!(err, PluginError::Denied(_) | PluginError::BudgetExceeded));
}

#[test]
fn bad_signature_hex_denied() {
    let sk = signing_key_from_seed(DEMO_MARKET_SEED);
    let mut entry = MarketEntry {
        id: "p".into(),
        version: "1.0.0".into(),
        description: "d".into(),
        path: "x".into(),
        content_hash: String::new(),
        signature: String::new(),
    };
    sign_entry(&mut entry, &sk);
    let catalog = MarketCatalog {
        version: 1,
        public_key: public_key_hex(&sk),
        entries: vec![],
    };
    entry.signature = "zz".into();
    assert!(verify_entry(&catalog, &entry).is_err());
}
