//! Paper space layouts, viewports, and plot styles.

use kelicad_math::Vec2;
use uuid::Uuid;

/// Layout id.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
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
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
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
    /// ISO A3 landscape-friendly.
    pub const A3: Self = Self {
        width_mm: 297.0,
        height_mm: 420.0,
    };
    /// ANSI Letter.
    pub const LETTER: Self = Self {
        width_mm: 215.9,
        height_mm: 279.4,
    };

    /// Swap width/height for landscape.
    pub fn landscape(self) -> Self {
        Self {
            width_mm: self.height_mm,
            height_mm: self.width_mm,
        }
    }
}

/// Plot style table (CTB/STB subset): named color remapping by ACI index.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PlotStyle {
    /// Name (e.g. `monochrome.ctb`).
    pub name: String,
    /// ACI 1..=255 → RGB override (`None` = use entity color).
    pub aci_rgb: Vec<Option<[u8; 3]>>,
    /// Lineweight override mm by ACI (`None` = default).
    pub aci_lw_mm: Vec<Option<f64>>,
}

impl PlotStyle {
    /// Empty table with 256 slots.
    pub fn empty(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            aci_rgb: vec![None; 256],
            aci_lw_mm: vec![None; 256],
        }
    }

    /// Monochrome: all pens → black, thin.
    pub fn monochrome() -> Self {
        let mut s = Self::empty("monochrome");
        for i in 1..256 {
            s.aci_rgb[i] = Some([0, 0, 0]);
            s.aci_lw_mm[i] = Some(0.25);
        }
        s
    }
}

/// Viewport on a layout mapping paper → model.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Viewport {
    /// Center on paper (mm).
    pub center_paper: Vec2,
    /// Width/height on paper (mm).
    pub size_paper: Vec2,
    /// Center in model space.
    pub center_model: Vec2,
    /// Scale: model units per paper mm.
    pub scale: f64,
    /// Locked against pan/zoom in UI.
    #[serde(default)]
    pub locked: bool,
    /// Display annotative objects at this scale (1 = 1:1).
    #[serde(default = "default_anno_scale")]
    pub annotation_scale: f64,
}

fn default_anno_scale() -> f64 {
    1.0
}

impl Viewport {
    /// Full-sheet viewport at scale.
    pub fn full_sheet(paper: PaperSize, center_model: Vec2, scale: f64) -> Self {
        Self {
            center_paper: Vec2::new(paper.width_mm * 0.5, paper.height_mm * 0.5),
            size_paper: Vec2::new(paper.width_mm * 0.9, paper.height_mm * 0.9),
            center_model,
            scale,
            locked: false,
            annotation_scale: scale.max(1e-9),
        }
    }
}

/// Named paper-space layout.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Layout {
    /// Id.
    pub id: LayoutId,
    /// Name (`Model` is reserved for model space conceptually; layouts are paper).
    pub name: String,
    /// Paper size.
    pub paper: PaperSize,
    /// Viewports.
    pub viewports: Vec<Viewport>,
    /// Optional plot style name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plot_style: Option<String>,
    /// Plot to PDF printable area margin mm.
    #[serde(default)]
    pub margin_mm: f64,
}

impl Layout {
    /// Create a named layout with one full-sheet viewport.
    pub fn with_viewport(id: LayoutId, name: impl Into<String>, paper: PaperSize, scale: f64) -> Self {
        Self {
            id,
            name: name.into(),
            paper,
            viewports: vec![Viewport::full_sheet(paper, Vec2::ZERO, scale)],
            plot_style: Some("monochrome".into()),
            margin_mm: 10.0,
        }
    }
}
