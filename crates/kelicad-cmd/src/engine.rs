//! Command state machine driving a [`kelicad_ops::DocSession`].

use kelicad_doc::{
    BlockDef, BlockId, DimKind, DimensionGeom, EntityId, EntityRecord, Geometry, HatchGeom,
    HatchPattern, InsertGeom, TextAlign, TextGeom,
};
use kelicad_geom::transform::offset_line;
use kelicad_geom::trim::{extend_line_to, trim_line_by_cut};
use kelicad_geom::{Arc, Circle, Line, Polyline};
use kelicad_math::{Tolerance, Vec2};
use kelicad_ops::{DocSession, Operation};
use kelicad_snap::{SnapEngine, SnapHit, SnapQuery};
use uuid::Uuid;

use crate::id_gen::IdGen;
use crate::input::{parse_number, parse_point};
use crate::registry::CommandId;
use crate::selection::Selection;

/// User / UI events fed into the engine.
#[derive(Debug, Clone)]
pub enum CommandEvent {
    /// Start a command by name/alias (or from palette).
    Start(CommandId),
    /// Free-text command-line token (command name or point/number/option).
    Text(String),
    /// World-space pick (already snapped by UI if desired).
    Pick(Vec2),
    /// Hover for ghost preview.
    Hover(Vec2),
    /// Enter / accept (finish polyline, etc.).
    Enter,
    /// Escape — cancel active command.
    Cancel,
    /// Toggle entity in selection (idle mode).
    SelectToggle(EntityId),
    /// Replace selection.
    SelectSet(Vec<EntityId>),
    /// Clear selection.
    SelectClear,
}

/// Prompt shown in the command line / dynamic input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnginePrompt {
    /// Short prompt, French default UI string.
    pub message_fr: String,
    /// English message.
    pub message_en: String,
}

/// Side effects for the UI after handling an event.
#[derive(Debug, Clone, Default)]
pub struct CommandOutput {
    /// Prompt to display (`None` = idle).
    pub prompt: Option<EnginePrompt>,
    /// Ghost geometry preview (world).
    pub ghost: Vec<Geometry>,
    /// Active snap marker.
    pub snap: Option<SnapHit>,
    /// Status / echo lines (French).
    pub echoes: Vec<String>,
    /// True if document mutated.
    pub document_changed: bool,
}

#[derive(Debug, Clone)]
enum Active {
    Line {
        points: Vec<Vec2>,
    },
    Pline {
        points: Vec<Vec2>,
    },
    CircleCenter {
        center: Option<Vec2>,
    },
    Arc {
        center: Option<Vec2>,
        start: Option<Vec2>,
    },
    Rectang {
        first: Option<Vec2>,
    },
    Move {
        base: Option<Vec2>,
    },
    Copy {
        base: Option<Vec2>,
    },
    Rotate {
        origin: Option<Vec2>,
    },
    Scale {
        origin: Option<Vec2>,
    },
    Mirror {
        first: Option<Vec2>,
    },
    Offset {
        distance: Option<f64>,
    },
    Trim,
    Extend,
    Erase,
    Text {
        position: Option<Vec2>,
    },
    DimLinear {
        def1: Option<Vec2>,
        def2: Option<Vec2>,
    },
    Hatch {
        points: Vec<Vec2>,
    },
    Block {
        name: Option<String>,
        base: Option<Vec2>,
    },
    Insert {
        name: Option<String>,
    },
}

/// Central command engine.
pub struct CommandEngine<G: IdGen> {
    /// Document + journal.
    pub session: DocSession,
    /// Selection set.
    pub selection: Selection,
    /// Snap engine.
    pub snap: SnapEngine,
    /// Id generator.
    pub ids: G,
    /// Last point (for `@` relative / polar).
    pub last_point: Option<Vec2>,
    /// Snap aperture in world units.
    pub snap_aperture: f64,
    active: Option<(CommandId, Active)>,
    /// Last finished command (repeat with Enter).
    last_command: Option<CommandId>,
}

impl<G: IdGen> CommandEngine<G> {
    /// Create with a session and id generator.
    pub fn new(session: DocSession, ids: G) -> Self {
        Self {
            session,
            selection: Selection::new(),
            snap: SnapEngine::default(),
            ids,
            last_point: None,
            snap_aperture: 0.5,
            active: None,
            last_command: None,
        }
    }

    /// Active command id, if any.
    pub fn active_command(&self) -> Option<CommandId> {
        self.active.as_ref().map(|(id, _)| *id)
    }

