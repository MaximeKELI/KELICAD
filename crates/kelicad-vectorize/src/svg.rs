//! Minimal SVG → geometry (line, polyline, polygon, rect, path M/L/H/V/Z).

use kelicad_doc::Geometry;
use kelicad_geom::{Line, Polyline};
use kelicad_math::Vec2;

use crate::error::VectorizeError;

/// Parse SVG bytes into geometries.
pub fn vectorize_svg(bytes: &[u8]) -> Result<Vec<Geometry>, VectorizeError> {
    let text = std::str::from_utf8(bytes).map_err(|_| VectorizeError::InvalidSvgText)?;
    let mut out = Vec::new();

    for tag in iter_tags(text) {
        let name = tag_name(&tag).to_ascii_lowercase();
        match name.as_str() {
            "line" => {
                let x1 = attr_f64(&tag, "x1").unwrap_or(0.0);
                let y1 = attr_f64(&tag, "y1").unwrap_or(0.0);
                let x2 = attr_f64(&tag, "x2").unwrap_or(0.0);
                let y2 = attr_f64(&tag, "y2").unwrap_or(0.0);
                out.push(Geometry::Line(Line::new(
                    Vec2::new(x1, y1),
                    Vec2::new(x2, y2),
                )));
            }
            "polyline" => {
                if let Some(pts) = attr_str(&tag, "points") {
                    let verts = parse_points(pts);
                    if verts.len() >= 2 {
                        out.push(Geometry::Polyline(Polyline {
                            vertices: verts,
                            closed: false,
                        }));
                    }
                }
            }
            "polygon" => {
                if let Some(pts) = attr_str(&tag, "points") {
                    let verts = parse_points(pts);
                    if verts.len() >= 3 {
                        out.push(Geometry::Polyline(Polyline {
                            vertices: verts,
                            closed: true,
                        }));
                    }
                }
            }
            "rect" => {
                let x = attr_f64(&tag, "x").unwrap_or(0.0);
                let y = attr_f64(&tag, "y").unwrap_or(0.0);
                let w = attr_f64(&tag, "width").unwrap_or(0.0);
                let h = attr_f64(&tag, "height").unwrap_or(0.0);
                if w > 0.0 && h > 0.0 {
                    out.push(Geometry::Polyline(Polyline {
                        vertices: vec![
                            Vec2::new(x, y),
                            Vec2::new(x + w, y),
                            Vec2::new(x + w, y + h),
                            Vec2::new(x, y + h),
                        ],
                        closed: true,
                    }));
                }
            }
            "path" => {
                if let Some(d) = attr_str(&tag, "d") {
                    out.extend(parse_path(d));
                }
            }
            _ => {}
        }
    }

    if out.is_empty() {
        return Err(VectorizeError::Empty);
    }
    Ok(out)
}

fn iter_tags(text: &str) -> Vec<String> {
    let mut tags = Vec::new();
    let mut rest = text;
    while let Some(start) = rest.find('<') {
        rest = &rest[start..];
        if rest.starts_with("<!--") {
            if let Some(end) = rest.find("-->") {
                rest = &rest[end + 3..];
                continue;
            }
            break;
        }
        if rest.starts_with("</") {
            if let Some(end) = rest.find('>') {
                rest = &rest[end + 1..];
                continue;
            }
            break;
        }
        if let Some(end) = rest.find('>') {
            let tag = rest[1..end].trim_end_matches('/').to_string();
            tags.push(tag);
            rest = &rest[end + 1..];
        } else {
            break;
        }
    }
    tags
}

fn tag_name(tag: &str) -> &str {
    tag.split_whitespace()
        .next()
        .unwrap_or(tag)
        .split_once(|c: char| c == '/' || c == '\n')
        .map(|(a, _)| a)
        .unwrap_or(tag)
        .trim()
}

fn attr_str<'a>(tag: &'a str, key: &str) -> Option<&'a str> {
    // key="..." or key='...'
    let patterns = [format!("{key}=\""), format!("{key}='")];
    for (i, pat) in patterns.iter().enumerate() {
        if let Some(pos) = tag.find(pat) {
            let start = pos + pat.len();
            let quote = if i == 0 { '"' } else { '\'' };
            let rest = &tag[start..];
            if let Some(end) = rest.find(quote) {
                return Some(&rest[..end]);
            }
        }
    }
    None
}

fn attr_f64(tag: &str, key: &str) -> Option<f64> {
    attr_str(tag, key)?.parse().ok()
}

fn parse_points(s: &str) -> Vec<Vec2> {
    let nums: Vec<f64> = s
        .split(|c: char| c.is_whitespace() || c == ',')
        .filter(|t| !t.is_empty())
        .filter_map(|t| t.parse().ok())
        .collect();
    nums.chunks_exact(2)
        .map(|c| Vec2::new(c[0], c[1]))
        .collect()
}

