//! Exhaustive smoke tests: every `CommandId` must start and complete a happy path
//! without panicking. Failures indicate broken verb–noun / pick / enter flows.

use kelicad_cmd::{CommandEngine, CommandEvent, CommandId, SequentialIdGen};
use kelicad_math::Vec2;
use kelicad_ops::DocSession;

fn eng() -> CommandEngine<SequentialIdGen> {
    CommandEngine::new(DocSession::new(), SequentialIdGen::new(1))
}

fn seed_line(e: &mut CommandEngine<SequentialIdGen>, a: Vec2, b: Vec2) {
    e.handle(CommandEvent::Start(CommandId::Line));
    e.handle(CommandEvent::Pick(a));
    e.handle(CommandEvent::Pick(b));
    e.handle(CommandEvent::Enter);
}

fn seed_two_lines(e: &mut CommandEngine<SequentialIdGen>) {
    seed_line(e, Vec2::ZERO, Vec2::new(10.0, 0.0));
    seed_line(e, Vec2::new(0.0, 5.0), Vec2::new(10.0, 5.0));
}

fn select_all(e: &mut CommandEngine<SequentialIdGen>) {
    e.selection.clear();
    let ids: Vec<_> = e.session.document.entities().map(|ent| ent.id).collect();
    for id in ids {
        e.selection.add(id);
    }
}

#[test]
fn every_command_id_parses_and_is_listed() {
    assert_eq!(CommandId::all().len(), 64);
    for id in CommandId::all() {
        assert_eq!(CommandId::parse(id.name()), Some(*id));
    }
}

#[test]
fn draw_line_pline_circle_arc_rect_poly_ellipse_spline() {
    let mut e = eng();
    // LINE
    e.handle(CommandEvent::Start(CommandId::Line));
    e.handle(CommandEvent::Pick(Vec2::ZERO));
    e.handle(CommandEvent::Pick(Vec2::new(5.0, 0.0)));
    e.handle(CommandEvent::Enter);
    assert!(e.session.document.entity_count() >= 1);

    // PLINE
    e.handle(CommandEvent::Start(CommandId::Pline));
    e.handle(CommandEvent::Pick(Vec2::new(0.0, 2.0)));
    e.handle(CommandEvent::Pick(Vec2::new(2.0, 2.0)));
    e.handle(CommandEvent::Pick(Vec2::new(2.0, 4.0)));
    e.handle(CommandEvent::Enter);
    assert!(e.session.document.entity_count() >= 2);

    // CIRCLE
    let n = e.session.document.entity_count();
    e.handle(CommandEvent::Start(CommandId::Circle));
    e.handle(CommandEvent::Pick(Vec2::new(20.0, 20.0)));
    e.handle(CommandEvent::Pick(Vec2::new(23.0, 20.0)));
    assert_eq!(e.session.document.entity_count(), n + 1);

    // ARC
    let n = e.session.document.entity_count();
    e.handle(CommandEvent::Start(CommandId::Arc));
    e.handle(CommandEvent::Pick(Vec2::new(30.0, 0.0)));
    e.handle(CommandEvent::Pick(Vec2::new(32.0, 0.0)));
    e.handle(CommandEvent::Pick(Vec2::new(30.0, 2.0)));
    assert!(e.session.document.entity_count() >= n);

    // RECTANG
    let n = e.session.document.entity_count();
    e.handle(CommandEvent::Start(CommandId::Rectang));
    e.handle(CommandEvent::Pick(Vec2::new(40.0, 0.0)));
    e.handle(CommandEvent::Pick(Vec2::new(45.0, 3.0)));
    assert_eq!(e.session.document.entity_count(), n + 1);

    // POLYGON
    let n = e.session.document.entity_count();
    e.handle(CommandEvent::Start(CommandId::Polygon));
    e.handle(CommandEvent::Text("6".into()));
    e.handle(CommandEvent::Pick(Vec2::new(50.0, 0.0)));
    e.handle(CommandEvent::Pick(Vec2::new(52.0, 0.0)));
    assert_eq!(e.session.document.entity_count(), n + 1);

    // ELLIPSE
    let n = e.session.document.entity_count();
    e.handle(CommandEvent::Start(CommandId::Ellipse));
    e.handle(CommandEvent::Pick(Vec2::new(60.0, 0.0)));
    e.handle(CommandEvent::Pick(Vec2::new(64.0, 0.0)));
    e.handle(CommandEvent::Pick(Vec2::new(60.0, 2.0)));
    assert_eq!(e.session.document.entity_count(), n + 1);

    // SPLINE
    let n = e.session.document.entity_count();
    e.handle(CommandEvent::Start(CommandId::Spline));
    e.handle(CommandEvent::Pick(Vec2::new(70.0, 0.0)));
    e.handle(CommandEvent::Pick(Vec2::new(72.0, 1.0)));
    e.handle(CommandEvent::Pick(Vec2::new(74.0, 0.0)));
    e.handle(CommandEvent::Enter);
    assert_eq!(e.session.document.entity_count(), n + 1);
}