    /// Handle an event; returns UI output.
    pub fn handle(&mut self, event: CommandEvent) -> CommandOutput {
        let mut out = CommandOutput::default();
        match event {
            CommandEvent::Cancel => {
                self.active = None;
                out.echoes.push("Commande annulée.".into());
                out.prompt = None;
            }
            CommandEvent::SelectClear => {
                if self.active.is_none() {
                    self.selection.clear();
                }
            }
            CommandEvent::SelectSet(ids) => {
                if self.active.is_none() {
                    self.selection.clear();
                    for id in ids {
                        self.selection.add(id);
                    }
                }
            }
            CommandEvent::SelectToggle(id) => {
                if self.active.is_none() {
                    self.selection.toggle(id);
                }
            }
            CommandEvent::Start(id) => {
                out = self.start_command(id);
            }
            CommandEvent::Enter => {
                if self.active.is_none() {
                    if let Some(id) = self.last_command {
                        out = self.start_command(id);
                    }
                } else {
                    out = self.on_enter();
                }
            }
            CommandEvent::Hover(p) => {
                let snapped = self.snap_point(p, &mut out);
                out.ghost = self.ghost_at(snapped);
                out.prompt = self.current_prompt();
            }
            CommandEvent::Pick(p) => {
                let snapped = self.snap_point(p, &mut out);
                let next = self.on_point(snapped);
                out = Self::merge_out(out, next);
            }
            CommandEvent::Text(text) => {
                out = self.on_text(&text);
            }
        }
        if out.prompt.is_none() {
            out.prompt = self.current_prompt();
        }
        out
    }

    fn merge_out(mut a: CommandOutput, b: CommandOutput) -> CommandOutput {
        a.echoes.extend(b.echoes);
        a.ghost = b.ghost;
        a.document_changed |= b.document_changed;
        if b.prompt.is_some() {
            a.prompt = b.prompt;
        }
        if b.snap.is_some() {
            a.snap = b.snap;
        }
        a
    }

    fn snap_point(&self, cursor: Vec2, out: &mut CommandOutput) -> Vec2 {
        let q = SnapQuery {
            cursor,
            aperture: self.snap_aperture,
            base: self.last_point,
        };
        if let Some(hit) = self.snap.resolve(&self.session.document, q) {
            out.snap = Some(hit.clone());
            hit.point
        } else {
            cursor
        }
    }

    fn start_command(&mut self, id: CommandId) -> CommandOutput {
        let mut out = CommandOutput::default();
        match id {
            CommandId::Undo => {
                match self.session.undo() {
                    Ok(true) => {
                        out.echoes.push("Annuler.".into());
                        out.document_changed = true;
                    }
                    Ok(false) => out.echoes.push("Rien à annuler.".into()),
                    Err(e) => out.echoes.push(format!("Erreur UNDO: {e}")),
                }
                self.active = None;
            }
            CommandId::Redo => {
                match self.session.redo() {
                    Ok(true) => {
                        out.echoes.push("Rétablir.".into());
                        out.document_changed = true;
                    }
                    Ok(false) => out.echoes.push("Rien à rétablir.".into()),
                    Err(e) => out.echoes.push(format!("Erreur REDO: {e}")),
                }
                self.active = None;
            }
            CommandId::Erase => {
                if self.selection.is_empty() {
                    self.active = Some((id, Active::Erase));
                    out.prompt = Some(prompt(
                        "Sélectionnez les objets à effacer, puis Entrée",
                        "Select objects to erase, then Enter",
                    ));
                } else {
                    out = self.erase_selection();
                }
            }
            CommandId::Text => {
                self.active = Some((id, Active::Text { position: None }));
                self.last_command = Some(id);
                out.prompt = self.current_prompt();
                out.echoes.push("Commande TEXT".into());
            }
            CommandId::DimLinear => {
                self.active = Some((
                    id,
                    Active::DimLinear {
                        def1: None,
                        def2: None,
                    },
                ));
                self.last_command = Some(id);
                out.prompt = self.current_prompt();
            }
            CommandId::Hatch => {
                self.active = Some((id, Active::Hatch { points: vec![] }));
                self.last_command = Some(id);
                out.prompt = self.current_prompt();
            }
            CommandId::Block => {
                if self.selection.is_empty() {
                    out.echoes
                        .push("Sélectionnez des objets avant BLOCK.".into());
                    return out;
                }
                self.active = Some((
                    id,
                    Active::Block {
                        name: None,
                        base: None,
                    },
                ));
                self.last_command = Some(id);
                out.prompt = self.current_prompt();
            }
            CommandId::Insert => {
                self.active = Some((id, Active::Insert { name: None }));
                self.last_command = Some(id);
                out.prompt = self.current_prompt();
            }
            CommandId::Move
            | CommandId::Copy
            | CommandId::Rotate
            | CommandId::Scale
            | CommandId::Mirror
            | CommandId::Offset
            | CommandId::Trim
            | CommandId::Extend => {
                if self.selection.is_empty()
                    && !matches!(id, CommandId::Offset | CommandId::Trim | CommandId::Extend)
                {
                    out.echoes
                        .push("Sélectionnez des objets avant la commande.".into());
                    return out;
                }
                let active = match id {
                    CommandId::Move => Active::Move { base: None },
                    CommandId::Copy => Active::Copy { base: None },
                    CommandId::Rotate => Active::Rotate { origin: None },
                    CommandId::Scale => Active::Scale { origin: None },
                    CommandId::Mirror => Active::Mirror { first: None },
                    CommandId::Offset => Active::Offset { distance: None },
                    CommandId::Trim => Active::Trim,
                    CommandId::Extend => Active::Extend,
                    _ => unreachable!(),
                };
                self.active = Some((id, active));
                self.last_command = Some(id);
                out.prompt = self.current_prompt();
            }
            other => {
                let active = match other {
                    CommandId::Line => Active::Line { points: vec![] },
                    CommandId::Pline => Active::Pline { points: vec![] },
                    CommandId::Circle => Active::CircleCenter { center: None },
                    CommandId::Arc => Active::Arc {
                        center: None,
                        start: None,
                    },
                    CommandId::Rectang => Active::Rectang { first: None },
                    _ => unreachable!(),
                };
                self.active = Some((other, active));
                self.last_command = Some(other);
                out.prompt = self.current_prompt();
                out.echoes.push(format!("Commande {}", other.name()));
            }
        }
        out
    }

