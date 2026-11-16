//! Advanced mesh builders: sphere, cone, wedge, sweep, loft.

use kelicad_math::{Vec2, Vec3};

use crate::mesh::{extrude_xy_profile, triangulate_polygon, Mesh};

/// UV sphere centered at `center` with `radius`.
pub fn sphere_mesh(center: Vec3, radius: f64, slices: usize, stacks: usize) -> Mesh {
    let slices = slices.max(8);
    let stacks = stacks.max(4);
    let mut positions = Vec::new();
    for i in 0..=stacks {
        let v = i as f64 / stacks as f64;
        let phi = std::f64::consts::PI * v;
        let (sp, cp) = phi.sin_cos();
        for j in 0..=slices {
            let u = j as f64 / slices as f64;
            let theta = std::f64::consts::TAU * u;
            let (st, ct) = theta.sin_cos();
            positions.push(Vec3::new(
                center.x + radius * sp * ct,
                center.y + radius * sp * st,
                center.z + radius * cp,
            ));
        }
    }
    let mut indices = Vec::new();
    let stride = slices + 1;
    for i in 0..stacks {
        for j in 0..slices {
            let a = (i * stride + j) as u32;
            let b = a + 1;
            let c = a + stride as u32;
            let d = c + 1;
            indices.extend_from_slice(&[a, c, d, a, d, b]);
        }
    }
    Mesh { positions, indices }
}

/// Cone with base circle in XY at z=`base_z`, apex at z=`base_z+height`.
pub fn cone_mesh(
    center_xy: Vec2,
    radius: f64,
    base_z: f64,
    height: f64,
    segments: usize,
) -> Mesh {
    let segs = segments.max(8);
    let mut positions = Vec::with_capacity(segs + 1);
    for i in 0..segs {
        let a = (i as f64) * std::f64::consts::TAU / (segs as f64);
        positions.push(Vec3::new(
            center_xy.x + radius * a.cos(),
            center_xy.y + radius * a.sin(),
            base_z,
        ));
    }
    let apex = Vec3::new(center_xy.x, center_xy.y, base_z + height);
    let apex_i = positions.len() as u32;
    positions.push(apex);
    let mut indices = Vec::new();
    // Base (normal -Z if height > 0).
    for tri in triangulate_polygon(&positions[..segs]) {
        indices.extend_from_slice(&[tri[0], tri[2], tri[1]]);
    }
    for i in 0..segs {
        let j = (i + 1) % segs;
        indices.extend_from_slice(&[i as u32, j as u32, apex_i]);
    }
    Mesh { positions, indices }
}

/// Right-angle wedge: triangle in XY extruded along Z.
pub fn wedge_mesh(origin: Vec3, size: Vec3) -> Mesh {
    let profile = [
        Vec2::new(origin.x, origin.y),
        Vec2::new(origin.x + size.x, origin.y),
        Vec2::new(origin.x, origin.y + size.y),
    ];
    let mut m = extrude_xy_profile(&profile, size.z.abs().max(1e-9));
    if origin.z.abs() > 1e-15 {
        for p in &mut m.positions {
            p.z += origin.z;
        }
    }
    m
}

