//! PNG threshold + marching-squares contours → polylines.

use image::GenericImageView;
use kelicad_doc::Geometry;
use kelicad_geom::Polyline;
use kelicad_math::Vec2;

use crate::error::VectorizeError;

/// Options for raster vectorization.
#[derive(Debug, Clone, Copy)]
pub struct PngOptions {
    /// Luma threshold (0–255); pixels darker than this are “ink”.
    pub threshold: u8,
    /// Max image dimension (longer side); larger images are skipped via subsample.
    pub max_dim: u32,
    /// Minimum contour vertices to keep.
    pub min_vertices: usize,
}

impl Default for PngOptions {
    fn default() -> Self {
        Self {
            threshold: 128,
            max_dim: 256,
            min_vertices: 4,
        }
    }
}

/// Decode PNG and extract closed/open contours as polylines (Y flipped to CAD-up).
pub fn vectorize_png(bytes: &[u8], opts: PngOptions) -> Result<Vec<Geometry>, VectorizeError> {
    let img = image::load_from_memory(bytes)
        .map_err(|e| VectorizeError::PngDecode(e.to_string()))?
        .to_luma8();
    let (w0, h0) = img.dimensions();
    if w0 == 0 || h0 == 0 {
        return Err(VectorizeError::Empty);
    }

    let scale = {
        let m = w0.max(h0);
        if m > opts.max_dim {
            (m as f64 / opts.max_dim as f64).ceil() as u32
        } else {
            1
        }
    };
    let w = (w0 + scale - 1) / scale;
    let h = (h0 + scale - 1) / scale;

    let mut grid = vec![false; (w * h) as usize];
    for y in 0..h {
        for x in 0..w {
            let px = img.get_pixel(x * scale, y * scale).0[0];
            grid[(y * w + x) as usize] = px < opts.threshold;
        }
    }

    let contours = marching_squares(&grid, w, h);
    let mut out = Vec::new();
    for contour in contours {
        if contour.len() < opts.min_vertices {
            continue;
        }
        // Flip Y so image top maps to larger world Y (CAD-ish).
        let verts: Vec<Vec2> = contour
            .into_iter()
            .map(|(x, y)| Vec2::new(x as f64 * scale as f64, (h0 as f64) - y as f64 * scale as f64))
            .collect();
        let closed = verts.len() >= 3
            && (verts[0].x - verts[verts.len() - 1].x).abs() < 1.5 * scale as f64
            && (verts[0].y - verts[verts.len() - 1].y).abs() < 1.5 * scale as f64;
        let vertices = if closed && verts.len() > 1 {
            verts[..verts.len() - 1].to_vec()
        } else {
            verts
        };
        if vertices.len() >= 2 {
            out.push(Geometry::Polyline(Polyline {
                vertices,
                closed,
            }));
        }
    }

    if out.is_empty() {
        return Err(VectorizeError::Empty);
    }
    Ok(out)
}

/// Classic marching squares on a boolean grid; returns pixel-space polylines.
fn marching_squares(grid: &[bool], w: u32, h: u32) -> Vec<Vec<(f32, f32)>> {
    let at = |x: i32, y: i32| -> bool {
        if x < 0 || y < 0 || x >= w as i32 || y >= h as i32 {
            return false;
        }
        grid[(y as u32 * w + x as u32) as usize]
    };

    // Edge midpoints relative to cell (x,y) of size 1:
    // 0 top, 1 right, 2 bottom, 3 left
    let edge_pt = |x: i32, y: i32, e: u8| -> (f32, f32) {
        match e {
            0 => (x as f32 + 0.5, y as f32),
            1 => (x as f32 + 1.0, y as f32 + 0.5),
            2 => (x as f32 + 0.5, y as f32 + 1.0),
            _ => (x as f32, y as f32 + 0.5),
        }
    };

    // Collect undirected edge segments, then chain them.
    let mut segments: Vec<((f32, f32), (f32, f32))> = Vec::new();
    for y in 0..h as i32 {
        for x in 0..w as i32 {
            let idx = (at(x, y) as u8)
                | ((at(x + 1, y) as u8) << 1)
                | ((at(x + 1, y + 1) as u8) << 2)
                | ((at(x, y + 1) as u8) << 3);
            let edges: &[(u8, u8)] = match idx {
                0 | 15 => continue,
                1 | 14 => &[(3, 0)],
                2 | 13 => &[(0, 1)],
                3 | 12 => &[(3, 1)],
                4 | 11 => &[(1, 2)],
                5 => &[(3, 0), (1, 2)],
                6 | 9 => &[(0, 2)],
                7 | 8 => &[(3, 2)],
                10 => &[(0, 1), (3, 2)],
                _ => continue,
            };
            for &(a, b) in edges {
                segments.push((edge_pt(x, y, a), edge_pt(x, y, b)));
            }
        }
    }

    chain_segments(segments)
}

fn nearly(a: (f32, f32), b: (f32, f32)) -> bool {
    (a.0 - b.0).abs() < 1e-3 && (a.1 - b.1).abs() < 1e-3
}

fn chain_segments(mut segments: Vec<((f32, f32), (f32, f32))>) -> Vec<Vec<(f32, f32)>> {
    let mut contours = Vec::new();
    while let Some((a, b)) = segments.pop() {
        let mut poly = vec![a, b];
        let mut grew = true;
        while grew {
            grew = false;
            let head = poly[0];
            let tail = *poly.last().unwrap();
            let mut i = 0;
            while i < segments.len() {
                let (s, e) = segments[i];
                if nearly(s, tail) {
                    poly.push(e);
                    segments.swap_remove(i);
                    grew = true;
                } else if nearly(e, tail) {
                    poly.push(s);
                    segments.swap_remove(i);
                    grew = true;
                } else if nearly(s, head) {
                    poly.insert(0, e);
                    segments.swap_remove(i);
                    grew = true;
                } else if nearly(e, head) {
                    poly.insert(0, s);
                    segments.swap_remove(i);
                    grew = true;
                } else {
                    i += 1;
                }
            }
        }
        // Decimate collinear-ish samples.
        let simplified = decimate(&poly, 0.35);
        if simplified.len() >= 2 {
            contours.push(simplified);
        }
    }
    contours
}

fn decimate(pts: &[(f32, f32)], eps: f32) -> Vec<(f32, f32)> {
    if pts.len() <= 2 {
        return pts.to_vec();
    }
    let mut out = vec![pts[0]];
    for p in &pts[1..] {
        let last = *out.last().unwrap();
        if (p.0 - last.0).abs() > eps || (p.1 - last.1).abs() > eps {
            out.push(*p);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filled_square_png() {
        // 8x8 white with black 4x4 square in center → PNG via image crate.
        let mut img = image::GrayImage::from_pixel(16, 16, image::Luma([255]));
        for y in 4..12 {
            for x in 4..12 {
                img.put_pixel(x, y, image::Luma([0]));
            }
        }
        let mut buf = Vec::new();
        {
            let mut cursor = std::io::Cursor::new(&mut buf);
            image::DynamicImage::ImageLuma8(img)
                .write_to(&mut cursor, image::ImageFormat::Png)
                .unwrap();
        }
        let geoms = vectorize_png(&buf, PngOptions::default()).unwrap();
        assert!(!geoms.is_empty());
    }
}
