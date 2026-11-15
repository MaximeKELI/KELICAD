//! Text, dimensions, and hatches.

use kelicad_math::Vec2;

use crate::entity::EntityId;

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
}

/// Multi-line text (simplified: stored as one string with `\n`).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct MTextGeom {
    /// Insertion point.
    pub position: Vec2,
    /// Height.
    pub height: f64,
    /// Rotation.
    pub rotation: f64,
    /// Content (newlines allowed).
    pub content: String,
    /// Reference width (0 = unconstrained).
    pub width: f64,
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
}

impl DimensionGeom {
    /// Measured value for linear/aligned (distance def1–def2).
    pub fn measured_length(&self) -> f64 {
        self.def1.distance(self.def2)
    }

    /// Auto text for the dimension.
    pub fn auto_text(&self) -> String {
        match self.kind {
            DimKind::Linear | DimKind::Aligned => format!("{:.3}", self.measured_length()),
            DimKind::Radius => format!("R{:.3}", self.measured_length()),
            DimKind::Diameter => format!("⌀{:.3}", self.measured_length()),
            DimKind::Angular => {
                let ang = (self.def2 - self.def1).angle().to_degrees().abs();
                format!("{ang:.2}°")
            }
        }
    }

    /// Display text.
    pub fn display_text(&self) -> String {
        self.text_override
            .clone()
            .unwrap_or_else(|| self.auto_text())
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
}
