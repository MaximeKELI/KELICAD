//! JSON encoding of geometry components for the entities table.

use kelicad_doc::Geometry;
use kelicad_geom::{Arc, Circle, Line, Polyline};
use kelicad_math::Vec2;
use serde::{Deserialize, Serialize};

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
            let verts = vertices
                .into_iter()
                .map(|[x, y]| Vec2::new(x, y))
                .collect();
            Geometry::Polyline(if closed {
                Polyline::closed(verts)
            } else {
                Polyline::new(verts)
            })
        }
    })
}