#[test]
fn draw_circle2p_3p_point_ray_xline_donut_star_triangle_capsule_arrow_grid() {
    let mut e = eng();

    e.handle(CommandEvent::Start(CommandId::Circle2p));
    e.handle(CommandEvent::Pick(Vec2::ZERO));
    e.handle(CommandEvent::Pick(Vec2::new(4.0, 0.0)));
    assert_eq!(e.session.document.entity_count(), 1);

    e.handle(CommandEvent::Start(CommandId::Circle3p));
    e.handle(CommandEvent::Pick(Vec2::new(10.0, 0.0)));
    e.handle(CommandEvent::Pick(Vec2::new(12.0, 0.0)));
    e.handle(CommandEvent::Pick(Vec2::new(11.0, 2.0)));
    assert!(e.session.document.entity_count() >= 2);

    e.handle(CommandEvent::Start(CommandId::Point));
    e.handle(CommandEvent::Pick(Vec2::new(15.0, 15.0)));
    e.handle(CommandEvent::Enter);

    e.handle(CommandEvent::Start(CommandId::Ray));
    e.handle(CommandEvent::Pick(Vec2::new(0.0, 20.0)));
    e.handle(CommandEvent::Pick(Vec2::new(1.0, 20.0)));

    e.handle(CommandEvent::Start(CommandId::Xline));
    e.handle(CommandEvent::Pick(Vec2::new(0.0, 25.0)));
    e.handle(CommandEvent::Pick(Vec2::new(1.0, 25.0)));

    e.handle(CommandEvent::Start(CommandId::Donut));
    e.handle(CommandEvent::Pick(Vec2::new(30.0, 30.0)));
    e.handle(CommandEvent::Text("2".into()));
    e.handle(CommandEvent::Text("1".into()));

    e.handle(CommandEvent::Start(CommandId::Star));
    e.handle(CommandEvent::Text("5".into()));
    e.handle(CommandEvent::Pick(Vec2::new(40.0, 40.0)));
    e.handle(CommandEvent::Pick(Vec2::new(43.0, 40.0)));

    e.handle(CommandEvent::Start(CommandId::Triangle));
    e.handle(CommandEvent::Pick(Vec2::new(50.0, 0.0)));
    e.handle(CommandEvent::Pick(Vec2::new(54.0, 0.0)));
    e.handle(CommandEvent::Pick(Vec2::new(52.0, 3.0)));

    e.handle(CommandEvent::Start(CommandId::Capsule));
    e.handle(CommandEvent::Pick(Vec2::new(60.0, 0.0)));
    e.handle(CommandEvent::Pick(Vec2::new(65.0, 0.0)));
    e.handle(CommandEvent::Pick(Vec2::new(65.0, 1.0)));

    e.handle(CommandEvent::Start(CommandId::Arrow));
    e.handle(CommandEvent::Pick(Vec2::new(70.0, 0.0)));
    e.handle(CommandEvent::Pick(Vec2::new(75.0, 0.0)));

    e.handle(CommandEvent::Start(CommandId::Grid));
    e.handle(CommandEvent::Text("3".into()));
    e.handle(CommandEvent::Text("3".into()));
    e.handle(CommandEvent::Pick(Vec2::new(80.0, 0.0)));
    e.handle(CommandEvent::Pick(Vec2::new(86.0, 6.0)));

    assert!(e.session.document.entity_count() >= 8);
}

