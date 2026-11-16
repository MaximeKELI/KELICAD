//! Minimal PDF exporter (no printpdf) — WASM-safe line drawings.

use kelicad_doc::Geometry;
use kelicad_ops::DocSession;

use crate::error::IoError;

/// Export a single-page A4 landscape PDF of model-space linework.
pub fn export_pdf_lite(session: &DocSession) -> Result<Vec<u8>, IoError> {
    let (min_x, min_y, max_x, max_y) = content_bounds(session);
    let content_w = (max_x - min_x).max(1.0);
    let content_h = (max_y - min_y).max(1.0);
    let page_w = 841.89_f64; // A4 landscape pt
    let page_h = 595.28_f64;
    let margin = 36.0;
    let usable_w = page_w - 2.0 * margin;
    let usable_h = page_h - 2.0 * margin;
    let scale = (usable_w / content_w).min(usable_h / content_h);

    let mut stream = String::from("0.2 w\n");
    let map = |x: f64, y: f64| -> (f64, f64) {
        (
            margin + (x - min_x) * scale,
            margin + (y - min_y) * scale,
        )
    };
    let mut line = |x0: f64, y0: f64, x1: f64, y1: f64| {
        let (a, b) = map(x0, y0);
        let (c, d) = map(x1, y1);
        stream.push_str(&format!("{a:.2} {b:.2} m {c:.2} {d:.2} l S\n"));
    };

    for ent in session.document.entities() {
        match &ent.geometry {
            Geometry::Line(l) => line(l.start.x, l.start.y, l.end.x, l.end.y),
            Geometry::Polyline(p) => {
                for seg in p.segments() {
                    line(seg.start.x, seg.start.y, seg.end.x, seg.end.y);
                }
            }
            Geometry::Circle(c) => {
                const N: usize = 48;
                for i in 0..N {
                    let a0 = (i as f64) / (N as f64) * std::f64::consts::TAU;
                    let a1 = ((i + 1) as f64) / (N as f64) * std::f64::consts::TAU;
                    let p0 = c.point_at_angle(a0);
                    let p1 = c.point_at_angle(a1);
                    line(p0.x, p0.y, p1.x, p1.y);
                }
            }
            Geometry::Arc(a) => {
                const N: usize = 32;
                for i in 0..N {
                    let t0 = (i as f64) / (N as f64);
                    let t1 = ((i + 1) as f64) / (N as f64);
                    let p0 = a.point_at(t0);
                    let p1 = a.point_at(t1);
                    line(p0.x, p0.y, p1.x, p1.y);
                }
            }
            Geometry::Dimension(d) => {
                line(d.def1.x, d.def1.y, d.def2.x, d.def2.y);
                line(d.def1.x, d.def1.y, d.dim_line.x, d.dim_line.y);
                line(d.def2.x, d.def2.y, d.dim_line.x, d.dim_line.y);
            }
            Geometry::Hatch(h) if h.boundary.len() >= 2 => {
                for w in h.boundary.windows(2) {
                    line(w[0].x, w[0].y, w[1].x, w[1].y);
                }
                if let (Some(a), Some(b)) = (h.boundary.first(), h.boundary.last()) {
                    if a.distance(*b) > 1e-9 {
                        line(b.x, b.y, a.x, a.y);
                    }
                }
            }
            Geometry::Text(t) => {
                let s = t.height.max(0.2);
                line(t.position.x, t.position.y, t.position.x + s * t.content.len().max(1) as f64 * 0.6, t.position.y);
                line(t.position.x, t.position.y, t.position.x, t.position.y + s);
            }
            Geometry::Ellipse(e) => {
                let poly = e.to_polyline(48);
                for seg in poly.segments() {
                    line(seg.start.x, seg.start.y, seg.end.x, seg.end.y);
                }
            }
            Geometry::Spline(s) => {
                let poly = s.to_polyline(10);
                for seg in poly.segments() {
                    line(seg.start.x, seg.start.y, seg.end.x, seg.end.y);
                }
            }
            _ => {}
        }
    }

    build_pdf(page_w, page_h, &stream)
}

