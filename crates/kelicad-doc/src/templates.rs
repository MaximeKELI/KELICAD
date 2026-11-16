//! Seed templates (A1–A4 frames + rich title block) — LibreCAD/QCAD–style project start.

use kelicad_geom::{Line, Polyline};
use kelicad_math::Vec2;

use crate::annotation::{TextAlign, TextGeom};
use crate::geometry::Geometry;

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
    /// A0 landscape frame + cartouche.
    A0Landscape,
    /// A1 landscape frame + cartouche.
    A1Landscape,
    /// A2 landscape frame + cartouche.
    A2Landscape,
    /// A3 landscape frame + cartouche.
    A3Landscape,
    /// A4 portrait frame + cartouche.
    A4Portrait,
    /// Mechanical title block (smaller sheet).
    MechA4,
    /// Architectural A3 with north arrow + scale bar.
    ArchA3,
}

impl TemplateKind {
    /// Parse alias (FR/EN).
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "blank" | "vide" | "empty" => Some(Self::Blank),
            "a0" | "a0l" | "a0-landscape" => Some(Self::A0Landscape),
            "a1" | "a1l" | "a1-landscape" => Some(Self::A1Landscape),
            "a2" | "a2l" | "a2-landscape" => Some(Self::A2Landscape),
            "a3" | "a3l" | "a3-landscape" => Some(Self::A3Landscape),
            "a4" | "a4p" | "a4-portrait" => Some(Self::A4Portrait),
            "mech" | "meca" | "mechanical" => Some(Self::MechA4),
            "arch" | "archi" | "architecture" | "archa3" => Some(Self::ArchA3),
            _ => None,
        }
    }

    /// Canonical name.
    pub fn name(self) -> &'static str {
        match self {
            Self::Blank => "blank",
            Self::A0Landscape => "a0",
            Self::A1Landscape => "a1",
            Self::A2Landscape => "a2",
            Self::A3Landscape => "a3",
            Self::A4Portrait => "a4",
            Self::MechA4 => "mech",
            Self::ArchA3 => "arch",
        }
    }

    /// All non-blank templates for UI lists.
    pub fn all_named() -> &'static [TemplateKind] {
        &[
            Self::A0Landscape,
            Self::A1Landscape,
            Self::A2Landscape,
            Self::A3Landscape,
            Self::A4Portrait,
            Self::MechA4,
            Self::ArchA3,
        ]
    }
}

/// Build template entities (caller creates layers + entities).
pub fn template_entities(kind: TemplateKind) -> Vec<TemplateEntity> {
    match kind {
        TemplateKind::Blank => vec![],
        TemplateKind::A0Landscape => sheet_with_title(1189.0, 841.0, "A0", TitleStyle::Iso),
        TemplateKind::A1Landscape => sheet_with_title(841.0, 594.0, "A1", TitleStyle::Iso),
        TemplateKind::A2Landscape => sheet_with_title(594.0, 420.0, "A2", TitleStyle::Iso),
        TemplateKind::A3Landscape => sheet_with_title(420.0, 297.0, "A3", TitleStyle::Iso),
        TemplateKind::A4Portrait => sheet_with_title(210.0, 297.0, "A4", TitleStyle::Iso),
        TemplateKind::MechA4 => sheet_with_title(297.0, 210.0, "MECA", TitleStyle::Mech),
        TemplateKind::ArchA3 => sheet_with_title(420.0, 297.0, "ARCH", TitleStyle::Arch),
    }
}

#[derive(Clone, Copy)]
enum TitleStyle {
    Iso,
    Mech,
    Arch,
}

