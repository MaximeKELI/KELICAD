//! Minimal ESRI Shapefile (.shp) reader → KELICAD geometries.
//!
//! Supports Point (1), PolyLine (3), Polygon (5). No `.shx`/`.dbf` required for geometry.
//! Format: ESRI Shapefile Technical Description (public).

use std::path::Path;

use kelicad_doc::{EntityId, EntityKind, EntityRecord, Geometry};
use kelicad_geom::Polyline;
use kelicad_math::Vec2;
use kelicad_ops::{DocSession, Operation};
use uuid::Uuid;

use crate::error::IoError;

/// Import report for shapefile.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ShpImportReport {
    /// Entities created.
    pub imported: usize,
    /// Skipped records.
    pub skipped: Vec<String>,
}

/// Import a `.shp` file into a new session (layer `0`).
pub fn import_shp(path: &Path) -> Result<(DocSession, ShpImportReport), IoError> {
    let bytes = std::fs::read(path).map_err(|e| IoError::Format(e.to_string()))?;
    import_shp_bytes(&bytes)
}

/// Import shapefile bytes.
pub fn import_shp_bytes(bytes: &[u8]) -> Result<(DocSession, ShpImportReport), IoError> {
    if bytes.len() < 100 {
        return Err(IoError::Format("shp too short".into()));
    }
    let file_code = read_i32_be(bytes, 0)?;
    if file_code != 9994 {
        return Err(IoError::Format(format!(
            "bad shp file code {file_code} (expected 9994)"
        )));
    }
    let mut report = ShpImportReport::default();
    let mut session = DocSession::new();
    let layer = session.document.default_layer_id();
    let mut offset = 100usize;
    let mut next_id: u128 = 5000;

    while offset + 8 <= bytes.len() {
        let _rec_num = read_i32_be(bytes, offset)?;
        let content_len_words = read_i32_be(bytes, offset + 4)? as usize;
        offset += 8;
        let content_bytes = content_len_words * 2;
        if offset + content_bytes > bytes.len() {
            report.skipped.push("truncated record".into());
            break;
        }
        let rec = &bytes[offset..offset + content_bytes];
        offset += content_bytes;
        if rec.len() < 4 {
            continue;
        }
        let shape_type = read_i32_le(rec, 0)?;
        match shape_type {
            0 => {} // null
            1 => {
                if rec.len() < 20 {
                    report.skipped.push("POINT truncated".into());
                    continue;
                }
                let x = read_f64_le(rec, 4)?;
                let y = read_f64_le(rec, 12)?;
                let id = EntityId::from_uuid(Uuid::from_u128(next_id));
                next_id += 1;
                session
                    .apply(Operation::CreateEntity {
                        record: EntityRecord {
                            id,
                            kind: EntityKind::Point,
                            geometry: Geometry::Point(Vec2::new(x, y)),
                            layer_id: layer,
                        },
                    })
                    .map_err(|e| IoError::Format(e.to_string()))?;
                report.imported += 1;
            }
            3 | 5 => {
                // PolyLine / Polygon
                if rec.len() < 44 {
                    report.skipped.push("polyline header truncated".into());
                    continue;
                }
                let n_parts = read_i32_le(rec, 36)? as usize;
                let n_points = read_i32_le(rec, 40)? as usize;
                let parts_off = 44;
                let points_off = parts_off + n_parts * 4;
                if points_off + n_points * 16 > rec.len() {
                    report.skipped.push("polyline points truncated".into());
                    continue;
                }
                let mut part_starts = Vec::with_capacity(n_parts);
                for i in 0..n_parts {
                    part_starts.push(read_i32_le(rec, parts_off + i * 4)? as usize);
                }
                part_starts.push(n_points);
                for w in part_starts.windows(2) {
                    let (a, b) = (w[0], w[1]);
                    if b <= a {
                        continue;
                    }
                    let mut verts = Vec::with_capacity(b - a);
                    for i in a..b {
                        let o = points_off + i * 16;
                        let x = read_f64_le(rec, o)?;
                        let y = read_f64_le(rec, o + 8)?;
                        verts.push(Vec2::new(x, y));
                    }
                    if verts.len() < 2 {
                        continue;
                    }
                    let closed = shape_type == 5
                        || (verts.first() == verts.last() && verts.len() >= 3);
                    if closed && verts.first() == verts.last() && verts.len() > 1 {
                        verts.pop();
                    }
                    let poly = if closed {
                        Polyline::closed(verts)
                    } else {
                        Polyline::new(verts)
                    };
                    let id = EntityId::from_uuid(Uuid::from_u128(next_id));
                    next_id += 1;
                    session
                        .apply(Operation::CreateEntity {
                            record: EntityRecord {
                                id,
                                kind: EntityKind::Polyline,
                                geometry: Geometry::Polyline(poly),
                                layer_id: layer,
                            },
                        })
                        .map_err(|e| IoError::Format(e.to_string()))?;
                    report.imported += 1;
                }
            }
            other => report.skipped.push(format!("unsupported shape type {other}")),
        }
    }

    Ok((session, report))
}

fn read_i32_be(b: &[u8], o: usize) -> Result<i32, IoError> {
    let a: [u8; 4] = b
        .get(o..o + 4)
        .ok_or_else(|| IoError::Format("eof".into()))?
        .try_into()
        .unwrap();
    Ok(i32::from_be_bytes(a))
}

fn read_i32_le(b: &[u8], o: usize) -> Result<i32, IoError> {
    let a: [u8; 4] = b
        .get(o..o + 4)
        .ok_or_else(|| IoError::Format("eof".into()))?
        .try_into()
        .unwrap();
    Ok(i32::from_le_bytes(a))
}

fn read_f64_le(b: &[u8], o: usize) -> Result<f64, IoError> {
    let a: [u8; 8] = b
        .get(o..o + 8)
        .ok_or_else(|| IoError::Format("eof".into()))?
        .try_into()
        .unwrap();
    Ok(f64::from_le_bytes(a))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Minimal Point shapefile (one point at 1,2).
    fn point_shp_bytes() -> Vec<u8> {
        let mut b = vec![0u8; 100];
        b[0..4].copy_from_slice(&9994i32.to_be_bytes());
        // file length in 16-bit words: (100 + 8 + 20) / 2 = 64
        let file_len_words = 64i32;
        b[24..28].copy_from_slice(&file_len_words.to_be_bytes());
        b[28..32].copy_from_slice(&1000i32.to_le_bytes()); // version
        b[32..36].copy_from_slice(&1i32.to_le_bytes()); // shape type Point
        // record
        let mut rec = Vec::new();
        rec.extend_from_slice(&1i32.to_be_bytes()); // record number
        rec.extend_from_slice(&10i32.to_be_bytes()); // content length words (20 bytes)
        rec.extend_from_slice(&1i32.to_le_bytes()); // Point
        rec.extend_from_slice(&1.0f64.to_le_bytes());
        rec.extend_from_slice(&2.0f64.to_le_bytes());
        b.extend_from_slice(&rec);
        b
    }

    #[test]
    fn import_point_shp() {
        let (session, report) = import_shp_bytes(&point_shp_bytes()).unwrap();
        assert_eq!(report.imported, 1);
        assert_eq!(session.document.entity_count(), 1);
        let geom = session
            .document
            .entities()
            .next()
            .expect("one entity")
            .geometry
            .clone();
        match geom {
            Geometry::Point(p) => {
                assert!((p.x - 1.0).abs() < 1e-9);
                assert!((p.y - 2.0).abs() < 1e-9);
            }
            _ => panic!("expected point"),
        }
    }
}
