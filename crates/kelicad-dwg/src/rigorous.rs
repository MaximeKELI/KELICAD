//! Rigorous DWG bridge edge cases.

use crate::bridge::{export_dwg, import_dwg, DwgBridge};
use crate::error::DwgError;
use crate::external::ExternalDwgBridge;
use crate::mock::{is_proxy_dwg, unwrap_dxf, wrap_dxf, MockDwgBridge, PROXY_MAGIC};

#[test]
fn reject_non_proxy_bytes() {
    let err = unwrap_dxf(b"AC1015 fake dwg").unwrap_err();
    assert!(matches!(err, DwgError::Unsupported(_)));
}

#[test]
fn proxy_magic_prefix_required() {
    assert!(!is_proxy_dwg(b""));
    assert!(!is_proxy_dwg(b"KELICAD"));
    let good = wrap_dxf("0\nEOF\n");
    assert!(is_proxy_dwg(&good));
    assert!(good.starts_with(PROXY_MAGIC));
}

#[test]
fn wrap_unwrap_preserves_utf8() {
    let dxf = "0\nSECTION\n2\nHEADER\n0\nENDSEC\n0\nEOF\n";
    let bytes = wrap_dxf(dxf);
    assert_eq!(unwrap_dxf(&bytes).unwrap(), dxf);
}

#[test]
fn import_rejects_non_proxy_via_mock() {
    let bridge = MockDwgBridge;
    let err = import_dwg(b"not-a-proxy", &bridge).unwrap_err();
    assert!(matches!(err, DwgError::Unsupported(_)));
}

#[test]
fn empty_session_export_roundtrip() {
    let bridge = MockDwgBridge;
    let session = kelicad_ops::DocSession::new();
    let bytes = export_dwg(&session, &bridge).unwrap();
    assert!(is_proxy_dwg(&bytes));
    let (loaded, n) = import_dwg(&bytes, &bridge).unwrap();
    assert_eq!(n, 0);
    assert_eq!(loaded.document.entity_count(), 0);
}

#[test]
fn external_missing_worker_errors() {
    let bridge = ExternalDwgBridge {
        worker: "/nonexistent/kelicad-dwg-worker-xyz".into(),
        timeout_secs: 1,
    };
    let err = bridge.dwg_to_dxf(b"x").unwrap_err();
    assert!(matches!(err, DwgError::Worker(_)));
}
