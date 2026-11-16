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
    let apex_i = positions.len() as u32;
    positions.push(Vec3::new(center_xy.x, center_xy.y, base_z + height));
    let mut indices = Vec::new();
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

/// Sweep a local XY profile along a 2D path.
///
/// At each path vertex the profile is rotated so +X follows the path tangent,
/// then extruded with a small Z thickness for a solid ribbon (2.5D).
pub fn sweep_xy_along_path(profile: &[Vec2], path: &[Vec2], thickness: f64) -> Mesh {
    if profile.len() < 3 || path.len() < 2 {
        return Mesh::empty();
    }
    let n = profile.len();
    let m = path.len();
    let thick = thickness.abs().max(1e-4);

    // Center profile on its centroid.
    let cx = profile.iter().map(|p| p.x).sum::<f64>() / n as f64;
    let cy = profile.iter().map(|p| p.y).sum::<f64>() / n as f64;
    let local: Vec<Vec2> = profile
        .iter()
        .map(|p| Vec2::new(p.x - cx, p.y - cy))
        .collect();

    // For each path node: bottom ring (z=0) then top ring (z=thick).
    let mut positions = Vec::with_capacity(n * m * 2);
    for k in 0..m {
        let tang = path_tangent(path, k);
        let ang = tang.angle();
        let (s, c) = ang.sin_cos();
        for &z in &[0.0, thick] {
            for p in &local {
                let rx = p.x * c - p.y * s;
                let ry = p.x * s + p.y * c;
                positions.push(Vec3::new(path[k].x + rx, path[k].y + ry, z));
            }
        }
    }

    let mut indices = Vec::new();
    let ring = n as u32;
    let station = 2 * ring;

    // Start caps
    cap_fan(&positions[..n], 0, true, &mut indices);
    cap_fan(&positions[n..2 * n], ring, false, &mut indices);

    for k in 0..m {
        let base = (k as u32) * station;
        let bot = base;
        let top = base + ring;
        // Vertical sides at station
        for i in 0..n {
            let j = ((i + 1) % n) as u32;
            let i = i as u32;
            indices.extend_from_slice(&[bot + i, bot + j, top + j, bot + i, top + j, top + i]);
        }
        if k + 1 < m {
            let next = base + station;
            for i in 0..n {
                let j = ((i + 1) % n) as u32;
                let i = i as u32;
                // Bottom strip
                indices.extend_from_slice(&[
                    bot + i,
                    bot + j,
                    next + j,
                    bot + i,
                    next + j,
                    next + i,
                ]);
                // Top strip
                let t0 = top;
                let t1 = next + ring;
                indices.extend_from_slice(&[t0 + i, t1 + i, t1 + j, t0 + i, t1 + j, t0 + j]);
            }
        }
    }

    // End caps
    let last = ((m - 1) as u32) * station;
    let last_bot = (m - 1) * 2 * n;
    cap_fan(
        &positions[last_bot..last_bot + n],
        last,
        false,
        &mut indices,
    );
    cap_fan(
        &positions[last_bot + n..last_bot + 2 * n],
        last + ring,
        true,
        &mut indices,
    );

    Mesh { positions, indices }
}

fn path_tangent(path: &[Vec2], k: usize) -> Vec2 {
    if k + 1 < path.len() {
        path[k + 1] - path[k]
    } else if k > 0 {
        path[k] - path[k - 1]
    } else {
        Vec2::X
    }
}

fn cap_fan(ring: &[Vec3], base: u32, flip: bool, indices: &mut Vec<u32>) {
    for tri in triangulate_polygon(ring) {
        if flip {
            indices.extend_from_slice(&[base + tri[0], base + tri[2], base + tri[1]]);
        } else {
            indices.extend_from_slice(&[base + tri[0], base + tri[1], base + tri[2]]);
        }
    }
}

