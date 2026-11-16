//! Text, dimensions, leaders, and hatches.

use kelicad_math::Vec2;

use crate::entity::EntityId;
use crate::styles::{DimStyleId, TextStyleId};

/// Horizontal/vertical text alignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TextAlign {
    /// Left / baseline.
    #[default]
    Left,
    /// Center.
    Center,
    /// Right.
    Right,
    /// Middle center (approx).
    Middle,
}

/// Single-line text entity payload.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct TextGeom {
    /// Insertion point.
    pub position: Vec2,
    /// Height in drawing units.
    pub height: f64,
    /// Rotation radians.
    pub rotation: f64,
    /// Content.
    pub content: String,
    /// Alignment.
    pub align: TextAlign,
    /// Text style (`None` = document current / Standard).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style_id: Option<TextStyleId>,
    /// Annotative flag (viewport-scaled).
    #[serde(default)]
    pub annotative: bool,
}

/// Multi-line text (DXF MTEXT-compatible payload).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct MTextGeom {
    /// Insertion point.
    pub position: Vec2,
    /// Height.
    pub height: f64,
    /// Rotation.
    pub rotation: f64,
    /// Content (newlines and DXF codes `\P`, `\H`, `\L`, … allowed).
    pub content: String,
    /// Reference width (0 = unconstrained).
    pub width: f64,
    /// Width factor (character aspect); 1.0 = default.
    #[serde(default = "default_width_factor")]
    pub width_factor: f64,
    /// Line spacing factor (1.0 = single).
    #[serde(default = "default_line_spacing")]
    pub line_spacing: f64,
    /// Attachment point.
    #[serde(default)]
    pub align: crate::mtext::MTextAlign,
    /// Text style.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style_id: Option<TextStyleId>,
    /// Annotative flag.
    #[serde(default)]
    pub annotative: bool,
}

fn default_width_factor() -> f64 {
    1.0
}
fn default_line_spacing() -> f64 {
    1.0
}

impl MTextGeom {
    /// Simple multi-line text at `position`.
    pub fn new(position: Vec2, height: f64, content: impl Into<String>) -> Self {
        Self {
            position,
            height,
            rotation: 0.0,
            content: content.into(),
            width: 0.0,
            width_factor: 1.0,
            line_spacing: 1.0,
            align: crate::mtext::MTextAlign::TopLeft,
            style_id: None,
            annotative: false,
        }
    }

    /// Plain lines after expanding control codes.
    pub fn plain_lines(&self) -> Vec<String> {
        crate::mtext::parse_mtext(&self.content).0
    }

    /// Styled runs after expanding control codes.
    pub fn runs(&self) -> Vec<crate::mtext::MTextRun> {
        crate::mtext::parse_mtext(&self.content).1
    }
}

/// Dimension kind for M2.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DimKind {
    /// Linear (horizontal or rotated).
    Linear,
    /// Aligned to the measured segment.
    Aligned,
    /// Radial.
    Radius,
    /// Diametric.
    Diameter,
    /// Angular.
    Angular,
}

/// Associative dimension. Measurement is always derived from geometry by the
/// core (never by the LLM).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct DimensionGeom {
    /// Dimension type.
    pub kind: DimKind,
    /// Definition point 1 (e.g. start of measured segment).
    pub def1: Vec2,
    /// Definition point 2.
    pub def2: Vec2,
    /// Dimension line location (third point / offset).
    pub dim_line: Vec2,
    /// Optional entity associations for associativity.
    pub assoc_a: Option<EntityId>,
    /// Optional second associated entity.
    pub assoc_b: Option<EntityId>,
    /// Override text (`None` = auto measurement string).
    pub text_override: Option<String>,
    /// Dimension style.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style_id: Option<DimStyleId>,
    /// Annotative flag.
    #[serde(default)]
    pub annotative: bool,
    /// Angular: third ray point (vertex is def1; rays through def2 and angle_p3).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub angle_p3: Option<Vec2>,
}