fn parse_path(d: &str) -> Vec<Geometry> {
    let tokens = tokenize_path(d);
    let mut geoms = Vec::new();
    let mut i = 0;
    let mut cursor = Vec2::ZERO;
    let mut start = Vec2::ZERO;
    let mut verts: Vec<Vec2> = Vec::new();
    let mut closed = false;

    let flush = |verts: &mut Vec<Vec2>, closed: &mut bool, geoms: &mut Vec<Geometry>| {
        if verts.len() >= 2 {
            if verts.len() == 2 && !*closed {
                geoms.push(Geometry::Line(Line::new(verts[0], verts[1])));
            } else {
                geoms.push(Geometry::Polyline(Polyline {
                    vertices: verts.clone(),
                    closed: *closed,
                }));
            }
        }
        verts.clear();
        *closed = false;
    };

    while i < tokens.len() {
        let cmd = &tokens[i];
        if cmd.len() == 1 && cmd.chars().next().unwrap().is_ascii_alphabetic() {
            let c = cmd.chars().next().unwrap();
            i += 1;
            match c {
                'M' | 'm' => {
                    flush(&mut verts, &mut closed, &mut geoms);
                    let rel = c == 'm';
                    if let Some(p) = read_pair(&tokens, &mut i) {
                        cursor = if rel {
                            cursor + p
                        } else {
                            p
                        };
                        start = cursor;
                        verts.push(cursor);
                        // Implicit lineto pairs after moveto.
                        while let Some(p2) = peek_pair(&tokens, i) {
                            i += 2;
                            cursor = if rel { cursor + p2 } else { p2 };
                            verts.push(cursor);
                        }
                    }
                }
                'L' | 'l' => {
                    let rel = c == 'l';
                    while let Some(p) = peek_pair(&tokens, i) {
                        i += 2;
                        cursor = if rel { cursor + p } else { p };
                        if verts.is_empty() {
                            verts.push(start);
                        }
                        verts.push(cursor);
                    }
                }
                'H' | 'h' => {
                    let rel = c == 'h';
                    while i < tokens.len() && is_number(&tokens[i]) {
                        let x: f64 = tokens[i].parse().unwrap_or(cursor.x);
                        i += 1;
                        cursor.x = if rel { cursor.x + x } else { x };
                        if verts.is_empty() {
                            verts.push(start);
                        }
                        verts.push(cursor);
                    }
                }
                'V' | 'v' => {
                    let rel = c == 'v';
                    while i < tokens.len() && is_number(&tokens[i]) {
                        let y: f64 = tokens[i].parse().unwrap_or(cursor.y);
                        i += 1;
                        cursor.y = if rel { cursor.y + y } else { y };
                        if verts.is_empty() {
                            verts.push(start);
                        }
                        verts.push(cursor);
                    }
                }
                'Z' | 'z' => {
                    closed = true;
                    cursor = start;
                    flush(&mut verts, &mut closed, &mut geoms);
                }
                _ => {
                    // Skip unsupported commands' numeric args.
                    while i < tokens.len() && is_number(&tokens[i]) {
                        i += 1;
                    }
                }
            }
        } else {
            i += 1;
        }
    }
    flush(&mut verts, &mut closed, &mut geoms);
    geoms
}

fn tokenize_path(d: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    for ch in d.chars() {
        if ch.is_ascii_alphabetic() {
            if !cur.is_empty() {
                out.push(std::mem::take(&mut cur));
            }
            out.push(ch.to_string());
        } else if ch == ',' || ch.is_whitespace() {
            if !cur.is_empty() {
                out.push(std::mem::take(&mut cur));
            }
        } else if ch == '-' || ch == '+' {
            if !cur.is_empty() && !cur.ends_with('e') && !cur.ends_with('E') {
                out.push(std::mem::take(&mut cur));
            }
            cur.push(ch);
        } else {
            cur.push(ch);
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

fn is_number(s: &str) -> bool {
    s.parse::<f64>().is_ok()
}

fn read_pair(tokens: &[String], i: &mut usize) -> Option<Vec2> {
    if *i + 1 >= tokens.len() {
        return None;
    }
    let x: f64 = tokens[*i].parse().ok()?;
    let y: f64 = tokens[*i + 1].parse().ok()?;
    *i += 2;
    Some(Vec2::new(x, y))
}

fn peek_pair(tokens: &[String], i: usize) -> Option<Vec2> {
    if i + 1 >= tokens.len() {
        return None;
    }
    if !is_number(&tokens[i]) || !is_number(&tokens[i + 1]) {
        return None;
    }
    Some(Vec2::new(
        tokens[i].parse().ok()?,
        tokens[i + 1].parse().ok()?,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_and_rect() {
        let svg = br#"<svg><line x1="0" y1="0" x2="10" y2="0"/><rect x="1" y="1" width="2" height="3"/></svg>"#;
        let geoms = vectorize_svg(svg).unwrap();
        assert_eq!(geoms.len(), 2);
        assert!(matches!(geoms[0], Geometry::Line(_)));
        assert!(matches!(geoms[1], Geometry::Polyline(_)));
    }

    #[test]
    fn path_square() {
        let svg = br#"<svg><path d="M0 0 L10 0 L10 10 L0 10 Z"/></svg>"#;
        let geoms = vectorize_svg(svg).unwrap();
        assert_eq!(geoms.len(), 1);
        let Geometry::Polyline(p) = &geoms[0] else {
            panic!();
        };
        assert!(p.closed);
        assert_eq!(p.vertices.len(), 4);
    }
}
