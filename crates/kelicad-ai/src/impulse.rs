//! Contextual drawing impulses / proposals (no network LLM).

use kelicad_doc::EntityKind;
use kelicad_ops::DocSession;
use serde::Serialize;

/// One clickable suggestion for the UI.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Impulse {
    /// Short French label.
    pub label_fr: String,
    /// Short English label.
    pub label_en: String,
    /// Natural-language utterance to feed `KeliAgent::ask`.
    pub utterance: String,
    /// Category for palette grouping.
    pub category: &'static str,
}

/// Build contextual impulses from document + selection size.
pub fn suggest_impulses(session: &DocSession, selection_len: usize) -> Vec<Impulse> {
    let mut out = Vec::new();
    let mut counts = KindCounts::default();
    for e in session.document.entities() {
        counts.bump(e.kind);
    }
    let total = counts.total;

    // Always-on easy shapes.
    out.push(chip(
        "Rectangle 4×3",
        "4×3 rectangle",
        "dessine un rectangle de 4 m × 3 m",
        "formes",
    ));
    out.push(chip(
        "Cercle Ø2",
        "Circle Ø2",
        "dessine un cercle de rayon 1",
        "formes",
    ));
    out.push(chip(
        "Étoile 5 branches",
        "5-point star",
        "dessine une étoile à 5 branches de rayon 2",
        "formes",
    ));
    out.push(chip(
        "Polygone hexagone",
        "Hexagon",
        "dessine un polygone régulier à 6 côtés de rayon 1.5",
        "formes",
    ));
    out.push(chip(
        "Donut / anneau",
        "Donut / ring",
        "dessine un donut rayon extérieur 2 intérieur 1",
        "formes",
    ));
    out.push(chip(
        "Capsule",
        "Capsule",
        "dessine une capsule de 4 m de long et rayon 0.5",
        "formes",
    ));
    out.push(chip(
        "Flèche",
        "Arrow",
        "dessine une flèche de (0,0) à (5,0)",
        "formes",
    ));
    out.push(chip(
        "Triangle",
        "Triangle",
        "dessine un triangle isocèle base 3 hauteur 2",
        "formes",
    ));
    out.push(chip(
        "Grille 4×3",
        "4×3 grid",
        "dessine une grille 4 colonnes 3 lignes pas 1",
        "formes",
    ));

    if total == 0 {
        out.push(chip(
            "Plan pièce type",
            "Starter room plan",
            "dessine un rectangle de 5 m × 4 m puis cote-le",
            "démarrage",
        ));
        return out;
    }

    if counts.lines >= 1 || counts.polylines >= 1 {
        out.push(chip(
            "Offset 0.2",
            "Offset 0.2",
            "offset de 0.2",
            "modifier",
        ));
        out.push(chip(
            "Cote auto",
            "Auto dimension",
            "cote automatique",
            "annoter",
        ));
    }

    if counts.circles >= 1 {
        out.push(chip(
            "Cote rayon",
            "Radius dim",
            "ajoute une cote",
            "annoter",
        ));
    }

    if selection_len >= 1 {
        out.push(chip(
            "Déplacer +2 en X",
            "Move +2 in X",
            "déplace de 2 m vers la droite",
            "modifier",
        ));
        out.push(chip(
            "Copier +3 en Y",
            "Copy +3 in Y",
            "copie de 3 m vers le haut",
            "modifier",
        ));
        out.push(chip(
            "Rotation 45°",
            "Rotate 45°",
            "rotation de 45 degrés autour de 0 0",
            "modifier",
        ));
    }

    if counts.polylines >= 1 && selection_len >= 1 {
        out.push(chip(
            "Hachurer",
            "Hatch",
            "hachure la région",
            "annoter",
        ));
    }

    out.push(chip(
        "Résumé du dessin",
        "Drawing summary",
        "résumé du document",
        "analyser",
    ));
    out.push(chip(
        "Nettoyer",
        "Cleanup",
        "nettoie le dessin",
        "analyser",
    ));
    out.push(chip(
        "Contrôle qualité",
        "QA check",
        "contrôle qualité",
        "analyser",
    ));

    // Soft cap for UI chips.
    out.truncate(16);
    out
}

fn chip(fr: &str, en: &str, utterance: &str, category: &'static str) -> Impulse {
    Impulse {
        label_fr: fr.into(),
        label_en: en.into(),
        utterance: utterance.into(),
        category,
    }
}

#[derive(Default)]
struct KindCounts {
    total: usize,
    lines: usize,
    circles: usize,
    polylines: usize,
}

impl KindCounts {
    fn bump(&mut self, kind: EntityKind) {
        self.total += 1;
        match kind {
            EntityKind::Line => self.lines += 1,
            EntityKind::Circle => self.circles += 1,
            EntityKind::Polyline => self.polylines += 1,
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kelicad_ops::DocSession;

    #[test]
    fn empty_doc_has_starter() {
        let s = DocSession::new();
        let imps = suggest_impulses(&s, 0);
        assert!(imps.iter().any(|i| i.category == "démarrage" || i.category == "formes"));
        assert!(imps.len() >= 5);
    }
}