    fn current_prompt(&self) -> Option<EnginePrompt> {
        let Some((id, state)) = &self.active else {
            return None;
        };
        Some(match (id, state) {
            (CommandId::Line, Active::Line { points }) if points.is_empty() => {
                prompt("Premier point", "First point")
            }
            (CommandId::Line, _) => prompt(
                "Point suivant ou Entrée pour terminer",
                "Next point or Enter",
            ),
            (CommandId::Pline, Active::Pline { points }) if points.is_empty() => {
                prompt("Point de départ", "Start point")
            }
            (CommandId::Pline, _) => prompt("Point suivant ou Entrée", "Next point or Enter"),
            (CommandId::Circle, Active::CircleCenter { center: None }) => {
                prompt("Centre du cercle", "Circle center")
            }
            (CommandId::Circle, _) => prompt("Point sur le cercle (rayon)", "Point on circle"),
            (CommandId::Arc, Active::Arc { center: None, .. }) => {
                prompt("Centre de l'arc", "Arc center")
            }
            (CommandId::Arc, Active::Arc { start: None, .. }) => {
                prompt("Point de départ", "Start point")
            }
            (CommandId::Arc, _) => prompt("Point d'arrivée", "End point"),
            (CommandId::Rectang, Active::Rectang { first: None }) => {
                prompt("Premier coin", "First corner")
            }
            (CommandId::Rectang, _) => prompt("Coin opposé", "Opposite corner"),
            (CommandId::Move, Active::Move { base: None }) => prompt("Point de base", "Base point"),
            (CommandId::Move, _) => prompt("Second point", "Second point"),
            (CommandId::Copy, Active::Copy { base: None }) => prompt("Point de base", "Base point"),
            (CommandId::Copy, _) => prompt("Second point", "Second point"),
            (CommandId::Rotate, Active::Rotate { origin: None }) => {
                prompt("Point de base", "Base point")
            }
            (CommandId::Rotate, _) => prompt("Angle (degrés) ou point", "Angle (deg) or point"),
            (CommandId::Scale, Active::Scale { origin: None }) => {
                prompt("Point de base", "Base point")
            }
            (CommandId::Scale, _) => prompt("Facteur ou point", "Factor or point"),
            (CommandId::Mirror, Active::Mirror { first: None }) => {
                prompt("Premier point de l'axe", "First point of mirror line")
            }
            (CommandId::Mirror, _) => {
                prompt("Second point de l'axe", "Second point of mirror line")
            }
            (CommandId::Offset, Active::Offset { distance: None }) => {
                prompt("Distance de décalage", "Offset distance")
            }
            (CommandId::Offset, _) => prompt(
                "Sélectionnez une ligne à décaler",
                "Select a line to offset",
            ),
            (CommandId::Trim, _) => prompt(
                "Sélectionnez le segment à couper (côté à garder près du clic)",
                "Select segment to trim",
            ),
            (CommandId::Extend, _) => prompt(
                "Sélectionnez le segment à prolonger",
                "Select segment to extend",
            ),
            (CommandId::Erase, _) => prompt(
                "Sélectionnez les objets, Entrée pour effacer",
                "Select objects, Enter to erase",
            ),
            _ => prompt(id.name(), id.name()),
        })
    }

