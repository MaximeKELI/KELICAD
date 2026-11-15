//! CSV nomenclature / schedule export.

use crate::element::BimElement;

/// Build a CSV schedule. `columns` are property keys; always includes kind, id, tag.
pub fn schedule_csv(elements: &[BimElement], columns: &[&str]) -> String {
    let mut cols: Vec<String> = vec!["kind".into(), "id".into(), "tag".into()];
    for c in columns {
        let key = (*c).to_string();
        if !cols.iter().any(|x| x == &key) {
            cols.push(key);
        }
    }
    let mut out = String::new();
    out.push_str(&cols.join(","));
    out.push('\n');
    for e in elements {
        let mut row = Vec::with_capacity(cols.len());
        for c in &cols {
            let cell = match c.as_str() {
                "kind" => format!("{:?}", e.kind).to_ascii_lowercase(),
                "id" => e.id.to_string(),
                "tag" => e.props.get("tag").cloned().unwrap_or_default(),
                other => e.props.get(other).cloned().unwrap_or_default(),
            };
            row.push(escape_csv(&cell));
        }
        out.push_str(&row.join(","));
        out.push('\n');
    }
    out
}

fn escape_csv(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}
