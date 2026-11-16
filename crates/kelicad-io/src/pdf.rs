//! PDF vector export via `printpdf` (MIT).

use std::io::BufWriter;

use kelicad_doc::Geometry;
use kelicad_ops::DocSession;
use printpdf::{BuiltinFont, Mm, PdfDocument, Point};

use crate::error::IoError;

/// Export a single-page PDF approximating model space on A4 landscape.
pub fn export_pdf(session: &DocSession) -> Result<Vec<u8>, IoError> {
    let (doc, page1, layer1) = PdfDocument::new("KELICAD", Mm(297.0), Mm(210.0), "Layer 1");
    let layer = doc.get_page(page1).get_layer(layer1);

    let (min_x, min_y, max_x, max_y) = content_bounds(session);
    let content_w = (max_x - min_x).max(1.0);
    let content_h = (max_y - min_y).max(1.0);
    let margin = 15.0_f32;
    let page_w = 297.0 - 2.0 * margin;
    let page_h = 210.0 - 2.0 * margin;
    let scale = (page_w / content_w as f32).min(page_h / content_h as f32);

    let map = |x: f64, y: f64| -> Point {
        let px = margin + (x as f32 - min_x as f32) * scale;
        let py = margin + (y as f32 - min_y as f32) * scale;
        Point::new(Mm(px), Mm(py))
    };

    for ent in session.document.entities() {
        match &ent.geometry {
            Geometry::Line(l) => {
                let points = vec![map(l.start.x, l.start.y), map(l.end.x, l.end.y)];
                layer.add_line(printpdf::Line {
                    points: points.into_iter().map(|p| (p, false)).collect(),
                    is_closed: false,
                });
            }
            Geometry::Polyline(p) if p.vertices.len() >= 2 => {
                let points: Vec<_> = p.vertices.iter().map(|v| (map(v.x, v.y), false)).collect();
                layer.add_line(printpdf::Line {
                    points,
                    is_closed: p.closed,
                });
            }
            Geometry::Circle(c) => {
                let mut points = Vec::new();
                const N: usize = 48;
                for i in 0..=N {
                    let a = (i as f64) / (N as f64) * std::f64::consts::TAU;
                    let pt = c.point_at_angle(a);
                    points.push((map(pt.x, pt.y), false));
                }
                layer.add_line(printpdf::Line {
                    points,
                    is_closed: true,
                });
            }
            Geometry::Dimension(d) => {
                layer.add_line(printpdf::Line {
                    points: vec![
                        (map(d.def1.x, d.def1.y), false),
                        (map(d.def2.x, d.def2.y), false),
                    ],
                    is_closed: false,
                });
            }
            Geometry::Hatch(h) if h.boundary.len() >= 2 => {
                let points: Vec<_> = h.boundary.iter().map(|v| (map(v.x, v.y), false)).collect();
                layer.add_line(printpdf::Line {
                    points,
                    is_closed: true,
                });
            }
            Geometry::Arc(a) => {
                let mut points = Vec::new();
                const N: usize = 32;
                for i in 0..=N {
                    let t = (i as f64) / (N as f64);
                    let pt = a.point_at(t);
                    points.push((map(pt.x, pt.y), false));
                }
                layer.add_line(printpdf::Line {
                    points,
                    is_closed: false,
                });
            }
            _ => {}
        }
    }

    let font = doc
        .add_builtin_font(BuiltinFont::Helvetica)
        .map_err(|e| IoError::Format(format!("{e}")))?;
    layer.use_text("KELICAD", 10.0, Mm(10.0), Mm(200.0), &font);

    let mut buf = Vec::new();
    {
        let mut writer = BufWriter::new(&mut buf);
        doc.save(&mut writer)
            .map_err(|e| IoError::Format(format!("{e}")))?;
    }
    Ok(buf)
}

fn content_bounds(session: &DocSession) -> (f64, f64, f64, f64) {
    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    let mut add = |x: f64, y: f64| {
        min_x = min_x.min(x);
        min_y = min_y.min(y);
        max_x = max_x.max(x);
        max_y = max_y.max(y);
    };
    for ent in session.document.entities() {
        match &ent.geometry {
            Geometry::Line(l) => {
                add(l.start.x, l.start.y);
                add(l.end.x, l.end.y);
            }
            Geometry::Circle(c) => {
                add(c.center.x - c.radius, c.center.y - c.radius);
                add(c.center.x + c.radius, c.center.y + c.radius);
            }
            Geometry::Polyline(p) => {
                for v in &p.vertices {
                    add(v.x, v.y);
                }
            }
            Geometry::Text(t) => add(t.position.x, t.position.y),
            Geometry::MText(t) => add(t.position.x, t.position.y),
            Geometry::Dimension(d) => {
                add(d.def1.x, d.def1.y);
                add(d.def2.x, d.def2.y);
            }
            Geometry::Hatch(h) => {
                for v in &h.boundary {
                    add(v.x, v.y);
                }
            }
            Geometry::Arc(a) => {
                add(a.center.x, a.center.y);
                add(a.start_point().x, a.start_point().y);
                add(a.end_point().x, a.end_point().y);
            }
            Geometry::Insert(i) => add(i.position.x, i.position.y),
            Geometry::Point(p) => add(p.x, p.y),
            Geometry::Ellipse(e) => {
                let bb = e.aabb();
                add(bb.min.x, bb.min.y);
                add(bb.max.x, bb.max.y);
            }
            Geometry::Spline(s) => {
                for v in &s.to_polyline(8).vertices {
                    add(v.x, v.y);
                }
            }
        }
    }
    if !min_x.is_finite() {
        (0.0, 0.0, 100.0, 100.0)
    } else {
        (min_x, min_y, max_x, max_y)
    }
}
