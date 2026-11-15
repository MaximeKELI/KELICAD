//! BIM element types.

use std::collections::HashMap;

use kelicad_solid::SolidId;
use uuid::Uuid;

/// BIM element id.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct BimId(pub Uuid);

impl BimId {
    /// New id.
    pub fn new() -> Self {
        Self(Uuid::now_v7())
    }
}

impl Default for BimId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for BimId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

/// BIM product kind (IFC-aligned subset).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BimKind {
    /// IfcWall.
    Wall,
    /// IfcSlab.
    Slab,
    /// IfcDoor.
    Door,
    /// IfcWindow.
    Window,
    /// IfcStair.
    Stair,
}

impl BimKind {
    /// IFC type name.
    pub fn ifc_type(self) -> &'static str {
        match self {
            Self::Wall => "IFCWALL",
            Self::Slab => "IFCSLAB",
            Self::Door => "IFCDOOR",
            Self::Window => "IFCWINDOW",
            Self::Stair => "IFCSTAIR",
        }
    }

    /// Parse from IFC type token.
    pub fn from_ifc_type(s: &str) -> Option<Self> {
        match s.to_ascii_uppercase().as_str() {
            "IFCWALL" | "IFCWALLSTANDARDCASE" => Some(Self::Wall),
            "IFCSLAB" => Some(Self::Slab),
            "IFCDOOR" => Some(Self::Door),
            "IFCWINDOW" => Some(Self::Window),
            "IFCSTAIR" => Some(Self::Stair),
            _ => None,
        }
    }
}

/// A BIM element referencing a solid body.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct BimElement {
    /// Id.
    pub id: BimId,
    /// Kind.
    pub kind: BimKind,
    /// Linked solid.
    pub solid_id: SolidId,
    /// Free-form properties (tag, material, height, …).
    pub props: HashMap<String, String>,
}

impl BimElement {
    /// Construct with empty props.
    pub fn new(kind: BimKind, solid_id: SolidId) -> Self {
        Self {
            id: BimId::new(),
            kind,
            solid_id,
            props: HashMap::new(),
        }
    }

    /// Set a property.
    pub fn with_prop(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.props.insert(key.into(), value.into());
        self
    }
}
