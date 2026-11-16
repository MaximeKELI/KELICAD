//! Named CAD linetypes (AutoCAD / LibreCAD-compatible dash patterns).

/// One linetype definition: repeating dash/gap lengths in drawing units.
///
/// Positive = drawn dash, negative = gap, zero = dot (rendered as a short dash).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Linetype {
    /// Canonical name (e.g. `DASHED`).
    pub name: String,
    /// Description.
    pub description: String,
    /// Pattern elements.
    pub pattern: Vec<f64>,
}

impl Linetype {
    /// Continuous solid line (empty pattern).
    pub fn continuous() -> Self {
        Self {
            name: "CONTINUOUS".into(),
            description: "Solid line".into(),
            pattern: vec![],
        }
    }

    /// Built-in ISO / AutoCAD-style set (MIT-authored lengths).
    pub fn builtins() -> Vec<Self> {
        vec![
            Self::continuous(),
            Self {
                name: "DASHED".into(),
                description: "Dashed __ __ __".into(),
                pattern: vec![0.5, -0.25],
            },
            Self {
                name: "DASHDOT".into(),
                description: "Dash dot __ . __ .".into(),
                pattern: vec![0.5, -0.2, 0.0, -0.2],
            },
            Self {
                name: "DOT".into(),
                description: "Dotted . . . .".into(),
                pattern: vec![0.0, -0.25],
            },
            Self {
                name: "CENTER".into(),
                description: "Center line ____ _ ____ _".into(),
                pattern: vec![1.25, -0.25, 0.25, -0.25],
            },
            Self {
                name: "HIDDEN".into(),
                description: "Hidden __ __ __".into(),
                pattern: vec![0.25, -0.125],
            },
            Self {
                name: "PHANTOM".into(),
                description: "Phantom ____ _ _ ____".into(),
                pattern: vec![1.25, -0.25, 0.25, -0.25, 0.25, -0.25],
            },
        ]
    }

    /// Look up a built-in by name (case-insensitive).
    pub fn builtin(name: &str) -> Option<Self> {
        let u = name.to_ascii_uppercase();
        Self::builtins().into_iter().find(|lt| lt.name == u)
    }

    /// True if this is a continuous (solid) line.
    pub fn is_continuous(&self) -> bool {
        self.pattern.is_empty()
            || self
                .pattern
                .iter()
                .all(|v| *v > 0.0 && self.pattern.len() == 1)
    }
}

/// Resolve a linetype name to a definition (builtin or continuous fallback).
pub fn resolve_linetype(name: &str) -> Linetype {
    Linetype::builtin(name).unwrap_or_else(Linetype::continuous)
}
