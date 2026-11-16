//! External DWG worker for `KELICAD_DWG_WORKER` (ADR 0012 / 0014).
//!
//! Protocol:
//! - stdin: bytes (DWG or DXF depending on mode)
//! - stdout: converted bytes
//! - env `KELICAD_DWG_MODE=import|export`
//! - exit 0 on success
//!
//! Without an ODA Toolkit license this binary uses the MIT **proxy DWG** container
//! (`KELICAD_DWG_PROXY` + DXF payload). Drop in a real ODA converter behind the same
//! CLI contract when available — no KELICAD core change required.

use std::io::{Read, Write};

use kelicad_dwg::{is_proxy_dwg, MockDwgBridge, DwgBridge, PROXY_MAGIC};

fn main() {
    if let Err(e) = run() {
        eprintln!("kelicad-dwg-worker: {e}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let mode = std::env::var("KELICAD_DWG_MODE").unwrap_or_else(|_| "import".into());
    let mut input = Vec::new();
    std::io::stdin()
        .read_to_end(&mut input)
        .map_err(|e| e.to_string())?;

    let bridge = MockDwgBridge;
    let out = match mode.as_str() {
        "import" => {
            if input.starts_with(b"AC10") || input.starts_with(b"AC2") {
                return Err(
                    "raw Autodesk DWG detected: configure an ODA-backed worker binary \
                     (same stdin/stdout protocol). Proxy DWG (KELICAD_DWG_PROXY) is supported."
                        .into(),
                );
            }
            if !is_proxy_dwg(&input) && !looks_like_dxf(&input) {
                return Err(format!(
                    "unrecognized input (expected proxy DWG starting with {:?} or DXF)",
                    String::from_utf8_lossy(PROXY_MAGIC)
                ));
            }
            let dxf = if looks_like_dxf(&input) {
                String::from_utf8(input).map_err(|e| e.to_string())?
            } else {
                bridge.dwg_to_dxf(&input).map_err(|e| e.to_string())?
            };
            dxf.into_bytes()
        }
        "export" => {
            let dxf = String::from_utf8(input).map_err(|e| e.to_string())?;
            bridge.dxf_to_dwg(&dxf).map_err(|e| e.to_string())?
        }
        other => return Err(format!("unknown KELICAD_DWG_MODE={other}")),
    };

    std::io::stdout()
        .write_all(&out)
        .map_err(|e| e.to_string())?;
    Ok(())
}

fn looks_like_dxf(bytes: &[u8]) -> bool {
    let s = String::from_utf8_lossy(bytes);
    let t = s.trim_start();
    t.starts_with('0') || t.to_ascii_uppercase().contains("SECTION")
}