    fn ghost_at(&self, cursor: Vec2) -> Vec<Geometry> {
        let Some((_, state)) = &self.active else {
            return vec![];
        };
        match state {
            Active::Line { points } if !points.is_empty() => {
                let last = *points.last().unwrap();
                vec![Geometry::Line(Line::new(last, cursor))]
            }
            Active::Pline { points } if !points.is_empty() => {
                let last = *points.last().unwrap();
                vec![Geometry::Line(Line::new(last, cursor))]
            }
            Active::CircleCenter { center: Some(c) } => {
                let r = c.distance(cursor);
                vec![Geometry::Circle(Circle::new(*c, r))]
            }
            Active::Rectang { first: Some(a) } => {
                vec![Geometry::Polyline(Polyline::rectangle(*a, cursor))]
            }
            Active::Arc {
                center: Some(c),
                start: Some(s),
            } => {
                let r = c.distance(*s);
                let start_ang = (*s - *c).angle();
                let end_ang = (cursor - *c).angle();
                vec![Geometry::Arc(Arc::new(*c, r, start_ang, end_ang))]
            }
            Active::Move { base: Some(b) } => {
                let delta = cursor - *b;
                self.selection
                    .iter()
                    .filter_map(|id| self.session.document.get_entity(id))
                    .map(|e| e.geometry.translated(delta))
                    .collect()
            }
            Active::Copy { base: Some(b) } => {
                let delta = cursor - *b;
                self.selection
                    .iter()
                    .filter_map(|id| self.session.document.get_entity(id))
                    .map(|e| e.geometry.translated(delta))
                    .collect()
            }
            _ => vec![],
        }
    }

    fn on_enter(&mut self) -> CommandOutput {
        let mut out = CommandOutput::default();
        let Some((id, state)) = self.active.clone() else {
            return out;
        };
        match state {
            Active::Line { .. } => {
                // Segments are committed on each pick; Enter only ends the command.
                out.echoes.push("LINE terminé.".into());
                self.active = None;
            }
            Active::Pline { points } => {
                if points.len() >= 2 {
                    out = self.commit_polyline(points, false);
                }
                self.active = None;
            }
            Active::Erase => {
                out = self.erase_selection();
                self.active = None;
            }
            Active::Hatch { points } => {
                if points.len() >= 3 {
                    let mut boundary = points;
                    if boundary.first() != boundary.last() {
                        if let Some(first) = boundary.first().copied() {
                            boundary.push(first);
                        }
                    }
                    out = self.commit_one(Geometry::Hatch(HatchGeom {
                        boundary,
                        pattern: HatchPattern::Solid,
                    }));
                } else {
                    out.echoes.push("HATCH : au moins 3 points.".into());
                }
                self.active = None;
            }
            _ => {
                out.echoes.push(format!(
                    "{} : Entrée ignorée (continuez la saisie ou Échap).",
                    id.name()
                ));
            }
        }
        out
    }

    fn on_text(&mut self, text: &str) -> CommandOutput {
        let t = text.trim();
        if t.is_empty() {
            return CommandOutput::default();
        }
        if self.active.is_none() {
            if let Some(id) = CommandId::parse(t) {
                return self.start_command(id);
            }
            return CommandOutput {
                echoes: vec![format!("Commande inconnue : {t}")],
                ..Default::default()
            };
        }

        // Text content / block names when waiting for string input.
        if let Some((CommandId::Text, Active::Text { position: Some(p) })) = self.active.clone() {
            let out = self.commit_one(Geometry::Text(TextGeom {
                position: p,
                height: 2.5,
                rotation: 0.0,
                content: t.to_string(),
                align: TextAlign::Left,
            }));
            self.active = None;
            return out;
        }
        if let Some((CommandId::Block, Active::Block { name: None, base })) = self.active.clone() {
            self.active = Some((
                CommandId::Block,
                Active::Block {
                    name: Some(t.to_string()),
                    base,
                },
            ));
            return CommandOutput {
                prompt: self.current_prompt(),
                echoes: vec![format!("Bloc « {t} »")],
                ..Default::default()
            };
        }
        if let Some((CommandId::Insert, Active::Insert { name: None })) = self.active.clone() {
            if self.session.document.block_by_name(t).is_none() {
                return CommandOutput {
                    echoes: vec![format!("Bloc inconnu : {t}")],
                    ..Default::default()
                };
            }
            self.active = Some((
                CommandId::Insert,
                Active::Insert {
                    name: Some(t.to_string()),
                },
            ));
            return CommandOutput {
                prompt: self.current_prompt(),
                ..Default::default()
            };
        }

        // Option keywords
        let upper = t.to_ascii_uppercase();
        if upper == "C" || upper == "CL" || upper == "CLOSE" {
            if let Some((CommandId::Pline, Active::Pline { points })) = self.active.clone() {
                if points.len() >= 3 {
                    let out = self.commit_polyline(points, true);
                    self.active = None;
                    return out;
                }
            }
        }

        if let Ok(parsed) = parse_point(t) {
            match parsed.resolve(self.last_point) {
                Ok(p) => return self.on_point(p),
                Err(e) => {
                    return CommandOutput {
                        echoes: vec![e.to_string()],
                        ..Default::default()
                    };
                }
            }
        }

        if let Ok(n) = parse_number(t) {
            return self.on_number(n);
        }

        CommandOutput {
            echoes: vec![format!("Entrée invalide : {t}")],
            ..Default::default()
        }
    }

