//! SVG vector export.

use std::fmt::Write;

use kelicad_doc::Geometry;
use kelicad_ops::DocSession;

use crate::error::IoError;

const STROKE: &str = "rgb(31,111,235)";
const FILL_SOFT: &str = "rgba(31,111,235,0.2)";
const TEXT_FILL: &str = "rgb(232,238,248)";

/// Export model space entities to a simple SVG document.
pub fn export_svg(session: &DocSession, width: f64, height: f64) -> Result<String, IoError> {
    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    for ent in session.document.entities() {
        expand_bounds(
            &ent.geometry,
            &mut min_x,
            &mut min_y,
            &mut max_x,
            &mut max_y,
        );
    }
    if !min_x.is_finite() {
        min_x = 0.0;
        min_y = 0.0;
        max_x = width;
        max_y = height;
    }
    let pad = ((max_x - min_x) + (max_y - min_y)) * 0.05 + 1.0;
    min_x -= pad;
    min_y -= pad;
    max_x += pad;
    max_y += pad;

    let vb_y = -max_y;
    let vb_w = max_x - min_x;
    let vb_h = max_y - min_y;

    let mut out = String::new();
    writeln!(
        out,
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{width}\" height=\"{height}\" \
viewBox=\"{min_x} {vb_y} {vb_w} {vb_h}\" fill=\"none\" stroke=\"{STROKE}\" stroke-width=\"0.05\">"
    )
    .map_err(|e| IoError::Format(e.to_string()))?;
    writeln!(out, "<g transform=\"scale(1,-1)\">").map_err(|e| IoError::Format(e.to_string()))?;

    for ent in session.document.entities() {
        write_geom(&mut out, &ent.geometry)?;
    }
    writeln!(out, "</g></svg>").map_err(|e| IoError::Format(e.to_string()))?;
    Ok(out)
}

