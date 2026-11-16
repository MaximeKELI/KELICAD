//! JSON encoding of geometry components for the entities table.

use kelicad_doc::{BlockId, EntityId};
use kelicad_doc::{
    DimKind, DimensionGeom, Geometry, HatchGeom, HatchPattern, InsertGeom, LeaderGeom, MTextGeom,
    RayGeom, TextAlign, TextGeom, XLineGeom, XrefId, XrefInsertGeom,
};
use kelicad_geom::{Arc, Circle, Ellipse, Line, Polyline, Spline};
use kelicad_math::Vec2;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::IoError;

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum GeomDto {
    Point {
        x: f64,
        y: f64,
    },
    Line {
        x0: f64,
        y0: f64,
        x1: f64,
        y1: f64,
    },
    Circle {
        cx: f64,
        cy: f64,
        r: f64,
    },
    Arc {
        cx: f64,
        cy: f64,
        r: f64,
        start_angle: f64,
        end_angle: f64,
        clockwise: bool,
    },
    Polyline {
        vertices: Vec<[f64; 2]>,
        closed: bool,
    },
    Text {
        x: f64,
        y: f64,
        height: f64,
        rotation: f64,
        content: String,
        align: String,
    },
    Mtext {
        x: f64,
        y: f64,
        height: f64,
        rotation: f64,
        content: String,
        width: f64,
    },
    Dimension {
        kind: String,
        x1: f64,
        y1: f64,
        x2: f64,
        y2: f64,
        dx: f64,
        dy: f64,
        text_override: Option<String>,
        assoc_a: Option<String>,
        assoc_b: Option<String>,
    },
    Hatch {
        boundary: Vec<[f64; 2]>,
        pattern: String,
    },
    Ellipse {
        cx: f64,
        cy: f64,
        mjx: f64,
        mjy: f64,
        ratio: f64,
        start_angle: f64,
        end_angle: f64,
    },
    Spline {
        degree: u8,
        closed: bool,
        controls: Vec<[f64; 2]>,
        #[serde(default)]
        fit: Vec<[f64; 2]>,
    },
    Insert {
        block_id: String,
        x: f64,
        y: f64,
        sx: f64,
        sy: f64,
        rotation: f64,
    },
    Leader {
        vertices: Vec<[f64; 2]>,
        text: String,
        text_height: f64,
        has_arrow: bool,
    },
    Ray {
        ox: f64,
        oy: f64,
        dx: f64,
        dy: f64,
    },
    Xline {
        x: f64,
        y: f64,
        dx: f64,
        dy: f64,
    },
    Xref {
        xref_id: String,
        x: f64,
        y: f64,
        sx: f64,
        sy: f64,
        rotation: f64,
        fade: f64,
    },
}

