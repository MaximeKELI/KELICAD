//! Point and number parsing for the command line.

use kelicad_math::Vec2;
use thiserror::Error;

/// Parse errors.
#[derive(Debug, Clone, PartialEq, Error)]
pub enum InputError {
    /// Empty token.
    #[error("empty input")]
    Empty,
    /// Could not parse.
    #[error("invalid point or number: {0}")]
    Invalid(String),
}

/// Parsed absolute / relative / polar point.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ParsedPoint {
    /// Absolute `x,y`.
    Absolute(Vec2),
    /// Relative `@dx,dy` from last point.
    Relative(Vec2),
    /// Polar `@dist<angle_degrees` from last point.
    Polar {
        /// Distance.
        dist: f64,
        /// Angle in degrees (CCW from +X).
        angle_deg: f64,
    },
}

impl ParsedPoint {
    /// Resolve against an optional base point (required for relative/polar).
    pub fn resolve(self, base: Option<Vec2>) -> Result<Vec2, InputError> {
        match self {
            Self::Absolute(p) => Ok(p),
            Self::Relative(d) => {
                let b = base.ok_or_else(|| InputError::Invalid("relative needs base".into()))?;
                Ok(b + d)
            }
            Self::Polar { dist, angle_deg } => {
                let b = base.ok_or_else(|| InputError::Invalid("polar needs base".into()))?;
                let r = angle_deg.to_radians();
                Ok(b + Vec2::new(r.cos(), r.sin()) * dist)
            }
        }
    }
}

/// Parse a point token: `x,y`, `@dx,dy`, `@dist<angle`.
pub fn parse_point(raw: &str) -> Result<ParsedPoint, InputError> {
    let s = raw.trim();
    if s.is_empty() {
        return Err(InputError::Empty);
    }
    if let Some(rest) = s.strip_prefix('@') {
        if let Some((dist_s, ang_s)) = rest.split_once('<') {
            let dist: f64 = dist_s
                .trim()
                .parse()
                .map_err(|_| InputError::Invalid(s.into()))?;
            let angle_deg: f64 = ang_s
                .trim()
                .parse()
                .map_err(|_| InputError::Invalid(s.into()))?;
            return Ok(ParsedPoint::Polar { dist, angle_deg });
        }
        let (x, y) = split_xy(rest)?;
        return Ok(ParsedPoint::Relative(Vec2::new(x, y)));
    }
    let (x, y) = split_xy(s)?;
    Ok(ParsedPoint::Absolute(Vec2::new(x, y)))
}

/// Parse a scalar (optional units m/cm later).
pub fn parse_number(raw: &str) -> Result<f64, InputError> {
    let s = raw.trim();
    if s.is_empty() {
        return Err(InputError::Empty);
    }
    // Inline calc: only allow digits, operators for a tiny safe eval later;
    // M1: plain f64 (+ optional m/cm/mm suffix).
    let (num, factor) = if let Some(v) = s.strip_suffix("cm") {
        (v.trim(), 0.01)
    } else if let Some(v) = s.strip_suffix('m') {
        (v.trim(), 1.0)
    } else if let Some(v) = s.strip_suffix("mm") {
        (v.trim(), 0.001)
    } else {
        (s, 1.0)
    };
    let n: f64 = num
        .parse()
        .map_err(|_| InputError::Invalid(s.to_string()))?;
    Ok(n * factor)
}

fn split_xy(s: &str) -> Result<(f64, f64), InputError> {
    // AutoCAD: `x,y` or `x,y,z` (Z ignored in 2D). Also `x;y` / `x y`.
    let normalized = s.replace(';', ",");
    let joined = if !normalized.contains(',') {
        let toks: Vec<&str> = normalized.split_whitespace().collect();
        if toks.len() >= 2 {
            format!("{},{}", toks[0], toks[1])
        } else {
            normalized.clone()
        }
    } else {
        normalized
    };
    let parts: Vec<&str> = joined
        .split(',')
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .collect();
    if parts.len() < 2 {
        return Err(InputError::Invalid(s.into()));
    }
    let x: f64 = parts[0]
        .parse()
        .map_err(|_| InputError::Invalid(s.into()))?;
    let y: f64 = parts[1]
        .parse()
        .map_err(|_| InputError::Invalid(s.into()))?;
    // parts[2] = Z — accepted and ignored for 2D drafting.
    Ok((x, y))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn absolute() {
        let p = parse_point("10,20").unwrap();
        assert_eq!(p.resolve(None).unwrap(), Vec2::new(10.0, 20.0));
    }

    #[test]
    fn relative() {
        let p = parse_point("@3,4").unwrap();
        assert_eq!(
            p.resolve(Some(Vec2::new(1.0, 1.0))).unwrap(),
            Vec2::new(4.0, 5.0)
        );
    }

    #[test]
    fn polar() {
        let p = parse_point("@10<0").unwrap();
        let v = p.resolve(Some(Vec2::ZERO)).unwrap();
        assert!((v.x - 10.0).abs() < 1e-9);
    }

    #[test]
    fn meters() {
        assert!((parse_number("2m").unwrap() - 2.0).abs() < 1e-12);
        assert!((parse_number("50cm").unwrap() - 0.5).abs() < 1e-12);
    }

    #[test]
    fn space_and_xyz() {
        let p = parse_point("10 20").unwrap();
        assert_eq!(p.resolve(None).unwrap(), Vec2::new(10.0, 20.0));
        let p2 = parse_point("1,2,3").unwrap();
        assert_eq!(p2.resolve(None).unwrap(), Vec2::new(1.0, 2.0));
        let p3 = parse_point("4;5").unwrap();
        assert_eq!(p3.resolve(None).unwrap(), Vec2::new(4.0, 5.0));
    }
}