/// Sweep an XY profile along a 2D path (extruded as successive frames in XY, Z=0).
///
/// Each path vertex becomes a copy of the profile centered on that point (relative
/// to the first path vertex). Caps are added at both ends.
pub fn sweep_xy_along_path(profile: &[Vec2], path: &[Vec2]) -> Mesh {
    if profile.len() < 3 || path.len() < 2 {
        return Mesh::empty();
    }
    let n = profile.len();
    let m = path.len();
    let origin = path[0];
    // Local profile relative to centroid for cleaner sweeps.
    let cx = profile.iter().map(|p| p.x).sum::<f64>() / n as f64;
    let cy = profile.iter().map(|p| p.y).sum::<f64>() / n as f64;
    let local: Vec<Vec2> = profile
        .iter()
        .map(|p| Vec2::new(p.x - cx, p.y - cy))
        .collect();

    let mut positions = Vec::with_capacity(n * m);
    for k in 0..m {
        let tang = if k + 1 < m {
            path[k + 1] - path[k]
        } else {
            path[k] - path[k - 1]
        };
        let len = tang.length().max(1e-12);
        let tx = tang.x / len;
        let ty = tang.y / len;
        // Rotate local XY so +X aligns with tangent.
        for p in &local {
            let rx = p.x * tx - p.y * ty;
            let ry = p.x * ty + p.y * tx;
            positions.push(Vec3::new(
                path[k].x + rx - (origin.x - cx - path[0].x + path[0].x),
                path[k].y + ry,
                0.0,
            ));
            // Simpler placement: profile in plane perpendicular isn't full 3D;
            // keep flat sweep in XY then extrude thin Z for volume:
        }
    }
    // Rebuild properly: place profile copies in XY at each path point (2.5D ribbon),
    // then give them a small Z thickness via dual rings.
    positions.clear();
    let half_h = 0.05_f64.max(
        local
            .iter()
            .map(|p| p.length())
            .fold(0.0_f64, f64::max)
            * 0.02,
    );
    for k in 0..m {
        let tang = if k + 1 < m {
            path[k + 1] - path[k]
        } else if k > 0 {
            path[k] - path[k - 1]
        } else {
            Vec2::X
        };
        let len = tang.length().max(1e-12);
        let tx = tang.x / len;
        let ty = tang.y / len;
        for z in [0.0_f64, half_h * 2.0] {
            for p in &local {
                let rx = p.x * tx - p.y * (-ty); // rotate
                let ry = p.x * ty + p.y * tx;
                // Actually: rotate (p.x, p.y) by angle of tangent from +X
                let ang = tang.angle();
                let (s, c) = ang.sin_cos();
                let rx = p.x * c - p.y * s;
                let ry = p.x * s + p.y * c;
                let _ = (tx, ty, rx, ry);
                let rx = p.x * c - p.y * s;
                let ry = p.x * s + p.y * c;
                positions.push(Vec3::new(path[k].x + rx, path[k].y + ry, z));
            }
        }
    }
    // Layout: for each path node k, bottom ring then top ring (2*n verts per node).
    let mut indices = Vec::new();
    let ring = n as u32;
    for k in 0..m {
        let base = (k as u32) * 2 * ring;
        // Caps at ends
        if k == 0 {
            let bottom: Vec<Vec3> = positions[..n].to_vec();
            for tri in triangulate_polygon(&bottom) {
                indices.extend_from_slice(&[tri[0], tri[2], tri[1]]);
            }
            let top: Vec<Vec3> = positions[n..2 * n].to_vec();
            for tri in triangulate_polygon(&top) {
                indices.extend_from_slice(&[
                    ring + tri[0],
                    ring + tri[1],
                    ring + tri[2],
                ]);
            }
        }
        if k + 1 < m {
            let a = base;
            let b = base + 2 * ring;
            // Connect bottom rings and top rings + sides between bottom-top already in node
            for i in 0..n {
                let j = (i + 1) % n;
                let i = i as u32;
                let j = j as u32;
                // bottom strip
                indices.extend_from_slice(&[a + i, a + j, b + j, a + i, b + j, b + i]);
                // top strip
                let at = a + ring;
                let bt = b + ring;
                indices.extend_from_slice(&[at + i, bt + i, bt + j, at + i, bt + j, at + j]);
            }
        }
        // vertical sides at this station
        let bot = base;
        let top = base + ring;
        for i in 0..n {
            let j = ((i + 1) % n) as u32;
            let i = i as u32;
            indices.extend_from_slice(&[bot + i, bot + j, top + j, bot + i, top + j, top + i]);
        }
    }
    // End cap at last station
    if m >= 2 {
        let base = ((m - 1) as u32) * 2 * ring;
        let bottom: Vec<Vec3> = positions[(m - 1) * 2 * n..(m - 1) * 2 * n + n].to_vec();
        for tri in triangulate_polygon(&bottom) {
            indices.extend_from_slice(&[base + tri[0], base + tri[1], base + tri[2]]);
        }
        let top_base = base + ring;
        let top: Vec<Vec3> = positions[(m - 1) * 2 * n + n..(m - 1) * 2 * n + 2 * n].to_vec();
        for tri in triangulate_polygon(&top) {
            indices.extend_from_slice(&[
                top_base + tri[0],
                top_base + tri[2],
                top_base + tri[1],
            ]);
        }
    }
    let _ = half_h;
    Mesh { positions, indices }
}