/// Serialize geometry to JSON text.
pub fn geometry_to_json(g: &Geometry) -> Result<String, IoError> {
    let dto = match g {
        Geometry::Point(p) => GeomDto::Point { x: p.x, y: p.y },
        Geometry::Line(l) => GeomDto::Line {
            x0: l.start.x,
            y0: l.start.y,
            x1: l.end.x,
            y1: l.end.y,
        },
        Geometry::Circle(c) => GeomDto::Circle {
            cx: c.center.x,
            cy: c.center.y,
            r: c.radius,
        },
        Geometry::Arc(a) => GeomDto::Arc {
            cx: a.center.x,
            cy: a.center.y,
            r: a.radius,
            start_angle: a.start_angle,
            end_angle: a.end_angle,
            clockwise: a.clockwise,
        },
        Geometry::Polyline(p) => GeomDto::Polyline {
            vertices: p.vertices.iter().map(|v| [v.x, v.y]).collect(),
            closed: p.closed,
        },
        Geometry::Text(t) => GeomDto::Text {
            x: t.position.x,
            y: t.position.y,
            height: t.height,
            rotation: t.rotation,
            content: t.content.clone(),
            align: match t.align {
                TextAlign::Left => "left",
                TextAlign::Center => "center",
                TextAlign::Right => "right",
                TextAlign::Middle => "middle",
            }
            .into(),
        },
        Geometry::MText(t) => GeomDto::Mtext {
            x: t.position.x,
            y: t.position.y,
            height: t.height,
            rotation: t.rotation,
            content: t.content.clone(),
            width: t.width,
        },
        Geometry::Dimension(d) => GeomDto::Dimension {
            kind: match d.kind {
                DimKind::Linear => "linear",
                DimKind::Aligned => "aligned",
                DimKind::Radius => "radius",
                DimKind::Diameter => "diameter",
                DimKind::Angular => "angular",
            }
            .into(),
            x1: d.def1.x,
            y1: d.def1.y,
            x2: d.def2.x,
            y2: d.def2.y,
            dx: d.dim_line.x,
            dy: d.dim_line.y,
            text_override: d.text_override.clone(),
            assoc_a: d.assoc_a.map(|id| id.to_string()),
            assoc_b: d.assoc_b.map(|id| id.to_string()),
        },
        Geometry::Hatch(h) => GeomDto::Hatch {
            boundary: h.boundary.iter().map(|v| [v.x, v.y]).collect(),
            pattern: match &h.pattern {
                HatchPattern::Solid => "SOLID".into(),
                HatchPattern::Named(n) => n.clone(),
            },
        },
        Geometry::Ellipse(e) => GeomDto::Ellipse {
            cx: e.center.x,
            cy: e.center.y,
            mjx: e.major.x,
            mjy: e.major.y,
            ratio: e.ratio,
            start_angle: e.start_angle,
            end_angle: e.end_angle,
        },
        Geometry::Spline(s) => GeomDto::Spline {
            degree: s.degree,
            closed: s.closed,
            controls: s.control_points.iter().map(|v| [v.x, v.y]).collect(),
            fit: s.fit_points.iter().map(|v| [v.x, v.y]).collect(),
        },
        Geometry::Insert(i) => GeomDto::Insert {
            block_id: i.block_id.to_string(),
            x: i.position.x,
            y: i.position.y,
            sx: i.scale.x,
            sy: i.scale.y,
            rotation: i.rotation,
        },
        Geometry::Leader(l) => GeomDto::Leader {
            vertices: l.vertices.iter().map(|v| [v.x, v.y]).collect(),
            text: l.text.clone(),
            text_height: l.text_height,
            has_arrow: l.has_arrow,
        },
        Geometry::Ray(r) => GeomDto::Ray {
            ox: r.origin.x,
            oy: r.origin.y,
            dx: r.direction.x,
            dy: r.direction.y,
        },
        Geometry::XLine(x) => GeomDto::Xline {
            x: x.point.x,
            y: x.point.y,
            dx: x.direction.x,
            dy: x.direction.y,
        },
        Geometry::Xref(x) => GeomDto::Xref {
            xref_id: x.xref_id.to_string(),
            x: x.position.x,
            y: x.position.y,
            sx: x.scale.x,
            sy: x.scale.y,
            rotation: x.rotation,
            fade: x.fade,
        },
    };
    Ok(serde_json::to_string(&dto)?)
}

