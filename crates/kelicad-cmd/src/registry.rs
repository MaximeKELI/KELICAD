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
    /// DIMALIGNED
    DimAligned,
    /// DIMRADIUS
    DimRadius,
    /// DIMDIAMETER
    DimDiameter,
    /// DIMANGULAR
    DimAngular,
    /// LEADER
    Leader,
    /// HATCH
    Hatch,
    /// BLOCK
    Block,
    /// INSERT
    Insert,
    /// POLYGON (regular, center+vertex)
    Polygon,
    /// ELLIPSE (center + major tip + ratio via second radius point)
    Ellipse,
    /// SPLINE (fit points)
    Spline,
    /// CIRCLE3P
    Circle3p,
    /// CIRCLE2P (diameter)
    Circle2p,
    /// PARALLEL through point
    Parallel,
    /// MIDLINE between two points
    Midline,
    /// FILLET
    Fillet,
    /// CHAMFER
    Chamfer,
    /// Tangent from point to circle
    TangentPc,
    /// Common tangents between two circles
    TangentCc,
    /// ARRAY rectangular
    ArrayRect,
    /// ARRAY polar
    ArrayPolar,
    /// STRETCH
    Stretch,
    /// BREAK
    Break,
    /// JOIN
    Join,
    /// EXPLODE
    Explode,
    /// LENGTHEN
    Lengthen,
    /// ALIGN
    Align,
    /// RAY
    Ray,
    /// XLINE
    Xline,
    /// XREF attach
    Xref,
    /// PDF underlay attach
    PdfUnderlay,
    /// STAR (regular star polygon)
    Star,
    /// DONUT (two concentric circles)
    Donut,
    /// CAPSULE / slot (stadium)
    Capsule,
    /// ARROW (shaft + head)
    Arrow,
    /// GRID construction lines
    Grid,
    /// TRIANGLE isosceles from base + height pick
    Triangle,
    /// POINT marker
    Point,
    /// REVCLOUD revision cloud
    Revcloud,
    /// CENTERMARK on a circle
    Centermark,
    /// CENTERLINE axis with ticks
    Centerline,
    /// TABLE grid
    Table,
    /// WIPEOUT mask rectangle
    Wipeout,
    /// DIMCONTINUE — chain from last dimension
    DimContinue,
    /// DIMBASELINE — baseline from last dimension
    DimBaseline,
    /// SECTION — section cut mark with labels
    Section,
    /// SETVAR
    Setvar,
    /// GETVAR
    Getvar,
    /// PEDIT
    Pedit,
    /// MATCHPROP
    Matchprop,
    /// DRAWORDER
    Draworder,
    /// GROUP
    Group,
    /// UNGROUP
    Ungroup,
    /// OVERKILL
    Overkill,
    /// OOPS restore last erase
    Oops,
    /// DIVIDE
    Divide,
    /// MEASURE
    Measure,
    /// BOUNDARY
    Boundary,
    /// SELECT
    Select,
    /// SELECTSIMILAR
    SelectSimilar,
    /// QSELECT
    Qselect,
    /// DDPTYPE
    Ddptype,
    /// WBLOCK
    Wblock,
    /// MTEXT
    Mtext,
    /// ARRAYPATH
    ArrayPath,
    /// LAYISO
    Layiso,
    /// LAYOFF
    Layoff,
    /// LAYFRZ
    Layfrz,
    /// LAYLCK
    Laylck,
    /// LAYCUR
    Laycur,
    /// SELECTALL
    SelectAll,
    /// SELECTPREV
    SelectPrev,
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
            Self::DimAligned => "DIMALIGNED",
            Self::DimRadius => "DIMRADIUS",
            Self::DimDiameter => "DIMDIAMETER",
            Self::DimAngular => "DIMANGULAR",
            Self::Leader => "LEADER",
            Self::Hatch => "HATCH",
            Self::Block => "BLOCK",
            Self::Insert => "INSERT",
            Self::Polygon => "POLYGON",
            Self::Ellipse => "ELLIPSE",
            Self::Spline => "SPLINE",
            Self::Circle3p => "CIRCLE3P",
            Self::Circle2p => "CIRCLE2P",
            Self::Parallel => "PARALLEL",
            Self::Midline => "MIDLINE",
            Self::Fillet => "FILLET",
            Self::Chamfer => "CHAMFER",
            Self::TangentPc => "TANGENTPC",
            Self::TangentCc => "TANGENTCC",
            Self::ArrayRect => "ARRAYRECT",
            Self::ArrayPolar => "ARRAYPOLAR",
            Self::Stretch => "STRETCH",
            Self::Break => "BREAK",
            Self::Join => "JOIN",
            Self::Explode => "EXPLODE",
            Self::Lengthen => "LENGTHEN",
            Self::Align => "ALIGN",
            Self::Ray => "RAY",
            Self::Xline => "XLINE",
            Self::Xref => "XREF",
            Self::PdfUnderlay => "PDFATTACH",
            Self::Star => "STAR",
            Self::Donut => "DONUT",
            Self::Capsule => "CAPSULE",
            Self::Arrow => "ARROW",
            Self::Grid => "GRID",
            Self::Triangle => "TRIANGLE",
            Self::Point => "POINT",
            Self::Revcloud => "REVCLOUD",
            Self::Centermark => "CENTERMARK",
            Self::Centerline => "CENTERLINE",
            Self::Table => "TABLE",
            Self::Wipeout => "WIPEOUT",
            Self::DimContinue => "DIMCONTINUE",
            Self::DimBaseline => "DIMBASELINE",
            Self::Section => "SECTION",
            Self::Setvar => "SETVAR",
            Self::Getvar => "GETVAR",
            Self::Pedit => "PEDIT",
            Self::Matchprop => "MATCHPROP",
            Self::Draworder => "DRAWORDER",
            Self::Group => "GROUP",
            Self::Ungroup => "UNGROUP",
            Self::Overkill => "OVERKILL",
            Self::Oops => "OOPS",
            Self::Divide => "DIVIDE",
            Self::Measure => "MEASURE",
            Self::Boundary => "BOUNDARY",
            Self::Select => "SELECT",
            Self::SelectSimilar => "SELECTSIMILAR",
            Self::Qselect => "QSELECT",
            Self::Ddptype => "DDPTYPE",
            Self::Wblock => "WBLOCK",
            Self::Mtext => "MTEXT",
            Self::ArrayPath => "ARRAYPATH",
            Self::Layiso => "LAYISO",
            Self::Layoff => "LAYOFF",
            Self::Layfrz => "LAYFRZ",
            Self::Laylck => "LAYLCK",
            Self::Laycur => "LAYCUR",
            Self::SelectAll => "SELECTALL",
            Self::SelectPrev => "SELECTPREV",
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
            "DIMALIGNED" | "DIMALI" | "DAL" => Self::DimAligned,
            "DIMRADIUS" | "DIMRAD" | "DRA" => Self::DimRadius,
            "DIMDIAMETER" | "DIMDIA" | "DDI" => Self::DimDiameter,
            "DIMANGULAR" | "DIMANG" | "DAN" => Self::DimAngular,
            "LEADER" | "LEAD" | "MLD" => Self::Leader,
            "HATCH" | "H" => Self::Hatch,
            "BLOCK" | "B" => Self::Block,
            "INSERT" | "I" => Self::Insert,
            "POLYGON" | "POL" | "PP" => Self::Polygon,
            "ELLIPSE" | "EL" => Self::Ellipse,
            "SPLINE" | "SPL" => Self::Spline,
            "CIRCLE3P" | "C3" | "3P" => Self::Circle3p,
            "CIRCLE2P" | "C2" | "2P" => Self::Circle2p,
            "PARALLEL" | "PAR" => Self::Parallel,
            "MIDLINE" | "ML" => Self::Midline,
            "FILLET" | "F" => Self::Fillet,
            "CHAMFER" | "CHA" => Self::Chamfer,
            "TANGENTPC" | "TANPC" | "LT" => Self::TangentPc,
            "TANGENTCC" | "TANCC" | "LC" => Self::TangentCc,
            "ARRAYRECT" | "ARRAY" | "AR" => Self::ArrayRect,
            "ARRAYPOLAR" | "APOLAR" => Self::ArrayPolar,
            "STRETCH" | "S" => Self::Stretch,
            "BREAK" | "BR" => Self::Break,
            "JOIN" | "J" => Self::Join,
            "EXPLODE" | "X" => Self::Explode,
            "LENGTHEN" | "LEN" => Self::Lengthen,
            "ALIGN" | "AL" => Self::Align,
            "RAY" => Self::Ray,
            "XLINE" | "XL" => Self::Xline,
            "XREF" | "XR" => Self::Xref,
            "PDFATTACH" | "PDF" => Self::PdfUnderlay,
            "STAR" | "ETOILE" => Self::Star,
            "DONUT" | "RING" | "ANNEAU" => Self::Donut,
            "CAPSULE" | "SLOT" | "STADIUM" => Self::Capsule,
            "ARROW" | "FLECHE" => Self::Arrow,
            "GRID" | "GRILLE" => Self::Grid,
            "TRIANGLE" | "TRI" => Self::Triangle,
            "POINT" | "PO" => Self::Point,
            "REVCLOUD" | "CLOUD" | "RC" => Self::Revcloud,
            "CENTERMARK" | "CMARK" | "CM" => Self::Centermark,
            "CENTERLINE" | "CLINE" | "CL" => Self::Centerline,
            "TABLE" | "TBL" => Self::Table,
            "WIPEOUT" | "WIPE" | "WO" => Self::Wipeout,
            "DIMCONTINUE" | "DIMCONT" | "DCO" => Self::DimContinue,
            "DIMBASELINE" | "DIMBASE" | "DBA" => Self::DimBaseline,
            "SECTION" | "COUPE" | "SEC" => Self::Section,
            _ => return None,
        })
    }

    /// All built-in commands (for palettes).
    pub fn all() -> &'static [CommandId] {
        &ALL_COMMANDS
    }
}

