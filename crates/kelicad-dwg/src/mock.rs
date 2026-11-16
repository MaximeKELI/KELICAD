//! Mock DWG bridge: DXF wrapped in a documented proxy container (not real DWG).

use crate::error::DwgError;

/// Magic ASCII prefix for the KELICAD DWG proxy container.
pub const PROXY_MAGIC: &[u8] = b"KELICAD_DWG_PROXY1\n";

/// True if bytes look like our proxy container.
pub fn is_proxy_dwg(bytes: &[u8]) -> bool {
    bytes.starts_with(PROXY_MAGIC)
}

/// Wrap UTF-8 DXF text into proxy DWG bytes.
pub fn wrap_dxf(dxf: &str) -> Vec<u8> {
    let mut out = PROXY_MAGIC.to_vec();
    out.extend_from_slice(dxf.as_bytes());
    out
}

/// Unwrap proxy bytes → DXF text.
pub fn unwrap_dxf(bytes: &[u8]) -> Result<String, DwgError> {
    if !is_proxy_dwg(bytes) {
        return Err(DwgError::Unsupported(
            "expected KELICAD_DWG_PROXY1 container (use ExternalDwgBridge for real DWG)".into(),
        ));
    }
    let body = &bytes[PROXY_MAGIC.len()..];
    String::from_utf8(body.to_vec()).map_err(|e| DwgError::Dxf(e.to_string()))
}

/// Default CI bridge.
#[derive(Debug, Clone, Default)]
pub struct MockDwgBridge;

impl crate::bridge::DwgBridge for MockDwgBridge {
    fn dwg_to_dxf(&self, dwg: &[u8]) -> Result<String, DwgError> {
        unwrap_dxf(dwg)
    }

    fn dxf_to_dwg(&self, dxf: &str) -> Result<Vec<u8>, DwgError> {
        Ok(wrap_dxf(dxf))
    }
}