    fn on_number(&mut self, n: f64) -> CommandOutput {
        let mut out = CommandOutput::default();
        let Some((id, state)) = self.active.clone() else {
            return out;
        };
        match (id, state) {
            (CommandId::Circle, Active::CircleCenter { center: Some(c) }) => {
                out = self.commit_one(Geometry::Circle(Circle::new(c, n.abs())));
                self.active = None;
            }
            (CommandId::Offset, Active::Offset { distance: None }) => {
                self.active = Some((CommandId::Offset, Active::Offset { distance: Some(n) }));
                out.prompt = self.current_prompt();
            }
            (CommandId::Rotate, Active::Rotate { origin: Some(o) }) => {
                let rad = n.to_radians();
                out = self.transform_selection(|g| g.rotated(o, rad));
                self.active = None;
            }
            (CommandId::Scale, Active::Scale { origin: Some(o) }) => {
                out = self.transform_selection(|g| g.scaled(o, n));
                self.active = None;
            }
            _ => {
                out.echoes.push("Nombre non attendu ici.".into());
            }
        }
        out
    }

    fn on_point(&mut self, p: Vec2) -> CommandOutput {
        let mut out = CommandOutput::default();
        self.last_point = Some(p);
        let Some((id, state)) = self.active.clone() else {
            // Idle pick → select nearest entity
            if let Some(eid) = self.pick_entity(p) {
                self.selection.set_single(eid);
                out.echoes.push(format!("Sélectionné {eid}"));
            }
            return out;
        };

        match (id, state) {
            (CommandId::Line, Active::Line { mut points }) => {
                points.push(p);
                if points.len() >= 2 {
                    let a = points[points.len() - 2];
                    let b = points[points.len() - 1];
                    let mut part = self.commit_one(Geometry::Line(Line::new(a, b)));
                    out.document_changed = true;
                    out.echoes.append(&mut part.echoes);
                }
                self.active = Some((CommandId::Line, Active::Line { points }));
                out.prompt = self.current_prompt();
            }
            (CommandId::Pline, Active::Pline { mut points }) => {
                points.push(p);
                self.active = Some((CommandId::Pline, Active::Pline { points }));
                out.prompt = self.current_prompt();
            }
            (CommandId::Circle, Active::CircleCenter { center: None }) => {
                self.active = Some((CommandId::Circle, Active::CircleCenter { center: Some(p) }));
                out.prompt = self.current_prompt();
            }
            (CommandId::Circle, Active::CircleCenter { center: Some(c) }) => {
                let r = c.distance(p);
                out = self.commit_one(Geometry::Circle(Circle::new(c, r)));
                self.active = None;
            }
            (CommandId::Arc, Active::Arc { center: None, .. }) => {
                self.active = Some((
                    CommandId::Arc,
                    Active::Arc {
                        center: Some(p),
                        start: None,
                    },
                ));
                out.prompt = self.current_prompt();
            }
            (
                CommandId::Arc,
                Active::Arc {
                    center: Some(c),
                    start: None,
                },
            ) => {
                self.active = Some((
                    CommandId::Arc,
                    Active::Arc {
                        center: Some(c),
                        start: Some(p),
                    },
                ));
                out.prompt = self.current_prompt();
            }
            (
                CommandId::Arc,
                Active::Arc {
                    center: Some(c),
                    start: Some(s),
                },
            ) => {
                let r = c.distance(s);
                let start_ang = (s - c).angle();
                let end_ang = (p - c).angle();
                out = self.commit_one(Geometry::Arc(Arc::new(c, r, start_ang, end_ang)));
                self.active = None;
            }
            (CommandId::Rectang, Active::Rectang { first: None }) => {
                self.active = Some((CommandId::Rectang, Active::Rectang { first: Some(p) }));
                out.prompt = self.current_prompt();
            }
            (CommandId::Rectang, Active::Rectang { first: Some(a) }) => {
                out = self.commit_one(Geometry::Polyline(Polyline::rectangle(a, p)));
                self.active = None;
            }
            (CommandId::Move, Active::Move { base: None }) => {
                self.active = Some((CommandId::Move, Active::Move { base: Some(p) }));
                out.prompt = self.current_prompt();
            }
            (CommandId::Move, Active::Move { base: Some(b) }) => {
                let delta = p - b;
                out = self.transform_selection(|g| g.translated(delta));
                self.active = None;
            }
            (CommandId::Copy, Active::Copy { base: None }) => {
                self.active = Some((CommandId::Copy, Active::Copy { base: Some(p) }));
                out.prompt = self.current_prompt();
            }
            (CommandId::Copy, Active::Copy { base: Some(b) }) => {
                let delta = p - b;
                out = self.copy_selection(delta);
                self.active = None;
            }
            (CommandId::Rotate, Active::Rotate { origin: None }) => {
                self.active = Some((CommandId::Rotate, Active::Rotate { origin: Some(p) }));
                out.prompt = self.current_prompt();
            }
            (CommandId::Rotate, Active::Rotate { origin: Some(o) }) => {
                let ang = (p - o).angle();
                out = self.transform_selection(|g| g.rotated(o, ang));
                self.active = None;
            }
            (CommandId::Scale, Active::Scale { origin: None }) => {
                self.active = Some((CommandId::Scale, Active::Scale { origin: Some(p) }));
                out.prompt = self.current_prompt();
            }
            (CommandId::Scale, Active::Scale { origin: Some(o) }) => {
                let factor = o.distance(p);
                out = self.transform_selection(|g| g.scaled(o, factor.max(1e-12)));
                self.active = None;
            }
            (CommandId::Mirror, Active::Mirror { first: None }) => {
                self.active = Some((CommandId::Mirror, Active::Mirror { first: Some(p) }));
                out.prompt = self.current_prompt();
            }
            (CommandId::Mirror, Active::Mirror { first: Some(a) }) => {
                out = self.transform_selection(|g| g.mirrored(a, p));
                self.active = None;
            }
            (CommandId::Offset, Active::Offset { distance: Some(d) }) => {
                if let Some(eid) = self.pick_entity(p) {
                    if let Some(ent) = self.session.document.get_entity(eid) {
                        if let Geometry::Line(line) = &ent.geometry {
                            // Side: sign from pick relative to line
                            let side = (p - line.start).cross(line.delta());
                            let signed = if side >= 0.0 { d } else { -d };
                            if let Some(off) = offset_line(*line, signed) {
                                out = self.commit_one(Geometry::Line(off));
                            }
                        } else {
                            out.echoes.push("OFFSET M1 : lignes uniquement.".into());
                        }
                    }
                }
                self.active = None;
            }
            (CommandId::Trim, Active::Trim) => {
                out = self.trim_at(p);
                self.active = None;
            }
            (CommandId::Extend, Active::Extend) => {
                out = self.extend_at(p);
                self.active = None;
            }
            (CommandId::Erase, Active::Erase) => {
                if let Some(eid) = self.pick_entity(p) {
                    self.selection.toggle(eid);
                    out.echoes
                        .push(format!("Sélection : {} objet(s)", self.selection.len()));
                }
                out.prompt = self.current_prompt();
            }
            _ => {}
        }
        out
    }

