//! Paper space layouts and viewports (M2 minimal).

use kelicad_math::Vec2;
use uuid::Uuid;

/// Layout id.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LayoutId(Uuid);

impl LayoutId {
    /// From UUID.
    pub const fn from_uuid(id: Uuid) -> Self {
        Self(id)
    }

    /// UUID.
    pub const fn as_uuid(self) -> Uuid {
        self.0
    }
}

impl std::fmt::Display for LayoutId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

/// Paper size in millimetres.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PaperSize {
    /// Width mm.
    pub width_mm: f64,
    /// Height mm.
    pub height_mm: f64,
}

impl PaperSize {
    /// ISO A4 portrait.
    pub const A4: Self = Self {
        width_mm: 210.0,
        height_mm: 297.0,
    };
}

/// Viewport on a layout mapping paper → model.
#[derive(Debug, Clone, PartialEq)]
pub struct Viewport {
    /// Center on paper (mm).
    pub center_paper: Vec2,
    /// Width/height on paper (mm).
    pub size_paper: Vec2,
    /// Center in model space.
    pub center_model: Vec2,
    /// Scale: model units per paper mm (e.g. 1/100 → 0.01 if units are metres… stored as ratio model/paper).
    pub scale: f64,
}

/// Named paper-space layout.
#[derive(Debug, Clone, PartialEq)]
pub struct Layout {
    /// Id.
    pub id: LayoutId,
    /// Name (`Model` is reserved for model space conceptually; layouts are paper).
    pub name: String,
    /// Paper size.
    pub paper: PaperSize,
    /// Viewports.
    pub viewports: Vec<Viewport>,
}
