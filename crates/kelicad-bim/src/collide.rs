//! AABB collision detection between solids.

use kelicad_solid::{SolidBody, SolidId};

/// Return pairs of solid ids whose AABBs overlap in volume.
pub fn collisions(bodies: &[SolidBody]) -> Vec<(SolidId, SolidId)> {
    let mut out = Vec::new();
    for i in 0..bodies.len() {
        for j in (i + 1)..bodies.len() {
            if bodies[i].aabb.overlaps_volume(bodies[j].aabb) {
                out.push((bodies[i].id, bodies[j].id));
            }
        }
    }
    out
}
