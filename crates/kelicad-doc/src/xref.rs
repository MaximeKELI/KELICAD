//! External references (XREF) and underlays (PDF / raster / DWG proxy).

use kelicad_math::Vec2;
use uuid::Uuid;

/// XREF / underlay definition id.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct XrefId(Uuid);

impl XrefId {
    /// From UUID.
    pub const fn from_uuid(id: Uuid) -> Self {
        Self(id)
    }

    /// Underlying UUID.
    pub const fn as_uuid(self) -> Uuid {
        self.0
    }
}

impl std::fmt::Display for XrefId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

/// Kind of external content.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum XrefKind {
    /// Nested KELICAD / DXF drawing (vector).
    Drawing,
    /// PDF underlay (page rasterized or vectorized externally).
    Pdf,
    /// Raster image underlay.
    Image,
    /// DWG attached via bridge (stored as path + proxy snapshot).
    Dwg,
}

/// Definition of an external reference (path + metadata). Geometry lives in inserts.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct XrefDef {
    /// Id.
    pub id: XrefId,
    /// Logical name (unique).
    pub name: String,
    /// Absolute or project-relative path / URI.
    pub path: String,
    /// Kind.
    pub kind: XrefKind,
    /// PDF page index (0-based); ignored otherwise.
    pub page: u32,
    /// Optional cached DXF snapshot for offline display (Drawing/Dwg).
    pub cached_dxf: Option<String>,
    /// Image / PDF intrinsic size in drawing units when known.
    pub source_size: Option<Vec2>,
}

impl XrefDef {
    /// Create a drawing/PDF/image/DWG xref definition.
    pub fn new(id: XrefId, name: impl Into<String>, path: impl Into<String>, kind: XrefKind) -> Self {
        Self {
            id,
            name: name.into(),
            path: path.into(),
            kind,
            page: 0,
            cached_dxf: None,
            source_size: None,
        }
    }
}

/// Insert of an XREF / underlay in model or paper space.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct XrefInsertGeom {
    /// Definition id.
    pub xref_id: XrefId,
    /// Insertion point.
    pub position: Vec2,
    /// Scale.
    pub scale: Vec2,
    /// Rotation radians.
    pub rotation: f64,
    /// Clip rectangle in underlay local coords (`None` = full).
    pub clip: Option<(Vec2, Vec2)>,
    /// Fade 0..1 (1 = fully visible).
    pub fade: f64,
}
