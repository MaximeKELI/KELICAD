//! kelicad-cli — batch convert, validate, plugins, macros, vectorize.

use std::fs;
use std::path::PathBuf;

use clap::{Parser, Subcommand};
use kelicad_doc::{EntityId, EntityRecord};
use kelicad_io::{export_dxf, import_dxf};
use kelicad_ops::{DocSession, Operation};
use kelicad_plugin::{
    macro_from_json, macro_to_json, record_macro, replay_macro, Manifest, PluginHost, ScriptGuest,
};
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
    }
}