impl DimensionGeom {
    /// Measured value for linear/aligned (distance def1–def2).
    pub fn measured_length(&self) -> f64 {
        self.def1.distance(self.def2)
    }

    /// Measured angle in degrees (Angular) or 0.
    pub fn measured_angle_deg(&self) -> f64 {
        let Some(p3) = self.angle_p3 else {
            return (self.def2 - self.def1).angle().to_degrees().abs();
        };
        let a = (self.def2 - self.def1).angle();
        let b = (p3 - self.def1).angle();
        let mut d = (b - a).to_degrees().abs();
        if d > 180.0 {
            d = 360.0 - d;
        }
        d
    }

    /// Auto text for the dimension (without style formatting).
    pub fn auto_text(&self) -> String {
        match self.kind {
            DimKind::Linear | DimKind::Aligned => format!("{:.3}", self.measured_length()),
            DimKind::Radius => format!("R{:.3}", self.measured_length()),
            DimKind::Diameter => format!("⌀{:.3}", self.measured_length()),
            DimKind::Angular => format!("{:.2}°", self.measured_angle_deg()),
        }
    }

    /// Display text, optionally applying a dim style formatter.
    pub fn display_text(&self) -> String {
        self.text_override
            .clone()
            .unwrap_or_else(|| self.auto_text())
    }

    /// Display text using a dim style (precision / suffix / factor).
    pub fn display_text_styled(&self, style: &crate::styles::DimStyle) -> String {
        if let Some(t) = &self.text_override {
            return t.clone();
        }
        match self.kind {
            DimKind::Linear | DimKind::Aligned => style.format_length(self.measured_length()),
            DimKind::Radius => format!("R{}", style.format_length(self.measured_length())),
            DimKind::Diameter => format!("⌀{}", style.format_length(self.measured_length())),
            DimKind::Angular => format!("{:.prec$}°", self.measured_angle_deg(), prec = style.precision as usize),
        }
    }
}

/// Multileader / leader annotation.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct LeaderGeom {
    /// Polyline vertices from arrow tip to landing (at least 2).
    pub vertices: Vec<Vec2>,
    /// Annotation text (empty = leader only).
    pub text: String,
    /// Text height.
    pub text_height: f64,
    /// Has arrowhead at first vertex.
    pub has_arrow: bool,
    /// Text style.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style_id: Option<TextStyleId>,
}

impl LeaderGeom {
    /// Simple two-point leader with text.
    pub fn new(tip: Vec2, landing: Vec2, text: impl Into<String>) -> Self {
        Self {
            vertices: vec![tip, landing],
            text: text.into(),
            text_height: 2.5,
            has_arrow: true,
            style_id: None,
        }
    }
}

/// Hatch pattern kind (M2: solid + named placeholder).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HatchPattern {
    /// Solid fill.
    Solid,
    /// Named pattern (ANSI31, etc.) — geometry approximated as boundary only in M2 render.
    Named(String),
}

/// Hatch region defined by a closed polyline boundary (+ optional islands later).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct HatchGeom {
    /// Outer boundary vertices (closed implied).
    pub boundary: Vec<Vec2>,
    /// Pattern.
    pub pattern: HatchPattern,
    /// Pattern scale (1.0 = as defined in PAT).
    #[serde(default = "default_hatch_scale")]
    pub scale: f64,
    /// Pattern rotation in degrees.
    #[serde(default)]
    pub angle: f64,
}

fn default_hatch_scale() -> f64 {
    1.0
}

impl HatchGeom {
    /// Solid hatch with default scale/angle.
    pub fn solid(boundary: Vec<Vec2>) -> Self {
        Self {
            boundary,
            pattern: HatchPattern::Solid,
            scale: 1.0,
            angle: 0.0,
        }
    }

    /// Named PAT pattern hatch.
    pub fn named(boundary: Vec<Vec2>, name: impl Into<String>, scale: f64, angle: f64) -> Self {
        Self {
            boundary,
            pattern: HatchPattern::Named(name.into()),
            scale,
            angle,
        }
    }
}
