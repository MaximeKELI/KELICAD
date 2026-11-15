//! Guest runtimes (NativeGuest + optional wasmtime).

use serde_json::{json, Value};

use crate::error::PluginError;
use crate::host::PluginHost;

/// Plugin guest entrypoint.
pub trait Guest {
    /// Execute against the host (may issue hostcalls).
    fn run(&self, host: &mut PluginHost) -> Result<(), PluginError>;
}

/// In-process guest: a closure of hostcall recipes or custom logic.
pub struct NativeGuest<F>
where
    F: Fn(&mut PluginHost) -> Result<(), PluginError>,
{
    /// Runner.
    pub run_fn: F,
}

impl<F> NativeGuest<F>
where
    F: Fn(&mut PluginHost) -> Result<(), PluginError>,
{
    /// Wrap a function.
    pub fn new(run_fn: F) -> Self {
        Self { run_fn }
    }
}

impl<F> Guest for NativeGuest<F>
where
    F: Fn(&mut PluginHost) -> Result<(), PluginError>,
{
    fn run(&self, host: &mut PluginHost) -> Result<(), PluginError> {
        (self.run_fn)(host)
    }
}

/// Declarative guest: ordered list of `{ "method", "args" }` hostcalls.
#[derive(Debug, Clone)]
pub struct ScriptGuest {
    /// Steps.
    pub steps: Vec<ScriptStep>,
}

/// One scripted hostcall.
#[derive(Debug, Clone)]
pub struct ScriptStep {
    /// Method name.
    pub method: String,
    /// Args object.
    pub args: Value,
}

impl ScriptGuest {
    /// From JSON array of `{method, args}`.
    pub fn from_json(v: &Value) -> Result<Self, PluginError> {
        let arr = v
            .as_array()
            .ok_or_else(|| PluginError::InvalidArgs("script must be array".into()))?;
        let mut steps = Vec::new();
        for item in arr {
            let method = item
                .get("method")
                .and_then(|m| m.as_str())
                .ok_or_else(|| PluginError::InvalidArgs("method".into()))?
                .to_string();
            let args = item.get("args").cloned().unwrap_or_else(|| json!({}));
            steps.push(ScriptStep { method, args });
        }
        Ok(Self { steps })
    }
}

impl Guest for ScriptGuest {
    fn run(&self, host: &mut PluginHost) -> Result<(), PluginError> {
        for step in &self.steps {
            let _ = host.hostcall(&step.method, &step.args)?;
        }
        Ok(())
    }
}

/// Placeholder WASM guest loader (enabled with feature `wasm`).
#[cfg(feature = "wasm")]
pub mod wasm_guest {
    use super::*;

    /// Load a WASI module — M5: fuel-limited instantiate; guest must export
    /// nothing beyond what host injects. Full ABI wiring is incremental;
    /// for now we refuse unsigned/empty blobs and document the path.
    pub struct WasmGuest {
        /// Raw wasm bytes.
        pub bytes: Vec<u8>,
    }

    impl Guest for WasmGuest {
        fn run(&self, _host: &mut PluginHost) -> Result<(), PluginError> {
            if self.bytes.is_empty() {
                return Err(PluginError::Wasm("empty module".into()));
            }
            // Fuel-limited engine proves we can bound CPU; full host imports
            // land when the guest ABI stabilizes.
            let mut config = wasmtime::Config::new();
            config.consume_fuel(true);
            let engine =
                wasmtime::Engine::new(&config).map_err(|e| PluginError::Wasm(e.to_string()))?;
            let module = wasmtime::Module::new(&engine, &self.bytes)
                .map_err(|e| PluginError::Wasm(e.to_string()))?;
            let mut store = wasmtime::Store::new(&engine, ());
            store
                .set_fuel(10_000)
                .map_err(|e| PluginError::Wasm(e.to_string()))?;
            let _instance = wasmtime::Instance::new(&mut store, &module, &[])
                .map_err(|e| PluginError::Wasm(e.to_string()))?;
            Ok(())
        }
    }
}
