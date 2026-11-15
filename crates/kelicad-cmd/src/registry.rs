//! Command identifiers and aliases.

/// Built-in command id.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CommandId {
    /// LINE
    Line,
    /// PLINE
    Pline,
    /// CIRCLE
    Circle,
    /// ARC
    Arc,
    /// RECTANG
    Rectang,
    /// MOVE
    Move,
    /// COPY
    Copy,
    /// ROTATE
    Rotate,
    /// SCALE
    Scale,
    /// MIRROR
    Mirror,
    /// OFFSET
    Offset,
    /// TRIM
    Trim,
    /// EXTEND
    Extend,
    /// ERASE
    Erase,
    /// UNDO
    Undo,
    /// REDO
    Redo,
    /// TEXT
    Text,
    /// DIMLINEAR
    DimLinear,
    /// HATCH
    Hatch,
    /// BLOCK
    Block,
    /// INSERT
    Insert,
}

impl CommandId {
    /// Canonical name.
    pub fn name(self) -> &'static str {
        match self {
            Self::Line => "LINE",
            Self::Pline => "PLINE",
            Self::Circle => "CIRCLE",
            Self::Arc => "ARC",
            Self::Rectang => "RECTANG",
            Self::Move => "MOVE",
            Self::Copy => "COPY",
            Self::Rotate => "ROTATE",
            Self::Scale => "SCALE",
            Self::Mirror => "MIRROR",
            Self::Offset => "OFFSET",
            Self::Trim => "TRIM",
            Self::Extend => "EXTEND",
            Self::Erase => "ERASE",
            Self::Undo => "UNDO",
            Self::Redo => "REDO",
            Self::Text => "TEXT",
            Self::DimLinear => "DIMLINEAR",
            Self::Hatch => "HATCH",
            Self::Block => "BLOCK",
            Self::Insert => "INSERT",
        }
    }

    /// Resolve a token (name or alias), case-insensitive.
    pub fn parse(token: &str) -> Option<Self> {
        let t = token.trim().to_ascii_uppercase();
        Some(match t.as_str() {
            "LINE" | "L" => Self::Line,
            "PLINE" | "PL" => Self::Pline,
            "CIRCLE" | "C" => Self::Circle,
            "ARC" | "A" => Self::Arc,
            "RECTANG" | "REC" | "RECTANGLE" => Self::Rectang,
            "MOVE" | "M" => Self::Move,
            "COPY" | "CO" | "CP" => Self::Copy,
            "ROTATE" | "RO" => Self::Rotate,
            "SCALE" | "SC" => Self::Scale,
            "MIRROR" | "MI" => Self::Mirror,
            "OFFSET" | "O" => Self::Offset,
            "TRIM" | "TR" => Self::Trim,
            "EXTEND" | "EX" => Self::Extend,
            "ERASE" | "E" | "DELETE" => Self::Erase,
            "UNDO" | "U" => Self::Undo,
            "REDO" => Self::Redo,
            "TEXT" | "DT" => Self::Text,
            "DIMLINEAR" | "DIMLIN" | "DLI" => Self::DimLinear,
            "HATCH" | "H" => Self::Hatch,
            "BLOCK" | "B" => Self::Block,
            "INSERT" | "I" => Self::Insert,
            _ => return None,
        })
    }
}
