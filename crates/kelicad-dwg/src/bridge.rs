//! DwgBridge trait and high-level import/export.

use kelicad_io::{export_dxf, import_dxf};
use kelicad_ops::DocSession;

use crate::error::DwgError;

/// Convert between DWG bytes and DXF text (ODA-pluggable).
pub trait DwgBridge {
    /// DWG (or proxy) bytes → DXF text.
    fn dwg_to_dxf(&self, dwg: &[u8]) -> Result<String, DwgError>;
    /// DXF text → DWG (or proxy) bytes.
    fn dxf_to_dwg(&self, dxf: &str) -> Result<Vec<u8>, DwgError>;
}

/// Import DWG bytes into a new session via the bridge.
pub fn import_dwg(bytes: &[u8], bridge: &dyn DwgBridge) -> Result<(DocSession, usize), DwgError> {
    let dxf = bridge.dwg_to_dxf(bytes)?;
    let (session, report) = import_dxf(&dxf).map_err(|e| DwgError::Dxf(e.to_string()))?;
    Ok((session, report.imported))
}

/// Export a session to DWG bytes via the bridge.
pub fn export_dwg(session: &DocSession, bridge: &dyn DwgBridge) -> Result<Vec<u8>, DwgError> {
    let dxf = export_dxf(session).map_err(|e| DwgError::Dxf(e.to_string()))?;
    bridge.dxf_to_dwg(&dxf)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock::{wrap_dxf, MockDwgBridge};

    #[test]
    fn mock_roundtrip_preserves_entities() {
        // Minimal DXF LINE via mock proxy (avoid pulling doc/geom into this crate's deps).
        let dxf = "0\nSECTION\n2\nENTITIES\n0\nLINE\n8\n0\n10\n0.0\n20\n0.0\n11\n10.0\n21\n0.0\n0\nENDSEC\n0\nEOF\n";
        let bridge = MockDwgBridge;
        let bytes = wrap_dxf(dxf);
        assert!(crate::mock::is_proxy_dwg(&bytes));
        let (loaded, n) = import_dwg(&bytes, &bridge).unwrap();
        assert!(n >= 1);
        assert!(loaded.document.entity_count() >= 1);
        let out = export_dwg(&loaded, &bridge).unwrap();
        let (again, n2) = import_dwg(&out, &bridge).unwrap();
        assert_eq!(again.document.entity_count(), loaded.document.entity_count());
        assert_eq!(n2, n);
    }
}
