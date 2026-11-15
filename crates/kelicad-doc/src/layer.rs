//! Drawing layers.

use uuid::Uuid;

/// Layer identifier (UUID v7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct LayerId(Uuid);

impl LayerId {
    /// From UUID.
    pub const fn from_uuid(id: Uuid) -> Self {
        Self(id)
    }

    /// Underlying UUID.
    pub const fn as_uuid(self) -> Uuid {
        self.0
    }
}

impl std::fmt::Display for LayerId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

/// Layer visibility / editability flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub struct LayerFlags {
    /// Frozen layers are not visible and not editable.
    pub frozen: bool,
    /// Locked layers are visible but not editable.
    pub locked: bool,
    /// Non-plottable layers are skipped on print/export.
    pub no_plot: bool,
}

/// A named layer with display attributes.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Layer {
    /// Stable id.
    pub id: LayerId,
    /// Display name (UI may localize later; stored as user string).
    pub name: String,
    /// RGB color (0–255 per channel).
    pub color: [u8; 3],
    /// Lineweight in hundredths of mm (CAD convention); 0 = default.
    pub lineweight: u16,
    /// Flags.
    pub flags: LayerFlags,
}

impl Layer {
    /// Create a visible, editable layer.
    pub fn new(id: LayerId, name: impl Into<String>, color: [u8; 3]) -> Self {
        Self {
            id,
            name: name.into(),
            color,
            lineweight: 0,
            flags: LayerFlags::default(),
        }
    }
}
