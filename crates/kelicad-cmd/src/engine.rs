//! Command state machine driving a [`kelicad_ops::DocSession`].

use std::collections::BTreeMap;

use kelicad_doc::{
    aabb_inside_rect, aabb_intersects_rect, expand_insert, geometry_aabb, BlockDef, BlockId,
    DimKind, DimensionGeom, EntityId, EntityKind, EntityProps, EntityRecord, Geometry, HatchGeom,
    InsertGeom, LeaderGeom, MTextGeom, RayGeom, TextAlign, TextGeom, XLineGeom, XrefDef, XrefId,
    XrefInsertGeom, XrefKind,
};
use kelicad_geom::construct::{
    arrow, capsule, center_mark, centerline, chamfer_lines, circle_from_2_points,
    circle_from_3_points, fillet_lines, grid_lines, isosceles_triangle,
    regular_polygon_center_vertex, regular_polygon_side, revision_cloud, star_polygon, table_grid,
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
use kelicad_snap::{SnapEngine, SnapHit, SnapModes, SnapQuery};
use uuid::Uuid;

use crate::id_gen::IdGen;
use crate::input::{parse_number, parse_point};
use crate::options::match_option;
use crate::registry::CommandId;
use crate::selection::Selection;
use crate::sysvars::{parse_sysvar_token, SysVarStore, SysVarValue};

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
    /// Idle pick that toggles (Shift+click).
    PickToggle(Vec2),
    /// Window / crossing selection rectangle.
    /// `crossing` = true → intersect (right→left AutoCAD style); false → fully inside.
    SelectRect {
        /// First corner.
        a: Vec2,
        /// Opposite corner.
        b: Vec2,
        /// Crossing (intersect) vs window (contain).
        crossing: bool,
        /// Keep existing selection and add.
        additive: bool,
    },
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum XlineKind {
    TwoPoint,
    Hor,
    Ver,
    Ang,
    Offset,
}

#[derive(Debug, Clone)]
enum Active {
    Line {
        points: Vec<Vec2>,
    },
    Pline {
        points: Vec<Vec2>,
        /// Constant width from PLINEWID / WIDTH option.
        width: f64,
        /// Waiting for numeric width after WIDTH keyword.
        wait_width: bool,
    },
    CircleCenter {
        center: Option<Vec2>,
        /// Diameter mode (second pick = diameter point through center).
        diameter: bool,
    },
    Arc {
        center: Option<Vec2>,
        start: Option<Vec2>,
    },
    /// ARC 3-point mode.
    Arc3p {
        points: Vec<Vec2>,
    },
    /// ARC start–end–angle (angle via number or third pick as direction proxy).
    ArcSea {
        start: Option<Vec2>,
        end: Option<Vec2>,
    },
    Rectang {
        first: Option<Vec2>,
        fillet: f64,
        chamfer: f64,
        wait_fillet: bool,
        wait_chamfer: bool,
        wait_dims: bool,
        dim_w: Option<f64>,
    },
    Move {
        base: Option<Vec2>,
    },
    Copy {
        base: Option<Vec2>,
        multiple: bool,
    },
    Rotate {
        origin: Option<Vec2>,
        copy: bool,
        reference: bool,
        ref_angle: Option<f64>,
    },
    Scale {
        origin: Option<Vec2>,
        copy: bool,
        reference: bool,
        ref_len: Option<f64>,
    },
    Mirror {
        first: Option<Vec2>,
        erase_source: bool,
        wait_erase: bool,
    },
    Offset {
        distance: Option<f64>,
        through: bool,
        multiple: bool,
        erase_source: bool,
    },
    Trim,
    Extend,
    Erase,
    /// Verb-noun: pick objects then Enter to continue the pending command.
    GatherSelection,
    Text {
        position: Option<Vec2>,
    },
    DimLinear {
        def1: Option<Vec2>,
        def2: Option<Vec2>,
    },
    DimContinue {
        next: Option<Vec2>,
        offset: Option<Vec2>,
    },
    DimBaseline {
        next: Option<Vec2>,
        offset: Option<Vec2>,
    },
    Section {
        a: Option<Vec2>,
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
        /// Edge mode: two points define a side.
        edge: bool,
        edge_a: Option<Vec2>,
    },
    Pedit {
        target: Option<EntityId>,
        wait_width: bool,
    },
    Matchprop {
        source: Option<EntityId>,
    },
    Draworder,
    Group {
        name: Option<String>,
    },
    Divide {
        count: Option<usize>,
        target: Option<EntityId>,
    },
    Measure {
        spacing: Option<f64>,
        target: Option<EntityId>,
    },
    Boundary {
        points: Vec<Vec2>,
    },
    SelectGather,
    Qselect {
        kind: Option<EntityKind>,
    },
    Ddptype,
    Wblock {
        name: Option<String>,
        base: Option<Vec2>,
    },
    Mtext {
        position: Option<Vec2>,
    },
    ArrayPath {
        count: Option<usize>,
        path: Option<EntityId>,
    },
    Setvar {
        name: Option<String>,
    },
    Getvar,
    XlineMode {
        /// Hor / Ver / Ang / Offset / TwoPoint
        mode: XlineKind,
        first: Option<Vec2>,
        angle_deg: Option<f64>,
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
    /// Leader waiting for annotation text after vertices.
    LeaderText {
        start: Vec2,
        end: Vec2,
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
    Fillet {
        radius: Option<f64>,
        multiple: bool,
        wait_radius: bool,
    },
    Chamfer {
        distance: Option<f64>,
        distance_b: Option<f64>,
        multiple: bool,
        wait_distance: bool,
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
    /// System variables (ORTHOMODE, OSMODE, FILLETRAD, …).
    pub sysvars: SysVarStore,
    /// Named groups of entity ids.
    pub groups: BTreeMap<String, Vec<EntityId>>,
    /// Previous selection (SELECTPREV).
    prev_selection: Vec<EntityId>,
    /// Last erased entities (OOPS).
    last_erased: Vec<EntityRecord>,
    active: Option<(CommandId, Active)>,
    /// Last finished command (repeat with Enter).
    last_command: Option<CommandId>,
    /// Last linear/aligned dimension (for DIMCONTINUE / DIMBASELINE).
    last_dim: Option<DimensionGeom>,
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
            sysvars: SysVarStore::new(),
            groups: BTreeMap::new(),
            prev_selection: Vec::new(),
            last_erased: Vec::new(),
            active: None,
            last_command: None,
            last_dim: None,
        }
    }

    /// Sync engine ortho/osnap/aperture/snap modes from sysvars.
    pub fn sync_from_sysvars(&mut self) {
        self.ortho = self.sysvars.get_int("ORTHOMODE") != 0;
        self.osnap_enabled = self.sysvars.get_int("OSMODE") != 0;
        self.snap_aperture = self.sysvars.get_real("APERTURE").max(0.01);
        let bits = self.sysvars.osmode_bits();
        if bits != 0 {
            self.snap.modes = SnapModes::from_osmode(bits);
        }
    }

    /// Current drafting overrides from CECOLOR / CELTYPE / CELWEIGHT.
    pub fn drafting_props(&self) -> EntityProps {
        EntityProps {
            color: parse_cecolor(&self.sysvars.get_str("CECOLOR")),
            linetype: parse_celtype(&self.sysvars.get_str("CELTYPE")),
            lineweight: parse_celweight(self.sysvars.get_int("CELWEIGHT")),
            draw_order: 0,
        }
    }

    /// Active command option chips for UI.
    pub fn active_option_chips(&self) -> Vec<crate::options::OptionChip> {
        match self.active_command() {
            Some(id) => crate::options::option_chips(id),
            None => Vec::new(),
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
            CommandEvent::PickToggle(p) => {
                if self.active.is_none() {
                    let snapped = self.snap_point(p, &mut out);
                    self.last_point = Some(snapped);
                    if let Some(eid) = self.pick_entity(snapped) {
                        self.selection.toggle(eid);
                        out.echoes
                            .push(format!("Sélection : {} objet(s)", self.selection.len()));
                    }
                } else {
                    let snapped = self.snap_point(p, &mut out);
                    out = Self::merge_out(out, self.on_point(snapped));
                }
            }
            CommandEvent::SelectRect {
                a,
                b,
                crossing,
                additive,
            } => {
                let gathering = matches!(
                    self.active.as_ref().map(|(_, s)| s),
                    Some(Active::GatherSelection | Active::Erase)
                );
                if self.active.is_none() || gathering {
                    out = self.select_by_rect(a, b, crossing, additive || gathering);
                    if gathering {
                        out.prompt = self.current_prompt();
                    }
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
        // Grid snap (F9 / SNAPMODE) — before ortho/polar so constraints stay on grid.
        if self.sysvars.get_int("SNAPMODE") != 0 {
            let sp = self.sysvars.get_real("SNAPUNIT").max(1e-6);
            p = Vec2::new((p.x / sp).round() * sp, (p.y / sp).round() * sp);
        }
        if self.ortho {
            if let Some(base) = self.last_point {
                let dx = (p.x - base.x).abs();
                let dy = (p.y - base.y).abs();
                p = if dx >= dy {
                    Vec2::new(p.x, base.y)
                } else {
                    Vec2::new(base.x, p.y)
                };
            }
        } else if self.sysvars.get_int("POLARMODE") != 0 {
            if let Some(base) = self.last_point {
                let v = p - base;
                let len = v.length();
                if len > 1e-9 {
                    let ang = v.y.atan2(v.x);
                    let step = std::f64::consts::FRAC_PI_4; // 45°
                    let snapped = (ang / step).round() * step;
                    p = base + Vec2::new(snapped.cos(), snapped.sin()) * len;
                }
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
            hit.point        } else {
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
            CommandId::Oops => {
                out = self.oops_restore();
            }
            CommandId::Overkill => {
                out = self.run_overkill();
            }
            CommandId::SelectAll => {
                self.remember_selection();
                self.selection.clear();
                for e in self.session.document.entities() {
                    self.selection.add(e.id);
                }
                out.echoes
                    .push(format!("Sélection : {} objet(s)", self.selection.len()));
                self.active = None;
            }
            CommandId::SelectPrev => {
                let prev = self.prev_selection.clone();
                self.remember_selection();
                self.selection.clear();
                for id in prev {
                    self.selection.add(id);
                }
                out.echoes
                    .push(format!("Sélection précédente : {}", self.selection.len()));
                self.active = None;
            }
            CommandId::Ungroup => {
                let before = self.groups.len();
                self.groups.clear();
                out.echoes.push(format!("Groupes effacés ({before})."));
                self.active = None;
            }
            CommandId::Laycur => {
                if let Some(id) = self.selection.iter().next() {
                    if let Some(ent) = self.session.document.get_entity(id) {
                        let lid = ent.layer_id;
                        let _ = self.session.apply(Operation::SetCurrentLayer { id: lid });
                        out.document_changed = true;
                        out.echoes
                            .push("Calque courant = calque de la sélection.".into());
                    }
                } else {
                    out.echoes.push("Sélectionnez un objet.".into());
                }
                self.active = None;
            }
            CommandId::Layiso | CommandId::Layoff | CommandId::Layfrz | CommandId::Laylck => {
                out = self.layer_tool(id);
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
            CommandId::DimContinue => {
                if self.last_dim.is_none() {
                    out.echoes
                        .push("DIMCONTINUE : créez d'abord une cote linéaire/alignée.".into());
                    self.active = None;
                } else {
                    self.active = Some((
                        id,
                        Active::DimContinue {
                            next: None,
                            offset: None,
                        },
                    ));
                    self.last_command = Some(id);
                    out.prompt = self.current_prompt();
                }
            }
            CommandId::DimBaseline => {
                if self.last_dim.is_none() {
                    out.echoes
                        .push("DIMBASELINE : créez d'abord une cote linéaire/alignée.".into());
                    self.active = None;
                } else {
                    self.active = Some((
                        id,
                        Active::DimBaseline {
                            next: None,
                            offset: None,
                        },
                    ));
                    self.last_command = Some(id);
                    out.prompt = self.current_prompt();
                }
            }
            CommandId::Section => {
                self.active = Some((id, Active::Section { a: None }));
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
                    self.active = Some((id, Active::GatherSelection));
                    self.last_command = Some(id);
                    out.prompt = Some(prompt(
                        "Sélectionnez les objets du bloc, puis Entrée",
                        "Select block objects, then Enter",
                    ));
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
                    self.active = Some((id, Active::GatherSelection));
                    self.last_command = Some(id);
                    out.prompt = Some(prompt(
                        "Sélectionnez 2 lignes (ou 1 polyligne), puis Entrée",
                        "Select 2 lines (or 1 polyline), then Enter",
                    ));
                    return out;
                }
                let active = match id {
                    CommandId::Fillet => Active::Fillet { radius: None, multiple: false, wait_radius: false },
                    CommandId::Chamfer => Active::Chamfer { distance: None, distance_b: None, multiple: false, wait_distance: false },
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
                    self.active = Some((id, Active::GatherSelection));
                    self.last_command = Some(id);
                    out.prompt = Some(prompt(
                        "Sélectionnez des objets, puis Entrée",
                        "Select objects, then Enter",
                    ));
                    return out;
                }
                let active = match id {
                    CommandId::Move => Active::Move { base: None },
                    CommandId::Copy => Active::Copy { base: None, multiple: false },
                    CommandId::Rotate => Active::Rotate { origin: None, copy: false, reference: false, ref_angle: None },
                    CommandId::Scale => Active::Scale { origin: None, copy: false, reference: false, ref_len: None },
                    CommandId::Mirror => Active::Mirror { first: None, erase_source: false, wait_erase: false },
                    CommandId::Offset => Active::Offset { distance: None, through: false, multiple: false, erase_source: false },
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
                    self.active = Some((id, Active::GatherSelection));
                    self.last_command = Some(id);
                    out.prompt = Some(prompt(
                        "Sélectionnez des objets, puis Entrée",
                        "Select objects, then Enter",
                    ));
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
                    CommandId::Pline => Active::Pline { points: vec![], width: 0.0, wait_width: false },
                    CommandId::Circle => Active::CircleCenter { center: None, diameter: false },
                    CommandId::Arc => Active::Arc {
                        center: None,
                        start: None,
                    },
                    CommandId::Rectang => Active::Rectang { first: None, fillet: 0.0, chamfer: 0.0, wait_fillet: false, wait_chamfer: false, wait_dims: false, dim_w: None },
                    CommandId::Polygon => Active::Polygon {
                        sides: None,
                        center: None,
                        edge: false,
                        edge_a: None,
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
                    CommandId::Mtext => Active::Mtext { position: None },
                    CommandId::Boundary => Active::Boundary { points: vec![] },
                    CommandId::Setvar => Active::Setvar { name: None },
                    CommandId::Getvar => Active::Getvar,
                    CommandId::Ddptype => Active::Ddptype,
                    CommandId::Pedit => Active::Pedit {
                        target: None,
                        wait_width: false,
                    },
                    CommandId::Matchprop => Active::Matchprop { source: None },
                    CommandId::Draworder => Active::Draworder,
                    CommandId::Group => Active::Group { name: None },
                    CommandId::Divide => Active::Divide {
                        count: None,
                        target: None,
                    },
                    CommandId::Measure => Active::Measure {
                        spacing: None,
                        target: None,
                    },
                    CommandId::Select | CommandId::SelectSimilar => Active::SelectGather,
                    CommandId::Qselect => Active::Qselect { kind: None },
                    CommandId::Wblock => Active::Wblock {
                        name: None,
                        base: None,
                    },
                    CommandId::ArrayPath => Active::ArrayPath {
                        count: None,
                        path: None,
                    },
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
                prompt(
                    "Spécifiez le premier point [X,Y]",
                    "Specify first point [X,Y]",
                )
            }
            (CommandId::Line, _) => prompt(
                "Spécifiez le point suivant [X,Y] ou [Annuler]:",
                "Specify next point [X,Y] or [Undo]:",
            ),
            (CommandId::Pline, Active::Pline { points, .. }) if points.is_empty() => {
                prompt(
                    "Spécifiez le point de départ [X,Y]",
                    "Specify start point [X,Y]",
                )
            }
            (CommandId::Pline, _) => prompt(
                "Spécifiez le point suivant [X,Y] ou [Arc/Fermer/Annuler]:",
                "Specify next point [X,Y] or [Arc/Close/Undo]:",
            ),
            (CommandId::Circle, Active::CircleCenter { center: None, diameter: false }) => {
                prompt(
                    "Spécifiez le centre du cercle [X,Y]",
                    "Specify center point for circle [X,Y]",
                )
            }
            (CommandId::Circle, Active::CircleCenter { diameter: true, .. }) => prompt(
                "Spécifiez le diamètre du cercle ou [point]:",
                "Specify diameter of circle or [point]:",
            ),
            (CommandId::Circle, _) => prompt(
                "Spécifiez le rayon du cercle ou [Diamètre]:",
                "Specify radius of circle or [Diameter]:",
            ),
            (CommandId::Arc, Active::Arc { center: None, .. }) => {
                prompt(
                    "Spécifiez le centre de l'arc [X,Y]",
                    "Specify center point of arc [X,Y]",
                )
            }
            (CommandId::Arc, Active::Arc { start: None, .. }) => {
                prompt(
                    "Spécifiez le point de départ de l'arc [X,Y]",
                    "Specify start point of arc [X,Y]",
                )
            }
            (CommandId::Arc, _) => prompt(
                "Spécifiez le point d'arrivée de l'arc [X,Y]",
                "Specify end point of arc [X,Y]",
            ),
            (CommandId::Rectang, Active::Rectang { first: None, fillet: 0.0, chamfer: 0.0, wait_fillet: false, wait_chamfer: false, wait_dims: false, dim_w: None }) => {
                prompt(
                    "Spécifiez le premier coin [X,Y]",
                    "Specify first corner point [X,Y]",
                )
            }
            (CommandId::Rectang, _) => prompt(
                "Spécifiez le coin opposé [X,Y] ou [Dimensions]:",
                "Specify other corner point [X,Y] or [Dimensions]:",
            ),
            (CommandId::Move, Active::Move { base: None }) => prompt(
                "Spécifiez le point de base [X,Y]",
                "Specify base point [X,Y]",
            ),
            (CommandId::Move, _) => prompt(
                "Spécifiez le second point [X,Y]",
                "Specify second point [X,Y]",
            ),
            (CommandId::Copy, Active::Copy { base: None, .. }) => prompt(
                "Spécifiez le point de base [X,Y]",
                "Specify base point [X,Y]",
            ),
            (CommandId::Copy, _) => prompt(
                "Spécifiez le second point [X,Y] ou [Multiple]:",
                "Specify second point [X,Y] or [Multiple]:",
            ),
            (CommandId::Rotate, Active::Rotate { origin: None, .. }) => {
                prompt(
                    "Spécifiez le point de base [X,Y]",
                    "Specify base point [X,Y]",
                )
            }
            (CommandId::Rotate, _) => prompt(
                "Spécifiez l'angle de rotation ou [Copier/Référence]:",
                "Specify rotation angle or [Copy/Reference]:",
            ),
            (CommandId::Scale, Active::Scale { origin: None, .. }) => {
                prompt(
                    "Spécifiez le point de base [X,Y]",
                    "Specify base point [X,Y]",
                )
            }
            (CommandId::Scale, _) => prompt(
                "Spécifiez le facteur d'échelle ou [Copier/Référence]:",
                "Specify scale factor or [Copy/Reference]:",
            ),
            (CommandId::Mirror, Active::Mirror { first: None, .. }) => {
                prompt(
                    "Spécifiez le premier point de l'axe [X,Y]",
                    "Specify first point of mirror line [X,Y]",
                )
            }
            (CommandId::Mirror, _) => {
                prompt(
                    "Spécifiez le second point de l'axe [X,Y]",
                    "Specify second point of mirror line [X,Y]",
                )
            }
            (CommandId::Offset, Active::Offset { distance: None, through: false, multiple: false, erase_source: false }) => {
                prompt(
                    "Spécifiez la distance de décalage ou [Par]:",
                    "Specify offset distance or [Through]:",
                )
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
            (_, Active::GatherSelection) => prompt(
                "Sélectionnez des objets, puis Entrée",
                "Select objects, then Enter",
            ),
            (CommandId::Leader, Active::LeaderText { .. }) => {
                prompt("Texte du repère", "Leader text")
            },
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
            (CommandId::DimContinue, Active::DimContinue { next: None, .. }) => {
                prompt("Point suivant de la cote", "Next dimension point")
            }
            (CommandId::DimContinue, _) => {
                prompt("Emplacement de la ligne de cote", "Dimension line location")
            }
            (CommandId::DimBaseline, Active::DimBaseline { next: None, .. }) => {
                prompt("Point suivant (base)", "Next baseline point")
            }
            (CommandId::DimBaseline, _) => {
                prompt("Emplacement ligne de cote", "Dimension line location")
            }
            (CommandId::Section, Active::Section { a: None }) => {
                prompt("Début du trait de coupe", "Section cut start")
            }
            (CommandId::Section, _) => prompt("Fin du trait de coupe", "Section cut end"),
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
                prompt(
                    "Entrez le nombre de côtés:",
                    "Enter number of sides:",
                )
            }
            (CommandId::Polygon, Active::Polygon { center: None, .. }) => {
                prompt(
                    "Spécifiez le centre du polygone [X,Y]",
                    "Specify center of polygon [X,Y]",
                )
            }
            (CommandId::Polygon, _) => prompt(
                "Spécifiez le sommet [X,Y] ou [Côté]:",
                "Specify vertex [X,Y] or [Edge]:",
            ),
            (CommandId::Ellipse, Active::Ellipse { center: None, .. }) => {
                prompt(
                    "Spécifiez le centre de l'ellipse [X,Y]",
                    "Specify center of ellipse [X,Y]",
                )
            }
            (CommandId::Ellipse, Active::Ellipse { major_tip: None, .. }) => {
                prompt(
                    "Spécifiez l'extrémité du grand axe [X,Y]",
                    "Specify endpoint of major axis [X,Y]",
                )
            }
            (CommandId::Ellipse, _) => {
                prompt(
                    "Spécifiez la distance au petit axe [X,Y]",
                    "Specify distance to minor axis [X,Y]",
                )
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
            (CommandId::Fillet, Active::Fillet { radius: None, multiple: false, wait_radius: false }) => {
                prompt("Rayon du congé", "Fillet radius")
            }
            (CommandId::Chamfer, Active::Chamfer { distance: None, distance_b: None, multiple: false, wait_distance: false }) => {
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
                prompt(
                    "Spécifiez le 1er point de la base [X,Y]",
                    "Specify first base point [X,Y]",
                )
            }
            (CommandId::Triangle, Active::Triangle { b: None, .. }) => {
                prompt(
                    "Spécifiez le 2e point de la base [X,Y]",
                    "Specify second base point [X,Y]",
                )
            }
            (CommandId::Triangle, _) => prompt(
                "Spécifiez le sommet [X,Y] ou l'angle/hauteur:",
                "Specify apex [X,Y] or angle/height:",
            ),
            (CommandId::Point, _) => prompt(
                "Spécifiez un point [X,Y]",
                "Specify a point [X,Y]",
            ),
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
            Active::Pline { points, .. } if !points.is_empty() => {
                let last = *points.last().unwrap();
                vec![Geometry::Line(Line::new(last, cursor))]
            }
            Active::CircleCenter { center: Some(c), diameter } => {
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
            Active::Polygon { sides: Some(n), center: Some(c), .. } => regular_polygon_center_vertex(*c, cursor, *n)
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
            Active::Rectang { first: Some(a), fillet, chamfer, .. } => {
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
            Active::Copy { base: Some(b), multiple } => {
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
            Active::Pline { points, .. } => {
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
            Active::GatherSelection => {
                if self.selection.is_empty() {
                    out.echoes
                        .push("Aucun objet sélectionné — continuez ou Échap.".into());
                    out.prompt = self.current_prompt();
                } else {
                    // Resume the pending command now that selection exists.
                    out = self.start_command(id);
                }
            }
            Active::LeaderText { start, end } => {
                // Enter without text → empty annotation.
                out = self.commit_one(Geometry::Leader(LeaderGeom::new(start, end, "")));
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
            Active::Boundary { points } => {
                if points.len() >= 3 {
                    let mut pts = points;
                    if let Some(first) = pts.first().copied() {
                        pts.push(first);
                    }
                    out = self.commit_one(Geometry::Polyline(Polyline {
                        vertices: pts,
                        closed: true,
                    }));
                } else {
                    out.echoes.push("BOUNDARY : ≥3 points.".into());
                }
                self.active = None;
            }
            Active::Copy { multiple: true, .. } => {
                out.echoes.push("COPY terminé.".into());
                self.active = None;
            }
            Active::SelectGather => {
                out.echoes
                    .push(format!("Sélection : {}", self.selection.len()));
                self.active = None;
            }
            Active::Matchprop { .. } => {
                out.echoes.push("MATCHPROP terminé.".into());
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
                    let start = points[0];
                    let end = *points.last().unwrap();
                    self.active = Some((id, Active::LeaderText { start, end }));
                    out.prompt = self.current_prompt();
                    out.echoes
                        .push("Repère placé — saisissez le texte puis Entrée.".into());
                } else {
                    out.echoes.push("LEADER : au moins 2 points.".into());
                    self.active = None;
                }
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
        if let Some((
            CommandId::Leader,
            Active::LeaderText { start, end },
        )) = self.active.clone()
        {
            let out = self.commit_one(Geometry::Leader(LeaderGeom::new(start, end, t)));
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

        // Option keywords (AutoCAD-style)
        if let Some((cid, _)) = self.active.clone() {
            if let Some(key) = match_option(cid, t) {
                return self.apply_option_keyword(cid, key);
            }
        }

        if let Some((CommandId::Setvar, Active::Setvar { name: Some(name) })) = self.active.clone()
        {
            let val = parse_sysvar_token(t);
            let mut out = CommandOutput::default();
            match self.sysvars.set(&name, val) {
                Ok(()) => {
                    self.sync_from_sysvars();
                    out.echoes.push(format!(
                        "{name} = {}",
                        self.sysvars.get(&name).unwrap().display()
                    ));
                }
                Err(e) => out.echoes.push(e),
            }
            self.active = None;
            return out;
        }
        if let Some((CommandId::Getvar, Active::Getvar)) = self.active.clone() {
            let mut out = CommandOutput::default();
            match self.sysvars.get(t) {
                Some(v) => {
                    out.echoes
                        .push(format!("{} = {}", t.to_ascii_uppercase(), v.display()))
                }
                None => out.echoes.push(format!("Variable inconnue : {t}")),
            }
            self.active = None;
            return out;
        }
        if let Some((CommandId::Setvar, Active::Setvar { name: None })) = self.active.clone() {
            self.active = Some((
                CommandId::Setvar,
                Active::Setvar {
                    name: Some(t.to_ascii_uppercase()),
                },
            ));
            return CommandOutput {
                prompt: Some(prompt("Nouvelle valeur", "New value")),
                ..Default::default()
            };
        }
        if let Some((CommandId::Group, Active::Group { name: None })) = self.active.clone() {
            let name = t.to_string();
            let ids: Vec<_> = self.selection.iter().collect();
            self.groups.insert(name.clone(), ids);
            self.active = None;
            return CommandOutput {
                echoes: vec![format!("Groupe « {name} » créé")],
                ..Default::default()
            };
        }
        if let Some((CommandId::Mtext, Active::Mtext { position: Some(p) })) = self.active.clone() {
            let out = self.commit_one(Geometry::MText(MTextGeom::new(p, 2.5, t)));
            self.active = None;
            return out;
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
            (CommandId::Circle, Active::CircleCenter { center: Some(c), diameter }) => {
                let r = if diameter { n.abs() * 0.5 } else { n.abs() };
                out = self.commit_one(Geometry::Circle(Circle::new(c, r)));
                self.active = None;
            }
            (CommandId::Offset, Active::Offset { distance: None, through, multiple, erase_source }) => {
                self.active = Some((
                    CommandId::Offset,
                    Active::Offset {
                        distance: Some(n),
                        through,
                        multiple,
                        erase_source,
                    },
                ));
                let _ = self.sysvars.set("OFFSETDIST", SysVarValue::Real(n.abs()));
                out.prompt = self.current_prompt();
            }
            (
                CommandId::Pline,
                Active::Pline {
                    points,
                    wait_width: true,
                    ..
                },
            ) => {
                let _ = self.sysvars.set("PLINEWID", SysVarValue::Real(n.abs()));
                self.active = Some((
                    CommandId::Pline,
                    Active::Pline {
                        points,
                        width: n.abs(),
                        wait_width: false,
                    },
                ));
                out.echoes.push(format!("Largeur = {n}"));
                out.prompt = self.current_prompt();
            }
            (
                CommandId::Rectang,
                Active::Rectang {
                    first,
                    wait_fillet: true,
                    chamfer,
                    wait_dims,
                    dim_w,
                    ..
                },
            ) => {
                self.active = Some((
                    CommandId::Rectang,
                    Active::Rectang {
                        first,
                        fillet: n.abs(),
                        chamfer,
                        wait_fillet: false,
                        wait_chamfer: false,
                        wait_dims,
                        dim_w,
                    },
                ));
                out.echoes.push(format!("Congé rectangle = {n}"));
                out.prompt = self.current_prompt();
            }
            (
                CommandId::Rectang,
                Active::Rectang {
                    first,
                    fillet,
                    wait_chamfer: true,
                    wait_dims,
                    dim_w,
                    ..
                },
            ) => {
                self.active = Some((
                    CommandId::Rectang,
                    Active::Rectang {
                        first,
                        fillet,
                        chamfer: n.abs(),
                        wait_fillet: false,
                        wait_chamfer: false,
                        wait_dims,
                        dim_w,
                    },
                ));
                out.echoes.push(format!("Chanfrein rectangle = {n}"));
                out.prompt = self.current_prompt();
            }
            (
                CommandId::Rectang,
                Active::Rectang {
                    first,
                    fillet,
                    chamfer,
                    wait_dims: true,
                    dim_w: None,
                    ..
                },
            ) => {
                self.active = Some((
                    CommandId::Rectang,
                    Active::Rectang {
                        first,
                        fillet,
                        chamfer,
                        wait_fillet: false,
                        wait_chamfer: false,
                        wait_dims: true,
                        dim_w: Some(n.abs()),
                    },
                ));
                out.echoes.push(format!("Largeur = {n}"));
                out.prompt = Some(prompt("Hauteur (pick ou nombre)", "Height (pick or number)"));
            }
            (
                CommandId::Rectang,
                Active::Rectang {
                    first: Some(a),
                    fillet,
                    chamfer,
                    wait_dims: true,
                    dim_w: Some(w),
                    ..
                },
            ) => {
                let h = n.abs();
                let corner = Vec2::new(a.x + w, a.y + h);
                let mut poly = Polyline::rectangle(a, corner);
                let tol = Tolerance::DEFAULT;
                if fillet > 1e-9 {
                    if let Some(f) = fillet_polyline(&poly, fillet, 8, tol) {
                        poly = f;
                    }
                } else if chamfer > 1e-9 {
                    if let Some(c) = chamfer_polyline(&poly, chamfer, tol) {
                        poly = c;
                    }
                }
                out = self.commit_one(Geometry::Polyline(poly));
                self.active = None;
            }
            (
                CommandId::Xline,
                Active::XlineMode {
                    mode: XlineKind::Ang,
                    first,
                    angle_deg: None,
                },
            ) => {
                self.active = Some((
                    CommandId::Xline,
                    Active::XlineMode {
                        mode: XlineKind::Ang,
                        first,
                        angle_deg: Some(n),
                    },
                ));
                out.echoes.push(format!("Angle = {n}°"));
                out.prompt = Some(prompt("Point de passage", "Through point"));
            }
            (
                CommandId::Fillet,
                Active::Fillet {
                    wait_radius: true,
                    multiple,
                    ..
                },
            ) => {
                let _ = self.sysvars.set("FILLETRAD", SysVarValue::Real(n.abs()));
                self.active = Some((
                    CommandId::Fillet,
                    Active::Fillet {
                        radius: Some(n.abs()),
                        multiple,
                        wait_radius: false,
                    },
                ));
                out.prompt = self.current_prompt();
            }
            (CommandId::Divide, Active::Divide { count: None, target }) => {
                self.active = Some((
                    CommandId::Divide,
                    Active::Divide {
                        count: Some(n.round().max(2.0) as usize),
                        target,
                    },
                ));
                out.prompt = Some(prompt("Objet à diviser", "Object to divide"));
            }
            (CommandId::Measure, Active::Measure { spacing: None, target }) => {
                self.active = Some((
                    CommandId::Measure,
                    Active::Measure {
                        spacing: Some(n.abs()),
                        target,
                    },
                ));
                out.prompt = Some(prompt("Objet à mesurer", "Object to measure"));
            }
            (CommandId::Ddptype, Active::Ddptype) => {
                let _ = self.sysvars.set("PDMODE", SysVarValue::Int(n.round() as i64));
                out.echoes.push(format!("PDMODE = {}", n.round()));
                self.active = None;
            }
            (CommandId::Rotate, Active::Rotate { origin: Some(o), copy, .. }) => {
                let rad = n.to_radians();
                out = if copy {
                    self.copy_transform_selection(|g| g.rotated(o, rad))
                } else {
                    self.transform_selection(|g| g.rotated(o, rad))
                };
                self.active = None;
            }
            (CommandId::Scale, Active::Scale { origin: Some(o), copy, .. }) => {
                out = if copy {
                    self.copy_transform_selection(|g| g.scaled(o, n))
                } else {
                    self.transform_selection(|g| g.scaled(o, n))
                };
                self.active = None;
            }
            (CommandId::Polygon, Active::Polygon { sides: None, center, .. }) => {
                let sides = n.round().max(3.0) as usize;
                self.active = Some((
                    CommandId::Polygon,
                    Active::Polygon {
                        sides: Some(sides),
                        center,
                     edge: false, edge_a: None },
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
            (CommandId::Fillet, Active::Fillet { radius: None, multiple: false, wait_radius: false }) => {
                out = self.apply_fillet(n.abs());
                self.active = None;
            }
            (CommandId::Chamfer, Active::Chamfer { distance: None, distance_b: None, multiple: false, wait_distance: false }) => {
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
            (CommandId::Pline, Active::Pline { mut points, width, wait_width }) => {
                if wait_width {
                    out.echoes.push("Saisissez une largeur numérique.".into());
                    out.prompt = self.current_prompt();
                    return out;
                }
                points.push(p);
                self.active = Some((
                    CommandId::Pline,
                    Active::Pline {
                        points,
                        width,
                        wait_width: false,
                    },
                ));
                out.prompt = self.current_prompt();
            }
            (CommandId::Circle, Active::CircleCenter { center: None, diameter }) => {
                self.active = Some((
                    CommandId::Circle,
                    Active::CircleCenter {
                        center: Some(p),
                        diameter,
                    },
                ));
                out.prompt = self.current_prompt();
            }
            (CommandId::Circle, Active::CircleCenter { center: Some(c), diameter }) => {
                let r = if diameter {
                    c.distance(p) * 0.5
                } else {
                    c.distance(p)
                };
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
            (CommandId::Arc, Active::Arc3p { mut points }) => {
                points.push(p);
                if points.len() >= 3 {
                    if let Some(c) =
                        circle_from_3_points(points[0], points[1], points[2], Tolerance::DEFAULT)
                    {
                        let start_ang = (points[0] - c.center).angle();
                        let end_ang = (points[2] - c.center).angle();
                        out = self.commit_one(Geometry::Arc(Arc::new(
                            c.center,
                            c.radius,
                            start_ang,
                            end_ang,
                        )));
                        self.active = None;
                    } else {
                        out.echoes.push("ARC 3P dégénéré.".into());
                        self.active = None;
                    }
                } else {
                    self.active = Some((CommandId::Arc, Active::Arc3p { points }));
                    out.prompt = self.current_prompt();
                }
            }
            (CommandId::Arc, Active::ArcSea { start: None, .. }) => {
                self.active = Some((
                    CommandId::Arc,
                    Active::ArcSea {
                        start: Some(p),
                        end: None,
                    },
                ));
                out.prompt = Some(prompt("Fin de l'arc", "Arc end"));
            }
            (CommandId::Arc, Active::ArcSea { start: Some(s), end: None }) => {
                self.active = Some((
                    CommandId::Arc,
                    Active::ArcSea {
                        start: Some(s),
                        end: Some(p),
                    },
                ));
                out.prompt = Some(prompt("Angle inclus (°) ou 3e point", "Included angle or 3rd pt"));
            }
            (CommandId::Arc, Active::ArcSea { start: Some(s), end: Some(e) }) => {
                // Third pick defines bulge via circumcircle through s, mid-ish, e using p as mid.
                if let Some(c) = circle_from_3_points(s, p, e, Tolerance::DEFAULT) {
                    let start_ang = (s - c.center).angle();
                    let end_ang = (e - c.center).angle();
                    out = self.commit_one(Geometry::Arc(Arc::new(
                        c.center,
                        c.radius,
                        start_ang,
                        end_ang,
                    )));
                } else {
                    out.echoes.push("ARC SEA invalide.".into());
                }
                self.active = None;
            }
            (
                CommandId::Rectang,
                Active::Rectang {
                    first: None,
                    fillet,
                    chamfer,
                    wait_fillet,
                    wait_chamfer,
                    wait_dims,
                    dim_w,
                },
            ) => {
                self.active = Some((
                    CommandId::Rectang,
                    Active::Rectang {
                        first: Some(p),
                        fillet,
                        chamfer,
                        wait_fillet,
                        wait_chamfer,
                        wait_dims,
                        dim_w,
                    },
                ));
                out.prompt = if wait_dims {
                    Some(prompt("Largeur", "Width"))
                } else {
                    self.current_prompt()
                };
            }
            (
                CommandId::Rectang,
                Active::Rectang {
                    first: Some(a),
                    fillet,
                    chamfer,
                    wait_dims: true,
                    dim_w: Some(w),
                    ..
                },
            ) => {
                // Height from second pick delta Y (or absolute Y distance).
                let h = (p.y - a.y).abs().max(1e-9);
                let corner = Vec2::new(a.x + w.copysign(p.x - a.x), a.y + h.copysign(p.y - a.y));
                let mut poly = Polyline::rectangle(a, corner);
                let tol = Tolerance::DEFAULT;
                if fillet > 1e-9 {
                    if let Some(f) = fillet_polyline(&poly, fillet, 8, tol) {
                        poly = f;
                    }
                } else if chamfer > 1e-9 {
                    if let Some(c) = chamfer_polyline(&poly, chamfer, tol) {
                        poly = c;
                    }
                }
                out = self.commit_one(Geometry::Polyline(poly));
                self.active = None;
            }
            (CommandId::Rectang, Active::Rectang { first: Some(a), fillet, chamfer, .. }) => {
                let mut poly = Polyline::rectangle(a, p);
                let tol = Tolerance::DEFAULT;
                if fillet > 1e-9 {
                    if let Some(f) = fillet_polyline(&poly, fillet, 8, tol) {
                        poly = f;
                    }
                } else if chamfer > 1e-9 {
                    if let Some(c) = chamfer_polyline(&poly, chamfer, tol) {
                        poly = c;
                    }
                }
                out = self.commit_one(Geometry::Polyline(poly));
                self.active = None;
            }
            (CommandId::Xline, Active::XlineMode { mode, first: None, angle_deg }) => {
                match mode {
                    XlineKind::Hor => {
                        out = self.commit_one(Geometry::XLine(XLineGeom {
                            point: p,
                            direction: Vec2::new(1.0, 0.0),
                        }));
                        self.active = None;
                    }
                    XlineKind::Ver => {
                        out = self.commit_one(Geometry::XLine(XLineGeom {
                            point: p,
                            direction: Vec2::new(0.0, 1.0),
                        }));
                        self.active = None;
                    }
                    XlineKind::Ang if angle_deg.is_some() => {
                        let deg = angle_deg.unwrap_or(0.0);
                        let rad = deg.to_radians();
                        out = self.commit_one(Geometry::XLine(XLineGeom {
                            point: p,
                            direction: Vec2::new(rad.cos(), rad.sin()),
                        }));
                        self.active = None;
                    }
                    _ => {
                        self.active = Some((
                            CommandId::Xline,
                            Active::XlineMode {
                                mode,
                                first: Some(p),
                                angle_deg,
                            },
                        ));
                        out.prompt = Some(prompt(
                            "Direction (2e point) ou angle",
                            "Direction (2nd point) or angle",
                        ));
                    }
                }
            }
            (
                CommandId::Xline,
                Active::XlineMode {
                    mode: XlineKind::Ang | XlineKind::Offset | XlineKind::TwoPoint,
                    first: Some(a),
                    ..
                },
            ) => {
                let dir = (p - a)
                    .try_normalize(Tolerance::DEFAULT)
                    .unwrap_or(Vec2::X);
                out = self.commit_one(Geometry::XLine(XLineGeom {
                    point: a,
                    direction: dir,
                }));
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
            (CommandId::Copy, Active::Copy { base: None, multiple }) => {
                self.active = Some((
                    CommandId::Copy,
                    Active::Copy {
                        base: Some(p),
                        multiple,
                    },
                ));
                out.prompt = self.current_prompt();
            }
            (CommandId::Copy, Active::Copy { base: Some(b), multiple }) => {
                let delta = p - b;
                out = self.copy_selection(delta);
                if multiple {
                    // Keep base for additional copies until Enter/Esc.
                    self.active = Some((
                        CommandId::Copy,
                        Active::Copy {
                            base: Some(b),
                            multiple: true,
                        },
                    ));
                    out.prompt = Some(prompt(
                        "Point suivant ou Entrée",
                        "Next point or Enter",
                    ));
                } else {
                    self.active = None;
                }
            }
            (CommandId::Rotate, Active::Rotate { origin: None, copy, reference, ref_angle }) => {
                self.active = Some((
                    CommandId::Rotate,
                    Active::Rotate {
                        origin: Some(p),
                        copy,
                        reference,
                        ref_angle,
                    },
                ));
                out.prompt = if reference {
                    Some(prompt("Angle de référence", "Reference angle"))
                } else {
                    self.current_prompt()
                };
            }
            (CommandId::Rotate, Active::Rotate { origin: Some(o), copy, reference, ref_angle: None })
                if reference =>
            {
                self.active = Some((
                    CommandId::Rotate,
                    Active::Rotate {
                        origin: Some(o),
                        copy,
                        reference: true,
                        ref_angle: Some((p - o).angle()),
                    },
                ));
                out.prompt = Some(prompt("Nouvel angle", "New angle"));
            }
            (CommandId::Rotate, Active::Rotate { origin: Some(o), copy, reference, ref_angle }) => {
                let ang = if reference {
                    (p - o).angle() - ref_angle.unwrap_or(0.0)
                } else {
                    (p - o).angle()
                };
                out = if copy {
                    self.copy_transform_selection(|g| g.rotated(o, ang))
                } else {
                    self.transform_selection(|g| g.rotated(o, ang))
                };
                self.active = None;
            }
            (CommandId::Scale, Active::Scale { origin: None, copy, reference, ref_len }) => {
                self.active = Some((
                    CommandId::Scale,
                    Active::Scale {
                        origin: Some(p),
                        copy,
                        reference,
                        ref_len,
                    },
                ));
                out.prompt = if reference {
                    Some(prompt("Longueur de référence", "Reference length"))
                } else {
                    self.current_prompt()
                };
            }
            (CommandId::Scale, Active::Scale { origin: Some(o), copy, reference, ref_len: None })
                if reference =>
            {
                self.active = Some((
                    CommandId::Scale,
                    Active::Scale {
                        origin: Some(o),
                        copy,
                        reference: true,
                        ref_len: Some(o.distance(p).max(1e-12)),
                    },
                ));
                out.prompt = Some(prompt("Nouvelle longueur", "New length"));
            }
            (CommandId::Scale, Active::Scale { origin: Some(o), copy, reference, ref_len }) => {
                let factor = if reference {
                    o.distance(p) / ref_len.unwrap_or(1.0).max(1e-12)
                } else {
                    o.distance(p).max(1e-12)
                };
                out = if copy {
                    self.copy_transform_selection(|g| g.scaled(o, factor))
                } else {
                    self.transform_selection(|g| g.scaled(o, factor))
                };
                self.active = None;
            }
            (CommandId::Mirror, Active::Mirror { first: None, erase_source, wait_erase }) => {
                self.active = Some((
                    CommandId::Mirror,
                    Active::Mirror {
                        first: Some(p),
                        erase_source,
                        wait_erase,
                    },
                ));
                out.prompt = self.current_prompt();
            }
            (CommandId::Mirror, Active::Mirror { first: Some(a), erase_source, .. }) => {
                if erase_source {
                    out = self.transform_selection(|g| g.mirrored(a, p));
                } else {
                    // Keep source: copy mirrored geometry.
                    let mut ops = Vec::new();
                    for id in self.selection.iter() {
                        if let Some(ent) = self.session.document.get_entity(id) {
                            let new_id = self.ids.next();
                            ops.push(Operation::CreateEntity {
                                record: EntityRecord {
                                    id: new_id,
                                    kind: ent.kind,
                                    geometry: ent.geometry.mirrored(a, p),
                                    layer_id: ent.layer_id,
                                    props: ent.props.clone(),
                                },
                            });
                        }
                    }
                    out = self.apply_batch(ops, "Miroir (source conservée).".into());
                }
                self.active = None;
            }
            (CommandId::Offset, Active::Offset { distance: Some(d), through, multiple, erase_source }) => {
                if let Some(eid) = self.pick_entity(p) {
                    if let Some(ent) = self.session.document.get_entity(eid) {
                        match &ent.geometry {
                            Geometry::Line(line) => {
                                let side = (p - line.start).cross(line.delta());
                                let signed = if side >= 0.0 { d } else { -d };
                                if let Some(off) = offset_line(*line, signed) {
                                    out = self.commit_one(Geometry::Line(off));
                                    self.active = None;
                                } else {
                                    out.echoes.push("OFFSET impossible sur cette ligne.".into());
                                    out.prompt = self.current_prompt();
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
                                        self.active = None;
                                    }
                                    None => {
                                        out.echoes.push("OFFSET polyligne impossible.".into());
                                        out.prompt = self.current_prompt();
                                    }
                                }
                            }
                            _ => {
                                out.echoes
                                    .push("OFFSET : ligne ou polyligne requise.".into());
                                out.prompt = self.current_prompt();
                            }
                        }
                    }
                } else {
                    out.echoes.push("OFFSET : aucun objet sous le curseur.".into());
                    out.prompt = self.current_prompt();
                }
            }
            (CommandId::Trim, Active::Trim) => {
                out = self.trim_at(p);
                // Stay in TRIM for multiple cuts (AutoCAD-like).
                out.prompt = self.current_prompt();
            }
            (CommandId::Extend, Active::Extend) => {
                out = self.extend_at(p);
                out.prompt = self.current_prompt();
            }
            (CommandId::Erase, Active::Erase)
            | (_, Active::GatherSelection) => {
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
                let dim = DimensionGeom {
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
                };
                self.last_dim = Some(dim.clone());
                out = self.commit_one(Geometry::Dimension(dim));
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
                Active::Polygon { sides: Some(n), center: None, .. },
            ) => {
                self.active = Some((
                    CommandId::Polygon,
                    Active::Polygon {
                        sides: Some(n),
                        center: Some(p),
                        edge: false,
                        edge_a: None,
                    },
                ));
                out.prompt = self.current_prompt();
            }
            (
                CommandId::Polygon,
                Active::Polygon {
                    sides: Some(n),
                    center: Some(c),
                    edge: false,
                    ..
                },
            ) => {
                if let Some(poly) = regular_polygon_center_vertex(c, p, n) {
                    out = self.commit_one(Geometry::Polyline(poly));
                } else {
                    out.echoes.push("POLYGON invalide.".into());
                }
                self.active = None;
            }
            (
                CommandId::Polygon,
                Active::Polygon {
                    sides: Some(n),
                    edge: true,
                    edge_a: None,
                    ..
                },
            ) => {
                self.active = Some((
                    CommandId::Polygon,
                    Active::Polygon {
                        sides: Some(n),
                        center: None,
                        edge: true,
                        edge_a: Some(p),
                    },
                ));
                out.prompt = self.current_prompt();
            }
            (
                CommandId::Polygon,
                Active::Polygon {
                    sides: Some(n),
                    edge: true,
                    edge_a: Some(a),
                    ..
                },
            ) => {
                if let Some(poly) = regular_polygon_side(a, p, n) {
                    out = self.commit_one(Geometry::Polyline(poly));
                } else {
                    out.echoes.push("POLYGON côté invalide.".into());
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
                            props: self.drafting_props(),
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
                            props: self.drafting_props(),
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
                                        props: self.drafting_props(),
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
                                props: self.drafting_props(),
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
                            props: self.drafting_props(),
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
                                            props: self.drafting_props(),
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
                                            props: self.drafting_props(),
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
                let dim = DimensionGeom {
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
                };
                self.last_dim = Some(dim.clone());
                out = self.commit_one(Geometry::Dimension(dim));
                self.active = None;
            }
            // DIMCONTINUE: next point + dim line offset; chains from last_dim.def2
            (
                CommandId::DimContinue,
                Active::DimContinue {
                    next: None,
                    ..
                },
            ) => {
                self.active = Some((
                    CommandId::DimContinue,
                    Active::DimContinue {
                        next: Some(p),
                        offset: None,
                    },
                ));
                out.prompt = self.current_prompt();
            }
            (
                CommandId::DimContinue,
                Active::DimContinue {
                    next: Some(next),
                    offset: None,
                },
            ) => {
                if let Some(prev) = self.last_dim.clone() {
                    let dim = DimensionGeom {
                        kind: prev.kind,
                        def1: prev.def2,
                        def2: next,
                        dim_line: p,
                        assoc_a: None,
                        assoc_b: None,
                        text_override: None,
                        style_id: Some(self.session.document.current_dim_style),
                        annotative: false,
                        angle_p3: None,
                    };
                    self.last_dim = Some(dim.clone());
                    out = self.commit_one(Geometry::Dimension(dim));
                } else {
                    out.echoes
                        .push("DIMCONTINUE : aucune cote précédente.".into());
                }
                self.active = None;
            }
            // DIMBASELINE: next point; shares def1 with last_dim, offset dim line
            (
                CommandId::DimBaseline,
                Active::DimBaseline {
                    next: None,
                    ..
                },
            ) => {
                self.active = Some((
                    CommandId::DimBaseline,
                    Active::DimBaseline {
                        next: Some(p),
                        offset: None,
                    },
                ));
                out.prompt = self.current_prompt();
            }
            (
                CommandId::DimBaseline,
                Active::DimBaseline {
                    next: Some(next),
                    offset: None,
                },
            ) => {
                if let Some(prev) = self.last_dim.clone() {
                    let dim = DimensionGeom {
                        kind: prev.kind,
                        def1: prev.def1,
                        def2: next,
                        dim_line: p,
                        assoc_a: None,
                        assoc_b: None,
                        text_override: None,
                        style_id: Some(self.session.document.current_dim_style),
                        annotative: false,
                        angle_p3: None,
                    };
                    self.last_dim = Some(dim.clone());
                    out = self.commit_one(Geometry::Dimension(dim));
                } else {
                    out.echoes
                        .push("DIMBASELINE : aucune cote précédente.".into());
                }
                self.active = None;
            }
            (CommandId::Section, Active::Section { a: None }) => {
                self.active = Some((CommandId::Section, Active::Section { a: Some(p) }));
                out.prompt = self.current_prompt();
            }
            (CommandId::Section, Active::Section { a: Some(a) }) => {
                out = self.commit_section_mark(a, p);
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
            (CommandId::Mtext, Active::Mtext { position: None }) => {
                self.active = Some((CommandId::Mtext, Active::Mtext { position: Some(p) }));
                out.prompt = Some(prompt("Saisissez le texte multiligne", "Enter mtext"));
            }
            (CommandId::Matchprop, Active::Matchprop { source: None }) => {
                if let Some(eid) = self.pick_entity(p) {
                    self.active = Some((
                        CommandId::Matchprop,
                        Active::Matchprop {
                            source: Some(eid),
                        },
                    ));
                    out.prompt = Some(prompt(
                        "Objets destination",
                        "Destination objects",
                    ));
                }
            }
            (CommandId::Matchprop, Active::Matchprop { source: Some(src) }) => {
                if let Some(dst) = self.pick_entity(p) {
                    if let (Some(s), Some(d)) = (
                        self.session.document.get_entity(src).cloned(),
                        self.session.document.get_entity(dst).cloned(),
                    ) {
                        out = self.apply_batch(
                            vec![
                                Operation::SetEntityLayer {
                                    id: dst,
                                    layer_id: s.layer_id,
                                },
                                Operation::SetEntityProps {
                                    id: dst,
                                    props: s.props.clone(),
                                },
                            ],
                            format!("MATCHPROP {src} → {dst}"),
                        );
                        // keep source for more destinations
                        let _ = d;
                        self.active = Some((
                            CommandId::Matchprop,
                            Active::Matchprop {
                                source: Some(src),
                            },
                        ));
                    }
                }
            }
            (CommandId::Draworder, Active::Draworder) => {
                if let Some(eid) = self.pick_entity(p) {
                    self.selection.set_single(eid);
                    out = self.draworder_shift(true);
                    self.active = None;
                }
            }
            (CommandId::Boundary, Active::Boundary { mut points }) => {
                points.push(p);
                self.active = Some((CommandId::Boundary, Active::Boundary { points }));
                out.prompt = Some(prompt(
                    "Point suivant ou Entrée pour fermer",
                    "Next point or Enter to close",
                ));
            }
            (CommandId::Select | CommandId::SelectSimilar, Active::SelectGather) => {
                if let Some(eid) = self.pick_entity(p) {
                    if matches!(id, CommandId::SelectSimilar) {
                        if let Some(src) = self.session.document.get_entity(eid) {
                            let kind = src.kind;
                            self.remember_selection();
                            self.selection.clear();
                            for e in self.session.document.entities() {
                                if e.kind == kind {
                                    self.selection.add(e.id);
                                }
                            }
                            out.echoes.push(format!(
                                "SELECTSIMILAR : {} objet(s)",
                                self.selection.len()
                            ));
                            self.active = None;
                        }
                    } else {
                        self.selection.toggle(eid);
                        out.echoes
                            .push(format!("Sélection : {}", self.selection.len()));
                    }
                }
            }
            (CommandId::Divide, Active::Divide { count: Some(n), target: None }) => {
                if let Some(eid) = self.pick_entity(p) {
                    out = self.divide_entity(eid, n);
                    self.active = None;
                }
            }
            (CommandId::Measure, Active::Measure { spacing: Some(sp), target: None }) => {
                if let Some(eid) = self.pick_entity(p) {
                    out = self.measure_entity(eid, sp);
                    self.active = None;
                }
            }
            (CommandId::Pedit, Active::Pedit { target: None, .. }) => {
                if let Some(eid) = self.pick_entity(p) {
                    self.selection.set_single(eid);
                    self.active = Some((
                        CommandId::Pedit,
                        Active::Pedit {
                            target: Some(eid),
                            wait_width: false,
                        },
                    ));
                    out.prompt = Some(prompt(
                        "[Close/Open/Join/Width/Reverse]",
                        "[Close/Open/Join/Width/Reverse]",
                    ));
                }
            }
            (CommandId::Wblock, Active::Wblock { name: None, .. }) => {
                // Point as base; name via text next — store base first using temp name.
                self.active = Some((
                    CommandId::Wblock,
                    Active::Wblock {
                        name: Some("WBLOCK".into()),
                        base: Some(p),
                    },
                ));
                if !self.selection.is_empty() {
                    out = self.create_block_from_selection("WBLOCK", p);
                    self.active = None;
                } else {
                    out.echoes.push("Sélectionnez des objets puis relancez WBLOCK.".into());
                    self.active = None;
                }
            }

            _ => {}
        }
        out
    }

    fn divide_entity(&mut self, eid: EntityId, count: usize) -> CommandOutput {
        let Some(ent) = self.session.document.get_entity(eid) else {
            return CommandOutput::default();
        };
        let pts: Vec<Vec2> = match &ent.geometry {
            Geometry::Line(l) => (1..count)
                .map(|i| l.point_at(i as f64 / count as f64))
                .collect(),
            Geometry::Polyline(poly) => {
                let segs: Vec<_> = poly.segments().collect();
                let total: f64 = segs.iter().map(|s| s.start.distance(s.end)).sum();
                if total < 1e-12 {
                    Vec::new()
                } else {
                    let mut out = Vec::new();
                    for i in 1..count {
                        let target = total * (i as f64 / count as f64);
                        let mut acc = 0.0;
                        for s in &segs {
                            let len = s.start.distance(s.end);
                            if acc + len >= target {
                                let t = ((target - acc) / len).clamp(0.0, 1.0);
                                out.push(s.point_at(t));
                                break;
                            }
                            acc += len;
                        }
                    }
                    out
                }
            }
            _ => Vec::new(),
        };
        if pts.is_empty() {
            return CommandOutput {
                echoes: vec!["DIVIDE : géométrie non supportée.".into()],
                ..Default::default()
            };
        }
        let ops: Vec<_> = pts
            .into_iter()
            .map(|pt| {
                let id = self.ids.next();
                Operation::CreateEntity {
                    record: EntityRecord::new(id, Geometry::Point(pt), self.session.document.current_layer),
                }
            })
            .collect();
        let n = ops.len();
        self.apply_batch(ops, format!("DIVIDE : {n} point(s)."))
    }

    fn measure_entity(&mut self, eid: EntityId, spacing: f64) -> CommandOutput {
        let Some(ent) = self.session.document.get_entity(eid) else {
            return CommandOutput::default();
        };
        let Geometry::Line(l) = &ent.geometry else {
            return CommandOutput {
                echoes: vec!["MEASURE : ligne uniquement pour l'instant.".into()],
                ..Default::default()
            };
        };
        let len = l.start.distance(l.end);
        if spacing <= 1e-12 || len < spacing {
            return CommandOutput {
                echoes: vec!["MEASURE : espacement invalide.".into()],
                ..Default::default()
            };
        }
        let mut ops = Vec::new();
        let mut d = spacing;
        while d < len - 1e-9 {
            let t = d / len;
            let id = self.ids.next();
            ops.push(Operation::CreateEntity {
                record: EntityRecord::new(
                    id,
                    Geometry::Point(l.point_at(t)),
                    self.session.document.current_layer,
                ),
            });
            d += spacing;
        }
        let n = ops.len();
        self.apply_batch(ops, format!("MEASURE : {n} point(s)."))
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
            props: self.drafting_props(),
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
                    props: self.drafting_props(),
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
                    props: self.drafting_props(),
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
                            props: ent.props.clone(),
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
                            props: ent.props.clone(),
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
                    props: ent.props.clone(),
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
                                props: ent.props.clone(),
        },
                        });
                    }
                }
                Geometry::Insert(ins) => {
                    let Some(def) = self.session.document.get_block(ins.block_id).cloned() else {
                        continue;
                    };
                    ops.push(Operation::DeleteEntity { id });
                    for geom in expand_insert(&def, &ins) {
                        let new_id = self.ids.next();
                        ops.push(Operation::CreateEntity {
                            record: EntityRecord {
                                id: new_id,
                                kind: geom.kind(),
                                geometry: geom,
                                layer_id: ent.layer_id,
                                props: ent.props.clone(),
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
                    props: self.drafting_props(),
        },
            },
        ];
        self.apply_batch(ops, format!("XREF « {name} »"))
    }

    fn erase_selection(&mut self) -> CommandOutput {
        let ids = self.selection.to_vec();
        self.remember_selection();
        self.last_erased.clear();
        for id in &ids {
            if let Some(ent) = self.session.document.get_entity(*id) {
                self.last_erased.push(ent.clone());
            }
        }
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
        let mut skipped = 0usize;
        for id in self.selection.iter() {
            if let Some(ent) = self.session.document.get_entity(id) {
                if self.layer_locked(ent.layer_id) {
                    skipped += 1;
                    continue;
                }
                ops.push(Operation::SetGeometry {
                    id,
                    geometry: f(&ent.geometry),
                });
            }
        }
        let n = ops.len();
        let mut out = self.apply_batch(ops, format!("{n} objet(s) modifié(s)"));
        if skipped > 0 {
            out.echoes
                .push(format!("{skipped} objet(s) sur calque verrouillé ignoré(s)."));
        }
        out
    }

    fn copy_selection(&mut self, delta: Vec2) -> CommandOutput {
        self.copy_transform_selection(|g| g.translated(delta))
    }

    fn copy_transform_selection(
        &mut self,
        f: impl Fn(&Geometry) -> Geometry,
    ) -> CommandOutput {
        let mut ops = Vec::new();
        for id in self.selection.iter() {
            if let Some(ent) = self.session.document.get_entity(id) {
                if self.layer_locked(ent.layer_id) {
                    continue;
                }
                let new_id = self.ids.next();
                ops.push(Operation::CreateEntity {
                    record: EntityRecord {
                        id: new_id,
                        kind: ent.kind,
                        geometry: f(&ent.geometry),
                        layer_id: ent.layer_id,
                        props: ent.props.clone(),
                    },
                });
            }
        }
        let n = ops.len();
        self.apply_batch(ops, format!("{n} objet(s) copié(s)"))
    }

    fn layer_locked(&self, layer_id: kelicad_doc::LayerId) -> bool {
        self.session
            .document
            .get_layer(layer_id)
            .map(|l| l.flags.locked)
            .unwrap_or(false)
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

    fn select_by_rect(
        &mut self,
        a: Vec2,
        b: Vec2,
        crossing: bool,
        additive: bool,
    ) -> CommandOutput {
        let min = Vec2::new(a.x.min(b.x), a.y.min(b.y));
        let max = Vec2::new(a.x.max(b.x), a.y.max(b.y));
        if !additive {
            self.selection.clear();
        }
        let mut n = 0usize;
        for ent in self.session.document.entities() {
            let mut box_ = geometry_aabb(&ent.geometry);
            if let Geometry::Insert(ins) = &ent.geometry {
                if let Some(def) = self.session.document.get_block(ins.block_id) {
                    let mut b = kelicad_geom::Aabb::EMPTY;
                    for g in expand_insert(def, ins) {
                        b = b.union(geometry_aabb(&g));
                    }
                    box_ = b;
                }
            }
            let hit = if crossing {
                aabb_intersects_rect(&box_, min, max)
            } else {
                aabb_inside_rect(&box_, min, max)
            };
            if hit {
                self.selection.add(ent.id);
                n += 1;
            }
        }
        let mode = if crossing { "crossing" } else { "fenêtre" };
        CommandOutput {
            echoes: vec![format!("Sélection {mode} : {n} objet(s)")],
            ..Default::default()
        }
    }

    fn commit_section_mark(&mut self, a: Vec2, b: Vec2) -> CommandOutput {
        let dir = (b - a).try_normalize(Tolerance::DEFAULT).unwrap_or(Vec2::X);
        let nrm = dir.perp();
        let tick = 2.5;
        let layer_id = self.session.document.current_layer;
        let mut ops = Vec::new();
        // Main cut line
        let id0 = self.ids.next();
        ops.push(Operation::CreateEntity {
            record: EntityRecord {
                id: id0,
                kind: EntityKind::Line,
                geometry: Geometry::Line(Line::new(a, b)),
                layer_id,
                props: self.drafting_props(),
        },
        });
        // End ticks (section arrows)
        for (p, sign) in [(a, 1.0), (b, -1.0)] {
            let tip = p + nrm * tick * sign;
            let id = self.ids.next();
            ops.push(Operation::CreateEntity {
                record: EntityRecord {
                    id,
                    kind: EntityKind::Line,
                    geometry: Geometry::Line(Line::new(p, tip)),
                    layer_id,
                    props: self.drafting_props(),
        },
            });
            let id2 = self.ids.next();
            ops.push(Operation::CreateEntity {
                record: EntityRecord {
                    id: id2,
                    kind: EntityKind::Line,
                    geometry: Geometry::Line(Line::new(tip, tip + dir * tick * 0.6 - nrm * tick * 0.35 * sign)),
                    layer_id,
                    props: self.drafting_props(),
        },
            });
        }
        // Labels A / A
        for (p, label, side) in [(a, "A", 1.0_f64), (b, "A", -1.0_f64)] {
            let id = self.ids.next();
            ops.push(Operation::CreateEntity {
                record: EntityRecord {
                    id,
                    kind: EntityKind::Text,
                    geometry: Geometry::Text(TextGeom {
                        position: p + nrm * (tick + 1.5) * side,
                        height: 2.5,
                        rotation: 0.0,
                        content: label.into(),
                        align: TextAlign::Left,
                        style_id: None,
                        annotative: false,
                    }),
                    layer_id,
                    props: self.drafting_props(),
                },
            });
        }
        self.apply_batch(ops, "Trait de coupe SECTION.".into())
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
                Geometry::Insert(i) => {
                    if let Some(def) = self.session.document.get_block(i.block_id) {
                        expand_insert(def, i)
                            .iter()
                            .map(|g| match g {
                                Geometry::Line(l) => l.distance_squared(p).sqrt(),
                                Geometry::Circle(c) => c.signed_distance(p).abs(),
                                Geometry::Polyline(poly) => poly
                                    .segments()
                                    .map(|s| s.distance_squared(p))
                                    .fold(f64::INFINITY, f64::min)
                                    .sqrt(),
                                Geometry::Arc(a) => {
                                    let radial = ((p - a.center).length() - a.radius).abs();
                                    a.start_point()
                                        .distance(p)
                                        .min(a.end_point().distance(p))
                                        .min(radial)
                                }
                                other => geometry_aabb(other).center().distance(p),
                            })
                            .fold(f64::INFINITY, f64::min)
                    } else {
                        i.position.distance(p)
                    }
                }
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

    fn remember_selection(&mut self) {
        if !self.selection.is_empty() {
            self.prev_selection = self.selection.to_vec();
        }
    }

    fn apply_option_keyword(&mut self, cid: CommandId, key: &str) -> CommandOutput {
        let mut out = CommandOutput::default();
        match (cid, key) {
            (CommandId::Line, "CLOSE") => {
                if let Some((_, Active::Line { points })) = self.active.clone() {
                    if points.len() >= 2 {
                        let first = points[0];
                        let last = *points.last().unwrap();
                        let mut part = self.commit_one(Geometry::Line(Line::new(last, first)));
                        out.document_changed = true;
                        out.echoes.append(&mut part.echoes);
                        self.active = None;
                        return out;
                    }
                }
                out.echoes.push("CLOSE : pas assez de points.".into());
            }
            (CommandId::Line, "UNDO") => {
                if let Some((_, Active::Line { mut points })) = self.active.clone() {
                    if !points.is_empty() {
                        points.pop();
                        self.active = Some((CommandId::Line, Active::Line { points }));
                        out.echoes.push("Point annulé.".into());
                    }
                }
            }
            (CommandId::Pline, "CLOSE") => {
                if let Some((_, Active::Pline { points, .. })) = self.active.clone() {
                    if points.len() >= 3 {
                        out = self.commit_polyline(points, true);
                        self.active = None;
                        return out;
                    }
                }
                out.echoes.push("CLOSE : ≥3 points requis.".into());
            }
            (CommandId::Pline, "UNDO") => {
                if let Some((
                    _,
                    Active::Pline {
                        mut points,
                        width,
                        wait_width,
                    },
                )) = self.active.clone()
                {
                    points.pop();
                    self.active = Some((
                        CommandId::Pline,
                        Active::Pline {
                            points,
                            width,
                            wait_width,
                        },
                    ));
                    out.echoes.push("Vertex annulé.".into());
                }
            }
            (CommandId::Pline, "WIDTH") => {
                if let Some((
                    _,
                    Active::Pline {
                        points,
                        width,
                        ..
                    },
                )) = self.active.clone()
                {
                    self.active = Some((
                        CommandId::Pline,
                        Active::Pline {
                            points,
                            width,
                            wait_width: true,
                        },
                    ));
                    out.prompt = Some(prompt("Nouvelle largeur", "New width"));
                }
            }
            (CommandId::Circle, "DIAMETER") => {
                self.active = Some((
                    CommandId::Circle,
                    Active::CircleCenter {
                        center: None,
                        diameter: true,
                    },
                ));
                out.prompt = Some(prompt("Centre du cercle", "Center of circle"));
                out.echoes.push("Mode diamètre.".into());
            }
            (CommandId::Circle, "2P") => {
                self.active = Some((CommandId::Circle2p, Active::Circle2p { first: None }));
                out.prompt = self.current_prompt();
            }
            (CommandId::Circle, "3P") => {
                self.active = Some((CommandId::Circle3p, Active::Circle3p { points: vec![] }));
                out.prompt = self.current_prompt();
            }
            (CommandId::Circle, "TTR") => {
                self.active = Some((CommandId::TangentPc, Active::TangentPc { point: None }));
                out.prompt = self.current_prompt();
                out.echoes.push("TTR : point puis cercle (approx).".into());
            }
            (CommandId::Arc, "3P") => {
                self.active = Some((CommandId::Arc, Active::Arc3p { points: vec![] }));
                out.prompt = Some(prompt("Premier point de l'arc", "First arc point"));
            }
            (CommandId::Arc, "SEA") => {
                self.active = Some((
                    CommandId::Arc,
                    Active::ArcSea {
                        start: None,
                        end: None,
                    },
                ));
                out.prompt = Some(prompt("Début de l'arc", "Arc start"));
            }
            (CommandId::Arc, "CSE") => {
                self.active = Some((
                    CommandId::Arc,
                    Active::Arc {
                        center: None,
                        start: None,
                    },
                ));
                out.prompt = self.current_prompt();
            }
            (CommandId::Rectang, "FILLET") => {
                if let Some((_, mut st)) = self.active.clone() {
                    if let Active::Rectang {
                        wait_fillet: ref mut w,
                        ..
                    } = st
                    {
                        *w = true;
                    }
                    self.active = Some((CommandId::Rectang, st));
                    out.prompt = Some(prompt("Rayon de congé", "Fillet radius"));
                }
            }
            (CommandId::Rectang, "CHAMFER") => {
                if let Some((_, mut st)) = self.active.clone() {
                    if let Active::Rectang {
                        wait_chamfer: ref mut w,
                        ..
                    } = st
                    {
                        *w = true;
                    }
                    self.active = Some((CommandId::Rectang, st));
                    out.prompt = Some(prompt("Distance de chanfrein", "Chamfer distance"));
                }
            }
            (CommandId::Polygon, "EDGE") => {
                if let Some((
                    _,
                    Active::Polygon {
                        sides,
                        ..
                    },
                )) = self.active.clone()
                {
                    self.active = Some((
                        CommandId::Polygon,
                        Active::Polygon {
                            sides,
                            center: None,
                            edge: true,
                            edge_a: None,
                        },
                    ));
                    out.prompt = Some(prompt("Premier point du côté", "First edge point"));
                }
            }
            (CommandId::Offset, "THROUGH") => {
                self.active = Some((
                    CommandId::Offset,
                    Active::Offset {
                        distance: None,
                        through: true,
                        multiple: false,
                        erase_source: false,
                    },
                ));
                out.prompt = Some(prompt("Objet à décaler", "Select object to offset"));
            }
            (CommandId::Offset, "MULTIPLE") => {
                if let Some((_, mut st)) = self.active.clone() {
                    if let Active::Offset {
                        multiple: ref mut m,
                        ..
                    } = st
                    {
                        *m = true;
                    }
                    self.active = Some((CommandId::Offset, st));
                    out.echoes.push("OFFSET multiple ON.".into());
                }
            }
            (CommandId::Offset, "ERASE") => {
                if let Some((_, mut st)) = self.active.clone() {
                    if let Active::Offset {
                        erase_source: ref mut e,
                        ..
                    } = st
                    {
                        *e = true;
                    }
                    self.active = Some((CommandId::Offset, st));
                    out.echoes.push("OFFSET efface la source.".into());
                }
            }
            (CommandId::Copy, "MULTIPLE") => {
                if let Some((_, Active::Copy { base, .. })) = self.active.clone() {
                    self.active = Some((
                        CommandId::Copy,
                        Active::Copy {
                            base,
                            multiple: true,
                        },
                    ));
                    out.echoes.push("COPY multiple ON.".into());
                }
            }
            (CommandId::Mirror, "YES") => {
                if let Some((_, mut st)) = self.active.clone() {
                    if let Active::Mirror {
                        erase_source: ref mut e,
                        ..
                    } = st
                    {
                        *e = true;
                    }
                    self.active = Some((CommandId::Mirror, st));
                    out.echoes.push("MIRROR : source effacée.".into());
                }
            }
            (CommandId::Mirror, "NO") => {
                if let Some((_, mut st)) = self.active.clone() {
                    if let Active::Mirror {
                        erase_source: ref mut e,
                        ..
                    } = st
                    {
                        *e = false;
                    }
                    self.active = Some((CommandId::Mirror, st));
                }
            }
            (CommandId::Fillet, "RADIUS") => {
                let r = self.sysvars.get_real("FILLETRAD");
                self.active = Some((
                    CommandId::Fillet,
                    Active::Fillet {
                        radius: None,
                        multiple: false,
                        wait_radius: true,
                    },
                ));
                out.prompt = Some(prompt(
                    &format!("Rayon <{r}>"),
                    &format!("Radius <{r}>"),
                ));
            }
            (CommandId::Fillet, "MULTIPLE") => {
                if let Some((_, mut st)) = self.active.clone() {
                    if let Active::Fillet {
                        multiple: ref mut m,
                        ..
                    } = st
                    {
                        *m = true;
                    }
                    self.active = Some((CommandId::Fillet, st));
                }
                out.echoes.push("FILLET multiple ON.".into());
            }
            (CommandId::Chamfer, "MULTIPLE") => {
                if let Some((_, mut st)) = self.active.clone() {
                    if let Active::Chamfer {
                        multiple: ref mut m,
                        ..
                    } = st
                    {
                        *m = true;
                    }
                    self.active = Some((CommandId::Chamfer, st));
                }
                out.echoes.push("CHAMFER multiple ON.".into());
            }
            (CommandId::Fillet, "POLYLINE") | (CommandId::Chamfer, "POLYLINE") => {
                out.echoes.push(
                    "Sélectionnez une polyligne puis Entrée (congé/chanfrein coins)."
                        .into(),
                );
            }
            (CommandId::Chamfer, "DISTANCE") => {
                if let Some((_, mut st)) = self.active.clone() {
                    if let Active::Chamfer {
                        wait_distance: ref mut w,
                        ..
                    } = st
                    {
                        *w = true;
                    }
                    self.active = Some((CommandId::Chamfer, st));
                    out.prompt = Some(prompt("1re distance", "First distance"));
                }
            }
            (CommandId::Trim, "QUICK") | (CommandId::Extend, "QUICK") => {
                let _ = self.sysvars.set("TRIMEXTENDMODE", SysVarValue::Int(1));
                out.echoes.push("Mode Quick.".into());
            }
            (CommandId::Trim, "STANDARD") | (CommandId::Extend, "STANDARD") => {
                let _ = self.sysvars.set("TRIMEXTENDMODE", SysVarValue::Int(0));
                out.echoes.push("Mode Standard.".into());
            }
            (CommandId::Trim, "FENCE") | (CommandId::Extend, "FENCE") => {
                out.echoes
                    .push("Fence : cliquez les objets à traiter (mode multi).".into());
            }
            (CommandId::Rectang, "DIMENSIONS") => {
                if let Some((_, mut st)) = self.active.clone() {
                    if let Active::Rectang {
                        wait_dims: ref mut w,
                        ..
                    } = st
                    {
                        *w = true;
                    }
                    self.active = Some((CommandId::Rectang, st));
                    out.prompt = Some(prompt("Largeur", "Width"));
                }
            }
            (CommandId::Polygon, "INSCRIBED") => {
                if let Some((
                    _,
                    Active::Polygon {
                        sides,
                        ..
                    },
                )) = self.active.clone()
                {
                    self.active = Some((
                        CommandId::Polygon,
                        Active::Polygon {
                            sides,
                            center: None,
                            edge: false,
                            edge_a: None,
                        },
                    ));
                    out.echoes.push("Polygone inscrit (centre + sommet).".into());
                    out.prompt = Some(prompt("Centre", "Center"));
                }
            }
            (CommandId::Polygon, "CIRCUMSCRIBED") => {
                if let Some((
                    _,
                    Active::Polygon {
                        sides,
                        ..
                    },
                )) = self.active.clone()
                {
                    // Approximation: same centre+vertex path (edge midpoint = radius later).
                    self.active = Some((
                        CommandId::Polygon,
                        Active::Polygon {
                            sides,
                            center: None,
                            edge: false,
                            edge_a: None,
                        },
                    ));
                    out.echoes
                        .push("Polygone circonscrit (centre + point sur côté).".into());
                    out.prompt = Some(prompt("Centre", "Center"));
                }
            }
            (CommandId::Rotate, "COPY") | (CommandId::Scale, "COPY") => {
                if let Some((_, mut st)) = self.active.clone() {
                    match &mut st {
                        Active::Rotate { copy, .. } | Active::Scale { copy, .. } => *copy = true,
                        _ => {}
                    }
                    self.active = Some((cid, st));
                    out.echoes.push("Mode copie ON.".into());
                }
            }
            (CommandId::Rotate, "REFERENCE") => {
                if let Some((_, mut st)) = self.active.clone() {
                    if let Active::Rotate {
                        reference: ref mut r,
                        ..
                    } = st
                    {
                        *r = true;
                    }
                    self.active = Some((CommandId::Rotate, st));
                    out.echoes.push("ROTATE référence ON.".into());
                }
            }
            (CommandId::Scale, "REFERENCE") => {
                if let Some((_, mut st)) = self.active.clone() {
                    if let Active::Scale {
                        reference: ref mut r,
                        ..
                    } = st
                    {
                        *r = true;
                    }
                    self.active = Some((CommandId::Scale, st));
                    out.echoes.push("SCALE référence ON.".into());
                }
            }
            (CommandId::Xline, "HOR") => {
                self.active = Some((
                    CommandId::Xline,
                    Active::XlineMode {
                        mode: XlineKind::Hor,
                        first: None,
                        angle_deg: None,
                    },
                ));
                out.prompt = Some(prompt("Point de passage", "Through point"));
            }
            (CommandId::Xline, "VER") => {
                self.active = Some((
                    CommandId::Xline,
                    Active::XlineMode {
                        mode: XlineKind::Ver,
                        first: None,
                        angle_deg: None,
                    },
                ));
                out.prompt = Some(prompt("Point de passage", "Through point"));
            }
            (CommandId::Xline, "ANG") => {
                self.active = Some((
                    CommandId::Xline,
                    Active::XlineMode {
                        mode: XlineKind::Ang,
                        first: None,
                        angle_deg: None,
                    },
                ));
                out.prompt = Some(prompt(
                    "Angle (°) ou point de passage",
                    "Angle (°) or through point",
                ));
                out.echoes.push("XLINE angle.".into());
            }
            (CommandId::Xline, "OFFSET") => {
                self.active = Some((
                    CommandId::Xline,
                    Active::XlineMode {
                        mode: XlineKind::Offset,
                        first: None,
                        angle_deg: None,
                    },
                ));
                out.prompt = Some(prompt("Point de passage", "Through point"));
                out.echoes.push("XLINE offset (2 points direction).".into());
            }
            (CommandId::Pline, "ARC") => {
                out.echoes
                    .push("PLINE Arc : utilisez ARC ou saisissez des points (bulge bientôt).".into());
            }
            (CommandId::Pline, "LINE") => {
                out.echoes.push("Mode ligne PLINE.".into());
            }
            (CommandId::Draworder, "FRONT") => {
                out = self.draworder_shift(true);
            }
            (CommandId::Draworder, "BACK") => {
                out = self.draworder_shift(false);
            }
            (CommandId::Pedit, "CLOSE" | "OPEN" | "JOIN" | "WIDTH" | "REVERSE") => {
                out = self.pedit_option(key);
            }
            _ => {
                out.echoes
                    .push(format!("Option {key} non gérée pour {}", cid.name()));
            }
        }
        if out.prompt.is_none() {
            out.prompt = self.current_prompt();
        }
        out
    }

    fn oops_restore(&mut self) -> CommandOutput {
        if self.last_erased.is_empty() {
            return CommandOutput {
                echoes: vec!["OOPS : rien à restaurer.".into()],
                ..Default::default()
            };
        }
        let records = std::mem::take(&mut self.last_erased);
        let ops: Vec<_> = records
            .into_iter()
            .map(|record| Operation::CreateEntity { record })
            .collect();
        self.apply_batch(ops, "OOPS : objets restaurés.".into())
    }

    fn run_overkill(&mut self) -> CommandOutput {
        // Merge near-duplicate collinear line pairs by deleting the shorter overlapping one.
        let lines: Vec<_> = self
            .session
            .document
            .entities()
            .filter_map(|e| match &e.geometry {
                Geometry::Line(l) => Some((e.id, *l)),
                _ => None,
            })
            .collect();
        let tol = Tolerance::DEFAULT;
        let mut to_delete = Vec::new();
        for i in 0..lines.len() {
            for j in (i + 1)..lines.len() {
                let (id_a, a) = lines[i];
                let (id_b, b) = lines[j];
                if join_collinear_lines(a, b, tol).is_some() {
                    let la = a.start.distance(a.end);
                    let lb = b.start.distance(b.end);
                    to_delete.push(if la <= lb { id_a } else { id_b });
                }
            }
        }
        to_delete.sort();
        to_delete.dedup();
        if to_delete.is_empty() {
            return CommandOutput {
                echoes: vec!["OVERKILL : aucun doublon.".into()],
                ..Default::default()
            };
        }
        let ops: Vec<_> = to_delete
            .into_iter()
            .map(|id| Operation::DeleteEntity { id })
            .collect();
        let n = ops.len();
        self.apply_batch(ops, format!("OVERKILL : {n} doublon(s) supprimé(s)."))
    }

    fn layer_tool(&mut self, id: CommandId) -> CommandOutput {
        let Some(eid) = self.selection.iter().next() else {
            return CommandOutput {
                echoes: vec!["Sélectionnez un objet sur le calque cible.".into()],
                ..Default::default()
            };
        };
        let Some(ent) = self.session.document.get_entity(eid).cloned() else {
            return CommandOutput::default();
        };
        let Some(mut layer) = self.session.document.get_layer(ent.layer_id).cloned() else {
            return CommandOutput::default();
        };
        match id {
            CommandId::Layoff => layer.flags.off = true,
            CommandId::Layfrz => layer.flags.frozen = true,
            CommandId::Laylck => layer.flags.locked = true,
            CommandId::Layiso => {
                // Freeze all other layers.
                let keep = layer.id;
                let mut ops = Vec::new();
                for l in self.session.document.layers() {
                    if l.id != keep {
                        let mut other = l.clone();
                        other.flags.frozen = true;
                        ops.push(Operation::UpdateLayer { layer: other });
                    }
                }
                return self.apply_batch(ops, "LAYISO : autres calques gelés.".into());
            }
            _ => {}
        }
        self.apply_batch(
            vec![Operation::UpdateLayer { layer }],
            format!("{} appliqué.", id.name()),
        )
    }

    fn draworder_shift(&mut self, front: bool) -> CommandOutput {
        let delta = if front { 1 } else { -1 };
        let mut ops = Vec::new();
        for id in self.selection.iter() {
            if let Some(ent) = self.session.document.get_entity(id) {
                let mut props = ent.props.clone();
                props.draw_order += delta;
                ops.push(Operation::SetEntityProps { id, props });
            }
        }
        if ops.is_empty() {
            return CommandOutput {
                echoes: vec!["DRAWORDER : sélection vide.".into()],
                ..Default::default()
            };
        }
        let msg = if front {
            "Objets amenés devant."
        } else {
            "Objets envoyés derrière."
        };
        self.apply_batch(ops, msg.into())
    }

    fn pedit_option(&mut self, key: &str) -> CommandOutput {
        let Some(id) = self.selection.iter().next() else {
            return CommandOutput {
                echoes: vec!["PEDIT : sélectionnez une polyligne.".into()],
                ..Default::default()
            };
        };
        let Some(ent) = self.session.document.get_entity(id).cloned() else {
            return CommandOutput::default();
        };
        let Geometry::Polyline(mut poly) = ent.geometry else {
            return CommandOutput {
                echoes: vec!["PEDIT : pas une polyligne.".into()],
                ..Default::default()
            };
        };
        match key {
            "CLOSE" => poly.closed = true,
            "OPEN" => poly.closed = false,
            "REVERSE" => {
                poly.vertices.reverse();
            }
            "JOIN" => {
                return CommandOutput {
                    echoes: vec!["PEDIT JOIN : utilisez JOIN sur la sélection.".into()],
                    ..Default::default()
                };
            }
            "WIDTH" => {
                let _ = self
                    .sysvars
                    .set("PLINEWID", SysVarValue::Real(self.sysvars.get_real("PLINEWID")));
                return CommandOutput {
                    echoes: vec!["Largeur courante conservée (PLINEWID).".into()],
                    ..Default::default()
                };
            }
            _ => {}
        }
        self.apply_batch(
            vec![Operation::SetGeometry {
                id,
                geometry: Geometry::Polyline(poly),
            }],
            format!("PEDIT {key}."),
        )
    }
}

fn prompt(fr: &str, en: &str) -> EnginePrompt {
    EnginePrompt {
        message_fr: fr.into(),
        message_en: en.into(),
    }
}

fn parse_cecolor(raw: &str) -> Option<[u8; 3]> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "" | "bylayer" | "byblock" => None,
        "red" | "1" => Some([255, 0, 0]),
        "yellow" | "2" => Some([255, 255, 0]),
        "green" | "3" => Some([0, 255, 0]),
        "cyan" | "4" => Some([0, 255, 255]),
        "blue" | "5" => Some([0, 0, 255]),
        "magenta" | "6" => Some([255, 0, 255]),
        "white" | "7" => Some([255, 255, 255]),
        other => {
            // "r,g,b"
            let parts: Vec<_> = other.split(',').collect();
            if parts.len() == 3 {
                if let (Ok(r), Ok(g), Ok(b)) = (
                    parts[0].trim().parse::<u8>(),
                    parts[1].trim().parse::<u8>(),
                    parts[2].trim().parse::<u8>(),
                ) {
                    return Some([r, g, b]);
                }
            }
            None
        }
    }
}

fn parse_celtype(raw: &str) -> Option<String> {
    let t = raw.trim();
    if t.is_empty() || t.eq_ignore_ascii_case("BYLAYER") || t.eq_ignore_ascii_case("BYBLOCK") {
        None
    } else {
        Some(t.to_ascii_uppercase())
    }
}

fn parse_celweight(v: i64) -> Option<u16> {
    if v < 0 {
        None
    } else {
        Some(v as u16)
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
        eng.handle(CommandEvent::Enter); // wait for text
        assert_eq!(eng.session.document.entity_count(), 1);
        eng.handle(CommandEvent::Text("note".into()));
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