/// Deserialize geometry from JSON text.
pub fn geometry_from_json(s: &str) -> Result<Geometry, IoError> {
    let dto: GeomDto = serde_json::from_str(s)?;
    Ok(match dto {
        GeomDto::Point { x, y } => Geometry::Point(Vec2::new(x, y)),
        GeomDto::Line { x0, y0, x1, y1 } => {
            Geometry::Line(Line::new(Vec2::new(x0, y0), Vec2::new(x1, y1)))
        }
        GeomDto::Circle { cx, cy, r } => Geometry::Circle(Circle::new(Vec2::new(cx, cy), r)),
        GeomDto::Arc {
            cx,
            cy,
            r,
            start_angle,
            end_angle,
            clockwise,
        } => Geometry::Arc(Arc {
            center: Vec2::new(cx, cy),
            radius: r,
            start_angle,
            end_angle,
            clockwise,
        }),
        GeomDto::Polyline { vertices, closed } => {
            let verts = vertices.into_iter().map(|[x, y]| Vec2::new(x, y)).collect();
            Geometry::Polyline(if closed {
                Polyline::closed(verts)
            } else {
                Polyline::new(verts)
            })
        }
        GeomDto::Text {
            x,
            y,
            height,
            rotation,
            content,
            align,
        } => Geometry::Text(TextGeom {
            position: Vec2::new(x, y),
            height,
            rotation,
            content,
            align: match align.as_str() {
                "center" => TextAlign::Center,
                "right" => TextAlign::Right,
                "middle" => TextAlign::Middle,
                _ => TextAlign::Left,
            },
            style_id: None,
            annotative: false,
}),
        GeomDto::Mtext {
            x,
            y,
            height,
            rotation,
            content,
            width,
        } => {
            let mut mt = MTextGeom::new(Vec2::new(x, y), height, content);
            mt.rotation = rotation;
            mt.width = width;
            Geometry::MText(mt)
        }
        GeomDto::Dimension {
            kind,
            x1,
            y1,
            x2,
            y2,
            dx,
            dy,
            text_override,
            assoc_a,
            assoc_b,
        } => Geometry::Dimension(DimensionGeom {
            kind: match kind.as_str() {
                "aligned" => DimKind::Aligned,
                "radius" => DimKind::Radius,
                "diameter" => DimKind::Diameter,
                "angular" => DimKind::Angular,
                _ => DimKind::Linear,
            },
            def1: Vec2::new(x1, y1),
            def2: Vec2::new(x2, y2),
            dim_line: Vec2::new(dx, dy),
            assoc_a: assoc_a
                .as_deref()
                .and_then(|s| Uuid::parse_str(s).ok())
                .map(EntityId::from_uuid),
            assoc_b: assoc_b
                .as_deref()
                .and_then(|s| Uuid::parse_str(s).ok())
                .map(EntityId::from_uuid),
            text_override,
            style_id: None,
            annotative: false,
            angle_p3: None,
}),
        GeomDto::Hatch { boundary, pattern } => Geometry::Hatch(HatchGeom {
            boundary: boundary.into_iter().map(|[x, y]| Vec2::new(x, y)).collect(),
            pattern: if pattern.eq_ignore_ascii_case("SOLID") {
                HatchPattern::Solid
            } else {
                HatchPattern::Named(pattern)
            },
            scale: 1.0,
            angle: 0.0,
        }),
        GeomDto::Ellipse {
            cx,
            cy,
            mjx,
            mjy,
            ratio,
            start_angle,
            end_angle,
        } => {
            let mut e = Ellipse {
                center: Vec2::new(cx, cy),
                major: Vec2::new(mjx, mjy),
                ratio,
                start_angle,
                end_angle,
            };
            if (end_angle - start_angle).abs() >= std::f64::consts::TAU - 1e-6
                || (start_angle == 0.0 && end_angle == 0.0)
            {
                e.start_angle = 0.0;
                e.end_angle = 0.0;
            }
            Geometry::Ellipse(e)
        }
        GeomDto::Spline {
            degree,
            closed,
            controls,
            fit,
        } => {
            let mut s = Spline::from_controls(
                controls.into_iter().map(|[x, y]| Vec2::new(x, y)).collect(),
                degree,
                closed,
            )
            .ok_or_else(|| IoError::Format("spline needs enough controls".into()))?;
            s.fit_points = fit.into_iter().map(|[x, y]| Vec2::new(x, y)).collect();
            Geometry::Spline(s)
        }
        GeomDto::Insert {
            block_id,
            x,
            y,
            sx,
            sy,
            rotation,
        } => Geometry::Insert(InsertGeom {
            block_id: BlockId::from_uuid(Uuid::parse_str(&block_id)?),
            position: Vec2::new(x, y),
            scale: Vec2::new(sx, sy),
            rotation,
            attribs: Default::default(),
        }),
        GeomDto::Leader {
            vertices,
            text,
            text_height,
            has_arrow,
        } => Geometry::Leader(LeaderGeom {
            vertices: vertices.iter().map(|v| Vec2::new(v[0], v[1])).collect(),
            text,
            text_height,
            has_arrow,
            style_id: None,
        }),
        GeomDto::Ray { ox, oy, dx, dy } => Geometry::Ray(RayGeom {
            origin: Vec2::new(ox, oy),
            direction: Vec2::new(dx, dy),
        }),
        GeomDto::Xline { x, y, dx, dy } => Geometry::XLine(XLineGeom {
            point: Vec2::new(x, y),
            direction: Vec2::new(dx, dy),
        }),
        GeomDto::Xref {
            xref_id,
            x,
            y,
            sx,
            sy,
            rotation,
            fade,
        } => Geometry::Xref(XrefInsertGeom {
            xref_id: XrefId::from_uuid(
                Uuid::parse_str(&xref_id).map_err(|e| IoError::Format(e.to_string()))?,
            ),
            position: Vec2::new(x, y),
            scale: Vec2::new(sx, sy),
            rotation,
            clip: None,
            fade,
        }),
    })
}
