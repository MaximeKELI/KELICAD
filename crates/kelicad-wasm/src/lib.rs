//! WASM façade for the KELICAD core.
//!
//! Document mutation always goes through [`kelicad_ops`]; this module only
//! exposes a safe JS surface.

#![forbid(unsafe_code)]

use kelicad_doc::{EntityId, EntityKind, EntityRecord, Geometry, LayerId};
use kelicad_geom::Line;
use kelicad_math::Vec2;
use kelicad_ops::{DocSession, Operation};
use uuid::Uuid;
use wasm_bindgen::prelude::*;

/// Install the panic hook once (call from JS on module init).
#[wasm_bindgen(start)]
pub fn start() {
    console_error_panic_hook::set_once();
}

/// Core crate version string.
#[wasm_bindgen]
pub fn core_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// Generate interleaved line endpoints `[x0,y0,x1,y1, …]` for the M0 render demo.
///
/// Deterministic layout: a grid of axis-aligned segments. Used so the web
/// canvas can stress GPU line throughput without depending on UI code for data.
#[wasm_bindgen]
pub fn generate_demo_line_vertices(count: u32) -> Vec<f32> {
    let n = count as usize;
    let mut out = Vec::with_capacity(n * 4);
    let cols = (n as f64).sqrt().ceil().max(1.0) as u32;
    for i in 0..n {
        let col = (i as u32) % cols;
        let row = (i as u32) / cols;
        let x0 = col as f32 * 2.0;
        let y0 = row as f32 * 2.0;
        let horizontal = i % 2 == 0;
        let (x1, y1) = if horizontal {
            (x0 + 1.5, y0)
        } else {
            (x0, y0 + 1.5)
        };
        out.extend_from_slice(&[x0, y0, x1, y1]);
    }
    out
}

/// Opaque document session handle for JS.
#[wasm_bindgen]
pub struct WasmDocSession {
    inner: DocSession,
}

#[wasm_bindgen]
impl WasmDocSession {
    /// Create an empty document session.
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            inner: DocSession::new(),
        }
    }

    /// Number of entities in the document.
    pub fn entity_count(&self) -> u32 {
        self.inner.document.entity_count() as u32
    }

    /// Create a line from `(x0,y0)` to `(x1,y1)` on the current layer.
    /// Returns the entity id as a hyphenated UUID string.
    pub fn add_line(&mut self, x0: f64, y0: f64, x1: f64, y1: f64) -> Result<String, JsValue> {
        let id = EntityId::from_uuid(Uuid::now_v7());
        let layer_id = self.inner.document.current_layer;
        let record = EntityRecord {
            id,
            kind: EntityKind::Line,
            geometry: Geometry::Line(Line::new(Vec2::new(x0, y0), Vec2::new(x1, y1))),
            layer_id,
        };
        self.inner
            .apply(Operation::CreateEntity { record })
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        Ok(id.to_string())
    }

    /// Undo last op; returns whether an op was undone.
    pub fn undo(&mut self) -> Result<bool, JsValue> {
        self.inner
            .undo()
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }

    /// Redo; returns whether an op was redone.
    pub fn redo(&mut self) -> Result<bool, JsValue> {
        self.inner
            .redo()
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }

    /// Current layer id string.
    pub fn current_layer_id(&self) -> String {
        LayerId::as_uuid(self.inner.document.current_layer).to_string()
    }
}

impl Default for WasmDocSession {
    fn default() -> Self {
        Self::new()
    }
}
