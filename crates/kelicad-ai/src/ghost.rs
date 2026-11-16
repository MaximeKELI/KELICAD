//! Ghost preview helpers for proposed geometries.

use kelicad_doc::Geometry;
use kelicad_geom::{Arc, Circle};
use kelicad_math::Vec2;

/// Append interleaved line-segment vertices (x0,y0,x1,y1,…) for GPU preview.
pub fn append_geometry_lines(geom: &Geometry, out: &mut Vec<f32>) {
    match geom {
        Geometry::Point(p) => {
            let s = 0.05;
            push_seg(out, *p + Vec2::new(-s, 0.0), *p + Vec2::new(s, 0.0));
            push_seg(out, *p + Vec2::new(0.0, -s), *p + Vec2::new(0.0, s));
        }
        Geometry::Line(l) => push_seg(out, l.start, l.end),
        Geometry::Circle(c) => tessellate_circle(out, *c, 32),
        Geometry::Arc(a) => tessellate_arc(out, *a, 24),
        Geometry::Polyline(p) => {
            for i in 0..p.segment_count() {
                if let Some(seg) = p.segment(i) {
                    push_seg(out, seg.start, seg.end);
                }
            }
        }
        Geometry::Ellipse(e) => {
            let poly = e.to_polyline(48);
            for i in 0..poly.segment_count() {
                if let Some(seg) = poly.segment(i) {
                    push_seg(out, seg.start, seg.end);
                }
            }
        }
        Geometry::Spline(s) => {
            let poly = s.to_polyline(8);
            for i in 0..poly.segment_count() {
                if let Some(seg) = poly.segment(i) {
                    push_seg(out, seg.start, seg.end);
                }
            }
        }
        Geometry::Text(t) => {
            let w = t.height * t.content.len().max(1) as f64 * 0.6;
            let h = t.height;
            let o = t.position;
            push_seg(out, o, o + Vec2::new(w, 0.0));
            push_seg(out, o + Vec2::new(w, 0.0), o + Vec2::new(w, h));
            push_seg(out, o + Vec2::new(w, h), o + Vec2::new(0.0, h));
            push_seg(out, o + Vec2::new(0.0, h), o);
        }
        Geometry::MText(t) => {
            let w = if t.width > 0.0 {
                t.width
            } else {
                t.height * 8.0
            };
            let h = t.height * 2.0;
            let o = t.position;
            push_seg(out, o, o + Vec2::new(w, 0.0));
            push_seg(out, o + Vec2::new(w, 0.0), o + Vec2::new(w, -h));
            push_seg(out, o + Vec2::new(w, -h), o + Vec2::new(0.0, -h));
            push_seg(out, o + Vec2::new(0.0, -h), o);
        }
        Geometry::Dimension(d) => {
            push_seg(out, d.def1, d.def2);
            push_seg(out, d.def1, d.dim_line);
            push_seg(out, d.def2, d.dim_line);
        }
        Geometry::Hatch(h) => {
            if h.boundary.len() >= 2 {
                for w in h.boundary.windows(2) {
                    push_seg(out, w[0], w[1]);
                }
                if let (Some(first), Some(last)) = (h.boundary.first(), h.boundary.last()) {
                    push_seg(out, *last, *first);
                }
            }
        }
        Geometry::Insert(i) => {
            let s = 0.25 * i.scale.x.max(i.scale.y);
            let o = i.position;
            push_seg(out, o + Vec2::new(-s, -s), o + Vec2::new(s, -s));
            push_seg(out, o + Vec2::new(s, -s), o + Vec2::new(s, s));
            push_seg(out, o + Vec2::new(s, s), o + Vec2::new(-s, s));
            push_seg(out, o + Vec2::new(-s, s), o + Vec2::new(-s, -s));
        }
    }
}

/// Flatten ghost list to vertices.
pub fn ghosts_to_vertices(ghosts: &[Geometry]) -> Vec<f32> {
    let mut out = Vec::new();
    for g in ghosts {
        append_geometry_lines(g, &mut out);
    }
    out
}

fn push_seg(out: &mut Vec<f32>, a: Vec2, b: Vec2) {
    out.push(a.x as f32);
    out.push(a.y as f32);
    out.push(b.x as f32);
    out.push(b.y as f32);
}

fn tessellate_circle(out: &mut Vec<f32>, c: Circle, n: usize) {
    let n = n.max(8);
    for i in 0..n {
        let a0 = std::f64::consts::TAU * (i as f64) / (n as f64);
        let a1 = std::f64::consts::TAU * ((i + 1) as f64) / (n as f64);
        push_seg(out, c.point_at_angle(a0), c.point_at_angle(a1));
    }
}

fn tessellate_arc(out: &mut Vec<f32>, a: Arc, n: usize) {
    let n = n.max(4);
    for i in 0..n {
        let t0 = i as f64 / n as f64;
        let t1 = (i + 1) as f64 / n as f64;
        push_seg(out, a.point_at(t0), a.point_at(t1));
    }
}
