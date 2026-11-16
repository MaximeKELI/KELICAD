//! External ODA worker bridge (`KELICAD_DWG_WORKER`).

use std::io::Write;
use std::process::{Command, Stdio};
use std::time::Duration;

use crate::error::DwgError;

/// Bridge that shells out to an external converter.
///
/// Protocol: worker reads DWG from stdin, writes DXF UTF-8 to stdout
/// (import). For export, env `KELICAD_DWG_MODE=export` and DXF on stdin → DWG
/// on stdout. Exit 0 = success.
#[derive(Debug, Clone)]
pub struct ExternalDwgBridge {
    /// Executable path (or from `KELICAD_DWG_WORKER`).
    pub worker: String,
    /// Soft timeout hint (seconds); best-effort via process wait.
    pub timeout_secs: u64,
}

impl Default for ExternalDwgBridge {
    fn default() -> Self {
        Self {
            worker: std::env::var("KELICAD_DWG_WORKER")
                .unwrap_or_else(|_| "kelicad-dwg-worker".into()),
            timeout_secs: 60,
        }
    }
}

impl ExternalDwgBridge {
    /// From environment / defaults.
    pub fn from_env() -> Self {
        Self::default()
    }

    fn run(&self, mode: &str, input: &[u8]) -> Result<Vec<u8>, DwgError> {
        let mut child = Command::new(&self.worker)
            .env("KELICAD_DWG_MODE", mode)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| DwgError::Worker(format!("spawn {}: {e}", self.worker)))?;
        {
            let mut stdin = child
                .stdin
                .take()
                .ok_or_else(|| DwgError::Worker("no stdin".into()))?;
            stdin
                .write_all(input)
                .map_err(|e| DwgError::Io(e.to_string()))?;
        }
        // Best-effort timeout: on unix we could kill; keep portable wait.
        let _ = Duration::from_secs(self.timeout_secs);
        let out = child
            .wait_with_output()
            .map_err(|e| DwgError::Worker(e.to_string()))?;
        if !out.status.success() {
            let err = String::from_utf8_lossy(&out.stderr);
            return Err(DwgError::Worker(format!(
                "exit {:?}: {err}",
                out.status.code()
            )));
        }
        Ok(out.stdout)
    }
}

impl crate::bridge::DwgBridge for ExternalDwgBridge {
    fn dwg_to_dxf(&self, dwg: &[u8]) -> Result<String, DwgError> {
        let bytes = self.run("import", dwg)?;
        String::from_utf8(bytes).map_err(|e| DwgError::Dxf(e.to_string()))
    }

    fn dxf_to_dwg(&self, dxf: &str) -> Result<Vec<u8>, DwgError> {
        self.run("export", dxf.as_bytes())
    }
}
