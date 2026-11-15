//! Deterministic FR/EN CAD utterance → tool calls (evals / offline UI).

use serde_json::json;

use crate::error::AiError;
use crate::tools::ToolCall;

/// Parse a natural-language CAD request into ordered tool calls.
pub fn plan_utterance(utterance: &str) -> Result<Vec<ToolCall>, AiError> {
    let raw = utterance.trim();
    if raw.is_empty() {
        return Err(AiError::Unplanned("empty utterance".into()));
    }
    let lower = raw.to_lowercase();
    // Normalize common unit/× glyphs for matching.
    let norm = lower
        .replace('×', "x")
        .replace(" mètres", " m")
        .replace(" metre", " m")
        .replace(" meters", " m")
        .replace(" meter", " m");

    let mut calls = Vec::new();

    // Compound: rectangle + cote + move (acceptance utterance).
    if let Some((w, h)) = parse_rectangle_wh(&norm) {
        calls.push(ToolCall {
            name: "draw_rectangle".into(),
            args: json!({ "width": w, "height": h, "origin": [0.0, 0.0] }),
        });
        if looks_like_dimension(&norm) {
            // Dimension after create: use width edge via entity from subsequent resolve.
            // Planner emits def1/def2 for the bottom edge of the rectangle at origin.
            calls.push(ToolCall {
                name: "add_dimension".into(),
                args: json!({
                    "def1": [0.0, 0.0],
                    "def2": [w, 0.0],
                    "dim_line": [w * 0.5, -0.5]
                }),
            });
        }
        if let Some(delta) = parse_move_delta(&norm) {
            calls.push(ToolCall {
                name: "move".into(),
                args: json!({ "delta": [delta.0, delta.1] }),
            });
        }
        return Ok(calls);
    }

    if let Some((start, end)) = parse_line(&norm) {
        calls.push(ToolCall {
            name: "draw_line".into(),
            args: json!({ "start": [start.0, start.1], "end": [end.0, end.1] }),
        });
        return Ok(calls);
    }

    if let Some((center, radius)) = parse_circle(&norm) {
        calls.push(ToolCall {
            name: "draw_circle".into(),
            args: json!({ "center": [center.0, center.1], "radius": radius }),
        });
        return Ok(calls);
    }

    if let Some((pos, text)) = parse_text(&norm, raw) {
        calls.push(ToolCall {
            name: "draw_text".into(),
            args: json!({ "position": [pos.0, pos.1], "content": text, "height": 0.25 }),
        });
        return Ok(calls);
    }

    if let Some(delta) = parse_move_delta(&norm) {
        if norm.contains("copie") || norm.contains("copy") {
            calls.push(ToolCall {
                name: "copy".into(),
                args: json!({ "delta": [delta.0, delta.1] }),
            });
        } else {
            calls.push(ToolCall {
                name: "move".into(),
                args: json!({ "delta": [delta.0, delta.1] }),
            });
        }
        return Ok(calls);
    }

    if let Some((origin, degrees)) = parse_rotate(&norm) {
        calls.push(ToolCall {
            name: "rotate".into(),
            args: json!({ "origin": [origin.0, origin.1], "degrees": degrees }),
        });
        return Ok(calls);
    }

    if let Some((origin, factor)) = parse_scale(&norm) {
        calls.push(ToolCall {
            name: "scale".into(),
            args: json!({ "origin": [origin.0, origin.1], "factor": factor }),
        });
        return Ok(calls);
    }

    if let Some((a, b)) = parse_measure_distance(&norm) {
        calls.push(ToolCall {
            name: "measure".into(),
            args: json!({ "mode": "distance", "a": [a.0, a.1], "b": [b.0, b.1] }),
        });
        return Ok(calls);
    }

    if (norm.contains("calque") || norm.contains("layer"))
        && (norm.contains("liste") || norm.contains("list") || norm.contains("quels"))
    {
        calls.push(ToolCall {
            name: "list_layers".into(),
            args: json!({}),
        });
        return Ok(calls);
    }

    if norm.contains("résumé")
        || norm.contains("resume")
        || norm.contains("summary")
        || norm.contains("combien")
        || norm.contains("summarize")
    {
        calls.push(ToolCall {
            name: "summarize_document".into(),
            args: json!({}),
        });
        return Ok(calls);
    }

    if norm.contains("nettoie")
        || norm.contains("nettoyage")
        || norm.contains("cleanup")
        || norm.contains("clean up")
    {
        calls.push(ToolCall {
            name: "cleanup_drawing".into(),
            args: json!({}),
        });
        return Ok(calls);
    }

    if norm.contains("cote auto")
        || norm.contains("cotation auto")
        || norm.contains("auto dimension")
        || norm.contains("autodim")
        || (norm.contains("cote") && norm.contains("automat"))
    {
        calls.push(ToolCall {
            name: "auto_dimension".into(),
            args: json!({}),
        });
        return Ok(calls);
    }

    if norm.contains("contrôle")
        || norm.contains("controle")
        || norm.contains("check")
        || norm.contains("vérifie")
        || norm.contains("verifie")
    {
        let ruleset = if norm.contains("pmr") {
            "pmr_distances"
        } else {
            "default"
        };
        calls.push(ToolCall {
            name: "run_check".into(),
            args: json!({ "ruleset": ruleset }),
        });
        return Ok(calls);
    }

    if (norm.contains("bloc") || norm.contains("block"))
        && (norm.contains("liste") || norm.contains("list"))
    {
        calls.push(ToolCall {
            name: "list_blocks".into(),
            args: json!({}),
        });
        return Ok(calls);
    }

    if norm.contains("sélection") || norm.contains("selection") {
        calls.push(ToolCall {
            name: "get_selection".into(),
            args: json!({}),
        });
        return Ok(calls);
    }

    if norm.contains("entité") || norm.contains("entite") || norm.contains("entities") {
        calls.push(ToolCall {
            name: "query_entities".into(),
            args: json!({}),
        });
        return Ok(calls);
    }

    if norm.contains("vue") || norm.contains("view") {
        calls.push(ToolCall {
            name: "get_view".into(),
            args: json!({}),
        });
        return Ok(calls);
    }

    if let Some(d) = parse_offset_distance(&norm) {
        calls.push(ToolCall {
            name: "offset".into(),
            args: json!({ "distance": d }),
        });
        return Ok(calls);
    }

    if looks_like_dimension(&norm) {
        // Dimension selection / all — expects entity_ids from context; use first line-like via measure plan.
        calls.push(ToolCall {
            name: "add_dimension".into(),
            args: json!({
                "def1": [0.0, 0.0],
                "def2": [1.0, 0.0],
                "dim_line": [0.5, 0.5]
            }),
        });
        return Ok(calls);
    }

    Err(AiError::Unplanned(raw.to_string()))
}