fn content_bounds(session: &DocSession) -> (f64, f64, f64, f64) {
    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    let mut any = false;
    for ent in session.document.entities() {
        let pts: Vec<(f64, f64)> = match &ent.geometry {
            Geometry::Line(l) => vec![(l.start.x, l.start.y), (l.end.x, l.end.y)],
            Geometry::Polyline(p) => p.vertices.iter().map(|v| (v.x, v.y)).collect(),
            Geometry::Circle(c) => vec![
                (c.center.x - c.radius, c.center.y - c.radius),
                (c.center.x + c.radius, c.center.y + c.radius),
            ],
            Geometry::Point(p) => vec![(p.x, p.y)],
            Geometry::Text(t) => vec![(t.position.x, t.position.y)],
            Geometry::Dimension(d) => {
                vec![
                    (d.def1.x, d.def1.y),
                    (d.def2.x, d.def2.y),
                    (d.dim_line.x, d.dim_line.y),
                ]
            }
            Geometry::Hatch(h) => h.boundary.iter().map(|v| (v.x, v.y)).collect(),
            _ => vec![],
        };
        for (x, y) in pts {
            any = true;
            min_x = min_x.min(x);
            min_y = min_y.min(y);
            max_x = max_x.max(x);
            max_y = max_y.max(y);
        }
    }
    if !any {
        (0.0, 0.0, 10.0, 10.0)
    } else {
        (min_x, min_y, max_x, max_y)
    }
}

fn build_pdf(page_w: f64, page_h: f64, content: &str) -> Result<Vec<u8>, IoError> {
    let stream_obj = format!("stream\n{content}\nendstream");
    let len = content.len();
    let objects = [
        "1 0 obj<< /Type /Catalog /Pages 2 0 R >>endobj\n".to_string(),
        "2 0 obj<< /Type /Pages /Kids [3 0 R] /Count 1 >>endobj\n".to_string(),
        format!(
            "3 0 obj<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {page_w:.2} {page_h:.2}] /Contents 4 0 R /Resources<<>> >>endobj\n"
        ),
        format!("4 0 obj<< /Length {len} >>{stream_obj}\nendobj\n"),
    ];
    let mut out = String::from("%PDF-1.4\n");
    let mut offsets = Vec::new();
    for obj in &objects {
        offsets.push(out.len());
        out.push_str(obj);
    }
    let xref_pos = out.len();
    out.push_str(&format!("xref\n0 {}\n", objects.len() + 1));
    out.push_str("0000000000 65535 f \n");
    for off in offsets {
        out.push_str(&format!("{off:010} 00000 n \n"));
    }
    out.push_str(&format!(
        "trailer<< /Size {} /Root 1 0 R >>\nstartxref\n{xref_pos}\n%%EOF\n",
        objects.len() + 1
    ));
    Ok(out.into_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;
    use kelicad_doc::{EntityId, EntityRecord, Geometry, LayerId};
    use kelicad_geom::Line;
    use kelicad_math::Vec2;
    use kelicad_ops::Operation;
    use uuid::Uuid;

    #[test]
    fn pdf_starts_with_header() {
        let mut s = DocSession::new();
        let id = EntityId::from_uuid(Uuid::nil());
        let layer = LayerId::from_uuid(Uuid::nil());
        let _ = s.apply(Operation::CreateEntity {
            record: EntityRecord {
                id,
                kind: Geometry::Line(Line::new(Vec2::ZERO, Vec2::X)).kind(),
                geometry: Geometry::Line(Line::new(Vec2::ZERO, Vec2::new(5.0, 3.0))),
                layer_id: layer,
                props: Default::default(),
        },
        });
        let bytes = export_pdf_lite(&s).unwrap();
        assert!(bytes.starts_with(b"%PDF"));
        assert!(bytes.len() > 100);
    }
}