fn sheet_with_title(w: f64, h: f64, label: &str, style: TitleStyle) -> Vec<TemplateEntity> {
    let margin = match style {
        TitleStyle::Arch => 15.0,
        _ => 10.0,
    };
    let frame = Polyline {
        vertices: vec![
            Vec2::new(margin, margin),
            Vec2::new(w - margin, margin),
            Vec2::new(w - margin, h - margin),
            Vec2::new(margin, h - margin),
        ],
        closed: true,
    };
    // Inner border (ISO-ish).
    let inner = margin + 5.0;
    let inner_frame = Polyline {
        vertices: vec![
            Vec2::new(inner, inner),
            Vec2::new(w - inner, inner),
            Vec2::new(w - inner, h - inner),
            Vec2::new(inner, h - inner),
        ],
        closed: true,
    };

    let (tw, th) = match style {
        TitleStyle::Mech => (140.0, 55.0),
        TitleStyle::Arch => (160.0, 50.0),
        TitleStyle::Iso => (120.0, 45.0),
    };
    let tx0 = w - margin - tw;
    let ty0 = margin;
    let title = Polyline {
        vertices: vec![
            Vec2::new(tx0, ty0),
            Vec2::new(tx0 + tw, ty0),
            Vec2::new(tx0 + tw, ty0 + th),
            Vec2::new(tx0, ty0 + th),
        ],
        closed: true,
    };

    let mut out = vec![
        TemplateEntity {
            layer: "Cadre",
            layer_color: [220, 220, 220],
            geometry: Geometry::Polyline(frame),
        },
        TemplateEntity {
            layer: "Cadre",
            layer_color: [180, 180, 180],
            geometry: Geometry::Polyline(inner_frame),
        },
        TemplateEntity {
            layer: "Cartouche",
            layer_color: [255, 200, 80],
            geometry: Geometry::Polyline(title),
        },
    ];

    // Horizontal dividers in title block.
    let rows = [0.28, 0.55, 0.78];
    for r in rows {
        let y = ty0 + th * r;
        out.push(TemplateEntity {
            layer: "Cartouche",
            layer_color: [255, 200, 80],
            geometry: Geometry::Line(Line::new(Vec2::new(tx0, y), Vec2::new(tx0 + tw, y))),
        });
    }
    // Vertical split.
    let vx = tx0 + tw * 0.62;
    out.push(TemplateEntity {
        layer: "Cartouche",
        layer_color: [255, 200, 80],
        geometry: Geometry::Line(Line::new(
            Vec2::new(vx, ty0),
            Vec2::new(vx, ty0 + th * 0.55),
        )),
    });

    let texts: &[(&str, f64, f64, f64)] = &[
        (&format!("KELICAD — {label}"), tx0 + 3.0, ty0 + th - 7.0, 4.0),
        ("Projet", tx0 + 3.0, ty0 + th * 0.62, 2.2),
        ("Échelle 1:100", tx0 + 3.0, ty0 + th * 0.35, 2.2),
        ("Date / Rev.", vx + 3.0, ty0 + th * 0.35, 2.0),
        ("Dessiné", tx0 + 3.0, ty0 + 4.0, 2.0),
        ("Vérifié", vx + 3.0, ty0 + 4.0, 2.0),
    ];
    for (content, x, y, height) in texts {
        out.push(TemplateEntity {
            layer: "Texte",
            layer_color: [255, 220, 120],
            geometry: Geometry::Text(TextGeom {
                position: Vec2::new(*x, *y),
                height: *height,
                rotation: 0.0,
                content: (*content).into(),
                align: TextAlign::Left,
                style_id: None,
                annotative: false,
            }),
        });
    }

    // Fold marks / centering ticks.
    let fold_x = w * 0.5;
    out.push(TemplateEntity {
        layer: "Construction",
        layer_color: [100, 140, 100],
        geometry: Geometry::Line(Line::new(
            Vec2::new(fold_x, margin - 2.0),
            Vec2::new(fold_x, margin + 2.0),
        )),
    });
    out.push(TemplateEntity {
        layer: "Construction",
        layer_color: [100, 140, 100],
        geometry: Geometry::Line(Line::new(
            Vec2::new(margin - 2.0, h * 0.5),
            Vec2::new(margin + 2.0, h * 0.5),
        )),
    });

    if matches!(style, TitleStyle::Arch | TitleStyle::Iso) {
        // North arrow (simple).
        let nx = margin + 25.0;
        let ny = h - margin - 30.0;
        out.push(TemplateEntity {
            layer: "Annotation",
            layer_color: [120, 200, 255],
            geometry: Geometry::Line(Line::new(Vec2::new(nx, ny - 12.0), Vec2::new(nx, ny + 12.0))),
        });
        out.push(TemplateEntity {
            layer: "Annotation",
            layer_color: [120, 200, 255],
            geometry: Geometry::Polyline(Polyline {
                vertices: vec![
                    Vec2::new(nx - 4.0, ny + 4.0),
                    Vec2::new(nx, ny + 12.0),
                    Vec2::new(nx + 4.0, ny + 4.0),
                ],
                closed: false,
            }),
        });
        out.push(TemplateEntity {
            layer: "Texte",
            layer_color: [120, 200, 255],
            geometry: Geometry::Text(TextGeom {
                position: Vec2::new(nx - 2.0, ny + 14.0),
                height: 3.0,
                rotation: 0.0,
                content: "N".into(),
                align: TextAlign::Left,
                style_id: None,
                annotative: false,
            }),
        });
    }

    if matches!(style, TitleStyle::Arch) {
        // Scale bar 0—10—20 m (graphic).
        let bx = margin + 50.0;
        let by = margin + 8.0;
        let unit = 5.0;
        out.push(TemplateEntity {
            layer: "Annotation",
            layer_color: [200, 200, 200],
            geometry: Geometry::Line(Line::new(Vec2::new(bx, by), Vec2::new(bx + unit * 4.0, by))),
        });
        for i in 0..=4 {
            let x = bx + unit * i as f64;
            out.push(TemplateEntity {
                layer: "Annotation",
                layer_color: [200, 200, 200],
                geometry: Geometry::Line(Line::new(Vec2::new(x, by), Vec2::new(x, by + 3.0))),
            });
        }
        out.push(TemplateEntity {
            layer: "Texte",
            layer_color: [200, 200, 200],
            geometry: Geometry::Text(TextGeom {
                position: Vec2::new(bx, by + 4.0),
                height: 2.0,
                rotation: 0.0,
                content: "0          20 m".into(),
                align: TextAlign::Left,
                style_id: None,
                annotative: false,
            }),
        });
    }

    if matches!(style, TitleStyle::Mech) {
        // Revision table stub on the left of title block.
        let rx0 = tx0 - 55.0;
        if rx0 > margin {
            out.push(TemplateEntity {
                layer: "Cartouche",
                layer_color: [255, 180, 60],
                geometry: Geometry::Polyline(Polyline {
                    vertices: vec![
                        Vec2::new(rx0, ty0),
                        Vec2::new(tx0, ty0),
                        Vec2::new(tx0, ty0 + th),
                        Vec2::new(rx0, ty0 + th),
                    ],
                    closed: true,
                }),
            });
            out.push(TemplateEntity {
                layer: "Texte",
                layer_color: [255, 220, 120],
                geometry: Geometry::Text(TextGeom {
                    position: Vec2::new(rx0 + 2.0, ty0 + th - 6.0),
                    height: 2.5,
                    rotation: 0.0,
                    content: "Rev / Indice".into(),
                    align: TextAlign::Left,
                    style_id: None,
                    annotative: false,
                }),
            });
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a3_has_frame_and_title() {
        let e = template_entities(TemplateKind::A3Landscape);
        assert!(e.len() >= 8);
        assert!(e.iter().any(|t| t.layer == "Cadre"));
        assert!(e.iter().any(|t| matches!(t.geometry, Geometry::Text(_))));
    }

    #[test]
    fn arch_has_north() {
        let e = template_entities(TemplateKind::ArchA3);
        assert!(e.iter().any(|t| match &t.geometry {
            Geometry::Text(t) => t.content == "N",
            _ => false,
        }));
    }

    #[test]
    fn parse_aliases() {
        assert_eq!(TemplateKind::parse("a1"), Some(TemplateKind::A1Landscape));
        assert_eq!(TemplateKind::parse("arch"), Some(TemplateKind::ArchA3));
    }
}
