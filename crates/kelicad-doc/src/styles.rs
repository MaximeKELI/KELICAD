//! Text and dimension styles (TEXTSTYLE / DIMSTYLE equivalents).

use uuid::Uuid;

/// Text style id.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct TextStyleId(Uuid);

impl TextStyleId {
    /// From UUID.
    pub const fn from_uuid(id: Uuid) -> Self {
        Self(id)
    }

    /// Underlying UUID.
    pub const fn as_uuid(self) -> Uuid {
        self.0
    }

    /// Built-in Standard style id (nil).
    pub const STANDARD: Self = Self(Uuid::nil());
}

impl std::fmt::Display for TextStyleId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

/// Named text style (DXF STYLE table subset).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct TextStyle {
    /// Id.
    pub id: TextStyleId,
    /// Name (unique, case-insensitive).
    pub name: String,
    /// Default height (0 = free).
    pub height: f64,
    /// Width factor.
    pub width_factor: f64,
    /// Oblique angle radians.
    pub oblique: f64,
    /// Font family hint (TTF/SHX name; rendering may substitute).
    pub font: String,
    /// Annotative: scale with viewport.
    pub annotative: bool,
}

impl TextStyle {
    /// AutoCAD-like Standard.
    pub fn standard() -> Self {
        Self {
            id: TextStyleId::STANDARD,
            name: "Standard".into(),
            height: 0.0,
            width_factor: 1.0,
            oblique: 0.0,
            font: "Arial".into(),
            annotative: false,
        }
    }
}

/// Dimension style id.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct DimStyleId(Uuid);

impl DimStyleId {
    /// From UUID.
    pub const fn from_uuid(id: Uuid) -> Self {
        Self(id)
    }

    /// Underlying UUID.
    pub const fn as_uuid(self) -> Uuid {
        self.0
    }

    /// Built-in Standard dimstyle (nil).
    pub const STANDARD: Self = Self(Uuid::nil());
}

impl std::fmt::Display for DimStyleId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

/// Arrow / tick kind for dimensions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DimArrowKind {
    /// Closed filled arrow.
    #[default]
    Arrow,
    /// Architectural tick.
    Tick,
    /// Dot.
    Dot,
    /// None.
    None,
}

/// Named dimension style (DXF DIMSTYLE subset).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct DimStyle {
    /// Id.
    pub id: DimStyleId,
    /// Name.
    pub name: String,
    /// Overall scale (DIMSCALE).
    pub scale: f64,
    /// Text height (DIMTXT).
    pub text_height: f64,
    /// Arrow size (DIMASZ).
    pub arrow_size: f64,
    /// Arrow kind.
    pub arrow: DimArrowKind,
    /// Extension line offset from origin (DIMEXO).
    pub ext_offset: f64,
    /// Extension beyond dim line (DIMEXE).
    pub ext_extend: f64,
    /// Decimal places.
    pub precision: u8,
    /// Linear scale factor (DIMLFAC).
    pub linear_factor: f64,
    /// Suffix (e.g. " mm").
    pub suffix: String,
    /// Linked text style.
    pub text_style: TextStyleId,
    /// Annotative.
    pub annotative: bool,
}

impl DimStyle {
    /// Default Standard dimstyle.
    pub fn standard() -> Self {
        Self {
            id: DimStyleId::STANDARD,
            name: "Standard".into(),
            scale: 1.0,
            text_height: 2.5,
            arrow_size: 2.5,
            arrow: DimArrowKind::Arrow,
            ext_offset: 1.0,
            ext_extend: 1.0,
            precision: 3,
            linear_factor: 1.0,
            suffix: String::new(),
            text_style: TextStyleId::STANDARD,
            annotative: false,
        }
    }

    /// Format a measured length using this style.
    pub fn format_length(&self, raw: f64) -> String {
        let v = raw * self.linear_factor;
        let p = self.precision as usize;
        format!("{v:.p$}{}", self.suffix)
    }
}