fn looks_like_dimension(s: &str) -> bool {
    s.contains("cote")
        || s.contains("côte")
        || s.contains("dimension")
        || s.contains("cote-le")
        || s.contains("cote le")
        || s.contains("dim it")
        || s.contains("dimension it")
}

fn parse_rectangle_wh(s: &str) -> Option<(f64, f64)> {
    if !(s.contains("rectangle") || s.contains("rectang")) {
        return None;
    }
    // Patterns: 4 m x 3 m, 4x3, 4 × 3 m, width 4 height 3
    if let Some(caps) = capture_two_numbers(s) {
        return Some(caps);
    }
    None
}

fn capture_two_numbers(s: &str) -> Option<(f64, f64)> {
    let nums = all_numbers(s);
    if nums.len() >= 2 {
        Some((nums[0], nums[1]))
    } else {
        None
    }
}

fn parse_move_delta(s: &str) -> Option<(f64, f64)> {
    let move_idx = [
        "déplace",
        "deplace",
        "move",
        "copie",
        "copy",
        "vers la droite",
        "to the right",
        "vers la gauche",
        "to the left",
    ]
    .iter()
    .filter_map(|k| s.find(k))
    .min();
    let idx = move_idx?;
    let focus = &s[idx..];
    // Explicit delta
    if let Some(i) = focus.find("delta") {
        if let Some((a, b)) = capture_two_numbers(&focus[i..]) {
            return Some((a, b));
        }
    }
    let dist =
        first_number_near_keywords(focus).or_else(|| all_numbers(focus).into_iter().next())?;

    if focus.contains("droite") || focus.contains("right") {
        return Some((dist, 0.0));
    }
    if focus.contains("gauche") || focus.contains("left") {
        return Some((-dist, 0.0));
    }
    if focus.contains("haut") || focus.contains(" upward") || focus.ends_with(" up") {
        return Some((0.0, dist));
    }
    if focus.contains("bas") || focus.contains("down") {
        return Some((0.0, -dist));
    }
    let nums = all_numbers(focus);
    if nums.len() >= 2 {
        return Some((nums[0], nums[1]));
    }
    Some((dist, 0.0))
}

fn first_number_near_keywords(s: &str) -> Option<f64> {
    for key in ["de ", "by ", "of "] {
        if let Some(i) = s.find(key) {
            let sub = &s[i + key.len()..];
            return all_numbers(sub).into_iter().next();
        }
    }
    None
}

