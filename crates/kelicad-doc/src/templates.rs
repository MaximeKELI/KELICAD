//! Seed templates (A3/A4 frames + title block) — LibreCAD/QCAD–style project start.

use kelicad_math::Vec2;

use crate::annotation::{TextAlign, TextGeom};
use crate::geometry::Geometry;
use crate::polyline::Polyline;
use kelicad_geom::Polyline as GeomPoly;

/// One entity to place when applying a template.
#[derive(Debug, Clone)]
pub struct TemplateEntity {
    /// Target layer name (created if missing).
    pub layer: &'static str,
    /// Layer RGB if created.
    pub layer_color: [u8; 3],
    /// Geometry in model units (mm for paper templates).
    pub geometry: Geometry,
}

/// Named project template.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TemplateKind {
    /// Empty drawing (layer 0 only conceptually).
    Blank,
    /// A3 landscape frame + cartouche.
    A3Landscape,
    /// A4 portrait frame + cartouche.
    A4Portrait,
    /// Mechanical title block (smaller sheet).
    MechA4,
}

impl TemplateKind {
    /// Parse alias (FR/EN).
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "blank" | "vide" | "empty" => Some(Self::Blank),
            "a3" | "a3l" | "a3-landscape" => Some(Self::A3Landscape),
            "a4" | "a4p" | "a4-portrait" => Some(Self::A4Portrait),
            "mech" | "meca" | "mechanical" => Some(Self::MechA4),
            _ => None,
        }
    }

    /// Canonical name.
    pub fn name(self) -> &'static str {
        match self {
            Self::Blank => "blank",
            Self::A3Landscape => "a3",
            Self::A4Portrait => "a4",
            Self::MechA4 => "mech",
        }
    }
}

/// Build template entities (caller creates layers + entities).
pub fn template_entities(kind: TemplateKind) -> Vec<TemplateEntity> {
    match kind {
        TemplateKind::Blank => vec![],
        TemplateKind::A3Landscape => sheet_with_title(420.0, 297.0, "A3"),
        TemplateKind::A4Portrait => sheet_with_title(210.0, 297.0, "A4"),
        TemplateKind::MechA4 => sheet_with_title(297.0, 210.0, "MECA"),
    }
}

fn sheet_with_title(w: f64, h: f64, label: &str) -> Vec<TemplateEntity> {
    let margin = 10.0;
    let frame = GeomPoly {
        vertices: vec![
            Vec2::new(margin, margin),
            Vec2::new(w - margin, margin),
            Vec2::new(w - margin, h - margin),
            Vec2::new(margin, h - margin),
        ],
        closed: true,
    };
    let tw = 110.0;
    let th = 40.0;
    let tx0 = w - margin - tw;
    let ty0 = margin;
    let title = GeomPoly {
        vertices: vec![
            Vec2::new(tx0, ty0),
            Vec2::new(tx0 + tw, ty0),
            Vec2::new(tx0 + tw, ty0 + th),
            Vec2::new(tx0, ty0 + th),
        ],
        closed: true,
    };
    // Inner divider in cartouche.
    let mid_y = ty0 + th * 0.45;
    let mut out = vec![
        TemplateEntity {
            layer: "Cadre",
            layer_color: [220, 220, 220],
            geometry: Geometry::Polyline(Polyline {
                vertices: frame.vertices.clone(),
                closed: true,
            }),
        },
        TemplateEntity {
            layer: "Cartouche",
            layer_color: [255, 200, 80],
            geometry: Geometry::Polyline(Polyline {
                vertices: title.vertices.clone(),
                closed: true,
            }),
        },
        TemplateEntity {
            layer: "Cartouche",
            layer_color: [255, 200, 80],
            geometry: Geometry::Line(kelicad_geom::Line::new(
                Vec2::new(tx0, mid_y),
                Vec2::new(tx0 + tw, mid_y),
            )),
        },
        TemplateEntity {
            layer: "Texte",
            layer_color: [255, 220, 120],
            geometry: Geometry::Text(TextGeom {
                position: Vec2::new(tx0 + 3.0, ty0 + th - 8.0),
                height: 4.0,
                rotation: 0.0,
                content: format!("KELICAD — {label}"),
                align: TextAlign::Left,
                style_id: None,
                annotative: false,
            }),
        },
        TemplateEntity {
            layer: "Texte",
            layer_color: [255, 220, 120],
            geometry: Geometry::Text(TextGeom {
                position: Vec2::new(tx0 + 3.0, ty0 + 4.0),
                height: 2.5,
                rotation: 0.0,
                content: "Projet / Échelle 1:100 / Date".into(),
                align: TextAlign::Left,
                style_id: None,
                annotative: false,
            }),
        },
    ];
    // Fold marks (cheat from ISO folding).
    let fold_x = w * 0.5;
    out.push(TemplateEntity {
        layer: "Construction",
        layer_color: [100, 140, 100],
        geometry: Geometry::Line(kelicad_geom::Line::new(
            Vec2::new(fold_x, margin - 2.0),
            Vec2::new(fold_x, margin + 2.0),
        )),
    });
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a3_has_frame_and_title() {
        let e = template_entities(TemplateKind::A3Landscape);
        assert!(e.len() >= 4);
        assert!(e.iter().any(|t| t.layer == "Cadre"));
        assert!(e.iter().any(|t| matches!(t.geometry, Geometry::Text(_))));
    }
}
