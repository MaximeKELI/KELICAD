//! PDF vector export via `printpdf` (MIT).

use kelicad_doc::Geometry;
use kelicad_ops::DocSession;
use printpdf::{BuiltinFont, Mm, PdfDocument, PdfLayerReference};

use crate::error::IoError;

/// Export a single-page PDF approximating model space on A4 landscape.
pub fn export_pdf(session: &DocSession) -> Result<Vec<u8>, IoError> {
    let (doc, page1, layer1) =
        PdfDocument::new("KELICAD", Mm(297.0), Mm(210.0), "Layer 1");
    let layer = doc.get_page(page1).get_layer(layer1);

    // Fit content into page with margin.
    let (min_x, min_y, max_x, max_y) = content_bounds(session);
    let content_w = (max_x - min_x).max(1.0);
    let content_h = (max_y - min_y).max(1.0);
    let margin = 15.0;
    let page_w = 297.0 - 2.0 * margin;
    let page_h = 210.0 - 2.0 * margin;
    let scale = (page_w / content_w).min(page_h / content_h);

    let map = |x: f64, y: f64| -> (Mm, Mm) {
        let px = margin + (x - min_x) * scale;
        let py = margin + (y - min_y) * scale;
        (Mm(px as f32), Mm(py as f32))
    };

    draw_entities(&layer, session, &map)?;

    let font = doc
        .add_builtin_font(BuiltinFont::Helvetica)
        .map_err(|e| IoError::Format(e.to_string()))?;
    layer.use_text("KELICAD", 10.0, Mm(10.0), Mm(200.0), &font);

    doc.save(&mut std::io::Cursor::new({
        let mut buf = Vec::new();
        // printpdf save API
        buf
    }))
    .ok();

    // Correct save path:
    let mut buf = Vec::new();
    {
        let mut cursor = std::io::Cursor::new(&mut buf);
        doc.save(&mut cursor)
            .map_err(|e| IoError::Format(e.to_string()))?;
    }
    let _ = font;
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
            }
            Geometry::Insert(i) => add(i.position.x, i.position.y),
            Geometry::Point(p) => add(p.x, p.y),
        }
    }
    if !min_x.is_finite() {
        (0.0, 0.0, 100.0, 100.0)
    } else {
        (min_x, min_y, max_x, max_y)
    }
}

fn draw_entities(
    layer: &PdfLayerReference,
    session: &DocSession,
    map: &dyn Fn(f64, f64) -> (Mm, Mm),
) -> Result<(), IoError> {
    use printpdf::Line as PdfLine;
    use printpdf::Point;

    for ent in session.document.entities() {
        if let Geometry::Line(l) = &ent.geometry {
            let (x0, y0) = map(l.start.x, l.start.y);
            let (x1, y1) = map(l.end.x, l.end.y);
            let line = PdfLine {
                points: vec![
                    (Point::new(x0, y0), false),
                    (Point::new(x1, y1), false),
                ],
                is_closed: false,
            };
            layer.add_line(line);
        }
        if let Geometry::Polyline(p) = &ent.geometry {
            if p.vertices.len() >= 2 {
                let points = p
                    .vertices
                    .iter()
                    .map(|v| {
                        let (x, y) = map(v.x, v.y);
                        (Point::new(x, y), false)
                    })
                    .collect();
                let line = PdfLine {
                    points,
                    is_closed: p.closed,
                };
                layer.add_line(line);
            }
        }
        if let Geometry::Circle(c) = &ent.geometry {
            // Approximate circle with polyline.
            let mut points = Vec::new();
            const N: usize = 48;
            for i in 0..=N {
                let a = (i as f64) / (N as f64) * std::f64::consts::TAU;
                let p = c.point_at_angle(a);
                let (x, y) = map(p.x, p.y);
                points.push((Point::new(x, y), false));
            }
            layer.add_line(PdfLine {
                points,
                is_closed: true,
            });
        }
        if let Geometry::Dimension(d) = &ent.geometry {
            let (x0, y0) = map(d.def1.x, d.def1.y);
            let (x1, y1) = map(d.def2.x, d.def2.y);
            layer.add_line(PdfLine {
                points: vec![
                    (Point::new(x0, y0), false),
                    (Point::new(x1, y1), false),
                ],
                is_closed: false,
            });
        }
    }
    Ok(())
}