fn parse_line(s: &str) -> Option<((f64, f64), (f64, f64))> {
    if !(s.contains("ligne") || s.contains("line") || s.contains("trace")) {
        return None;
    }
    if s.contains("rectangle") || s.contains("cercle") || s.contains("circle") {
        return None;
    }
    let nums = all_numbers(s);
    if nums.len() >= 4 {
        Some(((nums[0], nums[1]), (nums[2], nums[3])))
    } else if nums.len() == 2 {
        // Length along +X from origin when only one scalar pair is present.
        Some(((0.0, 0.0), (nums[0], 0.0)))
    } else if nums.len() == 1 {
        Some(((0.0, 0.0), (nums[0], 0.0)))
    } else {
        None
    }
}

fn parse_circle(s: &str) -> Option<((f64, f64), f64)> {
    if !(s.contains("cercle") || s.contains("circle")) {
        return None;
    }
    let nums = all_numbers(s);
    if nums.len() >= 3 {
        Some(((nums[0], nums[1]), nums[2]))
    } else if nums.len() == 1 {
        Some(((0.0, 0.0), nums[0]))
    } else {
        None
    }
}

fn parse_text(s: &str, raw: &str) -> Option<((f64, f64), String)> {
    if !(s.contains("texte") || s.contains("text") || s.contains("écrire") || s.contains("ecrire"))
    {
        return None;
    }
    // Content between quotes if present.
    let content = if let Some(i) = raw.find('"') {
        let rest = &raw[i + 1..];
        rest.find('"').map(|j| rest[..j].to_string())
    } else if let Some(i) = raw.find('«') {
        let rest = &raw[i + '«'.len_utf8()..];
        rest.find('»').map(|j| rest[..j].to_string())
    } else {
        Some("KELI".into())
    }?;
    let pos = capture_two_numbers(s).unwrap_or((0.0, 0.0));
    Some((pos, content))
}

fn parse_rotate(s: &str) -> Option<((f64, f64), f64)> {
    if !(s.contains("rotation") || s.contains("rotate") || s.contains("pivote")) {
        return None;
    }
    let nums = all_numbers(s);
    let degrees = *nums.last()?;
    let origin = if nums.len() >= 3 {
        (nums[0], nums[1])
    } else {
        (0.0, 0.0)
    };
    Some((origin, degrees))
}

fn parse_scale(s: &str) -> Option<((f64, f64), f64)> {
    if !(s.contains("échelle") || s.contains("echelle") || s.contains("scale")) {
        return None;
    }
    let nums = all_numbers(s);
    let factor = *nums.last()?;
    let origin = if nums.len() >= 3 {
        (nums[0], nums[1])
    } else {
        (0.0, 0.0)
    };
    Some((origin, factor))
}

fn parse_measure_distance(s: &str) -> Option<((f64, f64), (f64, f64))> {
    if !(s.contains("mesure") || s.contains("measure") || s.contains("distance")) {
        return None;
    }
    let nums = all_numbers(s);
    if nums.len() >= 4 {
        Some(((nums[0], nums[1]), (nums[2], nums[3])))
    } else {
        None
    }
}

fn parse_offset_distance(s: &str) -> Option<f64> {
    if !(s.contains("offset") || s.contains("décalage") || s.contains("decalage")) {
        return None;
    }
    all_numbers(s).last().copied()
}

fn all_numbers(s: &str) -> Vec<f64> {
    // Commas separate coordinates; decimals use '.'.
    let mut nums = Vec::new();
    let mut cur = String::new();
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '-' && cur.is_empty() {
            if chars
                .peek()
                .is_some_and(|p| p.is_ascii_digit() || *p == '.')
            {
                cur.push('-');
            }
        } else if c.is_ascii_digit() || c == '.' {
            cur.push(c);
        } else if !cur.is_empty() {
            if let Ok(v) = cur.parse() {
                nums.push(v);
            }
            cur.clear();
        }
    }
    if !cur.is_empty() {
        if let Ok(v) = cur.parse() {
            nums.push(v);
        }
    }
    nums
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn acceptance_utterance_plans_three_tools() {
        let u = "dessine un rectangle de 4 m × 3 m, cote-le et déplace-le de 2 m vers la droite";
        let plan = plan_utterance(u).unwrap();
        assert_eq!(plan.len(), 3);
        assert_eq!(plan[0].name, "draw_rectangle");
        assert_eq!(plan[1].name, "add_dimension");
        assert_eq!(plan[2].name, "move");
        assert_eq!(plan[0].args["width"], 4.0);
        assert_eq!(plan[0].args["height"], 3.0);
        assert_eq!(plan[2].args["delta"][0], 2.0);
    }

    #[test]
    fn cleanup_and_autodim_and_check() {
        let a = plan_utterance("nettoie le dessin").unwrap();
        assert_eq!(a[0].name, "cleanup_drawing");
        let b = plan_utterance("cote auto").unwrap();
        assert_eq!(b[0].name, "auto_dimension");
        let c = plan_utterance("vérifie PMR").unwrap();
        assert_eq!(c[0].name, "run_check");
        assert_eq!(c[0].args["ruleset"], "pmr_distances");
    }
}