#[test]
fn hatch_wipeout_revcloud() {
    let mut e = eng();
    e.handle(CommandEvent::Start(CommandId::Hatch));
    e.handle(CommandEvent::Pick(Vec2::ZERO));
    e.handle(CommandEvent::Pick(Vec2::new(4.0, 0.0)));
    e.handle(CommandEvent::Pick(Vec2::new(4.0, 3.0)));
    e.handle(CommandEvent::Enter);
    assert_eq!(e.session.document.entity_count(), 1);

    e.handle(CommandEvent::Start(CommandId::Wipeout));
    e.handle(CommandEvent::Pick(Vec2::new(10.0, 0.0)));
    e.handle(CommandEvent::Pick(Vec2::new(14.0, 2.0)));
    assert_eq!(e.session.document.entity_count(), 2);

    e.handle(CommandEvent::Start(CommandId::Revcloud));
    e.handle(CommandEvent::Pick(Vec2::new(20.0, 0.0)));
    e.handle(CommandEvent::Pick(Vec2::new(22.0, 1.0)));
    e.handle(CommandEvent::Pick(Vec2::new(24.0, 0.0)));
    e.handle(CommandEvent::Enter);
    assert_eq!(e.session.document.entity_count(), 3);
}

#[test]
fn modify_verb_noun_move_copy_rotate_scale_mirror() {
    let mut e = eng();
    seed_line(&mut e, Vec2::ZERO, Vec2::new(10.0, 0.0));
    assert_eq!(e.session.document.entity_count(), 1);

    // MOVE with empty selection → gather → enter → base/dest
    e.selection.clear();
    e.handle(CommandEvent::Start(CommandId::Move));
    assert!(e.active_command().is_some());
    e.handle(CommandEvent::Pick(Vec2::new(5.0, 0.0))); // select line
    e.handle(CommandEvent::Enter);
    e.handle(CommandEvent::Pick(Vec2::ZERO));
    e.handle(CommandEvent::Pick(Vec2::new(0.0, 2.0)));
    assert!(e.active_command().is_none());

    // COPY
    e.selection.clear();
    select_all(&mut e);
    e.handle(CommandEvent::Start(CommandId::Copy));
    e.handle(CommandEvent::Pick(Vec2::ZERO));
    e.handle(CommandEvent::Pick(Vec2::new(0.0, 5.0)));
    assert_eq!(e.session.document.entity_count(), 2);

    // ROTATE
    e.selection.clear();
    select_all(&mut e);
    e.handle(CommandEvent::Start(CommandId::Rotate));
    e.handle(CommandEvent::Pick(Vec2::ZERO));
    e.handle(CommandEvent::Text("90".into()));
    assert!(e.active_command().is_none());

    // SCALE
    e.selection.clear();
    select_all(&mut e);
    let n = e.session.document.entity_count();
    e.handle(CommandEvent::Start(CommandId::Scale));
    e.handle(CommandEvent::Pick(Vec2::ZERO));
    e.handle(CommandEvent::Text("2".into()));
    assert_eq!(e.session.document.entity_count(), n);

    // MIRROR
    e.selection.clear();
    select_all(&mut e);
    e.handle(CommandEvent::Start(CommandId::Mirror));
    e.handle(CommandEvent::Pick(Vec2::new(100.0, 0.0)));
    e.handle(CommandEvent::Pick(Vec2::new(100.0, 10.0)));
    assert!(e.active_command().is_none());
}

#[test]
fn modify_offset_trim_extend_fillet_chamfer_erase() {
    let mut e = eng();
    seed_two_lines(&mut e);
    // vertical cutter
    seed_line(&mut e, Vec2::new(5.0, -1.0), Vec2::new(5.0, 6.0));

    // OFFSET
    e.handle(CommandEvent::Start(CommandId::Offset));
    e.handle(CommandEvent::Text("1".into()));
    e.handle(CommandEvent::Pick(Vec2::new(2.0, 0.1)));
    assert!(e.session.document.entity_count() >= 4);

    // TRIM stays active
    e.handle(CommandEvent::Start(CommandId::Trim));
    e.handle(CommandEvent::Pick(Vec2::new(7.0, 0.0)));
    assert_eq!(e.active_command(), Some(CommandId::Trim));
    e.handle(CommandEvent::Enter);
    // Enter on Trim may ignore — cancel
    e.handle(CommandEvent::Cancel);

    // EXTEND
    e.handle(CommandEvent::Start(CommandId::Extend));
    e.handle(CommandEvent::Pick(Vec2::new(0.0, 5.0)));
    e.handle(CommandEvent::Cancel);

    // FILLET gather
    e.selection.clear();
    e.handle(CommandEvent::Start(CommandId::Fillet));
    assert!(e.active_command().is_some());
    // pick two horizontal lines if still present
    e.handle(CommandEvent::Pick(Vec2::new(2.0, 0.0)));
    e.handle(CommandEvent::Pick(Vec2::new(2.0, 5.0)));
    e.handle(CommandEvent::Enter);
    if e.active_command() == Some(CommandId::Fillet) {
        e.handle(CommandEvent::Text("0.5".into()));
    }
    e.handle(CommandEvent::Cancel);

    // CHAMFER gather
    e.selection.clear();
    seed_line(&mut e, Vec2::new(0.0, 20.0), Vec2::new(10.0, 20.0));
    seed_line(&mut e, Vec2::new(0.0, 20.0), Vec2::new(0.0, 30.0));
    e.handle(CommandEvent::Start(CommandId::Chamfer));
    e.handle(CommandEvent::Pick(Vec2::new(5.0, 20.0)));
    e.handle(CommandEvent::Pick(Vec2::new(0.0, 25.0)));
    e.handle(CommandEvent::Enter);
    if e.active_command() == Some(CommandId::Chamfer) {
        e.handle(CommandEvent::Text("0.5".into()));
    }
    e.handle(CommandEvent::Cancel);

    // ERASE
    e.selection.clear();
    e.handle(CommandEvent::Start(CommandId::Erase));
    e.handle(CommandEvent::Pick(Vec2::new(5.0, 20.0)));
    e.handle(CommandEvent::Enter);
}

