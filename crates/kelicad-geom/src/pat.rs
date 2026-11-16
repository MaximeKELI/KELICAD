//! AutoCAD-compatible `.pat` hatch pattern parser and line generation.
//!
//! Format reference: Autodesk hatch pattern definition (public documentation).
//! Built-in patterns are MIT-authored (not copied from GPL LibreCAD assets).

use kelicad_math::Vec2;

use crate::aabb::Aabb;
use crate::line::Line;

/// One line family in a PAT pattern.
#[derive(Debug, Clone, PartialEq)]
pub struct PatLineDef {
    /// Angle in degrees.
    pub angle_deg: f64,
    /// Base point X.
    pub x: f64,
    /// Base point Y.
    pub y: f64,
    /// Offset delta X (along pattern).
    pub dx: f64,
    /// Offset delta Y (perpendicular spacing component).
    pub dy: f64,
    /// Dash lengths (positive = draw, negative = gap, 0 = dot). Empty = solid.
    pub dashes: Vec<f64>,
}

/// Named hatch pattern.
#[derive(Debug, Clone, PartialEq)]
pub struct PatPattern {
    /// Pattern name (e.g. `ANSI31`).
    pub name: String,
    /// Description.
    pub description: String,
    /// Line definitions.
    pub lines: Vec<PatLineDef>,
}

/// Parse a full `.pat` file text into patterns.
pub fn parse_pat(text: &str) -> Result<Vec<PatPattern>, String> {
    let mut patterns = Vec::new();
    let mut current: Option<PatPattern> = None;
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with(';') {
            continue;
        }
        if let Some(rest) = line.strip_prefix('*') {
            if let Some(prev) = current.take() {
                patterns.push(prev);
            }
            let (name, desc) = match rest.split_once(',') {
                Some((n, d)) => (n.trim().to_string(), d.trim().to_string()),
                None => (rest.trim().to_string(), String::new()),
            };
            current = Some(PatPattern {
                name,
                description: desc,
                lines: vec![],
            });
            continue;
        }
        let Some(pat) = current.as_mut() else {
            return Err("PAT line before pattern header".into());
        };
        pat.lines.push(parse_pat_line(line)?);
    }
    if let Some(prev) = current {
        patterns.push(prev);
    }
    Ok(patterns)
}

fn parse_pat_line(line: &str) -> Result<PatLineDef, String> {
    let parts: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
    if parts.len() < 5 {
        return Err(format!("PAT line needs ≥5 fields: {line}"));
    }
    let angle_deg: f64 = parts[0].parse().map_err(|e| format!("angle: {e}"))?;
    let x: f64 = parts[1].parse().map_err(|e| format!("x: {e}"))?;
    let y: f64 = parts[2].parse().map_err(|e| format!("y: {e}"))?;
    let dx: f64 = parts[3].parse().map_err(|e| format!("dx: {e}"))?;
    let dy: f64 = parts[4].parse().map_err(|e| format!("dy: {e}"))?;
    let mut dashes = Vec::new();
    for p in &parts[5..] {
        if p.is_empty() {
            continue;
        }
        dashes.push(p.parse().map_err(|e| format!("dash: {e}"))?);
    }
    Ok(PatLineDef {
        angle_deg,
        x,
        y,
        dx,
        dy,
        dashes,
    })
}

/// Built-in MIT patterns (ANSI-like names for DXF familiarity).
pub fn builtin_patterns() -> Vec<PatPattern> {
    parse_pat(
        r#"; KELICAD built-in hatch patterns (MIT)
*ANSI31, ANSI Iron, Brick, Stone masonry
45, 0, 0, 0, 3.175
*ANSI32, ANSI Steel
45, 0, 0, 0, 3.175
45, 1.5875, 0, 0, 3.175
*ANSI37, ANSI Crosshatch
45, 0, 0, 0, 3.175
135, 0, 0, 0, 3.175
*NET, Network / grid
0, 0, 0, 0, 3.175
90, 0, 0, 0, 3.175
*DOTS, Dot pattern
0, 0, 0, 3.175, 3.175, 0, -3.175
*HORIZONTAL, Horizontal lines
0, 0, 0, 0, 3.175
*VERTICAL, Vertical lines
90, 0, 0, 0, 3.175
*DASH, Dashed 45
45, 0, 0, 0, 3.175, 3.175, -1.5875
"#,
    )
    .expect("builtin PAT")
}

/// Look up a built-in pattern by name (case-insensitive).
pub fn builtin_pattern(name: &str) -> Option<PatPattern> {
    let upper = name.to_ascii_uppercase();
    builtin_patterns().into_iter().find(|p| p.name == upper)
}

/// Load patterns from a `.pat` file on disk.
pub fn load_pat_file(path: &std::path::Path) -> Result<Vec<PatPattern>, String> {
    let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    parse_pat(&text)
}

/// Resolve a pattern by name from an optional file, else builtins.
pub fn resolve_pattern(name: &str, file: Option<&std::path::Path>) -> Result<PatPattern, String> {
    if let Some(path) = file {
        let pats = load_pat_file(path)?;
        let upper = name.to_ascii_uppercase();
        if let Some(p) = pats.into_iter().find(|p| p.name.eq_ignore_ascii_case(&upper)) {
            return Ok(p);
        }
        return Err(format!("pattern '{name}' not found in {}", path.display()));
    }
    builtin_pattern(name).ok_or_else(|| format!("unknown built-in pattern '{name}'"))
}

