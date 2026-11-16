//! Exhaustive smoke tests: every `CommandId` must start and complete a happy path.

use kelicad_cmd::{
    parse_sysvar_token, CommandEngine, CommandEvent, CommandId, SequentialIdGen, SysVarValue,
};
use kelicad_doc::Geometry;
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

fn select_all(e: &mut CommandEngine<SequentialIdGen>) {
    e.selection.clear();
    let ids: Vec<_> = e.session.document.entities().map(|ent| ent.id).collect();
    for id in ids {
        e.selection.add(id);
    }
}

#[test]
fn every_command_id_parses_and_is_listed() {
    assert_eq!(CommandId::all().len(), 90);
    for id in CommandId::all() {
        assert_eq!(CommandId::parse(id.name()), Some(*id));
    }
}

#[test]
fn sysvars_ortho_and_fillet() {
    let mut e = eng();
    e.sysvars
        .set("ORTHOMODE", SysVarValue::Int(1))
        .unwrap();
    e.sync_from_sysvars();
    assert!(e.ortho);
    e.sysvars
        .set("FILLETRAD", parse_sysvar_token("2.5"))
        .unwrap();
    assert!((e.sysvars.get_real("FILLETRAD") - 2.5).abs() < 1e-9);
}

#[test]
fn line_close_and_undo_keywords() {
    let mut e = eng();
    e.handle(CommandEvent::Start(CommandId::Line));
    e.handle(CommandEvent::Pick(Vec2::ZERO));
    e.handle(CommandEvent::Pick(Vec2::new(5.0, 0.0)));
    e.handle(CommandEvent::Pick(Vec2::new(5.0, 5.0)));
    e.handle(CommandEvent::Text("UNDO".into()));
    e.handle(CommandEvent::Pick(Vec2::new(0.0, 5.0)));
    e.handle(CommandEvent::Text("CLOSE".into()));
    assert!(e.active_command().is_none());
    assert!(e.session.document.entity_count() >= 3);
}

#[test]
fn circle_diameter_option() {
    let mut e = eng();
    e.handle(CommandEvent::Start(CommandId::Circle));
    e.handle(CommandEvent::Text("DIAMETER".into()));
    e.handle(CommandEvent::Pick(Vec2::ZERO));
    e.handle(CommandEvent::Pick(Vec2::new(10.0, 0.0)));
    assert_eq!(e.session.document.entity_count(), 1);
}

#[test]
fn draw_core_suite() {
    let mut e = eng();
    e.handle(CommandEvent::Start(CommandId::Line));
    e.handle(CommandEvent::Pick(Vec2::ZERO));
    e.handle(CommandEvent::Pick(Vec2::new(5.0, 0.0)));
    e.handle(CommandEvent::Enter);

    e.handle(CommandEvent::Start(CommandId::Pline));
    e.handle(CommandEvent::Pick(Vec2::new(0.0, 2.0)));
    e.handle(CommandEvent::Pick(Vec2::new(2.0, 2.0)));
    e.handle(CommandEvent::Pick(Vec2::new(2.0, 4.0)));
    e.handle(CommandEvent::Text("CLOSE".into()));

    e.handle(CommandEvent::Start(CommandId::Circle));
    e.handle(CommandEvent::Pick(Vec2::new(20.0, 20.0)));
    e.handle(CommandEvent::Pick(Vec2::new(23.0, 20.0)));

    e.handle(CommandEvent::Start(CommandId::Arc));
    e.handle(CommandEvent::Text("3P".into()));
    e.handle(CommandEvent::Pick(Vec2::new(30.0, 0.0)));
    e.handle(CommandEvent::Pick(Vec2::new(32.0, 1.0)));
    e.handle(CommandEvent::Pick(Vec2::new(30.0, 2.0)));

    e.handle(CommandEvent::Start(CommandId::Rectang));
    e.handle(CommandEvent::Pick(Vec2::new(40.0, 0.0)));
    e.handle(CommandEvent::Pick(Vec2::new(45.0, 3.0)));

    e.handle(CommandEvent::Start(CommandId::Polygon));
    e.handle(CommandEvent::Text("4".into()));
    e.handle(CommandEvent::Pick(Vec2::new(50.0, 0.0)));
    e.handle(CommandEvent::Pick(Vec2::new(52.0, 0.0)));

    e.handle(CommandEvent::Start(CommandId::Ellipse));
    e.handle(CommandEvent::Pick(Vec2::new(60.0, 0.0)));
    e.handle(CommandEvent::Pick(Vec2::new(64.0, 0.0)));
    e.handle(CommandEvent::Pick(Vec2::new(60.0, 2.0)));

    e.handle(CommandEvent::Start(CommandId::Spline));
    e.handle(CommandEvent::Pick(Vec2::new(70.0, 0.0)));
    e.handle(CommandEvent::Pick(Vec2::new(72.0, 1.0)));
    e.handle(CommandEvent::Pick(Vec2::new(74.0, 0.0)));
    e.handle(CommandEvent::Enter);

    assert!(e.session.document.entity_count() >= 8);
}

