//! Helpers to build wall / slab solids + BIM elements.

use kelicad_math::{Vec2, Vec3};
use kelicad_solid::{BuiltinSolidKernel, SolidBody, SolidKernel};

use crate::element::{BimElement, BimKind};
use crate::error::BimError;

/// Wall from baseline segment in XY, thickness and height.
pub fn wall_from_baseline(
    start: Vec2,
    end: Vec2,
    thickness: f64,
    height: f64,
) -> Result<(SolidBody, BimElement), BimError> {
    if thickness <= 0.0 || height <= 0.0 {
        return Err(BimError::Invalid(
            "wall thickness/height must be > 0".into(),
        ));
    }
    let dir = end - start;
    let len = dir.length();
    if len < 1e-9 {
        return Err(BimError::Invalid("wall baseline is degenerate".into()));
    }
    let tang = dir / len;
    let n = Vec2::new(-tang.y, tang.x) * (thickness * 0.5);
    // Local box along X of length `len`, Y thickness, then place via mesh transform manually:
    // Build AABB-aligned box from start-n to end+n in XY.
    let corners = [
        start + n,
        end + n,
        end - n,
        start - n,
    ];
    let min_x = corners.iter().map(|p| p.x).fold(f64::INFINITY, f64::min);
    let max_x = corners.iter().map(|p| p.x).fold(f64::NEG_INFINITY, f64::max);
    let min_y = corners.iter().map(|p| p.y).fold(f64::INFINITY, f64::min);
    let max_y = corners.iter().map(|p| p.y).fold(f64::NEG_INFINITY, f64::max);
    // For non-axis-aligned walls, use extruded oriented profile.
    let profile = kelicad_solid::Profile2::from_vertices(corners.to_vec())?;
    let k = BuiltinSolidKernel;
    let body = k.extrude(&profile, height)?;
    let elem = BimElement::new(BimKind::Wall, body.id)
        .with_prop("tag", "W-01")
        .with_prop("height", format!("{height}"))
        .with_prop("thickness", format!("{thickness}"))
        .with_prop("length", format!("{len}"))
        .with_prop("material", "Concrete");
    let _ = (min_x, max_x, min_y, max_y); // kept for potential AABB asserts
    Ok((body, elem))
}

/// Slab as extruded rectangle.
pub fn slab_from_rect(
    origin: Vec2,
    width: f64,
    depth: f64,
    thickness: f64,
) -> Result<(SolidBody, BimElement), BimError> {
    let k = BuiltinSolidKernel;
    let profile = kelicad_solid::Profile2::rectangle(origin, width, depth)?;
    let body = k.extrude(&profile, thickness)?;
    let elem = BimElement::new(BimKind::Slab, body.id)
        .with_prop("tag", "S-01")
        .with_prop("thickness", format!("{thickness}"))
        .with_prop("material", "Concrete");
    Ok((body, elem))
}

/// Axis-aligned box door opening solid (for difference).
pub fn door_opening_box(
    min: Vec3,
    max: Vec3,
) -> Result<(SolidBody, BimElement), BimError> {
    let k = BuiltinSolidKernel;
    let body = k.box_solid(min, max)?;
    let elem = BimElement::new(BimKind::Door, body.id).with_prop("tag", "D-01");
    Ok((body, elem))
}