    fn commit_one(&mut self, geometry: Geometry) -> CommandOutput {
        let id = self.ids.next();
        let layer_id = self.session.document.current_layer;
        let record = EntityRecord {
            id,
            kind: geometry.kind(),
            geometry,
            layer_id,
        };
        let mut out = CommandOutput::default();
        match self.session.apply(Operation::CreateEntity { record }) {
            Ok(()) => {
                out.document_changed = true;
                out.echoes.push(format!("Créé {id}"));
            }
            Err(e) => out.echoes.push(format!("Erreur: {e}")),
        }
        out
    }

    fn commit_polyline(&mut self, points: Vec<Vec2>, closed: bool) -> CommandOutput {
        let poly = if closed {
            Polyline::closed(points)
        } else {
            Polyline::new(points)
        };
        self.commit_one(Geometry::Polyline(poly))
    }

    fn erase_selection(&mut self) -> CommandOutput {
        let ids = self.selection.to_vec();
        self.selection.clear();
        let ops: Vec<_> = ids
            .iter()
            .map(|id| Operation::DeleteEntity { id: *id })
            .collect();
        let n = ops.len();
        self.apply_batch(ops, format!("{n} objet(s) effacé(s)"))
    }

    fn transform_selection(&mut self, f: impl Fn(&Geometry) -> Geometry) -> CommandOutput {
        let mut ops = Vec::new();
        for id in self.selection.iter() {
            if let Some(ent) = self.session.document.get_entity(id) {
                ops.push(Operation::SetGeometry {
                    id,
                    geometry: f(&ent.geometry),
                });
            }
        }
        let n = ops.len();
        self.apply_batch(ops, format!("{n} objet(s) modifié(s)"))
    }

