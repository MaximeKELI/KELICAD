//! Plugin host: permission-checked hostcalls over a DocSession.

use kelicad_doc::{EntityId, EntityKind, EntityRecord, Geometry, LayerId};
use kelicad_geom::Line;
use kelicad_math::Vec2;
use kelicad_ops::{DocSession, Operation};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::error::PluginError;
use crate::guest::Guest;
use crate::manifest::Manifest;

/// Capability-gated plugin host.
pub struct PluginHost {
    /// Installed manifest.
    pub manifest: Manifest,
    /// Drawing session.
    pub session: DocSession,
    hostcalls_used: u32,
}

impl PluginHost {
    /// Install a plugin against a session (takes ownership of session for the run).
    pub fn install(manifest: Manifest, session: DocSession) -> Self {
        Self {
            manifest,
            session,
            hostcalls_used: 0,
        }
    }

    /// Run a guest to completion.
    pub fn run_guest(&mut self, guest: &dyn Guest) -> Result<(), PluginError> {
        self.hostcalls_used = 0;
        guest.run(self)
    }

    /// Dispatch a hostcall by name.
    pub fn hostcall(&mut self, method: &str, args: &Value) -> Result<Value, PluginError> {
        self.hostcalls_used += 1;
        if self.hostcalls_used > self.manifest.max_hostcalls {
            return Err(PluginError::BudgetExceeded);
        }
        match method {
            "ping" => Ok(json!({ "ok": true, "plugin": self.manifest.id })),
            "summarize_document" => self.summarize(),
            "list_entities" => self.list_entities(),
            "create_line" => self.create_line(args),
            "http_get" => self.http_get(args),
            other => Err(PluginError::UnknownHostcall(other.into())),
        }
    }

    fn require_read(&self) -> Result<(), PluginError> {
        if self.manifest.permissions.doc_read {
            Ok(())
        } else {
            Err(PluginError::Denied("doc_read required".into()))
        }
    }

    fn summarize(&self) -> Result<Value, PluginError> {
        self.require_read()?;
        Ok(json!({
            "entity_count": self.session.document.entity_count(),
            "layer_count": self.session.document.layers().count(),
        }))
    }

    fn list_entities(&self) -> Result<Value, PluginError> {
        self.require_read()?;
        let entities: Vec<Value> = self
            .session
            .document
            .entities()
            .map(|e| {
                json!({
                    "id": e.id.to_string(),
                    "kind": format!("{:?}", e.kind).to_ascii_lowercase(),
                    "layer_id": e.layer_id.to_string(),
                })
            })
            .collect();
        Ok(json!({ "entities": entities }))
    }

    fn create_line(&mut self, args: &Value) -> Result<Value, PluginError> {
        let layer_name = args.get("layer").and_then(|v| v.as_str()).unwrap_or("0");
        if !self.manifest.permissions.can_write_layer(layer_name) {
            return Err(PluginError::Denied(format!(
                "doc_write_layers does not include '{layer_name}'"
            )));
        }
        let start = point2(args, "start")?;
        let end = point2(args, "end")?;
        let layer_id = self
            .session
            .document
            .layers()
            .find(|l| l.name.eq_ignore_ascii_case(layer_name))
            .map(|l| l.id)
            .unwrap_or_else(|| LayerId::from_uuid(Uuid::nil()));
        let id = EntityId::from_uuid(Uuid::now_v7());
        let geom = Geometry::Line(Line::new(start, end));
        self.session
            .apply(Operation::CreateEntity {
                record: EntityRecord {
                    id,
                    kind: EntityKind::Line,
                    geometry: geom,
                    layer_id,
                    props: Default::default(),
        },
            })
            .map_err(|e| PluginError::Execution(e.to_string()))?;
        Ok(json!({ "id": id.to_string() }))
    }

    fn http_get(&self, args: &Value) -> Result<Value, PluginError> {
        let url = args
            .get("url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| PluginError::InvalidArgs("url required".into()))?;
        let host = url_host(url).ok_or_else(|| PluginError::InvalidArgs("bad url".into()))?;
        if !self.manifest.permissions.can_net_host(&host) {
            return Err(PluginError::Denied(format!(
                "net_hosts does not include '{host}'"
            )));
        }
        // M5: do not perform real network I/O from the host by default —
        // permission gate is what we enforce. Return a stub body when allowed.
        Ok(json!({
            "status": 200,
            "host": host,
            "body": "",
            "note": "network I/O stubbed in M5; permission check passed",
        }))
    }
}

fn point2(args: &Value, key: &str) -> Result<Vec2, PluginError> {
    let arr = args
        .get(key)
        .and_then(|v| v.as_array())
        .ok_or_else(|| PluginError::InvalidArgs(format!("{key} [x,y] required")))?;
    if arr.len() != 2 {
        return Err(PluginError::InvalidArgs(format!("{key} needs 2 numbers")));
    }
    Ok(Vec2::new(
        arr[0]
            .as_f64()
            .ok_or_else(|| PluginError::InvalidArgs("x".into()))?,
        arr[1]
            .as_f64()
            .ok_or_else(|| PluginError::InvalidArgs("y".into()))?,
    ))
}

fn url_host(url: &str) -> Option<String> {
    let rest = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))?;
    let host = rest.split('/').next()?.split('@').next_back()?;
    let host = host.split(':').next()?.to_ascii_lowercase();
    if host.is_empty() {
        None
    } else {
        Some(host)
    }
}