#[test]
fn modify_array_stretch_break_join_explode_lengthen_align() {
    let mut e = eng();
    seed_line(&mut e, Vec2::ZERO, Vec2::new(4.0, 0.0));
    select_all(&mut e);

    e.handle(CommandEvent::Start(CommandId::ArrayRect));
    e.handle(CommandEvent::Text("2".into()));
    e.handle(CommandEvent::Text("2".into()));
    e.handle(CommandEvent::Text("5".into()));
    e.handle(CommandEvent::Text("5".into()));
    assert!(e.session.document.entity_count() >= 4);

    // BREAK
    e.selection.clear();
    seed_line(&mut e, Vec2::new(0.0, 50.0), Vec2::new(10.0, 50.0));
    e.handle(CommandEvent::Start(CommandId::Break));
    e.handle(CommandEvent::Pick(Vec2::new(5.0, 50.0)));
    assert!(e.session.document.entity_count() >= 2);

    // JOIN
    e.selection.clear();
    seed_line(&mut e, Vec2::new(0.0, 60.0), Vec2::new(5.0, 60.0));
    seed_line(&mut e, Vec2::new(5.0, 60.0), Vec2::new(10.0, 60.0));
    select_all(&mut e);
    // only select the two new lines — clear and pick near them
    e.selection.clear();
    e.handle(CommandEvent::Start(CommandId::Join));
    e.handle(CommandEvent::Pick(Vec2::new(2.0, 60.0)));
    e.handle(CommandEvent::Pick(Vec2::new(7.0, 60.0)));
    e.handle(CommandEvent::Enter);

    // EXPLODE rect
    e.handle(CommandEvent::Start(CommandId::Rectang));
    e.handle(CommandEvent::Pick(Vec2::new(100.0, 0.0)));
    e.handle(CommandEvent::Pick(Vec2::new(104.0, 2.0)));
    let rect_id = e.session.document.entities().last().unwrap().id;
    e.selection.clear();
    e.selection.set_single(rect_id);
    e.handle(CommandEvent::Start(CommandId::Explode));
    assert!(e.session.document.entity_count() >= 4);

    // LENGTHEN
    e.handle(CommandEvent::Start(CommandId::Lengthen));
    e.handle(CommandEvent::Text("1".into()));
    e.handle(CommandEvent::Pick(Vec2::new(102.0, 0.0)));
    e.handle(CommandEvent::Cancel);

    // ALIGN gather
    e.selection.clear();
    seed_line(&mut e, Vec2::new(0.0, 80.0), Vec2::new(3.0, 80.0));
    e.handle(CommandEvent::Start(CommandId::Align));
    e.handle(CommandEvent::Pick(Vec2::new(1.0, 80.0)));
    e.handle(CommandEvent::Enter);
    if e.active_command() == Some(CommandId::Align) {
        e.handle(CommandEvent::Pick(Vec2::new(0.0, 80.0)));
        e.handle(CommandEvent::Pick(Vec2::new(3.0, 80.0)));
        e.handle(CommandEvent::Pick(Vec2::new(10.0, 90.0)));
        e.handle(CommandEvent::Pick(Vec2::new(13.0, 90.0)));
    }

    // STRETCH
    e.selection.clear();
    seed_line(&mut e, Vec2::new(0.0, 100.0), Vec2::new(10.0, 100.0));
    select_all(&mut e);
    e.handle(CommandEvent::Start(CommandId::Stretch));
    // may need window — cancel if stuck
    e.handle(CommandEvent::Cancel);

    // ARRAY POLAR
    e.selection.clear();
    seed_line(&mut e, Vec2::new(0.0, 110.0), Vec2::new(2.0, 110.0));
    select_all(&mut e);
    e.handle(CommandEvent::Start(CommandId::ArrayPolar));
    e.handle(CommandEvent::Text("4".into()));
    e.handle(CommandEvent::Pick(Vec2::new(0.0, 110.0)));
    e.handle(CommandEvent::Text("360".into()));
}