    fn copy_selection(&mut self, delta: Vec2) -> CommandOutput {
        let mut ops = Vec::new();
        let layer_fallback = self.session.document.current_layer;
        for id in self.selection.iter() {
            if let Some(ent) = self.session.document.get_entity(id) {
                let new_id = self.ids.next();
                ops.push(Operation::CreateEntity {
                    record: EntityRecord {
                        id: new_id,
                        kind: ent.kind,
                        geometry: ent.geometry.translated(delta),
                        layer_id: ent.layer_id,
                    },
                });
            } else {
                let _ = layer_fallback;
            }
        }
        let n = ops.len();
        self.apply_batch(ops, format!("{n} objet(s) copié(s)"))
    }

    fn apply_batch(&mut self, ops: Vec<Operation>, ok_msg: String) -> CommandOutput {
        let mut out = CommandOutput::default();
        if ops.is_empty() {
            return out;
        }
        let op = if ops.len() == 1 {
            ops.into_iter().next().unwrap()
        } else {
            Operation::Batch { ops }
        };
        match self.session.apply(op) {
            Ok(()) => {
                out.document_changed = true;
                out.echoes.push(ok_msg);
            }
            Err(e) => out.echoes.push(format!("Erreur: {e}")),
        }
        out
    }

    fn pick_entity(&self, p: Vec2) -> Option<EntityId> {
        let tol = Tolerance::COARSE;
        let mut best: Option<(EntityId, f64)> = None;
        for ent in self.session.document.entities() {
            let d = match &ent.geometry {
                Geometry::Line(l) => l.distance_squared(p).sqrt(),
                Geometry::Circle(c) => c.signed_distance(p).abs(),
                Geometry::Arc(a) => {
                    let v = p - a.center;
                    if let Some(n) = v.try_normalize(tol) {
                        let on = a.center + n * a.radius;
                        let ang = (on - a.center).angle();
                        if a.contains_angle(ang, tol) {
                            on.distance(p)
                        } else {
                            a.start_point().distance(p).min(a.end_point().distance(p))
                        }
                    } else {
                        a.center.distance(p)
                    }
                }
                Geometry::Polyline(poly) => poly
                    .segments()
                    .map(|s| s.distance_squared(p))
                    .fold(f64::INFINITY, f64::min)
                    .sqrt(),
                Geometry::Point(q) => q.distance(p),
                Geometry::Text(t) => t.position.distance(p),
                Geometry::MText(t) => t.position.distance(p),
                Geometry::Dimension(d) => d
                    .def1
                    .distance(p)
                    .min(d.def2.distance(p))
                    .min(d.dim_line.distance(p)),
                Geometry::Hatch(h) => h
                    .boundary
                    .iter()
                    .map(|q| q.distance(p))
                    .fold(f64::INFINITY, f64::min),
                Geometry::Insert(i) => i.position.distance(p),
            };
            if d <= self.snap_aperture * 2.0 && best.map(|(_, bd)| d < bd).unwrap_or(true) {
                best = Some((ent.id, d));
            }
        }
        best.map(|(id, _)| id)
    }

    fn trim_at(&mut self, p: Vec2) -> CommandOutput {
        let Some(target_id) = self.pick_entity(p) else {
            return CommandOutput {
                echoes: vec!["Aucun objet sous le curseur.".into()],
                ..Default::default()
            };
        };
        let Some(target) = self.session.document.get_entity(target_id).cloned() else {
            return CommandOutput::default();
        };
        let Geometry::Line(target_line) = target.geometry else {
            return CommandOutput {
                echoes: vec!["TRIM M1 : lignes uniquement.".into()],
                ..Default::default()
            };
        };
        let tol = Tolerance::DEFAULT;
        // Use other selected lines as cutters, or any intersecting line.
        let cutters: Vec<Line> = self
            .session
            .document
            .entities()
            .filter(|e| e.id != target_id)
            .filter_map(|e| match &e.geometry {
                Geometry::Line(l) => Some(*l),
                Geometry::Polyline(poly) => poly.segments().find(|s| {
                    matches!(
                        kelicad_geom::line_line(target_line, *s, tol),
                        kelicad_geom::LineLineResult::Point(_)
                    )
                }),
                _ => None,
            })
            .collect();

        for cut in cutters {
            if let Some(trimmed) = trim_line_by_cut(target_line, cut, p, tol) {
                return self.apply_batch(
                    vec![Operation::SetGeometry {
                        id: target_id,
                        geometry: Geometry::Line(trimmed),
                    }],
                    "Coupé.".into(),
                );
            }
        }
        CommandOutput {
            echoes: vec!["Aucune intersection de coupe.".into()],
            ..Default::default()
        }
    }

