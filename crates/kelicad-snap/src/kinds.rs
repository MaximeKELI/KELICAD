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
}

impl SnapKind {
    /// Default priority (lower = preferred when distances tie).
    pub fn priority(self) -> u8 {
        match self {
            Self::Endpoint => 0,
            Self::Intersection => 1,
            Self::Midpoint => 2,
            Self::Center => 3,
            Self::Quadrant => 4,
            Self::Perpendicular => 5,
            Self::Nearest => 6,
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
        }
    }
}