fn expand_bounds(g: &Geometry, min_x: &mut f64, min_y: &mut f64, max_x: &mut f64, max_y: &mut f64) {
    let mut add = |x: f64, y: f64| {
        *min_x = min_x.min(x);
        *min_y = min_y.min(y);
        *max_x = max_x.max(x);
        *max_y = max_y.max(y);
    };
    match g {
        Geometry::Line(l) => {
            add(l.start.x, l.start.y);
            add(l.end.x, l.end.y);
        }
        Geometry::Circle(c) => {
            add(c.center.x - c.radius, c.center.y - c.radius);
            add(c.center.x + c.radius, c.center.y + c.radius);
        }
        Geometry::Arc(a) => {
            add(a.start_point().x, a.start_point().y);
            add(a.end_point().x, a.end_point().y);
            add(a.center.x, a.center.y);
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
            add(d.dim_line.x, d.dim_line.y);
        }
        Geometry::Hatch(h) => {
            for v in &h.boundary {
                add(v.x, v.y);
            }
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

fn write_geom(out: &mut String, g: &Geometry) -> Result<(), IoError> {
    match g {
        Geometry::Line(l) => {
            writeln!(
                out,
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" />",
                l.start.x, l.start.y, l.end.x, l.end.y
            )
            .map_err(|e| IoError::Format(e.to_string()))?;
        }
        Geometry::Circle(c) => {
            writeln!(
                out,
                "<circle cx=\"{}\" cy=\"{}\" r=\"{}\" />",
                c.center.x, c.center.y, c.radius
            )
            .map_err(|e| IoError::Format(e.to_string()))?;
        }
        Geometry::Arc(a) => {
            let p0 = a.start_point();
            let p1 = a.end_point();
            let large = if a.sweep_angle().abs() > std::f64::consts::PI {
                1
            } else {
                0
            };
            let sweep = if a.clockwise { 0 } else { 1 };
            writeln!(
                out,
                "<path d=\"M {} {} A {} {} 0 {} {} {} {}\" />",
                p0.x, p0.y, a.radius, a.radius, large, sweep, p1.x, p1.y
            )
            .map_err(|e| IoError::Format(e.to_string()))?;
        }
        Geometry::Polyline(p) => {
            let mut d = String::new();
            for (i, v) in p.vertices.iter().enumerate() {
                if i == 0 {
                    write!(d, "M {} {}", v.x, v.y).ok();
                } else {
                    write!(d, " L {} {}", v.x, v.y).ok();
                }
            }
            if p.closed {
                d.push_str(" Z");
            }
            writeln!(out, "<path d=\"{d}\" />").map_err(|e| IoError::Format(e.to_string()))?;
        }
        Geometry::Text(t) => {
            writeln!(
                out,
                "<text x=\"{}\" y=\"{}\" font-size=\"{}\" fill=\"{TEXT_FILL}\" stroke=\"none\">{}</text>",
                t.position.x,
                t.position.y,
                t.height,
                xml_escape(&t.content)
            )
            .map_err(|e| IoError::Format(e.to_string()))?;
        }
        Geometry::MText(t) => {
            writeln!(
                out,
                "<text x=\"{}\" y=\"{}\" font-size=\"{}\" fill=\"{TEXT_FILL}\" stroke=\"none\">{}</text>",
                t.position.x,
                t.position.y,
                t.height,
                xml_escape(&t.content)
            )
            .map_err(|e| IoError::Format(e.to_string()))?;
        }
        Geometry::Dimension(d) => {
            writeln!(
                out,
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" />",
                d.def1.x, d.def1.y, d.def2.x, d.def2.y
            )
            .map_err(|e| IoError::Format(e.to_string()))?;
            let mid = d.def1.lerp(d.def2, 0.5);
            writeln!(
                out,
                "<text x=\"{}\" y=\"{}\" font-size=\"0.35\" fill=\"{TEXT_FILL}\" stroke=\"none\">{}</text>",
                mid.x,
                mid.y,
                xml_escape(&d.display_text())
            )
            .map_err(|e| IoError::Format(e.to_string()))?;
        }
        Geometry::Hatch(h) => {
            let mut d = String::new();
            for (i, v) in h.boundary.iter().enumerate() {
                if i == 0 {
                    write!(d, "M {} {}", v.x, v.y).ok();
                } else {
                    write!(d, " L {} {}", v.x, v.y).ok();
                }
            }
            d.push_str(" Z");
            match &h.pattern {
                kelicad_doc::HatchPattern::Solid => {
                    writeln!(
                        out,
                        "<path d=\"{d}\" fill=\"{FILL_SOFT}\" stroke=\"{STROKE}\" />"
                    )
                    .map_err(|e| IoError::Format(e.to_string()))?;
                }
                kelicad_doc::HatchPattern::Named(name) => {
                    writeln!(
                        out,
                        "<path d=\"{d}\" fill=\"none\" stroke=\"{STROKE}\" />"
                    )
                    .map_err(|e| IoError::Format(e.to_string()))?;
                    if let Some(pat) = kelicad_geom::builtin_pattern(name) {
                        for seg in kelicad_geom::generate_hatch(
                            &h.boundary,
                            &pat,
                            h.scale,
                            h.angle,
                        ) {
                            writeln!(
                                out,
                                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" />",
                                seg.start.x, seg.start.y, seg.end.x, seg.end.y
                            )
                            .map_err(|e| IoError::Format(e.to_string()))?;
                        }
                    }
                }
            }
        }
        Geometry::Insert(i) => {
            writeln!(
                out,
                "<circle cx=\"{}\" cy=\"{}\" r=\"0.2\" fill=\"{STROKE}\" stroke=\"none\" />",
                i.position.x, i.position.y
            )
            .map_err(|e| IoError::Format(e.to_string()))?;
        }
        Geometry::Point(p) => {
            writeln!(
                out,
                "<circle cx=\"{}\" cy=\"{}\" r=\"0.1\" fill=\"{STROKE}\" stroke=\"none\" />",
                p.x, p.y
            )
            .map_err(|e| IoError::Format(e.to_string()))?;
        }
        Geometry::Ellipse(e) => {
            let poly = e.to_polyline(64);
            let mut d = String::new();
            for (i, v) in poly.vertices.iter().enumerate() {
                if i == 0 {
                    write!(d, "M {} {}", v.x, v.y).ok();
                } else {
                    write!(d, " L {} {}", v.x, v.y).ok();
                }
            }
            if poly.closed {
                d.push_str(" Z");
            }
            writeln!(out, "<path d=\"{d}\" />").map_err(|e| IoError::Format(e.to_string()))?;
        }
        Geometry::Spline(s) => {
            let poly = s.to_polyline(12);
            let mut d = String::new();
            for (i, v) in poly.vertices.iter().enumerate() {
                if i == 0 {
                    write!(d, "M {} {}", v.x, v.y).ok();
                } else {
                    write!(d, " L {} {}", v.x, v.y).ok();
                }
            }
            if poly.closed {
                d.push_str(" Z");
            }
            writeln!(out, "<path d=\"{d}\" />").map_err(|e| IoError::Format(e.to_string()))?;
        }
    }
    Ok(())
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}