/// Torus centered at `center`, major radius `R`, minor radius `r`.
pub fn torus_mesh(
    center: Vec3,
    major: f64,
    minor: f64,
    major_segs: usize,
    minor_segs: usize,
) -> Mesh {
    let major_segs = major_segs.max(8);
    let minor_segs = minor_segs.max(6);
    if major <= 0.0 || minor <= 0.0 {
        return Mesh::empty();
    }
    let mut positions = Vec::with_capacity((major_segs + 1) * (minor_segs + 1));
    for i in 0..=major_segs {
        let u = (i as f64) / (major_segs as f64) * std::f64::consts::TAU;
        let (su, cu) = u.sin_cos();
        for j in 0..=minor_segs {
            let v = (j as f64) / (minor_segs as f64) * std::f64::consts::TAU;
            let (sv, cv) = v.sin_cos();
            let x = (major + minor * cv) * cu;
            let y = (major + minor * cv) * su;
            let z = minor * sv;
            positions.push(Vec3::new(center.x + x, center.y + y, center.z + z));
        }
    }
    let mut indices = Vec::new();
    let stride = minor_segs + 1;
    for i in 0..major_segs {
        for j in 0..minor_segs {
            let a = (i * stride + j) as u32;
            let b = a + 1;
            let c = a + stride as u32;
            let d = c + 1;
            indices.extend_from_slice(&[a, c, d, a, d, b]);
        }
    }
    Mesh { positions, indices }
}

/// Regular n-gon pyramid: base in XY at z=0, apex above center.
pub fn pyramid_mesh(center_xy: Vec2, radius: f64, height: f64, sides: usize) -> Mesh {
    let n = sides.max(3);
    if radius <= 0.0 || height.abs() < 1e-12 {
        return Mesh::empty();
    }
    let mut positions = Vec::with_capacity(n + 1);
    for i in 0..n {
        let a = (i as f64) * std::f64::consts::TAU / (n as f64);
        positions.push(Vec3::new(
            center_xy.x + radius * a.cos(),
            center_xy.y + radius * a.sin(),
            0.0,
        ));
    }
    let apex = positions.len() as u32;
    positions.push(Vec3::new(center_xy.x, center_xy.y, height));
    let mut indices = Vec::new();
    for tri in triangulate_polygon(&positions[..n]) {
        indices.extend_from_slice(&[tri[0], tri[2], tri[1]]);
    }
    for i in 0..n {
        let j = ((i + 1) % n) as u32;
        indices.extend_from_slice(&[i as u32, j, apex]);
    }
    Mesh { positions, indices }
}

/// Regular prism: n-gon extruded along +Z.
pub fn prism_mesh(center_xy: Vec2, radius: f64, height: f64, sides: usize) -> Mesh {
    let n = sides.max(3);
    if radius <= 0.0 || height <= 0.0 {
        return Mesh::empty();
    }
    let mut profile = Vec::with_capacity(n);
    for i in 0..n {
        let a = (i as f64) * std::f64::consts::TAU / (n as f64);
        profile.push(Vec2::new(
            center_xy.x + radius * a.cos(),
            center_xy.y + radius * a.sin(),
        ));
    }
    extrude_xy_profile(&profile, height)
}

/// Loft between two XY profiles over `height` with `sections` intermediate steps.
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
    cap_fan(&positions[..n], 0, true, &mut indices);
    let top_base = (sections * n) as u32;
    cap_fan(&positions[sections * n..], top_base, false, &mut indices);
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
        let m = sweep_xy_along_path(&profile, &path, 0.1);
        assert!(m.triangle_count() > 0);
    }

    #[test]
    fn torus_nonempty() {
        let m = torus_mesh(Vec3::ZERO, 2.0, 0.4, 24, 12);
        assert!(m.triangle_count() > 100);
    }

    #[test]
    fn pyramid_hex() {
        let m = pyramid_mesh(Vec2::ZERO, 1.0, 2.0, 6);
        assert!(m.triangle_count() >= 12);
    }

    #[test]
    fn prism_square() {
        let m = prism_mesh(Vec2::ZERO, 1.0, 3.0, 4);
        assert!(m.triangle_count() > 0);
    }
}