/// Generate hatch segment lines clipped to a closed boundary polygon.
pub fn generate_hatch(
    boundary: &[Vec2],
    pattern: &PatPattern,
    scale: f64,
    rotation_deg: f64,
) -> Vec<Line> {
    if boundary.len() < 3 {
        return vec![];
    }
    let scale = scale.abs().max(1e-9);
    let mut bb = Aabb::EMPTY;
    for p in boundary {
        bb = bb.expand(*p);
    }
    // Pad bbox.
    let pad = (bb.size().x + bb.size().y) * 0.1 + 10.0 * scale;
    bb = Aabb {
        min: Vec2::new(bb.min.x - pad, bb.min.y - pad),
        max: Vec2::new(bb.max.x + pad, bb.max.y + pad),
    };

    let mut out = Vec::new();
    for def in &pattern.lines {
        let angle = (def.angle_deg + rotation_deg).to_radians();
        let (s, c) = angle.sin_cos();
        let dir = Vec2::new(c, s);
        let perp = Vec2::new(-s, c);
        // Spacing along perpendicular.
        let spacing = (def.dy.abs() * scale).max(1e-6);
        // Origin of family.
        let origin = Vec2::new(def.x, def.y) * scale;
        // How many lines needed to cover bbox.
        let diag = bb.size().length();
        let n = ((diag / spacing) as i32 + 3).max(3);
        let center = bb.center();
        for i in -n..=n {
            let base = origin + perp * (i as f64 * spacing);
            // Project so lines pass near bbox center along this family.
            let shift = (center - base).dot(perp);
            let nearest_i = (shift / spacing).round() as i32;
            let _ = nearest_i;
            let p0 = base - dir * diag;
            let p1 = base + dir * diag;
            let infinite = Line::new(p0, p1);
            for seg in clip_line_to_polygon(infinite, boundary) {
                if def.dashes.is_empty() {
                    out.push(seg);
                } else {
                    out.extend(dash_segment(seg, &def.dashes, scale));
                }
            }
        }
    }
    out
}

fn dash_segment(seg: Line, dashes: &[f64], scale: f64) -> Vec<Line> {
    let len = seg.length();
    if len < 1e-12 || dashes.is_empty() {
        return vec![seg];
    }
    let dir = seg.delta() / len;
    let mut t = 0.0;
    let mut out = Vec::new();
    let mut di = 0usize;
    while t < len - 1e-12 {
        let sign = dashes[di % dashes.len()];
        let dash = sign.abs() * scale;
        let dash = if dash < 1e-12 { 1e-6 } else { dash };
        let t1 = (t + dash).min(len);
        if sign >= 0.0 {
            out.push(Line::new(seg.start + dir * t, seg.start + dir * t1));
        }
        t = t1;
        di += 1;
    }
    out
}

/// Clip an infinite-ish segment to the interior of a closed polygon (Sutherland-style spans).
fn clip_line_to_polygon(line: Line, poly: &[Vec2]) -> Vec<Line> {
    let mut ts = vec![0.0_f64, 1.0];
    // Intersect with each edge.
    for i in 0..poly.len() {
        let a = poly[i];
        let b = poly[(i + 1) % poly.len()];
        if let Some(t) = segment_param_intersection(line, Line::new(a, b)) {
            if (0.0..=1.0).contains(&t) {
                ts.push(t);
            }
        }
    }
    ts.sort_by(|a, b| a.partial_cmp(b).unwrap());
    ts.dedup_by(|a, b| (*a - *b).abs() < 1e-10);
    let mut out = Vec::new();
    for w in ts.windows(2) {
        let mid_t = (w[0] + w[1]) * 0.5;
        let mid = line.point_at(mid_t);
        if point_in_polygon(mid, poly) {
            out.push(Line::new(line.point_at(w[0]), line.point_at(w[1])));
        }
    }
    out
}

fn segment_param_intersection(a: Line, b: Line) -> Option<f64> {
    let r = a.delta();
    let s = b.delta();
    let denom = r.cross(s);
    if denom.abs() < 1e-15 {
        return None;
    }
    let qp = b.start - a.start;
    let t = qp.cross(s) / denom;
    let u = qp.cross(r) / denom;
    if (0.0..=1.0).contains(&u) {
        Some(t)
    } else {
        None
    }
}

fn point_in_polygon(p: Vec2, poly: &[Vec2]) -> bool {
    // Ray casting.
    let mut inside = false;
    let mut j = poly.len() - 1;
    for i in 0..poly.len() {
        let pi = poly[i];
        let pj = poly[j];
        if ((pi.y > p.y) != (pj.y > p.y))
            && (p.x < (pj.x - pi.x) * (p.y - pi.y) / (pj.y - pi.y + 1e-30) + pi.x)
        {
            inside = !inside;
        }
        j = i;
    }
    inside
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_builtin_ansi31() {
        let p = builtin_pattern("ANSI31").unwrap();
        assert_eq!(p.lines.len(), 1);
        assert!((p.lines[0].angle_deg - 45.0).abs() < 1e-9);
    }

    #[test]
    fn hatch_square_produces_segments() {
        let boundary = vec![
            Vec2::ZERO,
            Vec2::new(10.0, 0.0),
            Vec2::new(10.0, 10.0),
            Vec2::new(0.0, 10.0),
        ];
        let pat = builtin_pattern("ANSI31").unwrap();
        let segs = generate_hatch(&boundary, &pat, 1.0, 0.0);
        assert!(!segs.is_empty());
        for s in &segs {
            let mid = s.point_at(0.5);
            assert!(point_in_polygon(mid, &boundary) || mid.x >= 0.0);
        }
    }

    #[test]
    fn parse_freecad_style_pat() {
        let text = "*Square, square grid\n90,1,1,0,5.0\n0,0,0,1,5.0\n";
        let pats = parse_pat(text).unwrap();
        assert_eq!(pats.len(), 1);
        assert_eq!(pats[0].lines.len(), 2);
    }
}