    fn extend_at(&mut self, p: Vec2) -> CommandOutput {
        let Some(target_id) = self.pick_entity(p) else {
            return CommandOutput {
                echoes: vec!["Aucun objet sous le curseur.".into()],
                ..Default::default()
            };
        };
        let Some(target) = self.session.document.get_entity(target_id).cloned() else {
            return CommandOutput::default();
        };
        let Geometry::Line(target_line) = target.geometry else {
            return CommandOutput {
                echoes: vec!["EXTEND M1 : lignes uniquement.".into()],
                ..Default::default()
            };
        };
        let tol = Tolerance::DEFAULT;
        let boundaries: Vec<Line> = self
            .session
            .document
            .entities()
            .filter(|e| e.id != target_id)
            .filter_map(|e| match &e.geometry {
                Geometry::Line(l) => Some(*l),
                _ => None,
            })
            .collect();
        for b in boundaries {
            if let Some(ext) = extend_line_to(target_line, b, p, tol) {
                return self.apply_batch(
                    vec![Operation::SetGeometry {
                        id: target_id,
                        geometry: Geometry::Line(ext),
                    }],
                    "Prolongé.".into(),
                );
            }
        }
        CommandOutput {
            echoes: vec!["Aucune frontière d'extension.".into()],
            ..Default::default()
        }
    }
}

fn prompt(fr: &str, en: &str) -> EnginePrompt {
    EnginePrompt {
        message_fr: fr.into(),
        message_en: en.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::id_gen::SequentialIdGen;

    fn engine() -> CommandEngine<SequentialIdGen> {
        CommandEngine::new(DocSession::new(), SequentialIdGen::new(1))
    }

    #[test]
    fn line_two_points_creates_entity() {
        let mut eng = engine();
        eng.handle(CommandEvent::Start(CommandId::Line));
        eng.handle(CommandEvent::Pick(Vec2::ZERO));
        eng.handle(CommandEvent::Pick(Vec2::new(10.0, 0.0)));
        assert_eq!(eng.session.document.entity_count(), 1);
        eng.handle(CommandEvent::Enter);
        assert!(eng.active_command().is_none());
    }

    #[test]
    fn circle_center_radius() {
        let mut eng = engine();
        eng.handle(CommandEvent::Text("C".into()));
        eng.handle(CommandEvent::Pick(Vec2::ZERO));
        eng.handle(CommandEvent::Text("5".into()));
        assert_eq!(eng.session.document.entity_count(), 1);
        let geom = eng
            .session
            .document
            .entities()
            .next()
            .unwrap()
            .geometry
            .clone();
        match geom {
            Geometry::Circle(c) => assert!((c.radius - 5.0).abs() < 1e-9),
            _ => panic!("expected circle"),
        }
    }

    #[test]
    fn rectang_and_undo() {
        let mut eng = engine();
        eng.handle(CommandEvent::Start(CommandId::Rectang));
        eng.handle(CommandEvent::Pick(Vec2::ZERO));
        eng.handle(CommandEvent::Pick(Vec2::new(4.0, 3.0)));
        assert_eq!(eng.session.document.entity_count(), 1);
        eng.handle(CommandEvent::Start(CommandId::Undo));
        assert_eq!(eng.session.document.entity_count(), 0);
        eng.handle(CommandEvent::Start(CommandId::Redo));
        assert_eq!(eng.session.document.entity_count(), 1);
    }

    #[test]
    fn move_selection() {
        let mut eng = engine();
        eng.handle(CommandEvent::Start(CommandId::Line));
        eng.handle(CommandEvent::Pick(Vec2::ZERO));
        eng.handle(CommandEvent::Pick(Vec2::X));
        eng.handle(CommandEvent::Enter);
        let id = eng.session.document.entities().next().unwrap().id;
        eng.selection.set_single(id);
        eng.handle(CommandEvent::Start(CommandId::Move));
        eng.handle(CommandEvent::Pick(Vec2::ZERO));
        eng.handle(CommandEvent::Pick(Vec2::new(5.0, 0.0)));
        match &eng.session.document.get_entity(id).unwrap().geometry {
            Geometry::Line(l) => {
                assert!(l.start.nearly_eq(Vec2::new(5.0, 0.0), Tolerance::DEFAULT));
            }
            _ => panic!("line"),
        }
    }

    #[test]
    fn erase_batch_undo() {
        let mut eng = engine();
        eng.handle(CommandEvent::Start(CommandId::Line));
        eng.handle(CommandEvent::Pick(Vec2::ZERO));
        eng.handle(CommandEvent::Pick(Vec2::X));
        eng.handle(CommandEvent::Enter);
        let id = eng.session.document.entities().next().unwrap().id;
        eng.selection.set_single(id);
        eng.handle(CommandEvent::Start(CommandId::Erase));
        assert_eq!(eng.session.document.entity_count(), 0);
        eng.handle(CommandEvent::Start(CommandId::Undo));
        assert_eq!(eng.session.document.entity_count(), 1);
    }
}
