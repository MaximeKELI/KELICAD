//! AutoCAD-like system variables (P1 subset).

use std::collections::BTreeMap;

/// Scope of a system variable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SysVarScope {
    /// Lives with the drawing session.
    Drawing,
    /// Session / UI preference (not persisted in entity journal yet).
    Session,
}

/// Typed system variable value.
#[derive(Debug, Clone, PartialEq)]
pub enum SysVarValue {
    /// Integer (modes, bitfields, counts).
    Int(i64),
    /// Floating point (distances, scales).
    Real(f64),
    /// String (color name, linetype, layer name).
    Str(String),
}

impl SysVarValue {
    /// Display for GETVAR echoes.
    pub fn display(&self) -> String {
        match self {
            Self::Int(v) => v.to_string(),
            Self::Real(v) => format!("{v}"),
            Self::Str(s) => s.clone(),
        }
    }
}

/// One registered system variable.
#[derive(Debug, Clone)]
pub struct SysVarDef {
    /// Canonical uppercase name.
    pub name: &'static str,
    /// Human tip (FR).
    pub tip: &'static str,
    /// Scope.
    pub scope: SysVarScope,
    /// Default value.
    pub default: SysVarValue,
}

/// Runtime store of system variables.
#[derive(Debug, Clone)]
pub struct SysVarStore {
    values: BTreeMap<String, SysVarValue>,
}

impl Default for SysVarStore {
    fn default() -> Self {
        Self::new()
    }
}

impl SysVarStore {
    /// Create with documented P1 defaults.
    pub fn new() -> Self {
        let mut s = Self {
            values: BTreeMap::new(),
        };
        for def in catalog() {
            s.values
                .insert(def.name.to_string(), def.default.clone());
        }
        s
    }

    /// Catalog of known variables.
    pub fn catalog() -> Vec<SysVarDef> {
        catalog()
    }

    /// Get a value (cloned).
    pub fn get(&self, name: &str) -> Option<SysVarValue> {
        let key = name.trim().to_ascii_uppercase();
        self.values.get(&key).cloned()
    }

    /// Set a value; returns error string if unknown or type mismatch.
    pub fn set(&mut self, name: &str, value: SysVarValue) -> Result<(), String> {
        let key = name.trim().to_ascii_uppercase();
        let cat = catalog();
        let Some(def) = cat.iter().find(|d| d.name == key) else {
            return Err(format!("Variable inconnue : {key}"));
        };
        let ok = match (&def.default, &value) {
            (SysVarValue::Int(_), SysVarValue::Int(_)) => true,
            (SysVarValue::Real(_), SysVarValue::Real(_)) => true,
            (SysVarValue::Real(_), SysVarValue::Int(_)) => true,
            (SysVarValue::Str(_), SysVarValue::Str(_)) => true,
            _ => false,
        };
        if !ok {
            return Err(format!("Type incorrect pour {key}"));
        }
        let stored = match (&def.default, value) {
            (SysVarValue::Real(_), SysVarValue::Int(i)) => SysVarValue::Real(i as f64),
            (_, v) => v,
        };
        self.values.insert(key, stored);
        Ok(())
    }

    /// Integer helper.
    pub fn get_int(&self, name: &str) -> i64 {
        match self.get(name) {
            Some(SysVarValue::Int(v)) => v,
            Some(SysVarValue::Real(v)) => v as i64,
            _ => 0,
        }
    }

    /// Real helper.
    pub fn get_real(&self, name: &str) -> f64 {
        match self.get(name) {
            Some(SysVarValue::Real(v)) => v,
            Some(SysVarValue::Int(v)) => v as f64,
            _ => 0.0,
        }
    }

    /// String helper.
    pub fn get_str(&self, name: &str) -> String {
        match self.get(name) {
            Some(SysVarValue::Str(s)) => s,
            Some(other) => other.display(),
            None => String::new(),
        }
    }

    /// JSON map for UI.
    pub fn to_json_map(&self) -> BTreeMap<String, String> {
        self.values
            .iter()
            .map(|(k, v)| (k.clone(), v.display()))
            .collect()
    }

    /// Apply OSMODE bitfield → individual mode flags.
    /// Bits (AutoCAD-inspired subset): 1 End, 2 Mid, 4 Cen, 8 Node, 16 Quad,
    /// 32 Int, 64 Ins, 128 Perp, 256 Tan, 512 Near, 1024 Ext, 2048 Par.
    pub fn osmode_bits(&self) -> i64 {
        self.get_int("OSMODE")
    }
}

/// Parse a SETVAR value token.
pub fn parse_sysvar_token(raw: &str) -> SysVarValue {
    let t = raw.trim();
    if let Ok(i) = t.parse::<i64>() {
        return SysVarValue::Int(i);
    }
    if let Ok(f) = t.parse::<f64>() {
        return SysVarValue::Real(f);
    }
    SysVarValue::Str(t.to_string())
}

