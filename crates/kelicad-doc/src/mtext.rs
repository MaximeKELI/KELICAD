//! MText content parsing (DXF-style control codes subset).

use crate::annotation::TextAlign;

/// Attachment / alignment for multi-line text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MTextAlign {
    /// Top-left.
    #[default]
    TopLeft,
    /// Top-center.
    TopCenter,
    /// Top-right.
    TopRight,
    /// Middle-left.
    MiddleLeft,
    /// Middle-center.
    MiddleCenter,
    /// Middle-right.
    MiddleRight,
    /// Bottom-left.
    BottomLeft,
    /// Bottom-center.
    BottomCenter,
    /// Bottom-right.
    BottomRight,
}

impl MTextAlign {
    /// Map to simple horizontal TextAlign for single-line fallbacks.
    pub fn to_text_align(self) -> TextAlign {
        match self {
            Self::TopLeft | Self::MiddleLeft | Self::BottomLeft => TextAlign::Left,
            Self::TopCenter | Self::MiddleCenter | Self::BottomCenter => TextAlign::Center,
            Self::TopRight | Self::MiddleRight | Self::BottomRight => TextAlign::Right,
        }
    }
}

/// One display run after expanding MText codes.
#[derive(Debug, Clone, PartialEq)]
pub struct MTextRun {
    /// Plain text for this run.
    pub text: String,
    /// Relative height factor (1.0 = default).
    pub height_factor: f64,
    /// True if underlined.
    pub underline: bool,
    /// True if overlined.
    pub overline: bool,
    /// Optional RGB override.
    pub color_rgb: Option<[u8; 3]>,
}

/// Normalize DXF MText escapes into plain lines + runs.
///
/// Supported subset:
/// - `\P` / `\\P` → paragraph break (newline)
/// - `\~` → non-breaking space
/// - `\\` → literal backslash
/// - `\{` / `\}` → literal braces
/// - `\H#;` / `\H#x;` → height factor (applied to following text until reset)
/// - `\L` / `\l` → underline on/off
/// - `\O` / `\o` → overline on/off
/// - `\C#;` → ACI color index → rough RGB
pub fn parse_mtext(raw: &str) -> (Vec<String>, Vec<MTextRun>) {
    let normalized = raw
        .replace("\\P", "\n")
        .replace("\\p", "\n")
        .replace("\\~", "\u{00A0}");
    let lines: Vec<String> = normalized.lines().map(|s| s.to_string()).collect();

    let mut runs = Vec::new();
    let mut height = 1.0f64;
    let mut underline = false;
    let mut overline = false;
    let mut color: Option<[u8; 3]> = None;
    let mut buf = String::new();

    let flush = |buf: &mut String,
                 runs: &mut Vec<MTextRun>,
                 height: f64,
                 underline: bool,
                 overline: bool,
                 color: Option<[u8; 3]>| {
        if !buf.is_empty() {
            runs.push(MTextRun {
                text: std::mem::take(buf),
                height_factor: height,
                underline,
                overline,
                color_rgb: color,
            });
        }
    };

    let chars: Vec<char> = normalized.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '\\' && i + 1 < chars.len() {
            let next = chars[i + 1];
            match next {
                '\\' => {
                    buf.push('\\');
                    i += 2;
                }
                '{' | '}' => {
                    buf.push(next);
                    i += 2;
                }
                'L' => {
                    flush(&mut buf, &mut runs, height, underline, overline, color);
                    underline = true;
                    i += 2;
                }
                'l' => {
                    flush(&mut buf, &mut runs, height, underline, overline, color);
                    underline = false;
                    i += 2;
                }
                'O' => {
                    flush(&mut buf, &mut runs, height, underline, overline, color);
                    overline = true;
                    i += 2;
                }
                'o' => {
                    flush(&mut buf, &mut runs, height, underline, overline, color);
                    overline = false;
                    i += 2;
                }
                'H' | 'h' => {
                    flush(&mut buf, &mut runs, height, underline, overline, color);
                    i += 2;
                    let (val, ni) = read_code_number(&chars, i);
                    height = val.unwrap_or(1.0).abs().max(1e-6);
                    i = ni;
                }
                'C' | 'c' => {
                    flush(&mut buf, &mut runs, height, underline, overline, color);
                    i += 2;
                    let (val, ni) = read_code_number(&chars, i);
                    color = val.map(|v| aci_to_rgb(v as u8));
                    i = ni;
                }
                _ => {
                    // Unknown code: skip backslash + letter, keep rest.
                    buf.push(next);
                    i += 2;
                }
            }
            continue;
        }
        if chars[i] == '\n' {
            flush(&mut buf, &mut runs, height, underline, overline, color);
            buf.push('\n');
            flush(&mut buf, &mut runs, height, underline, overline, color);
            i += 1;
            continue;
        }
        buf.push(chars[i]);
        i += 1;
    }
    flush(&mut buf, &mut runs, height, underline, overline, color);

    // Plain lines without control leftovers for simple consumers.
    let plain_lines: Vec<String> = lines
        .iter()
        .map(|l| strip_simple_codes(l))
        .collect();
    let _ = plain_lines;
    let plain: Vec<String> = runs_to_plain_lines(&runs);
    (plain, runs)
}

fn runs_to_plain_lines(runs: &[MTextRun]) -> Vec<String> {
    let mut text = String::new();
    for r in runs {
        text.push_str(&r.text);
    }
    if text.is_empty() {
        return vec![String::new()];
    }
    text.lines().map(|s| s.to_string()).collect()
}

fn strip_simple_codes(s: &str) -> String {
    // Best-effort for display without full parse.
    s.replace("\\L", "")
        .replace("\\l", "")
        .replace("\\O", "")
        .replace("\\o", "")
}

fn read_code_number(chars: &[char], mut i: usize) -> (Option<f64>, usize) {
    let start = i;
    while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.' || chars[i] == '-') {
        i += 1;
    }
    let num: Option<f64> = if i > start {
        chars[start..i].iter().collect::<String>().parse().ok()
    } else {
        None
    };
    // Optional trailing `x` (relative) and `;`
    if i < chars.len() && (chars[i] == 'x' || chars[i] == 'X') {
        i += 1;
    }
    if i < chars.len() && chars[i] == ';' {
        i += 1;
    }
    (num, i)
}

fn aci_to_rgb(aci: u8) -> [u8; 3] {
    match aci {
        1 => [255, 0, 0],
        2 => [255, 255, 0],
        3 => [0, 255, 0],
        4 => [0, 255, 255],
        5 => [0, 0, 255],
        6 => [255, 0, 255],
        7 => [255, 255, 255],
        8 => [128, 128, 128],
        _ => [200, 200, 200],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paragraph_and_nbsp() {
        let (lines, _) = parse_mtext("Hello\\PWorld\\~x");
        assert_eq!(lines.len(), 2);
        assert!(lines[1].contains('\u{00A0}') || lines[1].contains('x'));
    }

    #[test]
    fn underline_and_height() {
        let (_, runs) = parse_mtext("A\\Lbold\\l \\H2;Big");
        assert!(runs.iter().any(|r| r.underline && r.text.contains("bold")));
        assert!(runs.iter().any(|r| (r.height_factor - 2.0).abs() < 1e-9));
    }
}
