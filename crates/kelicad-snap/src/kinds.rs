//! Snap kind flags.

/// Object snap type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SnapKind {
    /// Endpoint of a segment / polyline / arc.
    Endpoint,
    /// Midpoint of a segment.
    Midpoint,
    /// Center of circle / arc.
    Center,
    /// Intersection of two curves.
    Intersection,
    /// Nearest point on geometry.
    Nearest,
    /// Perpendicular foot from a base point.
    Perpendicular,
    /// Quadrant of a circle (0°/90°/180°/270°).
    Quadrant,
    /// Point entity / node.
    Node,
    /// Tangent on circle/arc from base.
    Tangent,
    /// Extension of a segment beyond endpoint.
    Extension,
    /// Block insert point.
    Insertion,
    /// Parallel to a segment through cursor (guide).
    Parallel,
}

impl SnapKind {
    /// Default priority (lower = preferred when distances tie).
    pub fn priority(self) -> u8 {
        match self {
            Self::Endpoint => 0,
            Self::Intersection => 1,
            Self::Midpoint => 2,
            Self::Center => 3,
            Self::Insertion => 4,
            Self::Node => 5,
            Self::Quadrant => 6,
            Self::Perpendicular => 7,
            Self::Tangent => 8,
            Self::Extension => 9,
            Self::Parallel => 10,
            Self::Nearest => 11,
        }
    }
}

/// Enabled snap modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SnapModes {
    /// Endpoint.
    pub endpoint: bool,
    /// Midpoint.
    pub midpoint: bool,
    /// Center.
    pub center: bool,
    /// Intersection.
    pub intersection: bool,
    /// Nearest.
    pub nearest: bool,
    /// Perpendicular.
    pub perpendicular: bool,
    /// Quadrant.
    pub quadrant: bool,
    /// Node / point.
    pub node: bool,
    /// Tangent.
    pub tangent: bool,
    /// Extension.
    pub extension: bool,
    /// Insertion.
    pub insertion: bool,
    /// Parallel.
    pub parallel: bool,
}

impl Default for SnapModes {
    fn default() -> Self {
        Self {
            endpoint: true,
            midpoint: true,
            center: true,
            intersection: true,
            nearest: true,
            perpendicular: true,
            quadrant: true,
            node: true,
            tangent: true,
            extension: true,
            insertion: true,
            parallel: false,
        }
    }
}

impl SnapModes {
    /// Build from AutoCAD-like OSMODE bitfield.
    /// 1 End, 2 Mid, 4 Cen, 8 Node, 16 Quad, 32 Int, 64 Ins, 128 Perp,
    /// 256 Tan, 512 Near, 1024 Ext, 2048 Par.
    pub fn from_osmode(bits: i64) -> Self {
        Self {
            endpoint: bits & 1 != 0,
            midpoint: bits & 2 != 0,
            center: bits & 4 != 0,
            node: bits & 8 != 0,
            quadrant: bits & 16 != 0,
            intersection: bits & 32 != 0,
            insertion: bits & 64 != 0,
            perpendicular: bits & 128 != 0,
            tangent: bits & 256 != 0,
            nearest: bits & 512 != 0,
            extension: bits & 1024 != 0,
            parallel: bits & 2048 != 0,
        }
    }

    /// Encode to OSMODE bitfield.
    pub fn to_osmode(self) -> i64 {
        let mut b = 0i64;
        if self.endpoint {
            b |= 1;
        }
        if self.midpoint {
            b |= 2;
        }
        if self.center {
            b |= 4;
        }
        if self.node {
            b |= 8;
        }
        if self.quadrant {
            b |= 16;
        }
        if self.intersection {
            b |= 32;
        }
        if self.insertion {
            b |= 64;
        }
        if self.perpendicular {
            b |= 128;
        }
        if self.tangent {
            b |= 256;
        }
        if self.nearest {
            b |= 512;
        }
        if self.extension {
            b |= 1024;
        }
        if self.parallel {
            b |= 2048;
        }
        b
    }
}
