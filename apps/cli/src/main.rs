//! kelicad-cli — batch convert, validate, plugins, macros, vectorize, solid/BIM.

use std::fs;
use std::path::PathBuf;

use clap::{Parser, Subcommand};
use kelicad_bim::{
    export_ifc, import_ifc, schedule_csv, slab_from_rect, wall_from_baseline, BimElement,
};
use kelicad_doc::{
    template_entities, EntityId, EntityRecord, Layer, LayerId, TemplateKind,
};
use kelicad_dwg::{export_dwg, import_dwg, MockDwgBridge};
use kelicad_geom::{builtin_patterns, generate_hatch, load_pat_file, resolve_pattern};
use kelicad_io::{export_dxf, import_dxf, import_shp, load_document, save_document};
use kelicad_math::{Vec2, Vec3};
use kelicad_ops::{DocSession, Operation};
use kelicad_plugin::{
    macro_from_json, macro_to_json, record_macro, replay_macro, verify_catalog, verify_entry,
    Manifest, MarketCatalog, PluginHost, ScriptGuest,
};
use kelicad_solid::{derive_view, BuiltinSolidKernel, Profile2, SolidBody, SolidKernel, ViewKind};
use kelicad_vectorize::vectorize_bytes;
use uuid::Uuid;

#[derive(Parser)]
#[command(name = "kelicad-cli", about = "KELICAD batch / plugin CLI")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Convert DXF to snapshot JSON (stdout) or reverse with --from-json.
    Convert {
        /// Input path (.dxf or .json).
        input: PathBuf,
        /// Write output path (default stdout for json).
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// Treat input as snapshot JSON → DXF.
        #[arg(long)]
        from_json: bool,
    },
    /// Validate a DXF file parses.
    Validate {
        /// File to validate.
        input: PathBuf,
    },
    /// Plugin subcommands.
    Plugin {
        #[command(subcommand)]
        cmd: PluginCmd,
    },
    /// Replay a recorded macro JSON onto an empty drawing; print entity count.
    Macro {
        #[command(subcommand)]
        cmd: MacroCmd,
    },
    /// Vectorize SVG or PNG into snapshot JSON entities.
    Vectorize {
        /// Input .svg or .png.
        input: PathBuf,
        /// Output snapshot JSON path (stdout if omitted).
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Solid kernel demos.
    Solid {
        #[command(subcommand)]
        cmd: SolidCmd,
    },
    /// BIM schedule demo (wall + slab).
    Bim {
        #[command(subcommand)]
        cmd: BimCmd,
    },
    /// IFC subset export/import.
    Ifc {
        #[command(subcommand)]
        cmd: IfcCmd,
    },
    /// DWG import/export via Mock (or External) bridge.
    Dwg {
        #[command(subcommand)]
        cmd: DwgCmd,
    },
    /// Signed plugin marketplace catalog.
    Market {
        #[command(subcommand)]
        cmd: MarketCmd,
    },
    /// Import ESRI Shapefile (.shp).
    Shp {
        /// Input .shp path.
        input: PathBuf,
        /// Optional DXF output.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Hatch pattern tools (.pat).
    Hatch {
        #[command(subcommand)]
        cmd: HatchCmd,
    },
    /// Native `.kelicad` document ops (open / save / new / template).
    Doc {
        #[command(subcommand)]
        cmd: DocCmd,
    },
}

#[derive(Subcommand)]
enum DocCmd {
    /// Create a new empty `.kelicad` (optional template).
    New {
        /// Output path.
        #[arg(short, long)]
        output: PathBuf,
        /// Template: blank|a0|a1|a2|a3|a4|mech|arch.
        #[arg(long, default_value = "blank")]
        template: String,
    },
    /// Open a `.kelicad` and print summary.
    Open {
        /// Input `.kelicad`.
        input: PathBuf,
    },
    /// Save a DXF into `.kelicad` (or copy/round-trip `.kelicad`).
    Save {
        /// Input `.kelicad` or `.dxf`.
        input: PathBuf,
        /// Output `.kelicad`.
        #[arg(short, long)]
        output: PathBuf,
    },
    /// Apply a template onto an existing `.kelicad` (or create).
    Template {
        /// Template name.
        name: String,
        /// Output path.
        #[arg(short, long)]
        output: PathBuf,
    },
}

#[derive(Subcommand)]
enum HatchCmd {
    /// List built-in pattern names.
    List,
    /// List patterns from a .pat file.
    Parse {
        /// Path to .pat file.
        input: PathBuf,
    },
    /// Generate hatch segment count for a unit square with a pattern.
    Preview {
        /// Pattern name (e.g. ANSI31).
        name: String,
        /// Optional .pat file (else built-ins).
        #[arg(long)]
        pat: Option<PathBuf>,
        /// Scale.
        #[arg(long, default_value_t = 1.0)]
        scale: f64,
    },
}

#[derive(Subcommand)]
enum DwgCmd {
    /// Import proxy/DWG bytes → print entity count (Mock bridge).
    Import {
        /// Input .dwg / .kdwg proxy file.
        input: PathBuf,
    },
    /// Export DXF file wrapped as mock DWG proxy.
    Export {
        /// Input .dxf.
        input: PathBuf,
        /// Output path.
        #[arg(short, long)]
        output: PathBuf,
    },
}

#[derive(Subcommand)]
enum MarketCmd {
    /// List catalog entries.
    List {
        #[arg(long, default_value = "sdk/marketplace/catalog.json")]
        catalog: PathBuf,
    },
    /// Verify catalog signatures.
    Verify {
        #[arg(long, default_value = "sdk/marketplace/catalog.json")]
        catalog: PathBuf,
    },
    /// Install a verified plugin entry by copying its path to --dir.
    Install {
        /// Plugin id.
        id: String,
        #[arg(long, default_value = "sdk/marketplace/catalog.json")]
        catalog: PathBuf,
        #[arg(long, default_value = ".")]
        dir: PathBuf,
    },
}

#[derive(Subcommand)]
enum SolidCmd {
    /// Extrude a W×H rectangle to height; print AABB and optional plan view count.
    Extrude {
        /// Rectangle width.
        #[arg(long, default_value_t = 4.0)]
        width: f64,
        /// Rectangle height (Y).
        #[arg(long, default_value_t = 3.0)]
        depth: f64,
        /// Extrusion height (Z).
        #[arg(long, default_value_t = 2.5)]
        height: f64,
    },
    /// Section a box at mid Z; print geometry count.
    Section {
        #[arg(long, default_value_t = 4.0)]
        width: f64,
        #[arg(long, default_value_t = 3.0)]
        depth: f64,
        #[arg(long, default_value_t = 2.0)]
        height: f64,
    },
    /// UV sphere.
    Sphere {
        #[arg(long, default_value_t = 1.0)]
        radius: f64,
    },
    /// Cone along +Z.
    Cone {
        #[arg(long, default_value_t = 1.0)]
        radius: f64,
        #[arg(long, default_value_t = 2.0)]
        height: f64,
    },
    /// Loft square → smaller square.
    Loft {
        #[arg(long, default_value_t = 3.0)]
        height: f64,
    },
    /// Sweep a square profile along an L-path.
    Sweep {
        #[arg(long, default_value_t = 0.1)]
        thickness: f64,
    },
    /// Torus (major / minor).
    Torus {
        #[arg(long, default_value_t = 2.0)]
        major: f64,
        #[arg(long, default_value_t = 0.4)]
        minor: f64,
    },
    /// Cylinder along +Z.
    Cylinder {
        #[arg(long, default_value_t = 1.0)]
        radius: f64,
        #[arg(long, default_value_t = 3.0)]
        height: f64,
    },
    /// Regular pyramid.
    Pyramid {
        #[arg(long, default_value_t = 1.0)]
        radius: f64,
        #[arg(long, default_value_t = 2.0)]
        height: f64,
        #[arg(long, default_value_t = 4)]
        sides: usize,
    },
    /// Regular prism.
    Prism {
        #[arg(long, default_value_t = 1.0)]
        radius: f64,
        #[arg(long, default_value_t = 2.0)]
        height: f64,
        #[arg(long, default_value_t = 6)]
        sides: usize,
    },
    /// Hollow pipe.
    Pipe {
        #[arg(long, default_value_t = 1.0)]
        outer: f64,
        #[arg(long, default_value_t = 0.6)]
        inner: f64,
        #[arg(long, default_value_t = 3.0)]
        height: f64,
    },
    /// Revolve a rectangle profile around +Y.
    Revolve {
        #[arg(long, default_value_t = 360.0)]
        degrees: f64,
    },
}

#[derive(Subcommand)]
enum BimCmd {
    /// Print CSV schedule for a demo wall + slab.
    Schedule,
}

#[derive(Subcommand)]
enum IfcCmd {
    /// Export demo wall+slab IFC to a file (or stdout).
    Export {
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Import IFC subset and print element count.
    Import { input: PathBuf },
}

#[derive(Subcommand)]
enum PluginCmd {
    /// Scaffold a new plugin directory.
    New {
        /// Plugin id / directory name.
        name: String,
        /// Parent directory (default .).
        #[arg(short, long, default_value = ".")]
        dir: PathBuf,
    },
    /// Run a ScriptGuest against an empty drawing (or --dxf).
    Run {
        /// Path to plugin.json.
        #[arg(long)]
        manifest: PathBuf,
        /// Path to script.json hostcall list.
        #[arg(long)]
        script: PathBuf,
        /// Optional DXF to load first.
        #[arg(long)]
        dxf: Option<PathBuf>,
    },
}

#[derive(Subcommand)]
enum MacroCmd {
    /// Replay macro JSON file.
    Replay {
        /// Macro script path.
        input: PathBuf,
    },
    /// Record empty (demo) — use after plugin run in tests.
    Record {
        /// Output path.
        output: PathBuf,
    },
}

fn main() {
    let cli = Cli::parse();
    if let Err(e) = run(cli) {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

fn run(cli: Cli) -> Result<(), String> {
    match cli.command {
        Commands::Convert {
            input,
            output,
            from_json,
        } => {
            if from_json {
                return Err(
                    "JSON→DXF via snapshot import is M5-lite: use DXF round-trip from DXF input"
                        .into(),
                );
            }
            let text = fs::read_to_string(&input).map_err(|e| e.to_string())?;
            let (session, report) = import_dxf(&text).map_err(|e| e.to_string())?;
            let dxf = export_dxf(&session).map_err(|e| e.to_string())?;
            if let Some(out) = output {
                fs::write(out, dxf).map_err(|e| e.to_string())?;
            } else {
                print!("{dxf}");
            }
            eprintln!(
                "ok: imported {} skipped {}",
                report.imported,
                report.skipped.len()
            );
            Ok(())
        }
        Commands::Validate { input } => {
            let text = fs::read_to_string(&input).map_err(|e| e.to_string())?;
            if input.extension().and_then(|e| e.to_str()) == Some("dxf") {
                let (_s, report) = import_dxf(&text).map_err(|e| e.to_string())?;
                println!(
                    "valid dxf: imported={} skipped={}",
                    report.imported,
                    report.skipped.len()
                );
            } else {
                // Try macro or manifest JSON.
                let _: serde_json::Value =
                    serde_json::from_str(&text).map_err(|e| e.to_string())?;
                println!("valid json");
            }
            Ok(())
        }
        Commands::Plugin { cmd } => match cmd {
            PluginCmd::New { name, dir } => {
                let root = dir.join(&name);
                fs::create_dir_all(&root).map_err(|e| e.to_string())?;
                let manifest = Manifest {
                    id: name.clone(),
                    version: "0.1.0".into(),
                    name: name.clone(),
                    permissions: kelicad_plugin::Permissions {
                        doc_read: true,
                        doc_write_layers: vec!["0".into()],
                        net_hosts: vec![],
                    },
                    max_hostcalls: 32,
                };
                fs::write(
                    root.join("plugin.json"),
                    serde_json::to_string_pretty(&manifest).unwrap(),
                )
                .map_err(|e| e.to_string())?;
                fs::write(
                    root.join("script.json"),
                    serde_json::to_string_pretty(&serde_json::json!([
                        {"method": "ping", "args": {}}
                    ]))
                    .unwrap(),
                )
                .map_err(|e| e.to_string())?;
                fs::write(
                    root.join("README.md"),
                    format!("# {name}\n\nGenerated by `kelicad-cli plugin new`.\n"),
                )
                .map_err(|e| e.to_string())?;
                println!("created {}", root.display());
                Ok(())
            }
            PluginCmd::Run {
                manifest,
                script,
                dxf,
            } => {
                let man_text = fs::read_to_string(&manifest).map_err(|e| e.to_string())?;
                let man: Manifest = serde_json::from_str(&man_text).map_err(|e| e.to_string())?;
                let script_text = fs::read_to_string(&script).map_err(|e| e.to_string())?;
                let script_val: serde_json::Value =
                    serde_json::from_str(&script_text).map_err(|e| e.to_string())?;
                let guest = ScriptGuest::from_json(&script_val).map_err(|e| e.to_string())?;
                let session = if let Some(dxf_path) = dxf {
                    let t = fs::read_to_string(dxf_path).map_err(|e| e.to_string())?;
                    import_dxf(&t).map_err(|e| e.to_string())?.0
                } else {
                    DocSession::new()
                };
                let mut host = PluginHost::install(man, session);
                host.run_guest(&guest).map_err(|e| e.to_string())?;
                println!(
                    "plugin ok: entities={}",
                    host.session.document.entity_count()
                );
                let mac = record_macro(&host.session);
                eprintln!("{}", macro_to_json(&mac).unwrap_or_default());
                Ok(())
            }
        },
        Commands::Doc { cmd } => match cmd {
            DocCmd::New { output, template } => {
                let mut session = DocSession::new();
                if let Some(kind) = TemplateKind::parse(&template) {
                    apply_template_to_session(&mut session, kind)?;
                }
                save_document(&output, &session).map_err(|e| e.to_string())?;
                println!(
                    "new ok: {} entities={} template={template}",
                    output.display(),
                    session.document.entity_count()
                );
                Ok(())
            }
            DocCmd::Open { input } => {
                let session = load_document(&input).map_err(|e| e.to_string())?;
                println!(
                    "open ok: entities={} layers={} blocks={}",
                    session.document.entity_count(),
                    session.document.layers().count(),
                    session.document.blocks().count()
                );
                Ok(())
            }
            DocCmd::Save { input, output } => {
                let session = if input
                    .extension()
                    .and_then(|e| e.to_str())
                    .map(|e| e.eq_ignore_ascii_case("dxf"))
                    .unwrap_or(false)
                {
                    let text = fs::read_to_string(&input).map_err(|e| e.to_string())?;
                    import_dxf(&text).map_err(|e| e.to_string())?.0
                } else {
                    load_document(&input).map_err(|e| e.to_string())?
                };
                save_document(&output, &session).map_err(|e| e.to_string())?;
                println!(
                    "save ok: {} → {} (entities={})",
                    input.display(),
                    output.display(),
                    session.document.entity_count()
                );
                Ok(())
            }
            DocCmd::Template { name, output } => {
                let kind = TemplateKind::parse(&name)
                    .ok_or_else(|| "unknown template (blank|a0|a1|a2|a3|a4|mech|arch)".to_string())?;
                let mut session = DocSession::new();
                apply_template_to_session(&mut session, kind)?;
                save_document(&output, &session).map_err(|e| e.to_string())?;
                println!(
                    "template ok: {name} → {} entities={}",
                    output.display(),
                    session.document.entity_count()
                );
                Ok(())
            }
        },
        Commands::Macro { cmd } => match cmd {
            MacroCmd::Replay { input } => {
                let text = fs::read_to_string(input).map_err(|e| e.to_string())?;
                let script = macro_from_json(&text).map_err(|e| e.to_string())?;
                let mut session = DocSession::new();
                let n = replay_macro(&mut session, &script).map_err(|e| e.to_string())?;
                println!(
                    "replayed {n} steps; entities={}",
                    session.document.entity_count()
                );
                Ok(())
            }
            MacroCmd::Record { output } => {
                let script = record_macro(&DocSession::new());
                fs::write(output, macro_to_json(&script).map_err(|e| e.to_string())?)
                    .map_err(|e| e.to_string())?;
                Ok(())
            }
        },
        Commands::Vectorize { input, output } => {
            let bytes = fs::read(&input).map_err(|e| e.to_string())?;
            let hint = input
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("input.svg");
            let geoms = vectorize_bytes(&bytes, Some(hint)).map_err(|e| e.to_string())?;
            let mut session = DocSession::new();
            let layer = session.document.current_layer;
            let mut ops = Vec::new();
            for geom in geoms {
                let id = EntityId::from_uuid(Uuid::now_v7());
                ops.push(Operation::CreateEntity {
                    record: EntityRecord {
                        id,
                        kind: geom.kind(),
                        geometry: geom,
                        layer_id: layer,
                    },
                });
            }
            let n = ops.len();
            if !ops.is_empty() {
                session
                    .apply(Operation::Batch { ops })
                    .map_err(|e| e.to_string())?;
            }
            let entities: Vec<serde_json::Value> = session
                .document
                .entities()
                .map(|e| {
                    serde_json::json!({
                        "id": e.id.to_string(),
                        "kind": format!("{:?}", e.kind).to_ascii_lowercase(),
                        "layer_id": e.layer_id.to_string(),
                        "geometry": e.geometry,
                    })
                })
                .collect();
            let snap = serde_json::json!({
                "schema_version": 1,
                "entity_count": n,
                "entities": entities,
            });
            let text = serde_json::to_string_pretty(&snap).map_err(|e| e.to_string())?;
            if let Some(out) = output {
                fs::write(out, text).map_err(|e| e.to_string())?;
            } else {
                print!("{text}");
            }
            eprintln!("vectorize ok: entities={n}");
            Ok(())
        }
        Commands::Solid { cmd } => match cmd {
            SolidCmd::Extrude {
                width,
                depth,
                height,
            } => {
                let k = BuiltinSolidKernel;
                let profile =
                    Profile2::rectangle(Vec2::ZERO, width, depth).map_err(|e| e.to_string())?;
                let body = k.extrude(&profile, height).map_err(|e| e.to_string())?;
                let plan = derive_view(&body, ViewKind::Plan, None);
                println!(
                    "extrude ok: aabb_z={:.3} triangles={} plan_geoms={}",
                    body.aabb.size().z,
                    body.mesh.triangle_count(),
                    plan.len()
                );
                Ok(())
            }
            SolidCmd::Section {
                width,
                depth,
                height,
            } => {
                let k = BuiltinSolidKernel;
                let body = k
                    .box_solid(Vec3::ZERO, Vec3::new(width, depth, height))
                    .map_err(|e| e.to_string())?;
                let geoms = k.section(&body, kelicad_solid::Plane::z(height * 0.5));
                println!("section ok: geoms={}", geoms.len());
                Ok(())
            }
            SolidCmd::Sphere { radius } => {
                let k = BuiltinSolidKernel;
                let body = k
                    .sphere(Vec3::ZERO, radius, 24, 12)
                    .map_err(|e| e.to_string())?;
                println!(
                    "sphere ok: triangles={} aabb_diag={:.3}",
                    body.mesh.triangle_count(),
                    body.aabb.size().length()
                );
                Ok(())
            }
            SolidCmd::Cone { radius, height } => {
                let k = BuiltinSolidKernel;
                let body = k
                    .cone(Vec2::ZERO, radius, height, 24)
                    .map_err(|e| e.to_string())?;
                println!(
                    "cone ok: triangles={} height={:.3}",
                    body.mesh.triangle_count(),
                    body.aabb.size().z
                );
                Ok(())
            }
            SolidCmd::Loft { height } => {
                let k = BuiltinSolidKernel;
                let a = Profile2::rectangle(Vec2::ZERO, 4.0, 4.0).map_err(|e| e.to_string())?;
                let b =
                    Profile2::rectangle(Vec2::new(1.0, 1.0), 2.0, 2.0).map_err(|e| e.to_string())?;
                let body = k.loft(&a, &b, height, 6).map_err(|e| e.to_string())?;
                println!("loft ok: triangles={}", body.mesh.triangle_count());
                Ok(())
            }
            SolidCmd::Sweep { thickness } => {
                let k = BuiltinSolidKernel;
                let profile =
                    Profile2::rectangle(Vec2::new(-0.2, -0.2), 0.4, 0.4).map_err(|e| e.to_string())?;
                let path = [
                    Vec2::ZERO,
                    Vec2::new(5.0, 0.0),
                    Vec2::new(5.0, 3.0),
                ];
                let body = k
                    .sweep(&profile, &path, thickness)
                    .map_err(|e| e.to_string())?;
                println!("sweep ok: triangles={}", body.mesh.triangle_count());
                Ok(())
            }
            SolidCmd::Torus { major, minor } => {
                let k = BuiltinSolidKernel;
                let body = k
                    .torus(Vec3::ZERO, major, minor, 32, 16)
                    .map_err(|e| e.to_string())?;
                println!("torus ok: triangles={}", body.mesh.triangle_count());
                Ok(())
            }
            SolidCmd::Cylinder { radius, height } => {
                let k = BuiltinSolidKernel;
                let body = k
                    .cylinder(Vec2::ZERO, radius, height, 32)
                    .map_err(|e| e.to_string())?;
                println!("cylinder ok: triangles={}", body.mesh.triangle_count());
                Ok(())
            }
            SolidCmd::Pyramid {
                radius,
                height,
                sides,
            } => {
                let k = BuiltinSolidKernel;
                let body = k
                    .pyramid(Vec2::ZERO, radius, height, sides)
                    .map_err(|e| e.to_string())?;
                println!("pyramid ok: triangles={}", body.mesh.triangle_count());
                Ok(())
            }
            SolidCmd::Prism {
                radius,
                height,
                sides,
            } => {
                let k = BuiltinSolidKernel;
                let body = k
                    .prism(Vec2::ZERO, radius, height, sides)
                    .map_err(|e| e.to_string())?;
                println!("prism ok: triangles={}", body.mesh.triangle_count());
                Ok(())
            }
            SolidCmd::Pipe {
                outer,
                inner,
                height,
            } => {
                let k = BuiltinSolidKernel;
                let body = k
                    .pipe(Vec2::ZERO, outer, inner, height, 32)
                    .map_err(|e| e.to_string())?;
                println!("pipe ok: triangles={}", body.mesh.triangle_count());
                Ok(())
            }
            SolidCmd::Revolve { degrees } => {
                let k = BuiltinSolidKernel;
                let profile = Profile2::rectangle(Vec2::new(1.0, 0.0), 0.4, 1.0)
                    .map_err(|e| e.to_string())?;
                let body = k
                    .revolve(&profile, degrees, 36)
                    .map_err(|e| e.to_string())?;
                println!("revolve ok: triangles={}", body.mesh.triangle_count());
                Ok(())
            }
        },
        Commands::Bim { cmd } => match cmd {
            BimCmd::Schedule => {
                let (_wb, we) = wall_from_baseline(Vec2::ZERO, Vec2::new(5.0, 0.0), 0.2, 2.7)
                    .map_err(|e| e.to_string())?;
                let (_sb, se) =
                    slab_from_rect(Vec2::ZERO, 5.0, 4.0, 0.2).map_err(|e| e.to_string())?;
                let csv = schedule_csv(&[we, se], &["material", "height", "thickness"]);
                print!("{csv}");
                Ok(())
            }
        },
        Commands::Ifc { cmd } => match cmd {
            IfcCmd::Export { output } => {
                let (wb, we) = wall_from_baseline(Vec2::ZERO, Vec2::new(4.0, 0.0), 0.2, 2.5)
                    .map_err(|e| e.to_string())?;
                let (sb, se) =
                    slab_from_rect(Vec2::ZERO, 4.0, 3.0, 0.2).map_err(|e| e.to_string())?;
                let text = export_ifc(&[we, se], &[wb, sb]).map_err(|e| e.to_string())?;
                if let Some(out) = output {
                    fs::write(out, &text).map_err(|e| e.to_string())?;
                } else {
                    print!("{text}");
                }
                eprintln!("ifc export ok");
                Ok(())
            }
            IfcCmd::Import { input } => {
                let text = fs::read_to_string(input).map_err(|e| e.to_string())?;
                let (elems, bodies) = import_ifc(&text).map_err(|e| e.to_string())?;
                println!(
                    "ifc import ok: elements={} solids={}",
                    elems.len(),
                    bodies.len()
                );
                let _: (Vec<BimElement>, Vec<SolidBody>) = (elems, bodies);
                Ok(())
            }
        },
        Commands::Dwg { cmd } => match cmd {
            DwgCmd::Import { input } => {
                let bytes = fs::read(input).map_err(|e| e.to_string())?;
                let bridge = MockDwgBridge;
                let (session, n) = import_dwg(&bytes, &bridge).map_err(|e| e.to_string())?;
                println!(
                    "dwg import ok: imported={n} entities={}",
                    session.document.entity_count()
                );
                Ok(())
            }
            DwgCmd::Export { input, output } => {
                let text = fs::read_to_string(input).map_err(|e| e.to_string())?;
                let (session, report) = import_dxf(&text).map_err(|e| e.to_string())?;
                let bridge = MockDwgBridge;
                let bytes = export_dwg(&session, &bridge).map_err(|e| e.to_string())?;
                fs::write(output, bytes).map_err(|e| e.to_string())?;
                println!(
                    "dwg export ok: entities={}",
                    report.imported.max(session.document.entity_count())
                );
                Ok(())
            }
        },
        Commands::Market { cmd } => match cmd {
            MarketCmd::List { catalog } => {
                let text = fs::read_to_string(catalog).map_err(|e| e.to_string())?;
                let cat: MarketCatalog = serde_json::from_str(&text).map_err(|e| e.to_string())?;
                for e in &cat.entries {
                    println!("{} {} — {} ({})", e.id, e.version, e.description, e.path);
                }
                Ok(())
            }
            MarketCmd::Verify { catalog } => {
                let text = fs::read_to_string(catalog).map_err(|e| e.to_string())?;
                let cat: MarketCatalog = serde_json::from_str(&text).map_err(|e| e.to_string())?;
                verify_catalog(&cat).map_err(|e| e.to_string())?;
                println!("market verify ok: {} entries", cat.entries.len());
                Ok(())
            }
            MarketCmd::Install {
                id,
                catalog,
                dir,
            } => {
                let text = fs::read_to_string(&catalog).map_err(|e| e.to_string())?;
                let cat: MarketCatalog = serde_json::from_str(&text).map_err(|e| e.to_string())?;
                let entry = cat
                    .entries
                    .iter()
                    .find(|e| e.id == id)
                    .ok_or_else(|| format!("plugin '{id}' not in catalog"))?;
                verify_entry(&cat, entry).map_err(|e| e.to_string())?;
                let dest = dir.join(&entry.id);
                copy_dir_recursive(PathBuf::from(&entry.path), dest.clone())
                    .map_err(|e| e.to_string())?;
                println!("installed {} → {}", entry.id, dest.display());
                Ok(())
            }
        },
        Commands::Shp { input, output } => {
            let (session, report) = import_shp(&input).map_err(|e| e.to_string())?;
            if let Some(out) = output {
                let dxf = export_dxf(&session).map_err(|e| e.to_string())?;
                fs::write(out, dxf).map_err(|e| e.to_string())?;
            }
            println!(
                "shp import ok: imported={} skipped={} entities={}",
                report.imported,
                report.skipped.len(),
                session.document.entity_count()
            );
            Ok(())
        }
        Commands::Hatch { cmd } => match cmd {
            HatchCmd::List => {
                for p in builtin_patterns() {
                    println!("{} — {}", p.name, p.description);
                }
                Ok(())
            }
            HatchCmd::Parse { input } => {
                let pats = load_pat_file(&input).map_err(|e| e.to_string())?;
                for p in pats {
                    println!("{} ({} lines) — {}", p.name, p.lines.len(), p.description);
                }
                Ok(())
            }
            HatchCmd::Preview { name, pat, scale } => {
                let pattern = resolve_pattern(&name, pat.as_deref()).map_err(|e| e.to_string())?;
                let boundary = vec![
                    Vec2::ZERO,
                    Vec2::new(10.0, 0.0),
                    Vec2::new(10.0, 10.0),
                    Vec2::new(0.0, 10.0),
                ];
                let segs = generate_hatch(&boundary, &pattern, scale, 0.0);
                println!(
                    "hatch preview: pattern={} scale={} segments={}",
                    pattern.name,
                    scale,
                    segs.len()
                );
                Ok(())
            }
        },
    }
}

fn apply_template_to_session(
    session: &mut DocSession,
    kind: TemplateKind,
) -> Result<(), String> {
    if matches!(kind, TemplateKind::Blank) {
        return Ok(());
    }
    let seeds = template_entities(kind);
    let mut layer_ids: std::collections::HashMap<String, LayerId> =
        std::collections::HashMap::new();
    for l in session.document.layers() {
        layer_ids.insert(l.name.clone(), l.id);
    }
    let mut ops = Vec::new();
    for seed in seeds {
        let layer_id = if let Some(id) = layer_ids.get(seed.layer) {
            *id
        } else {
            let id = LayerId::from_uuid(Uuid::now_v7());
            layer_ids.insert(seed.layer.to_string(), id);
            ops.push(Operation::CreateLayer {
                layer: Layer::new(id, seed.layer, seed.layer_color),
            });
            id
        };
        let eid = EntityId::from_uuid(Uuid::now_v7());
        ops.push(Operation::CreateEntity {
            record: EntityRecord {
                id: eid,
                kind: seed.geometry.kind(),
                geometry: seed.geometry,
                layer_id,
            },
        });
    }
    if !ops.is_empty() {
        session
            .apply(Operation::Batch { ops })
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn copy_dir_recursive(src: PathBuf, dst: PathBuf) -> std::io::Result<()> {
    fs::create_dir_all(&dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let to = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_recursive(entry.path(), to)?;
        } else {
            fs::copy(entry.path(), to)?;
        }
    }
    Ok(())
}