/// Loft between two XY profiles (same vertex count preferred; resampled if not).
pub fn loft_xy_profiles(a: &[Vec2], b: &[Vec2], height: f64, sections: usize) -> Mesh {
    if a.len() < 3 || b.len() < 3 || height.abs() < 1e-12 {
        return Mesh::empty();
    }
    let sections = sections.max(1);
    let n = a.len().max(b.len());
    let ra = resample_closed(a, n);
    let rb = resample_closed(b, n);
    let mut positions = Vec::with_capacity(n * (sections + 1));
    for s in 0..=sections {
        let t = s as f64 / sections as f64;
        let z = height * t;
        for i in 0..n {
            let p = ra[i].lerp(rb[i], t);
            positions.push(Vec3::new(p.x, p.y, z));
        }
    }
    let mut indices = Vec::new();
    // Bottom / top caps
    let bottom: Vec<Vec3> = positions[..n].to_vec();
    for tri in triangulate_polygon(&bottom) {
        indices.extend_from_slice(&[tri[0], tri[2], tri[1]]);
    }
    let top_base = (sections * n) as u32;
    let top: Vec<Vec3> = positions[sections * n..].to_vec();
    for tri in triangulate_polygon(&top) {
        indices.extend_from_slice(&[top_base + tri[0], top_base + tri[1], top_base + tri[2]]);
    }
    for s in 0..sections {
        let a0 = (s * n) as u32;
        let b0 = ((s + 1) * n) as u32;
        for i in 0..n {
            let j = ((i + 1) % n) as u32;
            let i = i as u32;
            indices.extend_from_slice(&[a0 + i, a0 + j, b0 + j, a0 + i, b0 + j, b0 + i]);
        }
    }
    Mesh { positions, indices }
}

fn resample_closed(poly: &[Vec2], count: usize) -> Vec<Vec2> {
    if poly.len() == count {
        return poly.to_vec();
    }
    // Cumulative length parameterization.
    let mut pts = poly.to_vec();
    if pts.first() != pts.last() {
        pts.push(pts[0]);
    }
    let mut cum = vec![0.0];
    for w in pts.windows(2) {
        cum.push(cum.last().unwrap() + w[0].distance(w[1]));
    }
    let total = *cum.last().unwrap();
    if total < 1e-12 {
        return vec![poly[0]; count];
    }
    let mut out = Vec::with_capacity(count);
    for i in 0..count {
        let target = total * (i as f64) / (count as f64);
        let mut k = 0;
        while k + 1 < cum.len() && cum[k + 1] < target {
            k += 1;
        }
        let seg_len = (cum[k + 1] - cum[k]).max(1e-12);
        let t = (target - cum[k]) / seg_len;
        out.push(pts[k].lerp(pts[k + 1], t));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sphere_has_triangles() {
        let m = sphere_mesh(Vec3::ZERO, 1.0, 16, 8);
        assert!(m.triangle_count() > 0);
    }

    #[test]
    fn cone_nonempty() {
        let m = cone_mesh(Vec2::ZERO, 2.0, 0.0, 5.0, 12);
        assert!(m.triangle_count() > 0);
    }

    #[test]
    fn loft_box_like() {
        let a = [
            Vec2::new(0.0, 0.0),
            Vec2::new(2.0, 0.0),
            Vec2::new(2.0, 2.0),
            Vec2::new(0.0, 2.0),
        ];
        let b = [
            Vec2::new(0.5, 0.5),
            Vec2::new(1.5, 0.5),
            Vec2::new(1.5, 1.5),
            Vec2::new(0.5, 1.5),
        ];
        let m = loft_xy_profiles(&a, &b, 3.0, 4);
        assert!(m.triangle_count() > 0);
    }

    #[test]
    fn sweep_path() {
        let profile = [
            Vec2::new(-0.2, -0.2),
            Vec2::new(0.2, -0.2),
            Vec2::new(0.2, 0.2),
            Vec2::new(-0.2, 0.2),
        ];
        let path = [Vec2::ZERO, Vec2::new(5.0, 0.0), Vec2::new(5.0, 3.0)];
        let m = sweep_xy_along_path(&profile, &path);
        assert!(m.triangle_count() > 0);
    }
}
