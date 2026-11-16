//! Signed plugin marketplace catalog (ADR 0012).

use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::error::PluginError;

/// Marketplace catalog root.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketCatalog {
    /// Catalog schema version.
    pub version: u32,
    /// Hex-encoded ed25519 public key (32 bytes).
    pub public_key: String,
    /// Plugin entries.
    pub entries: Vec<MarketEntry>,
}

/// One catalog entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketEntry {
    /// Plugin id.
    pub id: String,
    /// Semver.
    pub version: String,
    /// Short description.
    pub description: String,
    /// Relative path to plugin directory / archive.
    pub path: String,
    /// SHA-256 hex of canonical payload (id|version|path|description).
    pub content_hash: String,
    /// ed25519 signature hex over `content_hash` bytes (utf-8 hex string).
    pub signature: String,
}

impl MarketEntry {
    /// Canonical content string used for hashing.
    pub fn canonical(&self) -> String {
        format!(
            "{}|{}|{}|{}",
            self.id, self.version, self.path, self.description
        )
    }

    /// Compute content hash.
    pub fn compute_hash(&self) -> String {
        let mut h = Sha256::new();
        h.update(self.canonical().as_bytes());
        hex::encode(h.finalize())
    }
}

/// Sign an entry with a signing key; fills content_hash + signature.
pub fn sign_entry(entry: &mut MarketEntry, signing_key: &SigningKey) {
    entry.content_hash = entry.compute_hash();
    let sig = signing_key.sign(entry.content_hash.as_bytes());
    entry.signature = hex::encode(sig.to_bytes());
}

/// Verify one entry against catalog public key.
pub fn verify_entry(catalog: &MarketCatalog, entry: &MarketEntry) -> Result<(), PluginError> {
    if entry.compute_hash() != entry.content_hash {
        return Err(PluginError::Denied(
            "marketplace content_hash mismatch".into(),
        ));
    }
    let pk_bytes = hex::decode(&catalog.public_key)
        .map_err(|e| PluginError::Denied(format!("bad public_key hex: {e}")))?;
    let pk_arr: [u8; 32] = pk_bytes
        .as_slice()
        .try_into()
        .map_err(|_| PluginError::Denied("public_key must be 32 bytes".into()))?;
    let vk = VerifyingKey::from_bytes(&pk_arr)
        .map_err(|e| PluginError::Denied(format!("bad public key: {e}")))?;
    let sig_bytes = hex::decode(&entry.signature)
        .map_err(|e| PluginError::Denied(format!("bad signature hex: {e}")))?;
    let sig_arr: [u8; 64] = sig_bytes
        .as_slice()
        .try_into()
        .map_err(|_| PluginError::Denied("signature must be 64 bytes".into()))?;
    let sig = Signature::from_bytes(&sig_arr);
    vk.verify(entry.content_hash.as_bytes(), &sig)
        .map_err(|_| PluginError::Denied("marketplace signature invalid".into()))?;
    Ok(())
}

/// Verify all entries.
pub fn verify_catalog(catalog: &MarketCatalog) -> Result<(), PluginError> {
    for e in &catalog.entries {
        verify_entry(catalog, e)?;
    }
    Ok(())
}

/// Deterministic test/dev signing key from a 32-byte seed.
pub fn signing_key_from_seed(seed: [u8; 32]) -> SigningKey {
    SigningKey::from_bytes(&seed)
}

/// Hex public key for a signing key.
pub fn public_key_hex(signing_key: &SigningKey) -> String {
    hex::encode(signing_key.verifying_key().to_bytes())
}

/// KELICAD marketplace demo seed (CI / on-prem sample — replace in production).
pub const DEMO_MARKET_SEED: [u8; 32] = [0x4b; 32]; // 'K' repeated

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sign_and_verify() {
        let sk = signing_key_from_seed(DEMO_MARKET_SEED);
        let mut entry = MarketEntry {
            id: "hello-offset".into(),
            version: "0.1.0".into(),
            description: "Example offset plugin".into(),
            path: "sdk/examples/hello-offset".into(),
            content_hash: String::new(),
            signature: String::new(),
        };
        sign_entry(&mut entry, &sk);
        let catalog = MarketCatalog {
            version: 1,
            public_key: public_key_hex(&sk),
            entries: vec![entry.clone()],
        };
        assert!(verify_catalog(&catalog).is_ok());
        let mut bad = entry;
        bad.signature = hex::encode([0u8; 64]);
        assert!(verify_entry(&catalog, &bad).is_err());
    }
}