const ALL_COMMANDS: [CommandId; 90] = [
    CommandId::Line,
    CommandId::Pline,
    CommandId::Circle,
    CommandId::Arc,
    CommandId::Rectang,
    CommandId::Move,
    CommandId::Copy,
    CommandId::Rotate,
    CommandId::Scale,
    CommandId::Mirror,
    CommandId::Offset,
    CommandId::Trim,
    CommandId::Extend,
    CommandId::Erase,
    CommandId::Undo,
    CommandId::Redo,
    CommandId::Text,
    CommandId::DimLinear,
    CommandId::DimAligned,
    CommandId::DimRadius,
    CommandId::DimDiameter,
    CommandId::DimAngular,
    CommandId::Leader,
    CommandId::Hatch,
    CommandId::Block,
    CommandId::Insert,
    CommandId::Polygon,
    CommandId::Ellipse,
    CommandId::Spline,
    CommandId::Circle3p,
    CommandId::Circle2p,
    CommandId::Parallel,
    CommandId::Midline,
    CommandId::Fillet,
    CommandId::Chamfer,
    CommandId::TangentPc,
    CommandId::TangentCc,
    CommandId::ArrayRect,
    CommandId::ArrayPolar,
    CommandId::Stretch,
    CommandId::Break,
    CommandId::Join,
    CommandId::Explode,
    CommandId::Lengthen,
    CommandId::Align,
    CommandId::Ray,
    CommandId::Xline,
    CommandId::Xref,
    CommandId::PdfUnderlay,
    CommandId::Star,
    CommandId::Donut,
    CommandId::Capsule,
    CommandId::Arrow,
    CommandId::Grid,
    CommandId::Triangle,
    CommandId::Point,
    CommandId::Revcloud,
    CommandId::Centermark,
    CommandId::Centerline,
    CommandId::Table,
    CommandId::Wipeout,
    CommandId::DimContinue,
    CommandId::DimBaseline,
    CommandId::Section,
    CommandId::Setvar,
    CommandId::Getvar,
    CommandId::Pedit,
    CommandId::Matchprop,
    CommandId::Draworder,
    CommandId::Group,
    CommandId::Ungroup,
    CommandId::Overkill,
    CommandId::Oops,
    CommandId::Divide,
    CommandId::Measure,
    CommandId::Boundary,
    CommandId::Select,
    CommandId::SelectSimilar,
    CommandId::Qselect,
    CommandId::Ddptype,
    CommandId::Wblock,
    CommandId::Mtext,
    CommandId::ArrayPath,
    CommandId::Layiso,
    CommandId::Layoff,
    CommandId::Layfrz,
    CommandId::Laylck,
    CommandId::Laycur,
    CommandId::SelectAll,
    CommandId::SelectPrev,
];
