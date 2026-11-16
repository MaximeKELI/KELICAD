//! In-command option keywords (AutoCAD-style [Close/Undo/…]).

use crate::registry::CommandId;

/// A recognized option for the active command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommandOption {
    /// Canonical keyword (uppercase).
    pub key: &'static str,
    /// Short aliases (uppercase).
    pub aliases: &'static [&'static str],
    /// French label for UI chips.
    pub label_fr: &'static str,
}

impl CommandOption {
    /// Match a typed token against this option.
    pub fn matches(self, token: &str) -> bool {
        let t = token.trim().to_ascii_uppercase();
        if t == self.key {
            return true;
        }
        self.aliases.iter().any(|a| t == *a)
    }
}

/// Options available while `id` is active (prompt chips / keyword parse).
pub fn options_for(id: CommandId) -> &'static [CommandOption] {
    match id {
        CommandId::Line => &[
            CommandOption {
                key: "CLOSE",
                aliases: &["CL"],
                label_fr: "Fermer",
            },
            CommandOption {
                key: "UNDO",
                aliases: &["U"],
                label_fr: "Annuler pt",
            },
        ],
        CommandId::Pline => &[
            CommandOption {
                key: "CLOSE",
                aliases: &["CL", "C"],
                label_fr: "Fermer",
            },
            CommandOption {
                key: "UNDO",
                aliases: &["U"],
                label_fr: "Annuler pt",
            },
            CommandOption {
                key: "WIDTH",
                aliases: &["W"],
                label_fr: "Largeur",
            },
            CommandOption {
                key: "ARC",
                aliases: &["A"],
                label_fr: "Arc",
            },
            CommandOption {
                key: "LINE",
                aliases: &["L"],
                label_fr: "Ligne",
            },
        ],
        CommandId::Circle => &[
            CommandOption {
                key: "DIAMETER",
                aliases: &["D"],
                label_fr: "Diamètre",
            },
            CommandOption {
                key: "2P",
                aliases: &["2"],
                label_fr: "2 points",
            },
            CommandOption {
                key: "3P",
                aliases: &["3"],
                label_fr: "3 points",
            },
            CommandOption {
                key: "TTR",
                aliases: &["T"],
                label_fr: "TTR",
            },
        ],
        CommandId::Arc => &[
            CommandOption {
                key: "3P",
                aliases: &["3"],
                label_fr: "3 points",
            },
            CommandOption {
                key: "CSE",
                aliases: &["C"],
                label_fr: "Centre",
            },
            CommandOption {
                key: "SEA",
                aliases: &["A"],
                label_fr: "Début-Fin-Angle",
            },
        ],
        CommandId::Rectang => &[
            CommandOption {
                key: "FILLET",
                aliases: &["F"],
                label_fr: "Congé",
            },
            CommandOption {
                key: "CHAMFER",
                aliases: &["C"],
                label_fr: "Chanfrein",
            },
            CommandOption {
                key: "DIMENSIONS",
                aliases: &["D"],
                label_fr: "Dimensions",
            },
        ],
        CommandId::Polygon => &[
            CommandOption {
                key: "EDGE",
                aliases: &["E"],
                label_fr: "Côté",
            },
            CommandOption {
                key: "INSCRIBED",
                aliases: &["I"],
                label_fr: "Inscrit",
            },
            CommandOption {
                key: "CIRCUMSCRIBED",
                aliases: &["C"],
                label_fr: "Circonscrit",
            },
        ],
        CommandId::Offset => &[
            CommandOption {
                key: "THROUGH",
                aliases: &["T"],
                label_fr: "Par point",
            },
            CommandOption {
                key: "MULTIPLE",
                aliases: &["M"],
                label_fr: "Multiple",
            },
            CommandOption {
                key: "ERASE",
                aliases: &["E"],
                label_fr: "Effacer src",
            },
        ],
        CommandId::Fillet => &[
            CommandOption {
                key: "RADIUS",
                aliases: &["R"],
                label_fr: "Rayon",
            },
            CommandOption {
                key: "POLYLINE",
                aliases: &["P"],
                label_fr: "Polyligne",
            },
            CommandOption {
                key: "MULTIPLE",
                aliases: &["M"],
                label_fr: "Multiple",
            },
        ],
        CommandId::Chamfer => &[
            CommandOption {
                key: "DISTANCE",
                aliases: &["D"],
                label_fr: "Distance",
            },
            CommandOption {
                key: "POLYLINE",
                aliases: &["P"],
                label_fr: "Polyligne",
            },
            CommandOption {
                key: "MULTIPLE",
                aliases: &["M"],
                label_fr: "Multiple",
            },
        ],
        CommandId::Copy => &[CommandOption {
            key: "MULTIPLE",
            aliases: &["M"],
            label_fr: "Multiple",
        }],
        CommandId::Mirror => &[CommandOption {
            key: "YES",
            aliases: &["Y"],
            label_fr: "Effacer src",
        }, CommandOption {
            key: "NO",
            aliases: &["N"],
            label_fr: "Garder src",
        }],
        CommandId::Rotate | CommandId::Scale => &[
            CommandOption {
                key: "COPY",
                aliases: &["C"],
                label_fr: "Copier",
            },
            CommandOption {
                key: "REFERENCE",
                aliases: &["R"],
                label_fr: "Référence",
            },
        ],
        CommandId::Trim | CommandId::Extend => &[
            CommandOption {
                key: "QUICK",
                aliases: &["Q"],
                label_fr: "Rapide",
            },
            CommandOption {
                key: "STANDARD",
                aliases: &["S"],
                label_fr: "Standard",
            },
            CommandOption {
                key: "FENCE",
                aliases: &["F"],
                label_fr: "Polyligne",
            },
            CommandOption {
                key: "UNDO",
                aliases: &["U"],
                label_fr: "Annuler",
            },
        ],
        CommandId::Xline => &[
            CommandOption {
                key: "HOR",
                aliases: &["H"],
                label_fr: "Horizontal",
            },
            CommandOption {
                key: "VER",
                aliases: &["V"],
                label_fr: "Vertical",
            },
            CommandOption {
                key: "ANG",
                aliases: &["A"],
                label_fr: "Angle",
            },
            CommandOption {
                key: "OFFSET",
                aliases: &["O"],
                label_fr: "Décalage",
            },
        ],
        CommandId::Draworder => &[
            CommandOption {
                key: "FRONT",
                aliases: &["F"],
                label_fr: "Avant",
            },
            CommandOption {
                key: "BACK",
                aliases: &["B"],
                label_fr: "Arrière",
            },
        ],
        CommandId::Pedit => &[
            CommandOption {
                key: "CLOSE",
                aliases: &["C"],
                label_fr: "Fermer",
            },
            CommandOption {
                key: "OPEN",
                aliases: &["O"],
                label_fr: "Ouvrir",
            },
            CommandOption {
                key: "JOIN",
                aliases: &["J"],
                label_fr: "Joindre",
            },
            CommandOption {
                key: "WIDTH",
                aliases: &["W"],
                label_fr: "Largeur",
            },
            CommandOption {
                key: "REVERSE",
                aliases: &["R"],
                label_fr: "Inverser",
            },
        ],
        _ => &[],
    }
}

/// Resolve a token to a canonical option key for the command, if any.
pub fn match_option(id: CommandId, token: &str) -> Option<&'static str> {
    options_for(id)
        .iter()
        .find(|o| o.matches(token))
        .map(|o| o.key)
}

/// UI chip descriptor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OptionChip {
    /// Canonical key.
    pub key: &'static str,
    /// French label.
    pub label: &'static str,
}

/// Chips for the active command (ribbon / cmdline).
pub fn option_chips(id: CommandId) -> Vec<OptionChip> {
    options_for(id)
        .iter()
        .map(|o| OptionChip {
            key: o.key,
            label: o.label_fr,
        })
        .collect()
}