#[test]
fn annotate_text_dims_leader_table_centermark_centerline_section() {
    let mut e = eng();

    e.handle(CommandEvent::Start(CommandId::Text));
    e.handle(CommandEvent::Pick(Vec2::new(0.0, 0.0)));
    e.handle(CommandEvent::Text("Hello".into()));
    assert_eq!(e.session.document.entity_count(), 1);

    e.handle(CommandEvent::Start(CommandId::DimLinear));
    e.handle(CommandEvent::Pick(Vec2::ZERO));
    e.handle(CommandEvent::Pick(Vec2::new(10.0, 0.0)));
    e.handle(CommandEvent::Pick(Vec2::new(5.0, 2.0)));
    assert_eq!(e.session.document.entity_count(), 2);

    e.handle(CommandEvent::Start(CommandId::DimAligned));
    e.handle(CommandEvent::Pick(Vec2::new(0.0, 10.0)));
    e.handle(CommandEvent::Pick(Vec2::new(5.0, 15.0)));
    e.handle(CommandEvent::Pick(Vec2::new(2.0, 14.0)));
    assert_eq!(e.session.document.entity_count(), 3);

    e.handle(CommandEvent::Start(CommandId::DimRadius));
    e.handle(CommandEvent::Pick(Vec2::new(20.0, 20.0)));
    e.handle(CommandEvent::Pick(Vec2::new(23.0, 20.0)));
    assert!(e.session.document.entity_count() >= 4);

    e.handle(CommandEvent::Start(CommandId::DimDiameter));
    e.handle(CommandEvent::Pick(Vec2::new(30.0, 20.0)));
    e.handle(CommandEvent::Pick(Vec2::new(34.0, 20.0)));
    assert!(e.session.document.entity_count() >= 5);

    e.handle(CommandEvent::Start(CommandId::DimAngular));
    e.handle(CommandEvent::Pick(Vec2::new(40.0, 0.0)));
    e.handle(CommandEvent::Pick(Vec2::new(45.0, 0.0)));
    e.handle(CommandEvent::Pick(Vec2::new(40.0, 5.0)));
    e.handle(CommandEvent::Pick(Vec2::new(42.0, 2.0)));
    assert!(e.session.document.entity_count() >= 6);

    e.handle(CommandEvent::Start(CommandId::Leader));
    e.handle(CommandEvent::Pick(Vec2::new(50.0, 0.0)));
    e.handle(CommandEvent::Pick(Vec2::new(55.0, 3.0)));
    e.handle(CommandEvent::Enter);
    e.handle(CommandEvent::Text("note".into()));
    assert!(e.session.document.entity_count() >= 7);

    e.handle(CommandEvent::Start(CommandId::Table));
    e.handle(CommandEvent::Text("2".into()));
    e.handle(CommandEvent::Text("3".into()));
    e.handle(CommandEvent::Pick(Vec2::new(60.0, 0.0)));
    e.handle(CommandEvent::Pick(Vec2::new(70.0, 6.0)));
    assert!(e.session.document.entity_count() >= 8);

    // circle for centermark
    e.handle(CommandEvent::Start(CommandId::Circle));
    e.handle(CommandEvent::Pick(Vec2::new(80.0, 80.0)));
    e.handle(CommandEvent::Pick(Vec2::new(82.0, 80.0)));
    e.handle(CommandEvent::Start(CommandId::Centermark));
    e.handle(CommandEvent::Pick(Vec2::new(80.0, 80.0)));

    e.handle(CommandEvent::Start(CommandId::Centerline));
    e.handle(CommandEvent::Pick(Vec2::new(90.0, 0.0)));
    e.handle(CommandEvent::Pick(Vec2::new(95.0, 0.0)));

    e.handle(CommandEvent::Start(CommandId::Section));
    e.handle(CommandEvent::Pick(Vec2::new(0.0, 100.0)));
    e.handle(CommandEvent::Pick(Vec2::new(10.0, 100.0)));

    e.handle(CommandEvent::Start(CommandId::DimContinue));
    e.handle(CommandEvent::Cancel);
    e.handle(CommandEvent::Start(CommandId::DimBaseline));
    e.handle(CommandEvent::Cancel);
}

