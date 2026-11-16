//! Command state machine driving a [`kelicad_ops::DocSession`].

use kelicad_doc::{
    BlockDef, BlockId, DimKind, DimensionGeom, EntityId, EntityKind, EntityRecord, Geometry,
    HatchGeom, InsertGeom, LeaderGeom, RayGeom, TextAlign, TextGeom, XLineGeom, XrefDef,
    XrefId, XrefInsertGeom, XrefKind,
};
use kelicad_geom::construct::{
    arrow, capsule, center_mark, centerline, chamfer_lines, circle_from_2_points,
    circle_from_3_points, fillet_lines, grid_lines, isosceles_triangle,
    regular_polygon_center_vertex, revision_cloud, star_polygon, table_grid,
    tangents_circle_circle, tangents_point_circle, wipeout_rect,
};
use kelicad_geom::fillet_poly::{chamfer_polyline, fillet_polyline, midline, parallel_through};
use kelicad_geom::offset::offset_polyline;
use kelicad_geom::modify::{
    align_2point, align_transform_point, break_line_near, join_collinear_lines, join_polylines,
    lengthen_line, polar_array_copies, rectangular_array_offsets, stretch_line, stretch_polyline,
};
use kelicad_geom::transform::offset_line;
use kelicad_geom::trim::{
    extend_line_to, extend_polyline_to, trim_arc_by_line, trim_line_by_cut, trim_polyline_by_cut,
};
use kelicad_geom::{Arc, Circle, Ellipse, Line, Polyline, Spline};
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
    Polygon {
        sides: Option<usize>,
        center: Option<Vec2>,
    },
    Ellipse {
        center: Option<Vec2>,
        major_tip: Option<Vec2>,
    },
    Spline {
        points: Vec<Vec2>,
    },
    Circle3p {
        points: Vec<Vec2>,
    },
    Fillet {
        radius: Option<f64>,
    },
    Chamfer {
        distance: Option<f64>,
    },
    TangentPc {
        point: Option<Vec2>,
    },
    TangentCc {
        first: Option<EntityId>,
    },
    DimAligned {
        def1: Option<Vec2>,
        def2: Option<Vec2>,
    },
    DimRadius {
        center: Option<Vec2>,
    },
    DimDiameter {
        center: Option<Vec2>,
    },
    DimAngular {
        vertex: Option<Vec2>,
        p2: Option<Vec2>,
    },
    Leader {
        points: Vec<Vec2>,
    },
    ArrayRect {
        cols: Option<usize>,
        rows: Option<usize>,
        dx: Option<f64>,
    },
    ArrayPolar {
        count: Option<usize>,
        center: Option<Vec2>,
        fill_deg: Option<f64>,
    },
    Stretch {
        corner1: Option<Vec2>,
        corner2: Option<Vec2>,
        base: Option<Vec2>,
    },
    Break,
    Join,
    Explode,
    Lengthen {
        delta: Option<f64>,
    },
    Align {
        src_from: Option<Vec2>,
        src_to: Option<Vec2>,
        dst_from: Option<Vec2>,
    },
    Ray {
        origin: Option<Vec2>,
    },
    Xline {
        first: Option<Vec2>,
    },
    Xref {
        name: Option<String>,
        path: Option<String>,
        kind: XrefKind,
    },
    Circle2p {
        first: Option<Vec2>,
    },
    Parallel {
        source: Option<EntityId>,
    },
    Midline {
        first: Option<Vec2>,
    },
    Star {
        tips: Option<usize>,
        center: Option<Vec2>,
    },
    Donut {
        center: Option<Vec2>,
        outer: Option<f64>,
    },
    Capsule {
        a: Option<Vec2>,
        b: Option<Vec2>,
    },
    Arrow {
        from: Option<Vec2>,
    },
    Grid {
        cols: Option<usize>,
        rows: Option<usize>,
        origin: Option<Vec2>,
    },
    Triangle {
        a: Option<Vec2>,
        b: Option<Vec2>,
    },
    Point,
    Revcloud {
        points: Vec<Vec2>,
    },
    Centermark,
    Centerline {
        first: Option<Vec2>,
    },
    Table {
        cols: Option<usize>,
        rows: Option<usize>,
        origin: Option<Vec2>,
    },
    Wipeout {
        first: Option<Vec2>,
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
    /// ORTHO mode (constrain to H/V from last point) — LibreCAD/AutoCAD F8 style.
    pub ortho: bool,
    /// Object snap master enable (F3 style).
    pub osnap_enabled: bool,
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
            ortho: false,
            osnap_enabled: true,
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
        let mut p = cursor;
        if self.ortho {
            if let Some(base) = self.last_point {
                let dx = (cursor.x - base.x).abs();
                let dy = (cursor.y - base.y).abs();
                p = if dx >= dy {
                    Vec2::new(cursor.x, base.y)
                } else {
                    Vec2::new(base.x, cursor.y)
                };
            }
        }
        if !self.osnap_enabled {
            return p;
        }
        let q = SnapQuery {
            cursor: p,
            aperture: self.snap_aperture,
            base: self.last_point,
        };
        if let Some(hit) = self.snap.resolve(&self.session.document, q) {
            out.snap = Some(hit.clone());
            hit.point
        } else {
            p
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
            CommandId::Fillet | CommandId::Chamfer => {
                let ok_poly = self.selection.len() == 1
                    && self
                        .selection
                        .iter()
                        .next()
                        .and_then(|id| self.session.document.get_entity(id))
                        .is_some_and(|e| matches!(e.geometry, Geometry::Polyline(_)));
                if self.selection.len() < 2 && !ok_poly {
                    out.echoes
                        .push("Sélectionnez 2 lignes ou 1 polyligne avant FILLET/CHAMFER.".into());
                    return out;
                }
                let active = match id {
                    CommandId::Fillet => Active::Fillet { radius: None },
                    CommandId::Chamfer => Active::Chamfer { distance: None },
                    _ => unreachable!(),
                };
                self.active = Some((id, active));
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
            CommandId::DimAligned => {
                self.active = Some((
                    id,
                    Active::DimAligned {
                        def1: None,
                        def2: None,
                    },
                ));
                self.last_command = Some(id);
                out.prompt = self.current_prompt();
            }
            CommandId::DimRadius => {
                self.active = Some((id, Active::DimRadius { center: None }));
                self.last_command = Some(id);
                out.prompt = self.current_prompt();
            }
            CommandId::DimDiameter => {
                self.active = Some((id, Active::DimDiameter { center: None }));
                self.last_command = Some(id);
                out.prompt = self.current_prompt();
            }
            CommandId::DimAngular => {
                self.active = Some((
                    id,
                    Active::DimAngular {
                        vertex: None,
                        p2: None,
                    },
                ));
                self.last_command = Some(id);
                out.prompt = self.current_prompt();
            }
            CommandId::Leader => {
                self.active = Some((id, Active::Leader { points: vec![] }));
                self.last_command = Some(id);
                out.prompt = self.current_prompt();
            }
            CommandId::ArrayRect
            | CommandId::ArrayPolar
            | CommandId::Stretch
            | CommandId::Break
            | CommandId::Join
            | CommandId::Explode
            | CommandId::Lengthen
            | CommandId::Align => {
                if self.selection.is_empty()
                    && !matches!(id, CommandId::Break | CommandId::Lengthen)
                {
                    out.echoes
                        .push("Sélectionnez des objets avant la commande.".into());
                    return out;
                }
                let active = match id {
                    CommandId::ArrayRect => Active::ArrayRect {
                        cols: None,
                        rows: None,
                        dx: None,
                    },
                    CommandId::ArrayPolar => Active::ArrayPolar {
                        count: None,
                        center: None,
                        fill_deg: None,
                    },
                    CommandId::Stretch => Active::Stretch {
                        corner1: None,
                        corner2: None,
                        base: None,
                    },
                    CommandId::Break => Active::Break,
                    CommandId::Join => Active::Join,
                    CommandId::Explode => Active::Explode,
                    CommandId::Lengthen => Active::Lengthen { delta: None },
                    CommandId::Align => Active::Align {
                        src_from: None,
                        src_to: None,
                        dst_from: None,
                    },
                    _ => unreachable!(),
                };
                // JOIN / EXPLODE can finish immediately on start when selection ready
                if matches!(id, CommandId::Join) {
                    out = self.apply_join();
                    self.active = None;
                    self.last_command = Some(id);
                    return out;
                }
                if matches!(id, CommandId::Explode) {
                    out = self.apply_explode();
                    self.active = None;
                    self.last_command = Some(id);
                    return out;
                }
                self.active = Some((id, active));
                self.last_command = Some(id);
                out.prompt = self.current_prompt();
            }
            CommandId::Ray => {
                self.active = Some((id, Active::Ray { origin: None }));
                self.last_command = Some(id);
                out.prompt = self.current_prompt();
            }
            CommandId::Xline => {
                self.active = Some((id, Active::Xline { first: None }));
                self.last_command = Some(id);
                out.prompt = self.current_prompt();
            }
            CommandId::Xref | CommandId::PdfUnderlay => {
                let kind = if id == CommandId::PdfUnderlay {
                    XrefKind::Pdf
                } else {
                    XrefKind::Drawing
                };
                self.active = Some((
                    id,
                    Active::Xref {
                        name: None,
                        path: None,
                        kind,
                    },
                ));
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
                    CommandId::Polygon => Active::Polygon {
                        sides: None,
                        center: None,
                    },
                    CommandId::Ellipse => Active::Ellipse {
                        center: None,
                        major_tip: None,
                    },
                    CommandId::Spline => Active::Spline { points: vec![] },
                    CommandId::Circle3p => Active::Circle3p { points: vec![] },
                    CommandId::Circle2p => Active::Circle2p { first: None },
                    CommandId::Parallel => Active::Parallel { source: None },
                    CommandId::Midline => Active::Midline { first: None },
                    CommandId::TangentPc => Active::TangentPc { point: None },
                    CommandId::TangentCc => Active::TangentCc { first: None },
                    CommandId::Star => Active::Star {
                        tips: None,
                        center: None,
                    },
                    CommandId::Donut => Active::Donut {
                        center: None,
                        outer: None,
                    },
                    CommandId::Capsule => Active::Capsule { a: None, b: None },
                    CommandId::Arrow => Active::Arrow { from: None },
                    CommandId::Grid => Active::Grid {
                        cols: None,
                        rows: None,
                        origin: None,
                    },
                    CommandId::Triangle => Active::Triangle { a: None, b: None },
                    CommandId::Point => Active::Point,
                    CommandId::Revcloud => Active::Revcloud { points: vec![] },
                    CommandId::Centermark => Active::Centermark,
                    CommandId::Centerline => Active::Centerline { first: None },
                    CommandId::Table => Active::Table {
                        cols: None,
                        rows: None,
                        origin: None,
                    },
                    CommandId::Wipeout => Active::Wipeout { first: None },
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
            (CommandId::Text, Active::Text { position: None }) => {
                prompt("Point d'insertion du texte", "Text insertion point")
            }
            (CommandId::Text, _) => prompt("Saisissez le texte", "Enter text"),
            (CommandId::DimLinear, Active::DimLinear { def1: None, .. }) => {
                prompt("Premier point d'extension", "First extension point")
            }
            (CommandId::DimLinear, Active::DimLinear { def2: None, .. }) => {
                prompt("Second point d'extension", "Second extension point")
            }
            (CommandId::DimLinear, _) => {
                prompt("Emplacement de la ligne de cote", "Dimension line location")
            }
            (CommandId::Hatch, _) => prompt(
                "Point du contour (Entrée pour fermer)",
                "Boundary point (Enter to close)",
            ),
            (CommandId::Block, Active::Block { name: None, .. }) => {
                prompt("Nom du bloc", "Block name")
            }
            (CommandId::Block, _) => prompt("Point de base du bloc", "Block base point"),
            (CommandId::Insert, Active::Insert { name: None }) => {
                prompt("Nom du bloc à insérer", "Block name to insert")
            }
            (CommandId::Insert, _) => prompt("Point d'insertion", "Insertion point"),
            (CommandId::Polygon, Active::Polygon { sides: None, .. }) => {
                prompt("Nombre de côtés (≥3)", "Number of sides (≥3)")
            }
            (CommandId::Polygon, Active::Polygon { center: None, .. }) => {
                prompt("Centre du polygone", "Polygon center")
            }
            (CommandId::Polygon, _) => prompt("Sommet du polygone", "Polygon vertex"),
            (CommandId::Ellipse, Active::Ellipse { center: None, .. }) => {
                prompt("Centre de l'ellipse", "Ellipse center")
            }
            (CommandId::Ellipse, Active::Ellipse { major_tip: None, .. }) => {
                prompt("Extrémité du grand axe", "Major-axis endpoint")
            }
            (CommandId::Ellipse, _) => {
                prompt("Point définissant le petit axe", "Point defining minor axis")
            }
            (CommandId::DimAligned, Active::DimAligned { def1: None, .. }) => {
                prompt("Premier point d'extension", "First extension point")
            }
            (CommandId::DimAligned, Active::DimAligned { def2: None, .. }) => {
                prompt("Second point d'extension", "Second extension point")
            }
            (CommandId::DimAligned, _) => {
                prompt("Emplacement de la ligne de cote", "Dimension line location")
            }
            (CommandId::DimRadius, Active::DimRadius { center: None }) => {
                prompt("Centre / point sur cercle", "Center or point on circle")
            }
            (CommandId::DimRadius, _) => prompt("Point sur le cercle", "Point on circle"),
            (CommandId::DimDiameter, Active::DimDiameter { center: None }) => {
                prompt("Centre du cercle", "Circle center")
            }
            (CommandId::DimDiameter, _) => prompt("Point sur le cercle", "Point on circle"),
            (CommandId::DimAngular, Active::DimAngular { vertex: None, .. }) => {
                prompt("Sommet de l'angle", "Angle vertex")
            }
            (CommandId::DimAngular, Active::DimAngular { p2: None, .. }) => {
                prompt("Premier côté (point)", "First side point")
            }
            (CommandId::DimAngular, _) => prompt(
                "Second côté puis emplacement",
                "Second side then dim location",
            ),
            (CommandId::Leader, Active::Leader { points }) if points.is_empty() => {
                prompt("Pointe de la flèche", "Leader arrow tip")
            }
            (CommandId::Leader, _) => prompt(
                "Point suivant ou Entrée + texte",
                "Next point or Enter then text",
            ),
            (CommandId::ArrayRect, Active::ArrayRect { cols: None, .. }) => {
                prompt("Nombre de colonnes", "Number of columns")
            }
            (CommandId::ArrayRect, Active::ArrayRect { rows: None, .. }) => {
                prompt("Nombre de lignes", "Number of rows")
            }
            (CommandId::ArrayRect, Active::ArrayRect { dx: None, .. }) => {
                prompt("Espacement X", "X spacing")
            }
            (CommandId::ArrayRect, _) => prompt("Espacement Y", "Y spacing"),
            (CommandId::ArrayPolar, Active::ArrayPolar { count: None, .. }) => {
                prompt("Nombre d'éléments", "Number of items")
            }
            (CommandId::ArrayPolar, Active::ArrayPolar { center: None, .. }) => {
                prompt("Centre du réseau", "Array center")
            }
            (CommandId::ArrayPolar, Active::ArrayPolar { fill_deg: None, .. }) => {
                prompt("Angle de remplissage (degrés)", "Fill angle (degrees)")
            }
            (CommandId::ArrayPolar, _) => prompt("Point de base sur le cercle", "Base point on circle"),
            (CommandId::Stretch, Active::Stretch { corner1: None, .. }) => {
                prompt("Premier coin de la fenêtre", "First crossing window corner")
            }
            (CommandId::Stretch, Active::Stretch { corner2: None, .. }) => {
                prompt("Coin opposé", "Opposite corner")
            }
            (CommandId::Stretch, Active::Stretch { base: None, .. }) => {
                prompt("Point de base", "Base point")
            }
            (CommandId::Stretch, _) => prompt("Second point", "Second point"),
            (CommandId::Break, _) => prompt("Point de rupture sur une ligne", "Break point on a line"),
            (CommandId::Lengthen, Active::Lengthen { delta: None }) => {
                prompt("Delta de longueur (+/-)", "Length delta (+/-)")
            }
            (CommandId::Lengthen, _) => prompt(
                "Cliquez près de l'extrémité à modifier",
                "Click near the end to modify",
            ),
            (CommandId::Align, Active::Align { src_from: None, .. }) => {
                prompt("Premier point source", "First source point")
            }
            (CommandId::Align, Active::Align { src_to: None, .. }) => {
                prompt("Second point source", "Second source point")
            }
            (CommandId::Align, Active::Align { dst_from: None, .. }) => {
                prompt("Premier point destination", "First destination point")
            }
            (CommandId::Align, _) => prompt("Second point destination", "Second destination point"),
            (CommandId::Ray, Active::Ray { origin: None }) => prompt("Origine du rayon", "Ray origin"),
            (CommandId::Ray, _) => prompt("Point sur le rayon", "Point through ray"),
            (CommandId::Xline, Active::Xline { first: None }) => {
                prompt("Premier point de la xline", "First xline point")
            }
            (CommandId::Xline, _) => prompt("Second point", "Second point"),
            (CommandId::Xref, Active::Xref { name: None, .. })
            | (CommandId::PdfUnderlay, Active::Xref { name: None, .. }) => {
                prompt("Nom de la référence", "Reference name")
            }
            (CommandId::Xref, Active::Xref { path: None, .. })
            | (CommandId::PdfUnderlay, Active::Xref { path: None, .. }) => {
                prompt("Chemin du fichier", "File path")
            }
            (CommandId::Xref, _) | (CommandId::PdfUnderlay, _) => {
                prompt("Point d'insertion", "Insertion point")
            }
            (CommandId::Spline, Active::Spline { points }) if points.is_empty() => {
                prompt("Premier point d'ajustement", "First fit point")
            }
            (CommandId::Spline, _) => prompt(
                "Point suivant ou Entrée pour terminer",
                "Next point or Enter to finish",
            ),
            (CommandId::Circle3p, _) => prompt(
                "Point sur le cercle (3 points)",
                "Point on circle (3 points)",
            ),
            (CommandId::Circle2p, Active::Circle2p { first: None }) => {
                prompt("Premier point du diamètre", "First diameter point")
            }
            (CommandId::Circle2p, _) => prompt("Second point du diamètre", "Second diameter point"),
            (CommandId::Parallel, Active::Parallel { source: None }) => {
                prompt("Sélectionnez la ligne source", "Select source line")
            }
            (CommandId::Parallel, _) => prompt("Point de passage", "Through point"),
            (CommandId::Midline, Active::Midline { first: None }) => {
                prompt("Premier point", "First point")
            }
            (CommandId::Midline, _) => prompt("Second point", "Second point"),
            (CommandId::Fillet, Active::Fillet { radius: None }) => {
                prompt("Rayon du congé", "Fillet radius")
            }
            (CommandId::Chamfer, Active::Chamfer { distance: None }) => {
                prompt("Distance de chanfrein", "Chamfer distance")
            }
            (CommandId::TangentPc, Active::TangentPc { point: None }) => {
                prompt("Point extérieur", "External point")
            }
            (CommandId::TangentPc, _) => prompt("Sélectionnez un cercle", "Select a circle"),
            (CommandId::TangentCc, Active::TangentCc { first: None }) => {
                prompt("Premier cercle", "First circle")
            }
            (CommandId::TangentCc, _) => prompt("Second cercle", "Second circle"),
            (CommandId::Star, Active::Star { tips: None, .. }) => {
                prompt("Nombre de branches (≥3)", "Number of tips (≥3)")
            }
            (CommandId::Star, Active::Star { center: None, .. }) => {
                prompt("Centre de l'étoile", "Star center")
            }
            (CommandId::Star, _) => prompt("Sommet extérieur", "Outer vertex"),
            (CommandId::Donut, Active::Donut { center: None, .. }) => {
                prompt("Centre du donut", "Donut center")
            }
            (CommandId::Donut, Active::Donut { outer: None, .. }) => {
                prompt("Rayon extérieur", "Outer radius point")
            }
            (CommandId::Donut, _) => prompt("Rayon intérieur", "Inner radius point"),
            (CommandId::Capsule, Active::Capsule { a: None, .. }) => {
                prompt("Premier bout", "First end")
            }
            (CommandId::Capsule, Active::Capsule { b: None, .. }) => {
                prompt("Second bout", "Second end")
            }
            (CommandId::Capsule, _) => prompt("Rayon (distance à l'axe)", "Radius (distance to axis)"),
            (CommandId::Arrow, Active::Arrow { from: None }) => {
                prompt("Origine de la flèche", "Arrow start")
            }
            (CommandId::Arrow, _) => prompt("Pointe", "Arrow tip"),
            (CommandId::Grid, Active::Grid { cols: None, .. }) => {
                prompt("Colonnes (≥1)", "Columns (≥1)")
            }
            (CommandId::Grid, Active::Grid { rows: None, .. }) => {
                prompt("Lignes (≥1)", "Rows (≥1)")
            }
            (CommandId::Grid, Active::Grid { origin: None, .. }) => {
                prompt("Origine de la grille", "Grid origin")
            }
            (CommandId::Grid, _) => prompt("Coin d'une cellule (dx, dy)", "Cell corner (dx, dy)"),
            (CommandId::Triangle, Active::Triangle { a: None, .. }) => {
                prompt("Premier coin de la base", "First base corner")
            }
            (CommandId::Triangle, Active::Triangle { b: None, .. }) => {
                prompt("Second coin de la base", "Second base corner")
            }
            (CommandId::Triangle, _) => prompt("Sommet (hauteur)", "Apex (height)"),
            (CommandId::Point, _) => prompt("Position du point", "Point location"),
            (CommandId::Revcloud, Active::Revcloud { points }) if points.is_empty() => {
                prompt("Premier point du nuage", "First cloud point")
            }
            (CommandId::Revcloud, _) => {
                prompt("Point suivant (Entrée pour fermer)", "Next point (Enter to close)")
            }
            (CommandId::Centermark, _) => {
                prompt("Sélectionnez un cercle", "Select a circle")
            }
            (CommandId::Centerline, Active::Centerline { first: None }) => {
                prompt("Premier point d'axe", "First centerline point")
            }
            (CommandId::Centerline, _) => prompt("Second point d'axe", "Second centerline point"),
            (CommandId::Table, Active::Table { cols: None, .. }) => {
                prompt("Colonnes (≥1)", "Columns (≥1)")
            }
            (CommandId::Table, Active::Table { rows: None, .. }) => {
                prompt("Lignes (≥1)", "Rows (≥1)")
            }
            (CommandId::Table, Active::Table { origin: None, .. }) => {
                prompt("Origine du tableau", "Table origin")
            }
            (CommandId::Table, _) => prompt("Coin d'une cellule", "Cell corner"),
            (CommandId::Wipeout, Active::Wipeout { first: None }) => {
                prompt("Premier coin", "First corner")
            }
            (CommandId::Wipeout, _) => prompt("Coin opposé", "Opposite corner"),
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
            Active::Ellipse {
                center: Some(c),
                major_tip: None,
            } => vec![Geometry::Line(Line::new(*c, cursor))],
            Active::Ellipse {
                center: Some(c),
                major_tip: Some(m),
            } => {
                let major_r = c.distance(*m);
                let minor_r = ((cursor - *c).dot((*m - *c).perp().try_normalize(Tolerance::DEFAULT).unwrap_or(Vec2::Y))).abs();
                let ratio = if major_r > 1e-12 {
                    (minor_r / major_r).clamp(1e-6, 1.0)
                } else {
                    1.0
                };
                vec![Geometry::Ellipse(Ellipse::full(*c, *m, ratio))]
            }
            Active::Polygon {
                sides: Some(n),
                center: Some(c),
            } => regular_polygon_center_vertex(*c, cursor, *n)
                .map(|p| vec![Geometry::Polyline(p)])
                .unwrap_or_default(),
            Active::Spline { points } if !points.is_empty() => {
                let mut pts = points.clone();
                pts.push(cursor);
                Spline::from_fit_points(pts, false)
                    .map(|s| vec![Geometry::Spline(s)])
                    .unwrap_or_default()
            }
            Active::Circle3p { points } if points.len() == 2 => {
                circle_from_3_points(points[0], points[1], cursor, Tolerance::DEFAULT)
                    .map(|c| vec![Geometry::Circle(c)])
                    .unwrap_or_default()
            }
            Active::Rectang { first: Some(a) } => {
                vec![Geometry::Polyline(Polyline::rectangle(*a, cursor))]
            }
            Active::Star {
                tips: Some(n),
                center: Some(c),
            } => star_polygon(*c, cursor, *n, 0.4)
                .map(|p| vec![Geometry::Polyline(p)])
                .unwrap_or_default(),
            Active::Donut {
                center: Some(c),
                outer: None,
            } => {
                let r = c.distance(cursor);
                vec![Geometry::Circle(Circle::new(*c, r))]
            }
            Active::Donut {
                center: Some(c),
                outer: Some(ro),
            } => {
                let ri = c.distance(cursor).min(*ro * 0.99);
                vec![
                    Geometry::Circle(Circle::new(*c, *ro)),
                    Geometry::Circle(Circle::new(*c, ri)),
                ]
            }
            Active::Capsule {
                a: Some(a),
                b: None,
            } => vec![Geometry::Line(Line::new(*a, cursor))],
            Active::Capsule {
                a: Some(a),
                b: Some(b),
            } => {
                let ab = *b - *a;
                let len = ab.length().max(1e-12);
                let nrm = ab.perp() / len;
                let radius = ((cursor - *a).dot(nrm)).abs().max(1e-6);
                capsule(*a, *b, radius, 10)
                    .map(|p| vec![Geometry::Polyline(p)])
                    .unwrap_or_default()
            }
            Active::Arrow { from: Some(f) } => arrow(*f, cursor, f.distance(cursor) * 0.25)
                .map(|p| vec![Geometry::Polyline(p)])
                .unwrap_or_default(),
            Active::Grid {
                cols: Some(c),
                rows: Some(r),
                origin: Some(o),
            } => {
                let dx = (cursor.x - o.x).abs().max(1e-6);
                let dy = (cursor.y - o.y).abs().max(1e-6);
                grid_lines(*o, *c, *r, dx, dy)
                    .into_iter()
                    .map(Geometry::Line)
                    .collect()
            }
            Active::Triangle {
                a: Some(a),
                b: Some(b),
            } => {
                let mid = a.lerp(*b, 0.5);
                let base = *b - *a;
                let len = base.length().max(1e-12);
                let nrm = base.perp() / len;
                let height = (cursor - mid).dot(nrm);
                isosceles_triangle(*a, *b, height)
                    .map(|p| vec![Geometry::Polyline(p)])
                    .unwrap_or_default()
            }
            Active::Triangle {
                a: Some(a),
                b: None,
            } => vec![Geometry::Line(Line::new(*a, cursor))],
            Active::Revcloud { points } if !points.is_empty() => {
                let mut pts = points.clone();
                pts.push(cursor);
                revision_cloud(&pts, 0.35, false)
                    .map(|p| vec![Geometry::Polyline(p)])
                    .unwrap_or_else(|| {
                        let last = *points.last().unwrap();
                        vec![Geometry::Line(Line::new(last, cursor))]
                    })
            }
            Active::Point => {
                let s = 0.15;
                vec![
                    Geometry::Line(Line::new(
                        Vec2::new(cursor.x - s, cursor.y),
                        Vec2::new(cursor.x + s, cursor.y),
                    )),
                    Geometry::Line(Line::new(
                        Vec2::new(cursor.x, cursor.y - s),
                        Vec2::new(cursor.x, cursor.y + s),
                    )),
                ]
            }
            Active::Centerline { first: Some(a) } => centerline(*a, cursor, a.distance(cursor) * 0.05)
                .map(|ls| ls.into_iter().map(Geometry::Line).collect())
                .unwrap_or_default(),
            Active::Table {
                cols: Some(c),
                rows: Some(r),
                origin: Some(o),
            } => {
                let dx = (cursor.x - o.x).abs().max(1e-6);
                let dy = (cursor.y - o.y).abs().max(1e-6);
                table_grid(*o, *c, *r, dx, dy)
                    .into_iter()
                    .map(Geometry::Line)
                    .collect()
            }
            Active::Wipeout { first: Some(a) } => {
                vec![Geometry::Polyline(wipeout_rect(*a, cursor))]
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
            Active::Revcloud { points } => {
                if points.len() >= 2 {
                    if let Some(poly) = revision_cloud(&points, 0.35, true) {
                        out = self.commit_one(Geometry::Polyline(poly));
                    } else {
                        out.echoes.push("REVCLOUD invalide.".into());
                    }
                } else {
                    out.echoes.push("REVCLOUD : au moins 2 points.".into());
                }
                self.active = None;
            }
            Active::Point => {
                out.echoes.push("POINT terminé.".into());
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
                    out = self.commit_one(Geometry::Hatch(HatchGeom::solid(boundary)));
                } else {
                    out.echoes.push("HATCH : au moins 3 points.".into());
                }
                self.active = None;
            }
            Active::Spline { points } => {
                if points.len() >= 2 {
                    if let Some(s) = Spline::from_fit_points(points, false) {
                        out = self.commit_one(Geometry::Spline(s));
                    }
                } else {
                    out.echoes.push("SPLINE : au moins 2 points.".into());
                }
                self.active = None;
            }
            Active::Leader { points } => {
                if points.len() >= 2 {
                    out = self.commit_one(Geometry::Leader(LeaderGeom::new(
                        points[0],
                        *points.last().unwrap(),
                        "",
                    )));
                    // Keep waiting for text via on_text if empty — commit leader now
                } else {
                    out.echoes.push("LEADER : au moins 2 points.".into());
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
                style_id: None,
                annotative: false,
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

        if let Some((cid, Active::Xref { name: None, path, kind })) = self.active.clone() {
            if matches!(cid, CommandId::Xref | CommandId::PdfUnderlay) {
                self.active = Some((
                    cid,
                    Active::Xref {
                        name: Some(t.to_string()),
                        path,
                        kind,
                    },
                ));
                return CommandOutput {
                    prompt: self.current_prompt(),
                    ..Default::default()
                };
            }
        }
        if let Some((cid, Active::Xref { name: Some(name), path: None, kind })) = self.active.clone()
        {
            if matches!(cid, CommandId::Xref | CommandId::PdfUnderlay) {
                self.active = Some((
                    cid,
                    Active::Xref {
                        name: Some(name),
                        path: Some(t.to_string()),
                        kind,
                    },
                ));
                return CommandOutput {
                    prompt: self.current_prompt(),
                    ..Default::default()
                };
            }
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
            (CommandId::Polygon, Active::Polygon { sides: None, center }) => {
                let sides = n.round().max(3.0) as usize;
                self.active = Some((
                    CommandId::Polygon,
                    Active::Polygon {
                        sides: Some(sides),
                        center,
                    },
                ));
                out.prompt = self.current_prompt();
            }
            (CommandId::Star, Active::Star { tips: None, center }) => {
                let tips = n.round().max(3.0) as usize;
                self.active = Some((
                    CommandId::Star,
                    Active::Star {
                        tips: Some(tips),
                        center,
                    },
                ));
                out.prompt = self.current_prompt();
            }
            (CommandId::Grid, Active::Grid { cols: None, rows, origin }) => {
                self.active = Some((
                    CommandId::Grid,
                    Active::Grid {
                        cols: Some(n.round().max(1.0) as usize),
                        rows,
                        origin,
                    },
                ));
                out.prompt = self.current_prompt();
            }
            (
                CommandId::Grid,
                Active::Grid {
                    cols: Some(c),
                    rows: None,
                    origin,
                },
            ) => {
                self.active = Some((
                    CommandId::Grid,
                    Active::Grid {
                        cols: Some(c),
                        rows: Some(n.round().max(1.0) as usize),
                        origin,
                    },
                ));
                out.prompt = self.current_prompt();
            }
            (CommandId::Table, Active::Table { cols: None, rows, origin }) => {
                self.active = Some((
                    CommandId::Table,
                    Active::Table {
                        cols: Some(n.round().max(1.0) as usize),
                        rows,
                        origin,
                    },
                ));
                out.prompt = self.current_prompt();
            }
            (
                CommandId::Table,
                Active::Table {
                    cols: Some(c),
                    rows: None,
                    origin,
                },
            ) => {
                self.active = Some((
                    CommandId::Table,
                    Active::Table {
                        cols: Some(c),
                        rows: Some(n.round().max(1.0) as usize),
                        origin,
                    },
                ));
                out.prompt = self.current_prompt();
            }
            (CommandId::Fillet, Active::Fillet { radius: None }) => {
                out = self.apply_fillet(n.abs());
                self.active = None;
            }
            (CommandId::Chamfer, Active::Chamfer { distance: None }) => {
                out = self.apply_chamfer(n.abs());
                self.active = None;
            }
            (CommandId::ArrayRect, Active::ArrayRect { cols: None, rows, dx }) => {
                self.active = Some((
                    CommandId::ArrayRect,
                    Active::ArrayRect {
                        cols: Some(n.round().max(1.0) as usize),
                        rows,
                        dx,
                    },
                ));
                out.prompt = self.current_prompt();
            }
            (
                CommandId::ArrayRect,
                Active::ArrayRect {
                    cols: Some(c),
                    rows: None,
                    dx,
                },
            ) => {
                self.active = Some((
                    CommandId::ArrayRect,
                    Active::ArrayRect {
                        cols: Some(c),
                        rows: Some(n.round().max(1.0) as usize),
                        dx,
                    },
                ));
                out.prompt = self.current_prompt();
            }
            (
                CommandId::ArrayRect,
                Active::ArrayRect {
                    cols: Some(c),
                    rows: Some(r),
                    dx: None,
                },
            ) => {
                self.active = Some((
                    CommandId::ArrayRect,
                    Active::ArrayRect {
                        cols: Some(c),
                        rows: Some(r),
                        dx: Some(n),
                    },
                ));
                out.prompt = self.current_prompt();
            }
            (
                CommandId::ArrayRect,
                Active::ArrayRect {
                    cols: Some(c),
                    rows: Some(r),
                    dx: Some(dx),
                },
            ) => {
                out = self.apply_array_rect(c, r, dx, n);
                self.active = None;
            }
            (
                CommandId::ArrayPolar,
                Active::ArrayPolar {
                    count: None,
                    center,
                    fill_deg,
                },
            ) => {
                self.active = Some((
                    CommandId::ArrayPolar,
                    Active::ArrayPolar {
                        count: Some(n.round().max(1.0) as usize),
                        center,
                        fill_deg,
                    },
                ));
                out.prompt = self.current_prompt();
            }
            (
                CommandId::ArrayPolar,
                Active::ArrayPolar {
                    count: Some(c),
                    center: Some(cen),
                    fill_deg: None,
                },
            ) => {
                self.active = Some((
                    CommandId::ArrayPolar,
                    Active::ArrayPolar {
                        count: Some(c),
                        center: Some(cen),
                        fill_deg: Some(n),
                    },
                ));
                out.prompt = self.current_prompt();
            }
            (CommandId::Lengthen, Active::Lengthen { delta: None }) => {
                self.active = Some((CommandId::Lengthen, Active::Lengthen { delta: Some(n) }));
                out.prompt = self.current_prompt();
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
                        match &ent.geometry {
                            Geometry::Line(line) => {
                                let side = (p - line.start).cross(line.delta());
                                let signed = if side >= 0.0 { d } else { -d };
                                if let Some(off) = offset_line(*line, signed) {
                                    out = self.commit_one(Geometry::Line(off));
                                }
                            }
                            Geometry::Polyline(poly) => {
                                // Sign from pick vs first segment.
                                let signed = if let Some(seg) = poly.segment(0) {
                                    let side = (p - seg.start).cross(seg.delta());
                                    if side >= 0.0 { d } else { -d }
                                } else {
                                    d
                                };
                                match offset_polyline(poly, signed, Tolerance::DEFAULT) {
                                    Some(off) => {
                                        out = self.commit_one(Geometry::Polyline(off));
                                    }
                                    None => out.echoes.push("OFFSET polyligne impossible.".into()),
                                }
                            }
                            _ => out
                                .echoes
                                .push("OFFSET : ligne ou polyligne requise.".into()),
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
            (CommandId::Text, Active::Text { position: None }) => {
                self.active = Some((CommandId::Text, Active::Text { position: Some(p) }));
                out.prompt = self.current_prompt();
            }
            (
                CommandId::DimLinear,
                Active::DimLinear {
                    def1: None,
                    def2: None,
                },
            ) => {
                self.active = Some((
                    CommandId::DimLinear,
                    Active::DimLinear {
                        def1: Some(p),
                        def2: None,
                    },
                ));
                out.prompt = self.current_prompt();
            }
            (
                CommandId::DimLinear,
                Active::DimLinear {
                    def1: Some(a),
                    def2: None,
                },
            ) => {
                self.active = Some((
                    CommandId::DimLinear,
                    Active::DimLinear {
                        def1: Some(a),
                        def2: Some(p),
                    },
                ));
                out.prompt = self.current_prompt();
            }
            (
                CommandId::DimLinear,
                Active::DimLinear {
                    def1: Some(a),
                    def2: Some(b),
                },
            ) => {
                out = self.commit_one(Geometry::Dimension(DimensionGeom {
                    kind: DimKind::Linear,
                    def1: a,
                    def2: b,
                    dim_line: p,
                    assoc_a: None,
                    assoc_b: None,
                    text_override: None,
                    style_id: Some(self.session.document.current_dim_style),
                    annotative: false,
                    angle_p3: None,
                }));
                self.active = None;
            }
            (CommandId::Hatch, Active::Hatch { mut points }) => {
                points.push(p);
                self.active = Some((CommandId::Hatch, Active::Hatch { points }));
                out.prompt = self.current_prompt();
            }
            (
                CommandId::Block,
                Active::Block {
                    name: Some(name),
                    base: None,
                },
            ) => {
                out = self.create_block_from_selection(&name, p);
                self.active = None;
            }
            (CommandId::Insert, Active::Insert { name: Some(name) }) => {
                if let Some(def) = self.session.document.block_by_name(&name) {
                    let block_id = def.id;
                    out = self.commit_one(Geometry::Insert(InsertGeom {
                        block_id,
                        position: p,
                        scale: Vec2::new(1.0, 1.0),
                        rotation: 0.0,
                        attribs: Default::default(),
                    }));
                } else {
                    out.echoes.push(format!("Bloc inconnu : {name}"));
                }
                self.active = None;
            }
            (
                CommandId::Polygon,
                Active::Polygon {
                    sides: Some(n),
                    center: None,
                },
            ) => {
                self.active = Some((
                    CommandId::Polygon,
                    Active::Polygon {
                        sides: Some(n),
                        center: Some(p),
                    },
                ));
                out.prompt = self.current_prompt();
            }
            (
                CommandId::Polygon,
                Active::Polygon {
                    sides: Some(n),
                    center: Some(c),
                },
            ) => {
                if let Some(poly) = regular_polygon_center_vertex(c, p, n) {
                    out = self.commit_one(Geometry::Polyline(poly));
                } else {
                    out.echoes.push("POLYGON invalide.".into());
                }
                self.active = None;
            }
            (CommandId::Ellipse, Active::Ellipse { center: None, .. }) => {
                self.active = Some((
                    CommandId::Ellipse,
                    Active::Ellipse {
                        center: Some(p),
                        major_tip: None,
                    },
                ));
                out.prompt = self.current_prompt();
            }
            (
                CommandId::Ellipse,
                Active::Ellipse {
                    center: Some(c),
                    major_tip: None,
                },
            ) => {
                self.active = Some((
                    CommandId::Ellipse,
                    Active::Ellipse {
                        center: Some(c),
                        major_tip: Some(p),
                    },
                ));
                out.prompt = self.current_prompt();
            }
            (
                CommandId::Ellipse,
                Active::Ellipse {
                    center: Some(c),
                    major_tip: Some(m),
                },
            ) => {
                let major_r = c.distance(m);
                let axis = (m - c)
                    .try_normalize(Tolerance::DEFAULT)
                    .unwrap_or(Vec2::X);
                let minor_r = (p - c).dot(axis.perp()).abs();
                let ratio = if major_r > 1e-12 {
                    (minor_r / major_r).clamp(1e-6, 1.0)
                } else {
                    1.0
                };
                out = self.commit_one(Geometry::Ellipse(Ellipse::full(c, m, ratio)));
                self.active = None;
            }
            (CommandId::Spline, Active::Spline { mut points }) => {
                points.push(p);
                self.active = Some((CommandId::Spline, Active::Spline { points }));
                out.prompt = self.current_prompt();
            }
            (CommandId::Circle3p, Active::Circle3p { mut points }) => {
                points.push(p);
                if points.len() >= 3 {
                    match circle_from_3_points(
                        points[0],
                        points[1],
                        points[2],
                        Tolerance::DEFAULT,
                    ) {
                        Some(c) => out = self.commit_one(Geometry::Circle(c)),
                        None => out.echoes.push("Points colinéaires.".into()),
                    }
                    self.active = None;
                } else {
                    self.active = Some((CommandId::Circle3p, Active::Circle3p { points }));
                    out.prompt = self.current_prompt();
                }
            }
            (CommandId::Circle2p, Active::Circle2p { first: None }) => {
                self.active = Some((CommandId::Circle2p, Active::Circle2p { first: Some(p) }));
                out.prompt = self.current_prompt();
            }
            (CommandId::Circle2p, Active::Circle2p { first: Some(a) }) => {
                match circle_from_2_points(a, p, Tolerance::DEFAULT) {
                    Some(c) => out = self.commit_one(Geometry::Circle(c)),
                    None => out.echoes.push("CIRCLE2P dégénéré.".into()),
                }
                self.active = None;
            }
            (CommandId::Parallel, Active::Parallel { source: None }) => {
                if let Some(eid) = self.pick_entity(p) {
                    if matches!(
                        self.session.document.get_entity(eid).map(|e| &e.geometry),
                        Some(Geometry::Line(_))
                    ) {
                        self.active =
                            Some((CommandId::Parallel, Active::Parallel { source: Some(eid) }));
                        out.prompt = self.current_prompt();
                    } else {
                        out.echoes.push("PARALLEL : sélectionnez une ligne.".into());
                    }
                }
            }
            (CommandId::Parallel, Active::Parallel { source: Some(sid) }) => {
                if let Some(ent) = self.session.document.get_entity(sid) {
                    if let Geometry::Line(line) = ent.geometry {
                        if let Some(pl) = parallel_through(line, p, Tolerance::DEFAULT) {
                            out = self.commit_one(Geometry::Line(pl));
                        }
                    }
                }
                self.active = None;
            }
            (CommandId::Midline, Active::Midline { first: None }) => {
                self.active = Some((CommandId::Midline, Active::Midline { first: Some(p) }));
                out.prompt = self.current_prompt();
            }
            (CommandId::Midline, Active::Midline { first: Some(a) }) => {
                match midline(a, p, Tolerance::DEFAULT) {
                    Some(l) => out = self.commit_one(Geometry::Line(l)),
                    None => out.echoes.push("MIDLINE dégénérée.".into()),
                }
                self.active = None;
            }
            (
                CommandId::Star,
                Active::Star {
                    tips: Some(n),
                    center: None,
                },
            ) => {
                self.active = Some((
                    CommandId::Star,
                    Active::Star {
                        tips: Some(n),
                        center: Some(p),
                    },
                ));
                out.prompt = self.current_prompt();
            }
            (
                CommandId::Star,
                Active::Star {
                    tips: Some(n),
                    center: Some(c),
                },
            ) => {
                if let Some(poly) = star_polygon(c, p, n, 0.4) {
                    out = self.commit_one(Geometry::Polyline(poly));
                } else {
                    out.echoes.push("STAR invalide.".into());
                }
                self.active = None;
            }
            (CommandId::Donut, Active::Donut { center: None, .. }) => {
                self.active = Some((
                    CommandId::Donut,
                    Active::Donut {
                        center: Some(p),
                        outer: None,
                    },
                ));
                out.prompt = self.current_prompt();
            }
            (
                CommandId::Donut,
                Active::Donut {
                    center: Some(c),
                    outer: None,
                },
            ) => {
                let ro = c.distance(p);
                if ro < 1e-9 {
                    out.echoes.push("Rayon extérieur nul.".into());
                    self.active = None;
                } else {
                    self.active = Some((
                        CommandId::Donut,
                        Active::Donut {
                            center: Some(c),
                            outer: Some(ro),
                        },
                    ));
                    out.prompt = self.current_prompt();
                }
            }
            (
                CommandId::Donut,
                Active::Donut {
                    center: Some(c),
                    outer: Some(ro),
                },
            ) => {
                let ri = c.distance(p).min(ro * 0.99).max(1e-9);
                let layer_id = self.session.document.current_layer;
                let mut ops = Vec::new();
                for (r, kind) in [(ro, EntityKind::Circle), (ri, EntityKind::Circle)] {
                    let id = self.ids.next();
                    ops.push(Operation::CreateEntity {
                        record: EntityRecord {
                            id,
                            kind,
                            geometry: Geometry::Circle(Circle::new(c, r)),
                            layer_id,
                        },
                    });
                }
                out = self.apply_batch(ops, "Donut créé".into());
                self.active = None;
            }
            (CommandId::Capsule, Active::Capsule { a: None, .. }) => {
                self.active = Some((CommandId::Capsule, Active::Capsule { a: Some(p), b: None }));
                out.prompt = self.current_prompt();
            }
            (CommandId::Capsule, Active::Capsule { a: Some(a), b: None }) => {
                self.active = Some((
                    CommandId::Capsule,
                    Active::Capsule {
                        a: Some(a),
                        b: Some(p),
                    },
                ));
                out.prompt = self.current_prompt();
            }
            (
                CommandId::Capsule,
                Active::Capsule {
                    a: Some(a),
                    b: Some(b),
                },
            ) => {
                let ab = b - a;
                let len = ab.length().max(1e-12);
                let nrm = ab.perp() / len;
                let radius = ((p - a).dot(nrm)).abs().max(1e-6);
                if let Some(poly) = capsule(a, b, radius, 12) {
                    out = self.commit_one(Geometry::Polyline(poly));
                } else {
                    out.echoes.push("CAPSULE invalide.".into());
                }
                self.active = None;
            }
            (CommandId::Arrow, Active::Arrow { from: None }) => {
                self.active = Some((CommandId::Arrow, Active::Arrow { from: Some(p) }));
                out.prompt = self.current_prompt();
            }
            (CommandId::Arrow, Active::Arrow { from: Some(f) }) => {
                let hs = f.distance(p) * 0.25;
                if let Some(poly) = arrow(f, p, hs) {
                    out = self.commit_one(Geometry::Polyline(poly));
                } else {
                    out.echoes.push("ARROW invalide.".into());
                }
                self.active = None;
            }
            (
                CommandId::Grid,
                Active::Grid {
                    cols: Some(c),
                    rows: Some(r),
                    origin: None,
                },
            ) => {
                self.active = Some((
                    CommandId::Grid,
                    Active::Grid {
                        cols: Some(c),
                        rows: Some(r),
                        origin: Some(p),
                    },
                ));
                out.prompt = self.current_prompt();
            }
            (
                CommandId::Grid,
                Active::Grid {
                    cols: Some(c),
                    rows: Some(r),
                    origin: Some(o),
                },
            ) => {
                let dx = (p.x - o.x).abs().max(1e-6);
                let dy = (p.y - o.y).abs().max(1e-6);
                let layer_id = self.session.document.current_layer;
                let mut ops = Vec::new();
                for line in grid_lines(o, c, r, dx, dy) {
                    let id = self.ids.next();
                    ops.push(Operation::CreateEntity {
                        record: EntityRecord {
                            id,
                            kind: EntityKind::Line,
                            geometry: Geometry::Line(line),
                            layer_id,
                        },
                    });
                }
                out = self.apply_batch(ops, format!("Grille {c}×{r}"));
                self.active = None;
            }
            (CommandId::Triangle, Active::Triangle { a: None, .. }) => {
                self.active = Some((CommandId::Triangle, Active::Triangle { a: Some(p), b: None }));
                out.prompt = self.current_prompt();
            }
            (CommandId::Triangle, Active::Triangle { a: Some(a), b: None }) => {
                self.active = Some((
                    CommandId::Triangle,
                    Active::Triangle {
                        a: Some(a),
                        b: Some(p),
                    },
                ));
                out.prompt = self.current_prompt();
            }
            (
                CommandId::Triangle,
                Active::Triangle {
                    a: Some(a),
                    b: Some(b),
                },
            ) => {
                let mid = a.lerp(b, 0.5);
                let base = b - a;
                let len = base.length().max(1e-12);
                let nrm = base.perp() / len;
                let height = (p - mid).dot(nrm);
                if let Some(poly) = isosceles_triangle(a, b, height) {
                    out = self.commit_one(Geometry::Polyline(poly));
                } else {
                    out.echoes.push("TRIANGLE invalide.".into());
                }
                self.active = None;
            }
            (CommandId::Point, Active::Point) => {
                out = self.commit_one(Geometry::Point(p));
                // Keep POINT active for multi-click (LibreCAD style).
                self.active = Some((CommandId::Point, Active::Point));
                out.prompt = self.current_prompt();
            }
            (CommandId::Revcloud, Active::Revcloud { mut points }) => {
                points.push(p);
                self.active = Some((CommandId::Revcloud, Active::Revcloud { points }));
                out.prompt = self.current_prompt();
            }
            (CommandId::Centermark, Active::Centermark) => {
                if let Some(eid) = self.pick_entity(p) {
                    if let Some(ent) = self.session.document.get_entity(eid) {
                        if let Geometry::Circle(c) = ent.geometry {
                            let layer_id = self.session.document.current_layer;
                            let mut ops = Vec::new();
                            for line in center_mark(c.center, c.radius, 1.15) {
                                let id = self.ids.next();
                                ops.push(Operation::CreateEntity {
                                    record: EntityRecord {
                                        id,
                                        kind: EntityKind::Line,
                                        geometry: Geometry::Line(line),
                                        layer_id,
                                    },
                                });
                            }
                            out = self.apply_batch(ops, "Center mark".into());
                        } else {
                            out.echoes.push("Sélectionnez un cercle.".into());
                        }
                    }
                }
                self.active = None;
            }
            (CommandId::Centerline, Active::Centerline { first: None }) => {
                self.active = Some((CommandId::Centerline, Active::Centerline { first: Some(p) }));
                out.prompt = self.current_prompt();
            }
            (CommandId::Centerline, Active::Centerline { first: Some(a) }) => {
                let tick = a.distance(p) * 0.05;
                if let Some(lines) = centerline(a, p, tick) {
                    let layer_id = self.session.document.current_layer;
                    let mut ops = Vec::new();
                    for line in lines {
                        let id = self.ids.next();
                        ops.push(Operation::CreateEntity {
                            record: EntityRecord {
                                id,
                                kind: EntityKind::Line,
                                geometry: Geometry::Line(line),
                                layer_id,
                            },
                        });
                    }
                    out = self.apply_batch(ops, "CENTERLINE".into());
                } else {
                    out.echoes.push("CENTERLINE invalide.".into());
                }
                self.active = None;
            }
            (
                CommandId::Table,
                Active::Table {
                    cols: Some(c),
                    rows: Some(r),
                    origin: None,
                },
            ) => {
                self.active = Some((
                    CommandId::Table,
                    Active::Table {
                        cols: Some(c),
                        rows: Some(r),
                        origin: Some(p),
                    },
                ));
                out.prompt = self.current_prompt();
            }
            (
                CommandId::Table,
                Active::Table {
                    cols: Some(c),
                    rows: Some(r),
                    origin: Some(o),
                },
            ) => {
                let dx = (p.x - o.x).abs().max(1e-6);
                let dy = (p.y - o.y).abs().max(1e-6);
                let layer_id = self.session.document.current_layer;
                let mut ops = Vec::new();
                for line in table_grid(o, c, r, dx, dy) {
                    let id = self.ids.next();
                    ops.push(Operation::CreateEntity {
                        record: EntityRecord {
                            id,
                            kind: EntityKind::Line,
                            geometry: Geometry::Line(line),
                            layer_id,
                        },
                    });
                }
                out = self.apply_batch(ops, format!("TABLE {c}×{r}"));
                self.active = None;
            }
            (CommandId::Wipeout, Active::Wipeout { first: None }) => {
                self.active = Some((CommandId::Wipeout, Active::Wipeout { first: Some(p) }));
                out.prompt = self.current_prompt();
            }
            (CommandId::Wipeout, Active::Wipeout { first: Some(a) }) => {
                let poly = wipeout_rect(a, p);
                out = self.commit_one(Geometry::Hatch(HatchGeom::solid(poly.vertices)));
                self.active = None;
            }
            (CommandId::TangentPc, Active::TangentPc { point: None }) => {
                self.active = Some((CommandId::TangentPc, Active::TangentPc { point: Some(p) }));
                out.prompt = self.current_prompt();
            }
            (CommandId::TangentPc, Active::TangentPc { point: Some(pt) }) => {
                if let Some(eid) = self.pick_entity(p) {
                    if let Some(ent) = self.session.document.get_entity(eid) {
                        if let Geometry::Circle(c) = ent.geometry {
                            let lines = tangents_point_circle(pt, c, Tolerance::DEFAULT);
                            if lines.is_empty() {
                                out.echoes.push("Aucune tangente (point intérieur).".into());
                            } else {
                                let mut ops = Vec::new();
                                let layer_id = self.session.document.current_layer;
                                for line in lines {
                                    let id = self.ids.next();
                                    ops.push(Operation::CreateEntity {
                                        record: EntityRecord {
                                            id,
                                            kind: EntityKind::Line,
                                            geometry: Geometry::Line(line),
                                            layer_id,
                                        },
                                    });
                                }
                                out = self.apply_batch(ops, "Tangente(s) créées".into());
                            }
                        } else {
                            out.echoes.push("Sélectionnez un cercle.".into());
                        }
                    }
                }
                self.active = None;
            }
            (CommandId::TangentCc, Active::TangentCc { first: None }) => {
                if let Some(eid) = self.pick_entity(p) {
                    if matches!(
                        self.session.document.get_entity(eid).map(|e| &e.geometry),
                        Some(Geometry::Circle(_))
                    ) {
                        self.active =
                            Some((CommandId::TangentCc, Active::TangentCc { first: Some(eid) }));
                        out.prompt = self.current_prompt();
                    } else {
                        out.echoes.push("Premier objet : cercle requis.".into());
                    }
                }
            }
            (CommandId::TangentCc, Active::TangentCc { first: Some(a_id) }) => {
                if let Some(b_id) = self.pick_entity(p) {
                    let ga = self.session.document.get_entity(a_id).map(|e| e.geometry.clone());
                    let gb = self.session.document.get_entity(b_id).map(|e| e.geometry.clone());
                    match (ga, gb) {
                        (Some(Geometry::Circle(ca)), Some(Geometry::Circle(cb))) => {
                            let lines = tangents_circle_circle(ca, cb, Tolerance::DEFAULT);
                            if lines.is_empty() {
                                out.echoes.push("Aucune tangente commune externe.".into());
                            } else {
                                let mut ops = Vec::new();
                                let layer_id = self.session.document.current_layer;
                                for line in lines {
                                    let id = self.ids.next();
                                    ops.push(Operation::CreateEntity {
                                        record: EntityRecord {
                                            id,
                                            kind: EntityKind::Line,
                                            geometry: Geometry::Line(line),
                                            layer_id,
                                        },
                                    });
                                }
                                out = self.apply_batch(ops, "Tangentes communes créées".into());
                            }
                        }
                        _ => out.echoes.push("Deux cercles requis.".into()),
                    }
                }
                self.active = None;
            }

            (
                CommandId::DimAligned,
                Active::DimAligned {
                    def1: None,
                    def2: None,
                },
            ) => {
                self.active = Some((
                    CommandId::DimAligned,
                    Active::DimAligned {
                        def1: Some(p),
                        def2: None,
                    },
                ));
                out.prompt = self.current_prompt();
            }
            (
                CommandId::DimAligned,
                Active::DimAligned {
                    def1: Some(a),
                    def2: None,
                },
            ) => {
                self.active = Some((
                    CommandId::DimAligned,
                    Active::DimAligned {
                        def1: Some(a),
                        def2: Some(p),
                    },
                ));
                out.prompt = self.current_prompt();
            }
            (
                CommandId::DimAligned,
                Active::DimAligned {
                    def1: Some(a),
                    def2: Some(b),
                },
            ) => {
                out = self.commit_one(Geometry::Dimension(DimensionGeom {
                    kind: DimKind::Aligned,
                    def1: a,
                    def2: b,
                    dim_line: p,
                    assoc_a: None,
                    assoc_b: None,
                    text_override: None,
                    style_id: Some(self.session.document.current_dim_style),
                    annotative: false,
                    angle_p3: None,
                }));
                self.active = None;
            }
            (CommandId::DimRadius, Active::DimRadius { center: None }) => {
                self.active = Some((CommandId::DimRadius, Active::DimRadius { center: Some(p) }));
                out.prompt = self.current_prompt();
            }
            (CommandId::DimRadius, Active::DimRadius { center: Some(c) }) => {
                out = self.commit_one(Geometry::Dimension(DimensionGeom {
                    kind: DimKind::Radius,
                    def1: c,
                    def2: p,
                    dim_line: p,
                    assoc_a: None,
                    assoc_b: None,
                    text_override: None,
                    style_id: Some(self.session.document.current_dim_style),
                    annotative: false,
                    angle_p3: None,
                }));
                self.active = None;
            }
            (CommandId::DimDiameter, Active::DimDiameter { center: None }) => {
                self.active =
                    Some((CommandId::DimDiameter, Active::DimDiameter { center: Some(p) }));
                out.prompt = self.current_prompt();
            }
            (CommandId::DimDiameter, Active::DimDiameter { center: Some(c) }) => {
                out = self.commit_one(Geometry::Dimension(DimensionGeom {
                    kind: DimKind::Diameter,
                    def1: c,
                    def2: p,
                    dim_line: p,
                    assoc_a: None,
                    assoc_b: None,
                    text_override: None,
                    style_id: Some(self.session.document.current_dim_style),
                    annotative: false,
                    angle_p3: None,
                }));
                self.active = None;
            }
            (
                CommandId::DimAngular,
                Active::DimAngular {
                    vertex: None,
                    p2: None,
                },
            ) => {
                self.active = Some((
                    CommandId::DimAngular,
                    Active::DimAngular {
                        vertex: Some(p),
                        p2: None,
                    },
                ));
                out.prompt = self.current_prompt();
            }
            (
                CommandId::DimAngular,
                Active::DimAngular {
                    vertex: Some(v),
                    p2: None,
                },
            ) => {
                self.active = Some((
                    CommandId::DimAngular,
                    Active::DimAngular {
                        vertex: Some(v),
                        p2: Some(p),
                    },
                ));
                out.prompt = self.current_prompt();
            }
            (
                CommandId::DimAngular,
                Active::DimAngular {
                    vertex: Some(v),
                    p2: Some(a),
                },
            ) => {
                // Third click = second side; dim_line midway for now, angle_p3 = p
                let mid = Vec2::new((a.x + p.x) * 0.5, (a.y + p.y) * 0.5);
                out = self.commit_one(Geometry::Dimension(DimensionGeom {
                    kind: DimKind::Angular,
                    def1: v,
                    def2: a,
                    dim_line: mid,
                    assoc_a: None,
                    assoc_b: None,
                    text_override: None,
                    style_id: Some(self.session.document.current_dim_style),
                    annotative: false,
                    angle_p3: Some(p),
                }));
                self.active = None;
            }
            (CommandId::Leader, Active::Leader { mut points }) => {
                points.push(p);
                self.active = Some((CommandId::Leader, Active::Leader { points }));
                out.prompt = self.current_prompt();
            }
            (
                CommandId::Stretch,
                Active::Stretch {
                    corner1: None,
                    corner2: None,
                    base: None,
                },
            ) => {
                self.active = Some((
                    CommandId::Stretch,
                    Active::Stretch {
                        corner1: Some(p),
                        corner2: None,
                        base: None,
                    },
                ));
                out.prompt = self.current_prompt();
            }
            (
                CommandId::Stretch,
                Active::Stretch {
                    corner1: Some(c1),
                    corner2: None,
                    base: None,
                },
            ) => {
                self.active = Some((
                    CommandId::Stretch,
                    Active::Stretch {
                        corner1: Some(c1),
                        corner2: Some(p),
                        base: None,
                    },
                ));
                out.prompt = self.current_prompt();
            }
            (
                CommandId::Stretch,
                Active::Stretch {
                    corner1: Some(c1),
                    corner2: Some(c2),
                    base: None,
                },
            ) => {
                self.active = Some((
                    CommandId::Stretch,
                    Active::Stretch {
                        corner1: Some(c1),
                        corner2: Some(c2),
                        base: Some(p),
                    },
                ));
                out.prompt = self.current_prompt();
            }
            (
                CommandId::Stretch,
                Active::Stretch {
                    corner1: Some(c1),
                    corner2: Some(c2),
                    base: Some(b),
                },
            ) => {
                out = self.apply_stretch(c1, c2, p - b);
                self.active = None;
            }
            (CommandId::Break, Active::Break) => {
                out = self.apply_break_at(p);
                self.active = None;
            }
            (CommandId::Lengthen, Active::Lengthen { delta: Some(d) }) => {
                out = self.apply_lengthen_at(p, d);
                self.active = None;
            }
            (
                CommandId::Align,
                Active::Align {
                    src_from: None,
                    src_to: None,
                    dst_from: None,
                },
            ) => {
                self.active = Some((
                    CommandId::Align,
                    Active::Align {
                        src_from: Some(p),
                        src_to: None,
                        dst_from: None,
                    },
                ));
                out.prompt = self.current_prompt();
            }
            (
                CommandId::Align,
                Active::Align {
                    src_from: Some(a),
                    src_to: None,
                    dst_from: None,
                },
            ) => {
                self.active = Some((
                    CommandId::Align,
                    Active::Align {
                        src_from: Some(a),
                        src_to: Some(p),
                        dst_from: None,
                    },
                ));
                out.prompt = self.current_prompt();
            }
            (
                CommandId::Align,
                Active::Align {
                    src_from: Some(a),
                    src_to: Some(b),
                    dst_from: None,
                },
            ) => {
                self.active = Some((
                    CommandId::Align,
                    Active::Align {
                        src_from: Some(a),
                        src_to: Some(b),
                        dst_from: Some(p),
                    },
                ));
                out.prompt = self.current_prompt();
            }
            (
                CommandId::Align,
                Active::Align {
                    src_from: Some(sf),
                    src_to: Some(st),
                    dst_from: Some(df),
                },
            ) => {
                out = self.apply_align(sf, st, df, p);
                self.active = None;
            }
            (CommandId::Ray, Active::Ray { origin: None }) => {
                self.active = Some((CommandId::Ray, Active::Ray { origin: Some(p) }));
                out.prompt = self.current_prompt();
            }
            (CommandId::Ray, Active::Ray { origin: Some(o) }) => {
                if let Some(r) = RayGeom::from_points(o, p) {
                    out = self.commit_one(Geometry::Ray(r));
                } else {
                    out.echoes.push("RAY dégénéré.".into());
                }
                self.active = None;
            }
            (CommandId::Xline, Active::Xline { first: None }) => {
                self.active = Some((CommandId::Xline, Active::Xline { first: Some(p) }));
                out.prompt = self.current_prompt();
            }
            (CommandId::Xline, Active::Xline { first: Some(a) }) => {
                if let Some(x) = XLineGeom::from_points(a, p) {
                    out = self.commit_one(Geometry::XLine(x));
                } else {
                    out.echoes.push("XLINE dégénérée.".into());
                }
                self.active = None;
            }
            (
                CommandId::Xref | CommandId::PdfUnderlay,
                Active::Xref {
                    name: Some(name),
                    path: Some(path),
                    kind,
                },
            ) => {
                out = self.apply_xref_insert(&name, &path, kind, p);
                self.active = None;
            }
            (
                CommandId::ArrayPolar,
                Active::ArrayPolar {
                    count: Some(count),
                    center: None,
                    fill_deg,
                },
            ) => {
                self.active = Some((
                    CommandId::ArrayPolar,
                    Active::ArrayPolar {
                        count: Some(count),
                        center: Some(p),
                        fill_deg,
                    },
                ));
                out.prompt = self.current_prompt();
            }
            (
                CommandId::ArrayPolar,
                Active::ArrayPolar {
                    count: Some(count),
                    center: Some(c),
                    fill_deg: Some(fill),
                },
            ) => {
                out = self.apply_array_polar(count, c, p, fill);
                self.active = None;
            }

            _ => {}
        }
        out
    }

    fn create_block_from_selection(&mut self, name: &str, base: Vec2) -> CommandOutput {
        let mut entities = Vec::new();
        let mut delete_ops = Vec::new();
        for id in self.selection.iter() {
            if let Some(ent) = self.session.document.get_entity(id) {
                entities.push(ent.geometry.translated(Vec2::new(-base.x, -base.y)));
                delete_ops.push(Operation::DeleteEntity { id });
            }
        }
        let block_id = BlockId::from_uuid(Uuid::now_v7());
        let def = BlockDef {
            id: block_id,
            name: name.to_string(),
            origin: Vec2::ZERO,
            entities,
            attributes: vec![],
        };
        let mut ops = vec![Operation::UpsertBlock { def }];
        ops.extend(delete_ops);
        self.selection.clear();
        self.apply_batch(ops, format!("Bloc « {name} » créé"))
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

    fn selected_two_lines(&self) -> Option<(EntityId, Line, EntityId, Line)> {
        let ids = self.selection.to_vec();
        if ids.len() < 2 {
            return None;
        }
        let a = self.session.document.get_entity(ids[0])?;
        let b = self.session.document.get_entity(ids[1])?;
        let Geometry::Line(la) = a.geometry else {
            return None;
        };
        let Geometry::Line(lb) = b.geometry else {
            return None;
        };
        Some((a.id, la, b.id, lb))
    }

    fn apply_fillet(&mut self, radius: f64) -> CommandOutput {
        // Prefer single selected polyline → fillet all corners.
        if self.selection.len() == 1 {
            let id = self.selection.to_vec()[0];
            if let Some(ent) = self.session.document.get_entity(id) {
                if let Geometry::Polyline(poly) = &ent.geometry {
                    return match fillet_polyline(poly, radius, 8, Tolerance::DEFAULT) {
                        Some(fp) => self.apply_batch(
                            vec![Operation::SetGeometry {
                                id,
                                geometry: Geometry::Polyline(fp),
                            }],
                            "Congé polyligne.".into(),
                        ),
                        None => CommandOutput {
                            echoes: vec!["FILLET polyligne impossible.".into()],
                            ..Default::default()
                        },
                    };
                }
            }
        }
        let Some((id_a, la, id_b, lb)) = self.selected_two_lines() else {
            return CommandOutput {
                echoes: vec!["FILLET : 2 lignes ou 1 polyligne.".into()],
                ..Default::default()
            };
        };
        let Some((ta, tb, arc)) = fillet_lines(la, lb, radius, Tolerance::DEFAULT) else {
            return CommandOutput {
                echoes: vec!["FILLET impossible (parallèles ou rayon trop grand).".into()],
                ..Default::default()
            };
        };
        let arc_id = self.ids.next();
        let layer_id = self.session.document.current_layer;
        let ops = vec![
            Operation::SetGeometry {
                id: id_a,
                geometry: Geometry::Line(ta),
            },
            Operation::SetGeometry {
                id: id_b,
                geometry: Geometry::Line(tb),
            },
            Operation::CreateEntity {
                record: EntityRecord {
                    id: arc_id,
                    kind: EntityKind::Arc,
                    geometry: Geometry::Arc(arc),
                    layer_id,
                },
            },
        ];
        self.apply_batch(ops, "Congé créé".into())
    }

    fn apply_chamfer(&mut self, distance: f64) -> CommandOutput {
        if self.selection.len() == 1 {
            let id = self.selection.to_vec()[0];
            if let Some(ent) = self.session.document.get_entity(id) {
                if let Geometry::Polyline(poly) = &ent.geometry {
                    return match chamfer_polyline(poly, distance, Tolerance::DEFAULT) {
                        Some(cp) => self.apply_batch(
                            vec![Operation::SetGeometry {
                                id,
                                geometry: Geometry::Polyline(cp),
                            }],
                            "Chanfrein polyligne.".into(),
                        ),
                        None => CommandOutput {
                            echoes: vec!["CHAMFER polyligne impossible.".into()],
                            ..Default::default()
                        },
                    };
                }
            }
        }
        let Some((id_a, la, id_b, lb)) = self.selected_two_lines() else {
            return CommandOutput {
                echoes: vec!["CHAMFER : 2 lignes ou 1 polyligne.".into()],
                ..Default::default()
            };
        };
        let Some((ta, tb, ch)) = chamfer_lines(la, lb, distance, distance, Tolerance::DEFAULT)
        else {
            return CommandOutput {
                echoes: vec!["CHAMFER impossible.".into()],
                ..Default::default()
            };
        };
        let ch_id = self.ids.next();
        let layer_id = self.session.document.current_layer;
        let ops = vec![
            Operation::SetGeometry {
                id: id_a,
                geometry: Geometry::Line(ta),
            },
            Operation::SetGeometry {
                id: id_b,
                geometry: Geometry::Line(tb),
            },
            Operation::CreateEntity {
                record: EntityRecord {
                    id: ch_id,
                    kind: EntityKind::Line,
                    geometry: Geometry::Line(ch),
                    layer_id,
                },
            },
        ];
        self.apply_batch(ops, "Chanfrein créé".into())
    }

    fn commit_polyline(&mut self, points: Vec<Vec2>, closed: bool) -> CommandOutput {
        let poly = if closed {
            Polyline::closed(points)
        } else {
            Polyline::new(points)
        };
        self.commit_one(Geometry::Polyline(poly))
    }

    fn apply_array_rect(&mut self, cols: usize, rows: usize, dx: f64, dy: f64) -> CommandOutput {
        let offsets = rectangular_array_offsets(cols, rows, dx, dy);
        let mut ops = Vec::new();
        for id in self.selection.iter() {
            if let Some(ent) = self.session.document.get_entity(id) {
                for off in &offsets {
                    let new_id = self.ids.next();
                    ops.push(Operation::CreateEntity {
                        record: EntityRecord {
                            id: new_id,
                            kind: ent.kind,
                            geometry: ent.geometry.translated(*off),
                            layer_id: ent.layer_id,
                        },
                    });
                }
            }
        }
        let n = ops.len();
        self.apply_batch(ops, format!("ARRAYRECT : {n} copie(s)"))
    }

    fn apply_array_polar(
        &mut self,
        count: usize,
        center: Vec2,
        base: Vec2,
        fill_deg: f64,
    ) -> CommandOutput {
        let copies = polar_array_copies(center, base, count, fill_deg);
        let mut ops = Vec::new();
        for id in self.selection.iter() {
            if let Some(ent) = self.session.document.get_entity(id) {
                for (pos, rot) in &copies {
                    let new_id = self.ids.next();
                    // Rotate about center then place: rotate geom about center by rot.
                    let g = ent.geometry.rotated(center, *rot);
                    // polar_array_copies already encodes rotation of base point; rotation of geom is enough.
                    let _ = pos;
                    ops.push(Operation::CreateEntity {
                        record: EntityRecord {
                            id: new_id,
                            kind: ent.kind,
                            geometry: g,
                            layer_id: ent.layer_id,
                        },
                    });
                }
            }
        }
        let n = ops.len();
        self.apply_batch(ops, format!("ARRAYPOLAR : {n} copie(s)"))
    }

    fn apply_stretch(&mut self, c1: Vec2, c2: Vec2, delta: Vec2) -> CommandOutput {
        let mut ops = Vec::new();
        let ids = if self.selection.is_empty() {
            self.session
                .document
                .entities()
                .map(|e| e.id)
                .collect::<Vec<_>>()
        } else {
            self.selection.to_vec()
        };
        for id in ids {
            if let Some(ent) = self.session.document.get_entity(id) {
                let g = match &ent.geometry {
                    Geometry::Line(l) => Some(Geometry::Line(stretch_line(*l, c1, c2, delta))),
                    Geometry::Polyline(pl) => {
                        Some(Geometry::Polyline(stretch_polyline(pl, c1, c2, delta)))
                    }
                    _ => None,
                };
                if let Some(geometry) = g {
                    let changed = match (&ent.geometry, &geometry) {
                        (Geometry::Line(a), Geometry::Line(b)) => a != b,
                        (Geometry::Polyline(a), Geometry::Polyline(b)) => a != b,
                        _ => false,
                    };
                    if changed {
                        ops.push(Operation::SetGeometry { id, geometry });
                    }
                }
            }
        }
        let n = ops.len();
        self.apply_batch(ops, format!("STRETCH : {n} objet(s)"))
    }

    fn apply_break_at(&mut self, p: Vec2) -> CommandOutput {
        let Some(eid) = self.pick_entity(p) else {
            return CommandOutput {
                echoes: vec!["Aucun objet sous le curseur.".into()],
                ..Default::default()
            };
        };
        let Some(ent) = self.session.document.get_entity(eid).cloned() else {
            return CommandOutput::default();
        };
        let Geometry::Line(line) = ent.geometry else {
            return CommandOutput {
                echoes: vec!["BREAK : lignes uniquement.".into()],
                ..Default::default()
            };
        };
        let Some((a, b)) = break_line_near(line, p, Tolerance::DEFAULT) else {
            return CommandOutput {
                echoes: vec!["Point de rupture invalide (extrémité).".into()],
                ..Default::default()
            };
        };
        let new_id = self.ids.next();
        let ops = vec![
            Operation::SetGeometry {
                id: eid,
                geometry: Geometry::Line(a),
            },
            Operation::CreateEntity {
                record: EntityRecord {
                    id: new_id,
                    kind: EntityKind::Line,
                    geometry: Geometry::Line(b),
                    layer_id: ent.layer_id,
                },
            },
        ];
        self.apply_batch(ops, "BREAK effectué.".into())
    }

    fn apply_join(&mut self) -> CommandOutput {
        let ids = self.selection.to_vec();
        if ids.len() < 2 {
            return CommandOutput {
                echoes: vec!["JOIN : sélectionnez au moins 2 objets.".into()],
                ..Default::default()
            };
        }
        let g0 = self.session.document.get_entity(ids[0]).map(|e| e.geometry.clone());
        let g1 = self.session.document.get_entity(ids[1]).map(|e| e.geometry.clone());
        match (g0, g1) {
            (Some(Geometry::Line(a)), Some(Geometry::Line(b))) => {
                let Some(j) = join_collinear_lines(a, b, Tolerance::DEFAULT) else {
                    return CommandOutput {
                        echoes: vec!["JOIN : lignes non colinéaires/jointives.".into()],
                        ..Default::default()
                    };
                };
                let ops = vec![
                    Operation::SetGeometry {
                        id: ids[0],
                        geometry: Geometry::Line(j),
                    },
                    Operation::DeleteEntity { id: ids[1] },
                ];
                self.selection.clear();
                self.apply_batch(ops, "JOIN lignes.".into())
            }
            (Some(Geometry::Polyline(a)), Some(Geometry::Polyline(b))) => {
                let Some(j) = join_polylines(&a, &b, Tolerance::DEFAULT) else {
                    return CommandOutput {
                        echoes: vec!["JOIN : polylignes non jointives.".into()],
                        ..Default::default()
                    };
                };
                let ops = vec![
                    Operation::SetGeometry {
                        id: ids[0],
                        geometry: Geometry::Polyline(j),
                    },
                    Operation::DeleteEntity { id: ids[1] },
                ];
                self.selection.clear();
                self.apply_batch(ops, "JOIN polylignes.".into())
            }
            _ => CommandOutput {
                echoes: vec!["JOIN : 2 lignes ou 2 polylignes.".into()],
                ..Default::default()
            },
        }
    }

    fn apply_explode(&mut self) -> CommandOutput {
        let mut ops = Vec::new();
        let ids = self.selection.to_vec();
        for id in ids {
            let Some(ent) = self.session.document.get_entity(id).cloned() else {
                continue;
            };
            match ent.geometry {
                Geometry::Polyline(pl) => {
                    ops.push(Operation::DeleteEntity { id });
                    for seg in pl.segments() {
                        let new_id = self.ids.next();
                        ops.push(Operation::CreateEntity {
                            record: EntityRecord {
                                id: new_id,
                                kind: EntityKind::Line,
                                geometry: Geometry::Line(seg),
                                layer_id: ent.layer_id,
                            },
                        });
                    }
                }
                Geometry::Insert(ins) => {
                    let Some(def) = self.session.document.get_block(ins.block_id).cloned() else {
                        continue;
                    };
                    ops.push(Operation::DeleteEntity { id });
                    for g in &def.entities {
                        let new_id = self.ids.next();
                        let sx = if ins.scale.x.abs() < 1e-12 {
                            1.0
                        } else {
                            ins.scale.x
                        };
                        let geom = g
                            .scaled(Vec2::ZERO, sx)
                            .rotated(Vec2::ZERO, ins.rotation)
                            .translated(ins.position);
                        ops.push(Operation::CreateEntity {
                            record: EntityRecord {
                                id: new_id,
                                kind: geom.kind(),
                                geometry: geom,
                                layer_id: ent.layer_id,
                            },
                        });
                    }
                }
                _ => {}
            }
        }
        self.selection.clear();
        let n = ops.len();
        if n == 0 {
            return CommandOutput {
                echoes: vec!["EXPLODE : rien à décomposer (polyligne/insert).".into()],
                ..Default::default()
            };
        }
        self.apply_batch(ops, format!("EXPLODE : {n} op(s)"))
    }

    fn apply_lengthen_at(&mut self, p: Vec2, delta: f64) -> CommandOutput {
        let Some(eid) = self.pick_entity(p) else {
            return CommandOutput {
                echoes: vec!["Aucun objet.".into()],
                ..Default::default()
            };
        };
        let Some(ent) = self.session.document.get_entity(eid).cloned() else {
            return CommandOutput::default();
        };
        let Geometry::Line(line) = ent.geometry else {
            return CommandOutput {
                echoes: vec!["LENGTHEN : lignes uniquement.".into()],
                ..Default::default()
            };
        };
        let Some(nl) = lengthen_line(line, p, delta, Tolerance::DEFAULT) else {
            return CommandOutput {
                echoes: vec!["LENGTHEN impossible.".into()],
                ..Default::default()
            };
        };
        self.apply_batch(
            vec![Operation::SetGeometry {
                id: eid,
                geometry: Geometry::Line(nl),
            }],
            "LENGTHEN OK.".into(),
        )
    }

    fn apply_align(&mut self, sf: Vec2, st: Vec2, df: Vec2, dt: Vec2) -> CommandOutput {
        let Some((origin, rot, scale)) = align_2point(sf, st, df, dt) else {
            return CommandOutput {
                echoes: vec!["ALIGN : points dégénérés.".into()],
                ..Default::default()
            };
        };
        let mut ops = Vec::new();
        for id in self.selection.iter() {
            if let Some(ent) = self.session.document.get_entity(id) {
                // Transform via scale+rotate about origin then translate origin→df
                let g = ent
                    .geometry
                    .scaled(origin, scale)
                    .rotated(origin, rot)
                    .translated(df - origin);
                // align_transform_point for verification
                let _ = align_transform_point(sf, origin, rot, scale, df);
                ops.push(Operation::SetGeometry { id, geometry: g });
            }
        }
        let n = ops.len();
        self.apply_batch(ops, format!("ALIGN : {n} objet(s)"))
    }

    fn apply_xref_insert(
        &mut self,
        name: &str,
        path: &str,
        kind: XrefKind,
        position: Vec2,
    ) -> CommandOutput {
        let xref_id = XrefId::from_uuid(Uuid::now_v7());
        let def = XrefDef::new(xref_id, name, path, kind);
        let layer_id = self.session.document.current_layer;
        let ent_id = self.ids.next();
        let geom = Geometry::Xref(XrefInsertGeom {
            xref_id,
            position,
            scale: Vec2::new(1.0, 1.0),
            rotation: 0.0,
            clip: None,
            fade: 1.0,
        });
        let ops = vec![
            Operation::UpsertXref { def },
            Operation::CreateEntity {
                record: EntityRecord {
                    id: ent_id,
                    kind: geom.kind(),
                    geometry: geom,
                    layer_id,
                },
            },
        ];
        self.apply_batch(ops, format!("XREF « {name} »"))
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
                Geometry::Ellipse(e) => e
                    .to_polyline(32)
                    .segments()
                    .map(|s| s.distance_squared(p))
                    .fold(f64::INFINITY, f64::min)
                    .sqrt(),
                Geometry::Spline(s) => s
                    .to_polyline(8)
                    .segments()
                    .map(|seg| seg.distance_squared(p))
                    .fold(f64::INFINITY, f64::min)
                    .sqrt(),
                Geometry::Leader(l) => l
                    .vertices
                    .iter()
                    .map(|q| q.distance(p))
                    .fold(f64::INFINITY, f64::min),
                Geometry::Ray(r) => {
                    let t = (p - r.origin).dot(r.direction).max(0.0);
                    r.point_at(t).distance(p)
                }
                Geometry::XLine(x) => {
                    let t = (p - x.point).dot(x.direction);
                    (x.point + x.direction * t).distance(p)
                }
                Geometry::Xref(x) => x.position.distance(p),
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
        let tol = Tolerance::DEFAULT;
        let cutters: Vec<Line> = self
            .session
            .document
            .entities()
            .filter(|e| e.id != target_id)
            .flat_map(|e| match &e.geometry {
                Geometry::Line(l) => vec![*l],
                Geometry::Polyline(poly) => poly.segments().collect(),
                _ => vec![],
            })
            .collect();

        match target.geometry {
            Geometry::Line(target_line) => {
                for cut in &cutters {
                    if let Some(trimmed) = trim_line_by_cut(target_line, *cut, p, tol) {
                        return self.apply_batch(
                            vec![Operation::SetGeometry {
                                id: target_id,
                                geometry: Geometry::Line(trimmed),
                            }],
                            "Coupé.".into(),
                        );
                    }
                }
            }
            Geometry::Polyline(poly) => {
                for cut in &cutters {
                    if let Some(trimmed) = trim_polyline_by_cut(&poly, *cut, p, tol) {
                        return self.apply_batch(
                            vec![Operation::SetGeometry {
                                id: target_id,
                                geometry: Geometry::Polyline(trimmed),
                            }],
                            "Polyligne coupée.".into(),
                        );
                    }
                }
            }
            Geometry::Arc(arc) => {
                for cut in &cutters {
                    if let Some(trimmed) = trim_arc_by_line(arc, *cut, p, tol) {
                        return self.apply_batch(
                            vec![Operation::SetGeometry {
                                id: target_id,
                                geometry: Geometry::Arc(trimmed),
                            }],
                            "Arc coupé.".into(),
                        );
                    }
                }
            }
            _ => {
                return CommandOutput {
                    echoes: vec!["TRIM : ligne, polyligne ou arc.".into()],
                    ..Default::default()
                };
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
        let tol = Tolerance::DEFAULT;
        let boundaries: Vec<Line> = self
            .session
            .document
            .entities()
            .filter(|e| e.id != target_id)
            .flat_map(|e| match &e.geometry {
                Geometry::Line(l) => vec![*l],
                Geometry::Polyline(poly) => poly.segments().collect(),
                _ => vec![],
            })
            .collect();
        match target.geometry {
            Geometry::Line(target_line) => {
                for b in &boundaries {
                    if let Some(ext) = extend_line_to(target_line, *b, p, tol) {
                        return self.apply_batch(
                            vec![Operation::SetGeometry {
                                id: target_id,
                                geometry: Geometry::Line(ext),
                            }],
                            "Prolongé.".into(),
                        );
                    }
                }
            }
            Geometry::Polyline(poly) => {
                for b in &boundaries {
                    if let Some(ext) = extend_polyline_to(&poly, *b, p, tol) {
                        return self.apply_batch(
                            vec![Operation::SetGeometry {
                                id: target_id,
                                geometry: Geometry::Polyline(ext),
                            }],
                            "Polyligne prolongée.".into(),
                        );
                    }
                }
            }
            _ => {
                return CommandOutput {
                    echoes: vec!["EXTEND : ligne ou polyligne ouverte.".into()],
                    ..Default::default()
                };
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

    #[test]
    fn array_rect_copies() {
        let mut eng = engine();
        eng.handle(CommandEvent::Start(CommandId::Line));
        eng.handle(CommandEvent::Pick(Vec2::ZERO));
        eng.handle(CommandEvent::Pick(Vec2::X));
        eng.handle(CommandEvent::Enter);
        let id = eng.session.document.entities().next().unwrap().id;
        eng.selection.set_single(id);
        eng.handle(CommandEvent::Start(CommandId::ArrayRect));
        eng.handle(CommandEvent::Text("2".into()));
        eng.handle(CommandEvent::Text("2".into()));
        eng.handle(CommandEvent::Text("5".into()));
        eng.handle(CommandEvent::Text("5".into()));
        // original + 3 copies
        assert_eq!(eng.session.document.entity_count(), 4);
    }

    #[test]
    fn break_line_mid() {
        let mut eng = engine();
        eng.handle(CommandEvent::Start(CommandId::Line));
        eng.handle(CommandEvent::Pick(Vec2::ZERO));
        eng.handle(CommandEvent::Pick(Vec2::new(10.0, 0.0)));
        eng.handle(CommandEvent::Enter);
        eng.handle(CommandEvent::Start(CommandId::Break));
        eng.handle(CommandEvent::Pick(Vec2::new(5.0, 0.0)));
        assert_eq!(eng.session.document.entity_count(), 2);
    }

    #[test]
    fn join_collinear_lines() {
        let mut eng = engine();
        eng.handle(CommandEvent::Start(CommandId::Line));
        eng.handle(CommandEvent::Pick(Vec2::ZERO));
        eng.handle(CommandEvent::Pick(Vec2::new(5.0, 0.0)));
        eng.handle(CommandEvent::Enter);
        eng.handle(CommandEvent::Start(CommandId::Line));
        eng.handle(CommandEvent::Pick(Vec2::new(5.0, 0.0)));
        eng.handle(CommandEvent::Pick(Vec2::new(10.0, 0.0)));
        eng.handle(CommandEvent::Enter);
        let ids: Vec<_> = eng.session.document.entities().map(|e| e.id).collect();
        eng.selection.clear();
        for id in ids {
            eng.selection.add(id);
        }
        eng.handle(CommandEvent::Start(CommandId::Join));
        assert_eq!(eng.session.document.entity_count(), 1);
    }

    #[test]
    fn explode_polyline() {
        let mut eng = engine();
        eng.handle(CommandEvent::Start(CommandId::Rectang));
        eng.handle(CommandEvent::Pick(Vec2::ZERO));
        eng.handle(CommandEvent::Pick(Vec2::new(2.0, 2.0)));
        let id = eng.session.document.entities().next().unwrap().id;
        eng.selection.set_single(id);
        eng.handle(CommandEvent::Start(CommandId::Explode));
        assert_eq!(eng.session.document.entity_count(), 4);
    }

    #[test]
    fn dim_aligned_and_leader() {
        let mut eng = engine();
        eng.handle(CommandEvent::Start(CommandId::DimAligned));
        eng.handle(CommandEvent::Pick(Vec2::ZERO));
        eng.handle(CommandEvent::Pick(Vec2::new(10.0, 0.0)));
        eng.handle(CommandEvent::Pick(Vec2::new(5.0, 2.0)));
        assert_eq!(eng.session.document.entity_count(), 1);
        eng.handle(CommandEvent::Start(CommandId::Leader));
        eng.handle(CommandEvent::Pick(Vec2::new(1.0, 1.0)));
        eng.handle(CommandEvent::Pick(Vec2::new(3.0, 3.0)));
        eng.handle(CommandEvent::Enter);
        assert_eq!(eng.session.document.entity_count(), 2);
    }

    #[test]
    fn ray_and_xline() {
        let mut eng = engine();
        eng.handle(CommandEvent::Start(CommandId::Ray));
        eng.handle(CommandEvent::Pick(Vec2::ZERO));
        eng.handle(CommandEvent::Pick(Vec2::X));
        eng.handle(CommandEvent::Start(CommandId::Xline));
        eng.handle(CommandEvent::Pick(Vec2::ZERO));
        eng.handle(CommandEvent::Pick(Vec2::Y));
        assert_eq!(eng.session.document.entity_count(), 2);
    }

    #[test]
    fn circle2p_and_offset_polyline() {
        let mut eng = engine();
        eng.handle(CommandEvent::Start(CommandId::Circle2p));
        eng.handle(CommandEvent::Pick(Vec2::ZERO));
        eng.handle(CommandEvent::Pick(Vec2::new(10.0, 0.0)));
        match &eng.session.document.entities().next().unwrap().geometry {
            Geometry::Circle(c) => assert!((c.radius - 5.0).abs() < 1e-9),
            _ => panic!("circle"),
        }
        // Rectangle far from the circle so OFFSET pick is unambiguous.
        eng.handle(CommandEvent::Start(CommandId::Rectang));
        eng.handle(CommandEvent::Pick(Vec2::new(100.0, 100.0)));
        eng.handle(CommandEvent::Pick(Vec2::new(110.0, 108.0)));
        eng.handle(CommandEvent::Start(CommandId::Offset));
        eng.handle(CommandEvent::Text("0.5".into()));
        eng.handle(CommandEvent::Pick(Vec2::new(100.1, 100.1)));
        let polys = eng
            .session
            .document
            .entities()
            .filter(|e| matches!(e.geometry, Geometry::Polyline(_)))
            .count();
        assert!(polys >= 2);
    }

    #[test]
    fn fillet_polyline_corners() {
        let mut eng = engine();
        eng.handle(CommandEvent::Start(CommandId::Rectang));
        eng.handle(CommandEvent::Pick(Vec2::ZERO));
        eng.handle(CommandEvent::Pick(Vec2::new(10.0, 10.0)));
        let id = eng.session.document.entities().next().unwrap().id;
        eng.selection.set_single(id);
        eng.handle(CommandEvent::Start(CommandId::Fillet));
        eng.handle(CommandEvent::Text("1".into()));
        match &eng.session.document.get_entity(id).unwrap().geometry {
            Geometry::Polyline(p) => assert!(p.vertices.len() > 4),
            _ => panic!("polyline"),
        }
    }

    #[test]
    fn star_and_donut_and_arrow() {
        let mut eng = engine();
        eng.handle(CommandEvent::Start(CommandId::Star));
        eng.handle(CommandEvent::Text("5".into()));
        eng.handle(CommandEvent::Pick(Vec2::ZERO));
        eng.handle(CommandEvent::Pick(Vec2::new(2.0, 0.0)));
        assert_eq!(eng.session.document.entity_count(), 1);

        eng.handle(CommandEvent::Start(CommandId::Donut));
        eng.handle(CommandEvent::Pick(Vec2::new(20.0, 0.0)));
        eng.handle(CommandEvent::Pick(Vec2::new(22.0, 0.0)));
        eng.handle(CommandEvent::Pick(Vec2::new(21.0, 0.0)));
        assert_eq!(
            eng.session
                .document
                .entities()
                .filter(|e| matches!(e.geometry, Geometry::Circle(_)))
                .count(),
            2
        );

        eng.handle(CommandEvent::Start(CommandId::Arrow));
        eng.handle(CommandEvent::Pick(Vec2::new(0.0, 10.0)));
        eng.handle(CommandEvent::Pick(Vec2::new(5.0, 10.0)));
        assert!(eng
            .session
            .document
            .entities()
            .any(|e| matches!(e.geometry, Geometry::Polyline(_))));
    }

    #[test]
    fn capsule_triangle_grid() {
        let mut eng = engine();
        eng.handle(CommandEvent::Start(CommandId::Capsule));
        eng.handle(CommandEvent::Pick(Vec2::ZERO));
        eng.handle(CommandEvent::Pick(Vec2::new(4.0, 0.0)));
        eng.handle(CommandEvent::Pick(Vec2::new(2.0, 0.5)));
        assert_eq!(eng.session.document.entity_count(), 1);

        eng.handle(CommandEvent::Start(CommandId::Triangle));
        eng.handle(CommandEvent::Pick(Vec2::new(10.0, 0.0)));
        eng.handle(CommandEvent::Pick(Vec2::new(13.0, 0.0)));
        eng.handle(CommandEvent::Pick(Vec2::new(11.5, 2.0)));
        assert_eq!(eng.session.document.entity_count(), 2);

        eng.handle(CommandEvent::Start(CommandId::Grid));
        eng.handle(CommandEvent::Text("2".into()));
        eng.handle(CommandEvent::Text("2".into()));
        eng.handle(CommandEvent::Pick(Vec2::new(50.0, 50.0)));
        eng.handle(CommandEvent::Pick(Vec2::new(51.0, 51.0)));
        let lines = eng
            .session
            .document
            .entities()
            .filter(|e| matches!(e.geometry, Geometry::Line(_)))
            .count();
        assert_eq!(lines, 6); // 3 vert + 3 horiz
    }
}