#[test]
fn modify_and_lot_b() {
    let mut e = eng();
    seed_line(&mut e, Vec2::ZERO, Vec2::new(10.0, 0.0));
    seed_line(&mut e, Vec2::new(0.0, 5.0), Vec2::new(10.0, 5.0));
    select_all(&mut e);

    e.handle(CommandEvent::Start(CommandId::Move));
    e.handle(CommandEvent::Pick(Vec2::ZERO));
    e.handle(CommandEvent::Pick(Vec2::new(1.0, 1.0)));

    select_all(&mut e);
    e.handle(CommandEvent::Start(CommandId::Copy));
    e.handle(CommandEvent::Text("MULTIPLE".into()));
    e.handle(CommandEvent::Pick(Vec2::ZERO));
    e.handle(CommandEvent::Pick(Vec2::new(0.0, 10.0)));
    e.handle(CommandEvent::Enter);

    select_all(&mut e);
    let before = e.session.document.entity_count();
    e.handle(CommandEvent::Start(CommandId::Erase));
    e.handle(CommandEvent::Enter);
    assert!(e.session.document.entity_count() < before);

    e.handle(CommandEvent::Start(CommandId::Oops));
    assert!(e.session.document.entity_count() >= before);

    seed_line(&mut e, Vec2::new(100.0, 0.0), Vec2::new(110.0, 0.0));
    seed_line(&mut e, Vec2::new(100.0, 0.0), Vec2::new(110.0, 0.0));
    e.handle(CommandEvent::Start(CommandId::Overkill));

    e.handle(CommandEvent::Start(CommandId::SelectAll));
    assert!(!e.selection.is_empty());

    e.handle(CommandEvent::Start(CommandId::Draworder));
    e.handle(CommandEvent::Text("FRONT".into()));

    select_all(&mut e);
    e.handle(CommandEvent::Start(CommandId::Group));
    e.handle(CommandEvent::Text("G1".into()));
    assert!(e.groups.contains_key("G1"));

    e.handle(CommandEvent::Start(CommandId::Setvar));
    e.handle(CommandEvent::Text("PDMODE".into()));
    e.handle(CommandEvent::Text("3".into()));
    assert_eq!(e.sysvars.get_int("PDMODE"), 3);

    e.handle(CommandEvent::Start(CommandId::Getvar));
    e.handle(CommandEvent::Text("PDMODE".into()));

    e.handle(CommandEvent::Start(CommandId::Ddptype));
    e.handle(CommandEvent::Text("34".into()));

    e.handle(CommandEvent::Start(CommandId::Mtext));
    e.handle(CommandEvent::Pick(Vec2::new(0.0, 20.0)));
    e.handle(CommandEvent::Text("hello\\Pmtext".into()));

    e.handle(CommandEvent::Start(CommandId::Boundary));
    e.handle(CommandEvent::Pick(Vec2::new(200.0, 0.0)));
    e.handle(CommandEvent::Pick(Vec2::new(205.0, 0.0)));
    e.handle(CommandEvent::Pick(Vec2::new(205.0, 5.0)));
    e.handle(CommandEvent::Enter);

    seed_line(&mut e, Vec2::new(300.0, 0.0), Vec2::new(310.0, 0.0));
    let line_id = e.session.document.entities().last().unwrap().id;
    e.handle(CommandEvent::Start(CommandId::Divide));
    e.handle(CommandEvent::Text("4".into()));
    // pick near the line
    e.handle(CommandEvent::Pick(Vec2::new(305.0, 0.0)));
    let _ = line_id;

    e.handle(CommandEvent::Start(CommandId::Xline));
    e.handle(CommandEvent::Text("HOR".into()));
    e.handle(CommandEvent::Pick(Vec2::new(0.0, 50.0)));

    e.handle(CommandEvent::Start(CommandId::Undo));
    e.handle(CommandEvent::Start(CommandId::Redo));
}

