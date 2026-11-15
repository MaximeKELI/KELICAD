//! Minimal IFC STEP subset export / import (walls, slabs, doors, windows, stairs).

use std::collections::HashMap;

use kelicad_math::Vec3;
use kelicad_solid::{BuiltinSolidKernel, SolidBody, SolidId, SolidKernel};

use crate::element::{BimElement, BimKind};
use crate::error::BimError;

/// Export BIM elements + bodies to a STEP-text IFC subset.
pub fn export_ifc(elements: &[BimElement], bodies: &[SolidBody]) -> Result<String, BimError> {
    let by_id: HashMap<SolidId, &SolidBody> = bodies.iter().map(|b| (b.id, b)).collect();
    let mut lines = vec![
        "ISO-10303-21;".into(),
        "HEADER;".into(),
        "FILE_DESCRIPTION(('KELICAD IFC subset'),'2;1');".into(),
        "FILE_NAME('kelicad.ifc','',('KELICAD'),('KELICAD'),'kelicad-bim','','');".into(),
        "FILE_SCHEMA(('IFC2X3'));".into(),
        "ENDSEC;".into(),
        "DATA;".into(),
    ];

    let mut n = 1u32;
    let project = n;
    lines.push(format!("#{project}=IFCPROJECT('KELICAD',$,$,$,$,$,$,$,$);"));
    n += 1;
    let storey = n;
    lines.push(format!(
        "#{storey}=IFCBUILDINGSTOREY('Level 0',$,$,$,$,$,$,$,$);"
    ));
    n += 1;

    for e in elements {
        let Some(body) = by_id.get(&e.solid_id) else {
            continue;
        };
        let tag = e.props.get("tag").map(|s| s.as_str()).unwrap_or("");
        let cx = (body.aabb.min.x + body.aabb.max.x) * 0.5;
        let cy = (body.aabb.min.y + body.aabb.max.y) * 0.5;
        let cz = body.aabb.min.z;
        let dx = body.aabb.size().x;
        let dy = body.aabb.size().y;
        let dz = body.aabb.size().z;
        let place = n;
        lines.push(format!(
            "#{place}=IFCLOCALPLACEMENT($,IFCCARTESIANPOINT(({cx:.6},{cy:.6},{cz:.6})));"
        ));
        n += 1;
        let prod = n;
        let ty = e.kind.ifc_type();
        lines.push(format!(
            "#{prod}={ty}('{tag}',$,$,$,$,#{place},$,$,$); /* aabb {dx:.6} {dy:.6} {dz:.6} solid {} */",
            body.id
        ));
        n += 1;
        let rel = n;
        lines.push(format!(
            "#{rel}=IFCRELCONTAINEDINSPATIALSTRUCTURE($,$,$,$,(#{prod}),#{storey});"
        ));
        n += 1;
        let _ = project;
    }

    lines.push("ENDSEC;".into());
    lines.push("END-ISO-10303-21;".into());
    Ok(lines.join("\n"))
}

/// Import walls/slabs (and other known types) from our subset export.
pub fn import_ifc(text: &str) -> Result<(Vec<BimElement>, Vec<SolidBody>), BimError> {
    let k = BuiltinSolidKernel;
    let mut elements = Vec::new();
    let mut bodies = Vec::new();

    for line in text.lines() {
        let upper = line.to_ascii_uppercase();
        let kind = if upper.contains("=IFCWALL") {
            Some(BimKind::Wall)
        } else if upper.contains("=IFCSLAB") {
            Some(BimKind::Slab)
        } else if upper.contains("=IFCDOOR") {
            Some(BimKind::Door)
        } else if upper.contains("=IFCWINDOW") {
            Some(BimKind::Window)
        } else if upper.contains("=IFCSTAIR") {
            Some(BimKind::Stair)
        } else {
            None
        };
        let Some(kind) = kind else {
            continue;
        };

        let (cx, cy, cz) = parse_placement_near(text, line).unwrap_or((0.0, 0.0, 0.0));
        let (dx, dy, dz) = parse_aabb_comment(line).unwrap_or((1.0, 1.0, 1.0));
        let tag = parse_quoted_name(line).unwrap_or_default();

        let min = Vec3::new(cx - dx * 0.5, cy - dy * 0.5, cz);
        let max = Vec3::new(cx + dx * 0.5, cy + dy * 0.5, cz + dz);
        let body = k.box_solid(min, max).map_err(BimError::from)?;
        let mut elem = BimElement::new(kind, body.id);
        if !tag.is_empty() {
            elem = elem.with_prop("tag", tag);
        }
        elements.push(elem);
        bodies.push(body);
    }

    if elements.is_empty() {
        return Err(BimError::Ifc("no BIM products found".into()));
    }
    Ok((elements, bodies))
}

