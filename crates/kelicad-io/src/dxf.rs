//! Native DXF R12-oriented import/export (original implementation).

use std::collections::HashMap;
use std::fmt::Write;

use kelicad_doc::{
    DimKind, DimensionGeom, Document, EntityId, EntityRecord, Geometry, HatchGeom, HatchPattern,
    TextAlign, TextGeom,
};
use kelicad_geom::{Arc, Circle, Ellipse, Line, Polyline, Spline};
use kelicad_math::Vec2;
use kelicad_ops::{DocSession, Operation};
use uuid::Uuid;

use crate::error::IoError;

/// Report produced by DXF import.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ImportReport {
    /// Entities successfully imported.
    pub imported: usize,
    /// Entities skipped with reasons.
    pub skipped: Vec<String>,
    /// Warnings (approximations).
    pub warnings: Vec<String>,
}

/// Export the document model space to DXF (ASCII).
pub fn export_dxf(session: &DocSession) -> Result<String, IoError> {
    let mut out = String::new();
    writeln!(
        out,
        "0\nSECTION\n2\nHEADER\n9\n$ACADVER\n1\nAC1009\n0\nENDSEC"
    )
    .map_err(|e| IoError::Format(e.to_string()))?;
    writeln!(out, "0\nSECTION\n2\nTABLES\n0\nTABLE\n2\nLAYER").map_err(io_fmt)?;
    for layer in session.document.layers() {
        writeln!(
            out,
            "0\nLAYER\n2\n{}\n70\n0\n62\n7\n6\n{}",
            sanitize(&layer.name),
            sanitize(&layer.linetype)
        )
        .map_err(io_fmt)?;
    }
    writeln!(out, "0\nENDTAB\n0\nENDSEC").map_err(io_fmt)?;

    // Blocks
    writeln!(out, "0\nSECTION\n2\nBLOCKS").map_err(io_fmt)?;
    for block in session.document.blocks() {
        writeln!(
            out,
            "0\nBLOCK\n2\n{}\n70\n0\n10\n{}\n20\n{}\n30\n0.0",
            sanitize(&block.name),
            block.origin.x,
            block.origin.y
        )
        .map_err(io_fmt)?;
        for g in &block.entities {
            write_entity(&mut out, g, "0", &session.document)?;
        }
        writeln!(out, "0\nENDBLK").map_err(io_fmt)?;
    }
    writeln!(out, "0\nENDSEC").map_err(io_fmt)?;

    writeln!(out, "0\nSECTION\n2\nENTITIES").map_err(io_fmt)?;
    for ent in session.document.entities() {
        let layer = session
            .document
            .get_layer(ent.layer_id)
            .map(|l| l.name.as_str())
            .unwrap_or("0");
        write_entity(&mut out, &ent.geometry, layer, &session.document)?;
    }
    writeln!(out, "0\nENDSEC\n0\nEOF").map_err(io_fmt)?;
    Ok(out)
}

fn io_fmt(e: std::fmt::Error) -> IoError {
    IoError::Format(e.to_string())
}