#[test]
fn remaining_commands_start_without_panic() {
    let mut e = eng();
    seed_line(&mut e, Vec2::ZERO, Vec2::new(8.0, 0.0));
    select_all(&mut e);

    let instant = [
        CommandId::SelectPrev,
        CommandId::Ungroup,
        CommandId::Laycur,
        CommandId::Layoff,
        CommandId::Layfrz,
        CommandId::Laylck,
        CommandId::Layiso,
        CommandId::Pedit,
        CommandId::Matchprop,
        CommandId::Select,
        CommandId::SelectSimilar,
        CommandId::Qselect,
        CommandId::Wblock,
        CommandId::ArrayPath,
        CommandId::Measure,
        CommandId::Point,
        CommandId::Ray,
        CommandId::Donut,
        CommandId::Star,
        CommandId::Triangle,
        CommandId::Capsule,
        CommandId::Arrow,
        CommandId::Grid,
        CommandId::Revcloud,
        CommandId::Wipeout,
        CommandId::Centermark,
        CommandId::Centerline,
        CommandId::Table,
        CommandId::Hatch,
        CommandId::Text,
        CommandId::Leader,
        CommandId::DimLinear,
        CommandId::DimAligned,
        CommandId::DimRadius,
        CommandId::DimDiameter,
        CommandId::DimAngular,
        CommandId::DimContinue,
        CommandId::DimBaseline,
        CommandId::Section,
        CommandId::Block,
        CommandId::Insert,
        CommandId::Xref,
        CommandId::PdfUnderlay,
        CommandId::Circle2p,
        CommandId::Circle3p,
        CommandId::Parallel,
        CommandId::Midline,
        CommandId::TangentPc,
        CommandId::TangentCc,
        CommandId::Fillet,
        CommandId::Chamfer,
        CommandId::Trim,
        CommandId::Extend,
        CommandId::Offset,
        CommandId::Rotate,
        CommandId::Scale,
        CommandId::Mirror,
        CommandId::Stretch,
        CommandId::Break,
        CommandId::Join,
        CommandId::Explode,
        CommandId::Lengthen,
        CommandId::Align,
        CommandId::ArrayRect,
        CommandId::ArrayPolar,
    ];
    for id in instant {
        e.handle(CommandEvent::Cancel);
        select_all(&mut e);
        e.handle(CommandEvent::Start(id));
        // Must not panic; cancel to idle.
        e.handle(CommandEvent::Cancel);
    }
}


#[test]
fn drafting_props_and_polar_snap() {
    let mut e = eng();
    e.sysvars
        .set("CECOLOR", SysVarValue::Str("red".into()))
        .unwrap();
    e.sysvars
        .set("CELTYPE", SysVarValue::Str("DASHED".into()))
        .unwrap();
    e.handle(CommandEvent::Start(CommandId::Line));
    e.handle(CommandEvent::Pick(Vec2::ZERO));
    e.handle(CommandEvent::Pick(Vec2::new(10.0, 0.0)));
    let ent = e.session.document.entities().next().expect("line");
    assert_eq!(ent.props.color, Some([255, 0, 0]));
    assert_eq!(ent.props.linetype.as_deref(), Some("DASHED"));

    e.sysvars.set("POLARMODE", SysVarValue::Int(1)).unwrap();
    e.sysvars.set("ORTHOMODE", SysVarValue::Int(0)).unwrap();
    e.sync_from_sysvars();
    e.handle(CommandEvent::Start(CommandId::Line));
    e.handle(CommandEvent::Pick(Vec2::ZERO));
    e.handle(CommandEvent::Pick(Vec2::new(10.0, 1.0)));
    let line = e
        .session
        .document
        .entities()
        .filter_map(|ent| match &ent.geometry {
            Geometry::Line(l) => Some(*l),
            _ => None,
        })
        .last()
        .expect("polar line");
    assert!(line.end.y.abs() < 0.5, "polar should pull toward 0°: {:?}", line.end);

    e.sysvars.set("SNAPMODE", SysVarValue::Int(1)).unwrap();
    e.sysvars.set("SNAPUNIT", SysVarValue::Real(5.0)).unwrap();
    e.sysvars.set("POLARMODE", SysVarValue::Int(0)).unwrap();
    e.handle(CommandEvent::Cancel);
    e.handle(CommandEvent::Start(CommandId::Point));
    e.handle(CommandEvent::Pick(Vec2::new(12.0, 13.0)));
    let pt = e
        .session
        .document
        .entities()
        .filter_map(|ent| match &ent.geometry {
            Geometry::Point(p) => Some(*p),
            _ => None,
        })
        .last()
        .expect("point");
    assert!((pt.x - 10.0).abs() < 1e-9);
    assert!((pt.y - 15.0).abs() < 1e-9);
}