fn parse_quoted_name(line: &str) -> Option<String> {
    let start = line.find('(')?;
    let rest = &line[start + 1..];
    if let Some(stripped) = rest.strip_prefix('\'') {
        let end = stripped.find('\'')?;
        return Some(stripped[..end].to_string());
    }
    None
}

fn parse_aabb_comment(line: &str) -> Option<(f64, f64, f64)> {
    let idx = line.find("aabb ")?;
    let rest = &line[idx + 5..];
    let mut nums = rest
        .split_whitespace()
        .filter_map(|t| t.trim_matches(|c: char| c == '*' || c == '/').parse().ok());
    Some((nums.next()?, nums.next()?, nums.next()?))
}

fn parse_placement_near(full: &str, product_line: &str) -> Option<(f64, f64, f64)> {
    // Look for IFCLOCALPLACEMENT referenced as #N in the product line.
    let hash = product_line.find('#')?;
    // Find placement id in $,$,$,$,$,#PLACE
    let place_ref = product_line
        .split('#')
        .nth(1)?
        .chars()
        .take_while(|c| c.is_ascii_digit())
        .collect::<String>();
    let needle = format!("#{place_ref}=IFCLOCALPLACEMENT");
    for line in full.lines() {
        if line
            .to_ascii_uppercase()
            .contains(&needle.to_ascii_uppercase())
            || (line.starts_with(&format!("#{place_ref}="))
                && line.to_ascii_uppercase().contains("IFCLOCALPLACEMENT"))
        {
            return parse_cartesian(line);
        }
    }
    let _ = hash;
    None
}

fn parse_cartesian(line: &str) -> Option<(f64, f64, f64)> {
    let idx = line.to_ascii_uppercase().find("IFCCARTESIANPOINT")?;
    let rest = &line[idx..];
    let open = rest.find("((")?;
    let close = rest[open..].find("))")?;
    let inner = &rest[open + 2..open + close];
    let nums: Vec<f64> = inner
        .split(',')
        .filter_map(|t| t.trim().parse().ok())
        .collect();
    if nums.len() >= 3 {
        Some((nums[0], nums[1], nums[2]))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use kelicad_math::Vec2;

    use super::*;
    use crate::collide::collisions;
    use crate::schedule::schedule_csv;
    use crate::wall::{slab_from_rect, wall_from_baseline};

    #[test]
    fn schedule_wall_and_slab() {
        let (wb, we) = wall_from_baseline(Vec2::ZERO, Vec2::new(5.0, 0.0), 0.2, 2.7).unwrap();
        let (sb, se) = slab_from_rect(Vec2::ZERO, 5.0, 4.0, 0.2).unwrap();
        let csv = schedule_csv(&[we, se], &["material", "height", "thickness"]);
        assert!(csv.lines().count() >= 3);
        assert!(csv.contains("wall"));
        assert!(csv.contains("slab"));
        let _ = (wb, sb);
    }

    #[test]
    fn collisions_overlap_and_separate() {
        let k = BuiltinSolidKernel;
        let a = k.box_solid(Vec3::ZERO, Vec3::new(1.0, 1.0, 1.0)).unwrap();
        let b = k
            .box_solid(Vec3::new(0.5, 0.5, 0.5), Vec3::new(1.5, 1.5, 1.5))
            .unwrap();
        let c = k
            .box_solid(Vec3::new(3.0, 3.0, 3.0), Vec3::new(4.0, 4.0, 4.0))
            .unwrap();
        assert_eq!(collisions(&[a.clone(), b.clone()]).len(), 1);
        assert!(collisions(&[a, c]).is_empty());
    }

    #[test]
    fn ifc_roundtrip_wall() {
        let (body, elem) = wall_from_baseline(Vec2::ZERO, Vec2::new(4.0, 0.0), 0.2, 2.5).unwrap();
        let text = export_ifc(std::slice::from_ref(&elem), std::slice::from_ref(&body)).unwrap();
        assert!(text.to_ascii_uppercase().contains("IFCWALL"));
        let (elems, bodies) = import_ifc(&text).unwrap();
        assert_eq!(elems.len(), 1);
        assert_eq!(elems[0].kind, BimKind::Wall);
        assert_eq!(bodies.len(), 1);
        assert!((bodies[0].aabb.size().z - body.aabb.size().z).abs() < 0.05);
    }
}