fn sanitize(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

fn write_entity(
    out: &mut String,
    g: &Geometry,
    layer: &str,
    doc: &Document,
) -> Result<(), IoError> {
    match g {
        Geometry::Line(l) => {
            writeln!(
                out,
                "0\nLINE\n8\n{layer}\n10\n{}\n20\n{}\n30\n0.0\n11\n{}\n21\n{}\n31\n0.0",
                l.start.x, l.start.y, l.end.x, l.end.y
            )
            .map_err(io_fmt)?;
        }
        Geometry::Circle(c) => {
            writeln!(
                out,
                "0\nCIRCLE\n8\n{layer}\n10\n{}\n20\n{}\n30\n0.0\n40\n{}",
                c.center.x, c.center.y, c.radius
            )
            .map_err(io_fmt)?;
        }
        Geometry::Arc(a) => {
            let a0 = a.start_angle.to_degrees();
            let a1 = a.end_angle.to_degrees();
            writeln!(
                out,
                "0\nARC\n8\n{layer}\n10\n{}\n20\n{}\n30\n0.0\n40\n{}\n50\n{}\n51\n{}",
                a.center.x, a.center.y, a.radius, a0, a1
            )
            .map_err(io_fmt)?;
        }
        Geometry::Polyline(p) => {
            writeln!(
                out,
                "0\nLWPOLYLINE\n8\n{layer}\n90\n{}\n70\n{}",
                p.vertices.len(),
                if p.closed { 1 } else { 0 }
            )
            .map_err(io_fmt)?;
            for v in &p.vertices {
                writeln!(out, "10\n{}\n20\n{}", v.x, v.y).map_err(io_fmt)?;
            }
        }
        Geometry::Text(t) => {
            writeln!(
                out,
                "0\nTEXT\n8\n{layer}\n10\n{}\n20\n{}\n30\n0.0\n40\n{}\n1\n{}\n50\n{}",
                t.position.x,
                t.position.y,
                t.height,
                t.content.replace('\n', " "),
                t.rotation.to_degrees()
            )
            .map_err(io_fmt)?;
        }
        Geometry::MText(t) => {
            writeln!(
                out,
                "0\nMTEXT\n8\n{layer}\n10\n{}\n20\n{}\n30\n0.0\n40\n{}\n1\n{}",
                t.position.x,
                t.position.y,
                t.height,
                t.content.replace('\n', "\\P")
            )
            .map_err(io_fmt)?;
        }
        Geometry::Dimension(d) => {
            writeln!(
                out,
                "0\nDIMENSION\n8\n{layer}\n10\n{}\n20\n{}\n30\n0.0\n13\n{}\n23\n{}\n14\n{}\n24\n{}\n1\n{}",
                d.dim_line.x,
                d.dim_line.y,
                d.def1.x,
                d.def1.y,
                d.def2.x,
                d.def2.y,
                d.display_text()
            )
            .map_err(io_fmt)?;
        }
        Geometry::Hatch(h) => {
            writeln!(
                out,
                "0\nHATCH\n8\n{layer}\n70\n1\n91\n1\n92\n1\n93\n{}",
                h.boundary.len()
            )
            .map_err(io_fmt)?;
            for v in &h.boundary {
                writeln!(out, "10\n{}\n20\n{}", v.x, v.y).map_err(io_fmt)?;
            }
            let pat = match &h.pattern {
                HatchPattern::Solid => "SOLID",
                HatchPattern::Named(n) => n.as_str(),
            };
            writeln!(out, "75\n0\n76\n1\n2\n{pat}").map_err(io_fmt)?;
        }
        Geometry::Insert(i) => {
            let name = doc
                .get_block(i.block_id)
                .map(|b| sanitize(&b.name))
                .unwrap_or_else(|| "UNKNOWN".into());
            writeln!(
                out,
                "0\nINSERT\n8\n{layer}\n2\n{name}\n10\n{}\n20\n{}\n30\n0.0\n41\n{}\n42\n{}\n50\n{}",
                i.position.x,
                i.position.y,
                i.scale.x,
                i.scale.y,
                i.rotation.to_degrees()
            )
            .map_err(io_fmt)?;
        }
        Geometry::Point(p) => {
            writeln!(
                out,
                "0\nPOINT\n8\n{layer}\n10\n{}\n20\n{}\n30\n0.0",
                p.x, p.y
            )
            .map_err(io_fmt)?;
        }
        Geometry::Ellipse(e) => {
            // DXF groups 11/21: major-axis endpoint relative to center.
            writeln!(
                out,
                "0\nELLIPSE\n8\n{layer}\n10\n{}\n20\n{}\n30\n0.0\n11\n{}\n21\n{}\n31\n0.0\n40\n{}\n41\n{}\n42\n{}",
                e.center.x,
                e.center.y,
                e.major.x,
                e.major.y,
                e.ratio,
                e.start_angle,
                if e.is_full(kelicad_math::Tolerance::DEFAULT) {
                    std::f64::consts::TAU
                } else {
                    e.end_angle
                }
            )
            .map_err(io_fmt)?;
        }
        Geometry::Spline(s) => {
            writeln!(
                out,
                "0\nSPLINE\n8\n{layer}\n70\n{}\n71\n{}\n73\n{}",
                if s.closed { 1 } else { 0 },
                s.degree,
                s.control_points.len()
            )
            .map_err(io_fmt)?;
            for p in &s.control_points {
                writeln!(out, "10\n{}\n20\n{}\n30\n0.0", p.x, p.y).map_err(io_fmt)?;
            }
        }
        Geometry::Leader(l) => {
            writeln!(out, "0\nLEADER\n8\n{layer}\n71\n1\n76\n{}", l.vertices.len())
                .map_err(io_fmt)?;
            for v in &l.vertices {
                writeln!(out, "10\n{}\n20\n{}\n30\n0.0", v.x, v.y).map_err(io_fmt)?;
            }
            if !l.text.is_empty() {
                writeln!(out, "0\nTEXT\n8\n{layer}\n10\n{}\n20\n{}\n30\n0.0\n40\n{}\n1\n{}",
                    l.vertices.last().map(|v| v.x).unwrap_or(0.0),
                    l.vertices.last().map(|v| v.y).unwrap_or(0.0),
                    l.text_height,
                    l.text.replace('\n', " "),
                ).map_err(io_fmt)?;
            }
        }
        Geometry::Ray(r) => {
            writeln!(
                out,
                "0\nRAY\n8\n{layer}\n10\n{}\n20\n{}\n30\n0.0\n11\n{}\n21\n{}\n31\n0.0",
                r.origin.x, r.origin.y, r.direction.x, r.direction.y
            )
            .map_err(io_fmt)?;
        }
        Geometry::XLine(x) => {
            writeln!(
                out,
                "0\nXLINE\n8\n{layer}\n10\n{}\n20\n{}\n30\n0.0\n11\n{}\n21\n{}\n31\n0.0",
                x.point.x, x.point.y, x.direction.x, x.direction.y
            )
            .map_err(io_fmt)?;
        }
        Geometry::Xref(x) => {
            // Export as INSERT-like POINT marker (full XREF table later).
            writeln!(
                out,
                "0\nPOINT\n8\n{layer}\n10\n{}\n20\n{}\n30\n0.0",
                x.position.x, x.position.y
            )
            .map_err(io_fmt)?;
        }
    }
    Ok(())
}

/// Import DXF ASCII into a new session.
pub fn import_dxf(text: &str) -> Result<(DocSession, ImportReport), IoError> {
    let pairs = parse_pairs(text)?;
    let mut report = ImportReport::default();
    let mut session = DocSession::new();
    let mut i = 0usize;
    let mut in_entities = false;
    let mut next_id: u128 = 1000;

    while i < pairs.len() {
        let (code, val) = &pairs[i];
        if *code == 0 && val == "SECTION" {
            if let Some((_, name)) = pairs.get(i + 1).filter(|(c, _)| *c == 2) {
                in_entities = name == "ENTITIES";
                i += 2;
                continue;
            }
        }
        if *code == 0 && val == "ENDSEC" {
            in_entities = false;
            i += 1;
            continue;
        }
        if !in_entities || *code != 0 {
            i += 1;
            continue;
        }

        let entity_type = val.as_str();
        let start = i;
        i += 1;
        while i < pairs.len() && pairs[i].0 != 0 {
            i += 1;
        }
        let chunk = &pairs[start..i];
        match read_entity(entity_type, chunk, &mut report) {
            Some(geometry) => {
                let id = EntityId::from_uuid(Uuid::from_u128(next_id));
                next_id += 1;
                let layer_id = session.document.default_layer_id();
                let record = EntityRecord {
                    id,
                    kind: geometry.kind(),
                    geometry,
                    layer_id,
                };
                if let Err(e) = session.apply(Operation::CreateEntity { record }) {
                    report.skipped.push(format!("{entity_type}: {e}"));
                } else {
                    report.imported += 1;
                }
            }
            None => {
                if entity_type != "ENDSEC" && entity_type != "EOF" {
                    // already recorded in read_entity or unknown
                }
            }
        }
    }

    // Clear journal so import isn't one giant undo chain for the user.
    session.journal = kelicad_ops::OpJournal::new();
    Ok((session, report))
}

fn parse_pairs(text: &str) -> Result<Vec<(i32, String)>, IoError> {
    let mut lines = text.lines().map(|l| l.trim_end().to_string());
    let mut pairs = Vec::new();
    while let Some(code_s) = lines.next() {
        if code_s.trim().is_empty() {
            continue;
        }
        let code: i32 = code_s
            .trim()
            .parse()
            .map_err(|_| IoError::Format(format!("bad group code '{code_s}'")))?;
        let val = lines
            .next()
            .ok_or_else(|| IoError::Format("truncated DXF pair".into()))?;
        pairs.push((code, val));
    }
    Ok(pairs)
}

fn group_map(chunk: &[(i32, String)]) -> HashMap<i32, Vec<String>> {
    let mut m: HashMap<i32, Vec<String>> = HashMap::new();
    for (c, v) in chunk.iter().skip(1) {
        m.entry(*c).or_default().push(v.clone());
    }
    m
}

fn f(m: &HashMap<i32, Vec<String>>, code: i32, idx: usize) -> Option<f64> {
    m.get(&code)?.get(idx)?.parse().ok()
}

fn read_entity(ty: &str, chunk: &[(i32, String)], report: &mut ImportReport) -> Option<Geometry> {
    let m = group_map(chunk);
    match ty {
        "LINE" => {
            let x0 = f(&m, 10, 0)?;
            let y0 = f(&m, 20, 0)?;
            let x1 = f(&m, 11, 0)?;
            let y1 = f(&m, 21, 0)?;
            Some(Geometry::Line(Line::new(
                Vec2::new(x0, y0),
                Vec2::new(x1, y1),
            )))
        }
        "CIRCLE" => {
            let cx = f(&m, 10, 0)?;
            let cy = f(&m, 20, 0)?;
            let r = f(&m, 40, 0)?;
            Some(Geometry::Circle(Circle::new(Vec2::new(cx, cy), r)))
        }
        "ARC" => {
            let cx = f(&m, 10, 0)?;
            let cy = f(&m, 20, 0)?;
            let r = f(&m, 40, 0)?;
            let a0 = f(&m, 50, 0)?.to_radians();
            let a1 = f(&m, 51, 0)?.to_radians();
            Some(Geometry::Arc(Arc::new(Vec2::new(cx, cy), r, a0, a1)))
        }
        "LWPOLYLINE" => {
            let xs = m.get(&10).cloned().unwrap_or_default();
            let ys = m.get(&20).cloned().unwrap_or_default();
            let mut verts = Vec::new();
            for (x, y) in xs.iter().zip(ys.iter()) {
                if let (Ok(xv), Ok(yv)) = (x.parse::<f64>(), y.parse::<f64>()) {
                    verts.push(Vec2::new(xv, yv));
                }
            }
            let closed = m
                .get(&70)
                .and_then(|v| v.first())
                .and_then(|s| s.parse::<i32>().ok())
                .map(|f| f & 1 == 1)
                .unwrap_or(false);
            if verts.len() < 2 {
                report.skipped.push("LWPOLYLINE: too few vertices".into());
                return None;
            }
            Some(Geometry::Polyline(if closed {
                Polyline::closed(verts)
            } else {
                Polyline::new(verts)
            }))
        }
        "TEXT" => {
            let x = f(&m, 10, 0)?;
            let y = f(&m, 20, 0)?;
            let h = f(&m, 40, 0).unwrap_or(2.5);
            let content = m
                .get(&1)
                .and_then(|v| v.first())
                .cloned()
                .unwrap_or_default();
            let rot = f(&m, 50, 0).unwrap_or(0.0).to_radians();
            Some(Geometry::Text(TextGeom {
                position: Vec2::new(x, y),
                height: h,
                rotation: rot,
                content,
                align: TextAlign::Left,
                style_id: None,
                annotative: false,
}))
        }
        "MTEXT" => {
            let x = f(&m, 10, 0)?;
            let y = f(&m, 20, 0)?;
            let h = f(&m, 40, 0).unwrap_or(2.5);
            let content = m
                .get(&1)
                .and_then(|v| v.first())
                .cloned()
                .unwrap_or_default()
                .replace("\\P", "\n");
            let mut mt = kelicad_doc::MTextGeom::new(Vec2::new(x, y), h, content);
            mt.width = f(&m, 41, 0).unwrap_or(0.0);
            Some(Geometry::MText(mt))
        }
        "DIMENSION" => {
            let dx = f(&m, 10, 0).unwrap_or(0.0);
            let dy = f(&m, 20, 0).unwrap_or(0.0);
            let x1 = f(&m, 13, 0).unwrap_or(0.0);
            let y1 = f(&m, 23, 0).unwrap_or(0.0);
            let x2 = f(&m, 14, 0).unwrap_or(0.0);
            let y2 = f(&m, 24, 0).unwrap_or(0.0);
            let text = m.get(&1).and_then(|v| v.first()).cloned();
            let text_override = text.filter(|t| !t.is_empty() && t != "<>");
            Some(Geometry::Dimension(DimensionGeom {
                kind: DimKind::Aligned,
                def1: Vec2::new(x1, y1),
                def2: Vec2::new(x2, y2),
                dim_line: Vec2::new(dx, dy),
                assoc_a: None,
                assoc_b: None,
                text_override,
                style_id: None,
                annotative: false,
                angle_p3: None,
}))
        }
        "ELLIPSE" => {
            let cx = f(&m, 10, 0)?;
            let cy = f(&m, 20, 0)?;
            let mx = f(&m, 11, 0)?;
            let my = f(&m, 21, 0)?;
            let ratio = f(&m, 40, 0).unwrap_or(1.0);
            let a0 = f(&m, 41, 0).unwrap_or(0.0);
            let a1 = f(&m, 42, 0).unwrap_or(std::f64::consts::TAU);
            let mut e = Ellipse {
                center: Vec2::new(cx, cy),
                major: Vec2::new(mx, my),
                ratio: ratio.clamp(1e-12, 1.0),
                start_angle: 0.0,
                end_angle: 0.0,
            };
            if (a1 - a0).abs() < std::f64::consts::TAU - 1e-6 {
                e.start_angle = a0;
                e.end_angle = a1;
            }
            Some(Geometry::Ellipse(e))
        }
        "SPLINE" => {
            let xs = m.get(&10).cloned().unwrap_or_default();
            let ys = m.get(&20).cloned().unwrap_or_default();
            let mut pts = Vec::new();
            for (x, y) in xs.iter().zip(ys.iter()) {
                if let (Ok(xv), Ok(yv)) = (x.parse::<f64>(), y.parse::<f64>()) {
                    pts.push(Vec2::new(xv, yv));
                }
            }
            let degree = m
                .get(&71)
                .and_then(|v| v.first())
                .and_then(|s| s.parse().ok())
                .unwrap_or(3u8);
            let closed = m
                .get(&70)
                .and_then(|v| v.first())
                .and_then(|s| s.parse::<i32>().ok())
                .map(|f| f & 1 == 1)
                .unwrap_or(false);
            let spline = Spline::from_controls(pts, degree, closed).or_else(|| {
                report.skipped.push("SPLINE: too few control points".into());
                None
            })?;
            Some(Geometry::Spline(spline))
        }
        "HATCH" => {
            let xs = m.get(&10).cloned().unwrap_or_default();
            let ys = m.get(&20).cloned().unwrap_or_default();
            let mut boundary = Vec::new();
            for (x, y) in xs.iter().zip(ys.iter()) {
                if let (Ok(xv), Ok(yv)) = (x.parse::<f64>(), y.parse::<f64>()) {
                    boundary.push(Vec2::new(xv, yv));
                }
            }
            if boundary.len() < 3 {
                report.skipped.push("HATCH: incomplete boundary".into());
                return None;
            }
            let pat = m
                .get(&2)
                .and_then(|v| v.first())
                .cloned()
                .unwrap_or_else(|| "SOLID".into());
            report
                .warnings
                .push("HATCH: boundary-only approximation".into());
            Some(Geometry::Hatch(HatchGeom {
                boundary,
                pattern: if pat.eq_ignore_ascii_case("SOLID") {
                    HatchPattern::Solid
                } else {
                    HatchPattern::Named(pat)
                },
                scale: 1.0,
                angle: 0.0,
                islands: vec![],
                associative: false,
            }))
        }
        "POINT" => {
            let x = f(&m, 10, 0)?;
            let y = f(&m, 20, 0)?;
            Some(Geometry::Point(Vec2::new(x, y)))
        }
        "INSERT" => {
            report
                .skipped
                .push("INSERT: block table not fully wired on import M2".into());
            // Still import as a point marker so round-trip count isn't silent loss without report.
            let x = f(&m, 10, 0).unwrap_or(0.0);
            let y = f(&m, 20, 0).unwrap_or(0.0);
            report
                .warnings
                .push(format!("INSERT at ({x},{y}) imported as POINT placeholder"));
            Some(Geometry::Point(Vec2::new(x, y)))
        }
        other => {
            report.skipped.push(format!("{other}: unsupported entity"));
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kelicad_doc::{EntityKind, EntityRecord};
    use kelicad_geom::Line;

    #[test]
    fn round_trip_line_circle() {
        let mut session = DocSession::new();
        session
            .apply(Operation::CreateEntity {
                record: EntityRecord {
                    id: EntityId::from_uuid(Uuid::from_u128(1)),
                    kind: EntityKind::Line,
                    geometry: Geometry::Line(Line::new(Vec2::ZERO, Vec2::new(10.0, 0.0))),
                    layer_id: session.document.default_layer_id(),
                },
            })
            .unwrap();
        session
            .apply(Operation::CreateEntity {
                record: EntityRecord {
                    id: EntityId::from_uuid(Uuid::from_u128(2)),
                    kind: EntityKind::Circle,
                    geometry: Geometry::Circle(Circle::new(Vec2::new(5.0, 5.0), 2.0)),
                    layer_id: session.document.default_layer_id(),
                },
            })
            .unwrap();
        let dxf = export_dxf(&session).unwrap();
        let (loaded, report) = import_dxf(&dxf).unwrap();
        assert_eq!(report.imported, 2);
        assert_eq!(loaded.document.entity_count(), 2);
    }

    #[test]
    fn corpus_line_circle_file() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../formats/corpus/dxf/line_circle.dxf"
        );
        let text = std::fs::read_to_string(path).expect("corpus file");
        let (session, report) = import_dxf(&text).unwrap();
        assert_eq!(report.imported, 2);
        assert_eq!(session.document.entity_count(), 2);
        let again = export_dxf(&session).unwrap();
        let (session2, report2) = import_dxf(&again).unwrap();
        assert_eq!(report2.imported, 2);
        assert_eq!(session2.document.entity_count(), 2);
    }

    #[test]
    fn corpus_polyline_text_file() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../formats/corpus/dxf/polyline_text.dxf"
        );
        let text = std::fs::read_to_string(path).unwrap();
        let (session, report) = import_dxf(&text).unwrap();
        assert_eq!(report.imported, 2);
        assert!(session
            .document
            .entities()
            .any(|e| matches!(e.geometry, Geometry::Text(_))));
    }

    #[test]
    fn svg_and_pdf_smoke() {
        let mut session = DocSession::new();
        session
            .apply(Operation::CreateEntity {
                record: EntityRecord {
                    id: EntityId::from_uuid(Uuid::from_u128(9)),
                    kind: EntityKind::Line,
                    geometry: Geometry::Line(Line::new(Vec2::ZERO, Vec2::new(5.0, 5.0))),
                    layer_id: session.document.default_layer_id(),
                },
            })
            .unwrap();
        let svg = crate::export_svg(&session, 800.0, 600.0).unwrap();
        assert!(svg.contains("<svg"));
        let pdf = crate::export_pdf(&session).unwrap();
        assert!(pdf.starts_with(b"%PDF"));
    }
}