fn catalog() -> Vec<SysVarDef> {
    vec![
        SysVarDef {
            name: "ORTHOMODE",
            tip: "Contrainte orthogonale (F8)",
            scope: SysVarScope::Session,
            default: SysVarValue::Int(0),
        },
        SysVarDef {
            name: "OSMODE",
            tip: "Masque d'accroches objet",
            scope: SysVarScope::Session,
            default: SysVarValue::Int(4133),
        },
        SysVarDef {
            name: "POLARMODE",
            tip: "Traçage polaire",
            scope: SysVarScope::Session,
            default: SysVarValue::Int(0),
        },
        SysVarDef {
            name: "GRIDMODE",
            tip: "Affichage grille (F7)",
            scope: SysVarScope::Session,
            default: SysVarValue::Int(1),
        },
        SysVarDef {
            name: "SNAPMODE",
            tip: "Accrochage grille (F9)",
            scope: SysVarScope::Session,
            default: SysVarValue::Int(0),
        },
        SysVarDef {
            name: "SNAPUNIT",
            tip: "Pas d'accrochage grille",
            scope: SysVarScope::Drawing,
            default: SysVarValue::Real(10.0),
        },
        SysVarDef {
            name: "GRIDUNIT",
            tip: "Pas d'affichage grille",
            scope: SysVarScope::Drawing,
            default: SysVarValue::Real(10.0),
        },
        SysVarDef {
            name: "DYNMODE",
            tip: "Saisie dynamique",
            scope: SysVarScope::Session,
            default: SysVarValue::Int(1),
        },
        SysVarDef {
            name: "APERTURE",
            tip: "Taille viseur accroche",
            scope: SysVarScope::Session,
            default: SysVarValue::Real(0.5),
        },
        SysVarDef {
            name: "PICKBOX",
            tip: "Taille boîte de sélection",
            scope: SysVarScope::Session,
            default: SysVarValue::Real(0.35),
        },
        SysVarDef {
            name: "FILLETRAD",
            tip: "Rayon de congé",
            scope: SysVarScope::Drawing,
            default: SysVarValue::Real(0.0),
        },
        SysVarDef {
            name: "CHAMFERA",
            tip: "Première distance de chanfrein",
            scope: SysVarScope::Drawing,
            default: SysVarValue::Real(0.0),
        },
        SysVarDef {
            name: "CHAMFERB",
            tip: "Seconde distance de chanfrein",
            scope: SysVarScope::Drawing,
            default: SysVarValue::Real(0.0),
        },
        SysVarDef {
            name: "OFFSETDIST",
            tip: "Distance de décalage",
            scope: SysVarScope::Drawing,
            default: SysVarValue::Real(1.0),
        },
        SysVarDef {
            name: "PLINEWID",
            tip: "Largeur courante de polyligne",
            scope: SysVarScope::Drawing,
            default: SysVarValue::Real(0.0),
        },
        SysVarDef {
            name: "LTSCALE",
            tip: "Échelle globale des types de ligne",
            scope: SysVarScope::Drawing,
            default: SysVarValue::Real(1.0),
        },
        SysVarDef {
            name: "LWDISPLAY",
            tip: "Afficher les épaisseurs",
            scope: SysVarScope::Session,
            default: SysVarValue::Int(1),
        },
        SysVarDef {
            name: "PDMODE",
            tip: "Style d'affichage des points",
            scope: SysVarScope::Drawing,
            default: SysVarValue::Int(0),
        },
        SysVarDef {
            name: "PDSIZE",
            tip: "Taille d'affichage des points",
            scope: SysVarScope::Drawing,
            default: SysVarValue::Real(0.0),
        },
        SysVarDef {
            name: "TRIMEXTENDMODE",
            tip: "0=Standard 1=Quick",
            scope: SysVarScope::Session,
            default: SysVarValue::Int(1),
        },
        SysVarDef {
            name: "MIRRTEXT",
            tip: "Miroir du texte",
            scope: SysVarScope::Drawing,
            default: SysVarValue::Int(0),
        },
        SysVarDef {
            name: "CECOLOR",
            tip: "Couleur courante",
            scope: SysVarScope::Drawing,
            default: SysVarValue::Str("BYLAYER".into()),
        },
        SysVarDef {
            name: "CELTYPE",
            tip: "Type de ligne courant",
            scope: SysVarScope::Drawing,
            default: SysVarValue::Str("BYLAYER".into()),
        },
        SysVarDef {
            name: "CELWEIGHT",
            tip: "Épaisseur courante (-1=BYLAYER)",
            scope: SysVarScope::Drawing,
            default: SysVarValue::Int(-1),
        },
        SysVarDef {
            name: "CLAYER",
            tip: "Calque courant (nom)",
            scope: SysVarScope::Drawing,
            default: SysVarValue::Str("0".into()),
        },
        SysVarDef {
            name: "POLARANG",
            tip: "Incrément d'angle polaire (degrés)",
            scope: SysVarScope::Session,
            default: SysVarValue::Real(45.0),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_and_set() {
        let mut s = SysVarStore::new();
        assert_eq!(s.get_int("ORTHOMODE"), 0);
        s.set("ORTHOMODE", SysVarValue::Int(1)).unwrap();
        assert_eq!(s.get_int("ORTHOMODE"), 1);
        s.set("FILLETRAD", SysVarValue::Real(2.5)).unwrap();
        assert!((s.get_real("FILLETRAD") - 2.5).abs() < 1e-9);
        assert!(s.set("NOPE", SysVarValue::Int(1)).is_err());
    }
}