#[test]
fn construction_parallel_midline_tangents() {
    let mut e = eng();
    seed_line(&mut e, Vec2::ZERO, Vec2::new(10.0, 0.0));

    e.handle(CommandEvent::Start(CommandId::Parallel));
    e.handle(CommandEvent::Pick(Vec2::new(5.0, 0.0)));
    e.handle(CommandEvent::Pick(Vec2::new(5.0, 3.0)));
    assert!(e.session.document.entity_count() >= 2);

    e.handle(CommandEvent::Start(CommandId::Midline));
    e.handle(CommandEvent::Pick(Vec2::new(0.0, 10.0)));
    e.handle(CommandEvent::Pick(Vec2::new(10.0, 12.0)));
    assert!(e.session.document.entity_count() >= 3);

    e.handle(CommandEvent::Start(CommandId::Circle));
    e.handle(CommandEvent::Pick(Vec2::new(20.0, 20.0)));
    e.handle(CommandEvent::Pick(Vec2::new(22.0, 20.0)));

    e.handle(CommandEvent::Start(CommandId::TangentPc));
    e.handle(CommandEvent::Pick(Vec2::new(10.0, 20.0)));
    e.handle(CommandEvent::Pick(Vec2::new(20.0, 20.0)));

    e.handle(CommandEvent::Start(CommandId::Circle));
    e.handle(CommandEvent::Pick(Vec2::new(40.0, 20.0)));
    e.handle(CommandEvent::Pick(Vec2::new(42.0, 20.0)));
    e.handle(CommandEvent::Start(CommandId::TangentCc));
    e.handle(CommandEvent::Pick(Vec2::new(20.0, 20.0)));
    e.handle(CommandEvent::Pick(Vec2::new(40.0, 20.0)));
}

#[test]
fn blocks_insert_explode_and_xref_pdf_prompts() {
    let mut e = eng();
    seed_line(&mut e, Vec2::ZERO, Vec2::new(3.0, 0.0));
    e.selection.clear();
    e.handle(CommandEvent::Start(CommandId::Block));
    e.handle(CommandEvent::Pick(Vec2::new(1.0, 0.0)));
    e.handle(CommandEvent::Enter);
    e.handle(CommandEvent::Text("MyBlk".into()));
    e.handle(CommandEvent::Pick(Vec2::ZERO));

    e.handle(CommandEvent::Start(CommandId::Insert));
    e.handle(CommandEvent::Text("MyBlk".into()));
    e.handle(CommandEvent::Pick(Vec2::new(10.0, 10.0)));

    e.handle(CommandEvent::Start(CommandId::Xref));
    e.handle(CommandEvent::Text("ref1".into()));
    e.handle(CommandEvent::Text("/tmp/a.dxf".into()));
    e.handle(CommandEvent::Pick(Vec2::new(0.0, 50.0)));
    e.handle(CommandEvent::Cancel);

    e.handle(CommandEvent::Start(CommandId::PdfUnderlay));
    e.handle(CommandEvent::Text("pdf1".into()));
    e.handle(CommandEvent::Text("/tmp/a.pdf".into()));
    e.handle(CommandEvent::Cancel);
}

#[test]
fn undo_redo_cancel_and_aliases() {
    let mut e = eng();
    e.handle(CommandEvent::Text("L".into()));
    e.handle(CommandEvent::Pick(Vec2::ZERO));
    e.handle(CommandEvent::Pick(Vec2::new(1.0, 0.0)));
    e.handle(CommandEvent::Enter);
    assert_eq!(e.session.document.entity_count(), 1);
    e.handle(CommandEvent::Start(CommandId::Undo));
    assert_eq!(e.session.document.entity_count(), 0);
    e.handle(CommandEvent::Start(CommandId::Redo));
    assert_eq!(e.session.document.entity_count(), 1);
    e.handle(CommandEvent::Start(CommandId::Line));
    e.handle(CommandEvent::Cancel);
    assert!(e.active_command().is_none());
}
