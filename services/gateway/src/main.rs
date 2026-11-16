//! KELICAD collaboration gateway — rooms, WS sync, JWT RBAC, audit (M8).

mod auth;

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use axum::body::Bytes;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{FromRequestParts, Path, Query, State};
use axum::http::header::CONTENT_TYPE;
use axum::http::request::Parts;
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::routing::{delete, get, post};
use axum::{Json, Router};
use futures_util::{SinkExt, StreamExt};
use kelicad_audit::AuditLog;
use kelicad_sync::{Comment, OpEnvelope, RestorePoint, WireMessage};
use kelicad_vectorize::vectorize_bytes;
use serde::{Deserialize, Serialize};
use tokio::sync::{broadcast, Mutex};
use tower_http::cors::CorsLayer;
use uuid::Uuid;

use crate::auth::{AuthConfig, AuthMode, AuthUser, Role};

#[derive(Clone, Default)]
struct Room {
    ops: Vec<OpEnvelope>,
    comments: Vec<Comment>,
    versions: Vec<RestorePoint>,
}

struct AppState {
    rooms: Mutex<HashMap<String, Room>>,
    buses: Mutex<HashMap<String, broadcast::Sender<String>>>,
    auth: AuthConfig,
    audit: Mutex<AuditLog>,
    /// Per-subject GDPR export bags (simple in-memory store).
    subjects: Mutex<HashMap<String, serde_json::Value>>,
}

impl AppState {
    async fn audit(
        &self,
        user: &AuthUser,
        action: &str,
        resource: &str,
        detail: Option<serde_json::Value>,
    ) {
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let mut log = self.audit.lock().await;
        log.append(&user.sub, action, resource, ts, detail);
    }
}

/// Extractor that loads AuthUser using AppState.auth.
struct GatewayUser(AuthUser);

impl FromRequestParts<Arc<AppState>> for GatewayUser {
    type Rejection = StatusCode;

    fn from_request_parts<'life0, 'life1, 'async_trait>(
        parts: &'life0 mut Parts,
        state: &'life1 Arc<AppState>,
    ) -> core::pin::Pin<
        Box<dyn core::future::Future<Output = Result<Self, Self::Rejection>> + Send + 'async_trait>,
    >
    where
        'life0: 'async_trait,
        'life1: 'async_trait,
        Self: 'async_trait,
    {
        Box::pin(async move {
            let cfg = &state.auth;
            if cfg.mode == AuthMode::Off {
                return Ok(GatewayUser(AuthUser {
                    sub: "anonymous".into(),
                    org: "local".into(),
                    role: Role::Editor,
                }));
            }
            let auth = parts
                .headers
                .get(axum::http::header::AUTHORIZATION)
                .and_then(|v| v.to_str().ok())
                .ok_or(StatusCode::UNAUTHORIZED)?;
            let token = auth
                .strip_prefix("Bearer ")
                .ok_or(StatusCode::UNAUTHORIZED)?;
            Ok(GatewayUser(cfg.decode_token(token)?))
        })
    }
}

#[derive(Serialize)]
struct CreateRoomResponse {
    room_id: String,
    ws_url: String,
}

#[tokio::main]
async fn main() {
    let auth = AuthConfig::from_env();
    eprintln!(
        "kelicad-gateway auth_mode={:?} issuer={}",
        auth.mode, auth.issuer
    );
    let state = Arc::new(AppState {
        rooms: Mutex::new(HashMap::new()),
        buses: Mutex::new(HashMap::new()),
        auth,
        audit: Mutex::new(AuditLog::new()),
        subjects: Mutex::new(HashMap::new()),
    });

    let app = Router::new()
        .route("/health", get(|| async { "ok" }))
        .route(
            "/v1/health",
            get(|| async { Json(serde_json::json!({"ok": true})) }),
        )
        .route("/v1/auth/dev-token", post(dev_token))
        .route("/rooms", post(create_room))
        .route("/v1/rooms", post(create_room))
        .route("/rooms/:id/ws", get(ws_handler))
        .route("/v1/rooms/:id/snapshot", get(get_snapshot))
        .route("/v1/rooms/:id/ops", post(post_ops))
        .route("/v1/vectorize", post(vectorize))
        .route("/v1/audit", get(get_audit))
        .route("/v1/me/export", get(me_export))
        .route("/v1/me", delete(me_delete))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let addr = SocketAddr::from(([127, 0, 0, 1], 8787));
    eprintln!("kelicad-gateway listening on http://{addr}");
    let listener = tokio::net::TcpListener::bind(addr).await.expect("bind");
    axum::serve(listener, app).await.expect("serve");
}

#[derive(Deserialize)]
struct DevTokenRequest {
    sub: String,
    #[serde(default = "default_org")]
    org: String,
    #[serde(default = "default_role")]
    role: String,
}

fn default_org() -> String {
    "org-1".into()
}
fn default_role() -> String {
    "editor".into()
}

async fn dev_token(
    State(state): State<Arc<AppState>>,
    Json(body): Json<DevTokenRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    if state.auth.mode != AuthMode::Dev {
        return Err(StatusCode::FORBIDDEN);
    }
    let role = Role::parse(&body.role).ok_or(StatusCode::BAD_REQUEST)?;
    let token = state
        .auth
        .issue_token(&body.sub, &body.org, role, 3600)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(
        serde_json::json!({ "access_token": token, "token_type": "Bearer" }),
    ))
}

async fn create_room(
    State(state): State<Arc<AppState>>,
    GatewayUser(user): GatewayUser,
) -> Result<Json<CreateRoomResponse>, StatusCode> {
    user.require(Role::Editor)?;
    let room_id = Uuid::now_v7().to_string();
    {
        let mut rooms = state.rooms.lock().await;
        rooms.insert(room_id.clone(), Room::default());
    }
    {
        let (tx, _rx) = broadcast::channel(256);
        state.buses.lock().await.insert(room_id.clone(), tx);
    }
    state
        .audit(
            &user,
            "room.create",
            &room_id,
            Some(serde_json::json!({"org": user.org})),
        )
        .await;
    {
        let mut subjects = state.subjects.lock().await;
        subjects.insert(
            user.sub.clone(),
            serde_json::json!({
                "sub": user.sub,
                "org": user.org,
                "last_room": room_id,
            }),
        );
    }
    Ok(Json(CreateRoomResponse {
        ws_url: format!("ws://127.0.0.1:8787/rooms/{room_id}/ws"),
        room_id,
    }))
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    Path(id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, id, state))
}

#[derive(Serialize)]
struct SnapshotResponse {
    room_id: String,
    ops: Vec<OpEnvelope>,
    comments: Vec<Comment>,
    versions: Vec<RestorePoint>,
}

async fn get_snapshot(
    Path(id): Path<String>,
    State(state): State<Arc<AppState>>,
    GatewayUser(user): GatewayUser,
) -> Result<Json<SnapshotResponse>, StatusCode> {
    user.require(Role::Viewer)?;
    let rooms = state.rooms.lock().await;
    let room = rooms.get(&id).ok_or(StatusCode::NOT_FOUND)?;
    Ok(Json(SnapshotResponse {
        room_id: id,
        ops: room.ops.clone(),
        comments: room.comments.clone(),
        versions: room.versions.clone(),
    }))
}

#[derive(Deserialize)]
struct PostOpsBody {
    envelope: OpEnvelope,
}

async fn post_ops(
    Path(id): Path<String>,
    State(state): State<Arc<AppState>>,
    GatewayUser(user): GatewayUser,
    Json(body): Json<PostOpsBody>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    user.require(Role::Editor)?;
    let tx = {
        let buses = state.buses.lock().await;
        buses.get(&id).cloned()
    };
    {
        let mut rooms = state.rooms.lock().await;
        let room = rooms.get_mut(&id).ok_or(StatusCode::NOT_FOUND)?;
        if !room.ops.iter().any(|e| e.op_id == body.envelope.op_id) {
            room.ops.push(body.envelope.clone());
        }
    }
    if let Some(tx) = tx {
        if let Ok(out) = serde_json::to_string(&WireMessage::Op {
            envelope: body.envelope.clone(),
        }) {
            let _ = tx.send(out);
        }
    }
    state.audit(&user, "ops.append", &id, None).await;
    Ok(Json(serde_json::json!({ "ok": true })))
}

#[derive(Deserialize)]
struct VectorizeQuery {
    #[serde(default)]
    filename: Option<String>,
}

async fn vectorize(
    State(state): State<Arc<AppState>>,
    GatewayUser(user): GatewayUser,
    Query(q): Query<VectorizeQuery>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Json<serde_json::Value>, StatusCode> {
    user.require(Role::Editor)?;
    let ct = headers
        .get(CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let hint = q.filename.unwrap_or_else(|| {
        if ct.contains("svg") {
            "input.svg".into()
        } else if ct.contains("png") {
            "input.png".into()
        } else {
            "input.bin".into()
        }
    });
    let geoms = vectorize_bytes(&body, Some(&hint)).map_err(|_| StatusCode::BAD_REQUEST)?;
    let entities: Vec<_> = geoms
        .into_iter()
        .map(|g| serde_json::json!({ "geometry": g }))
        .collect();
    state
        .audit(
            &user,
            "vectorize",
            &hint,
            Some(serde_json::json!({"count": entities.len()})),
        )
        .await;
    Ok(Json(serde_json::json!({
        "ok": true,
        "count": entities.len(),
        "entities": entities,
    })))
}

async fn get_audit(
    State(state): State<Arc<AppState>>,
    GatewayUser(user): GatewayUser,
) -> Result<Json<serde_json::Value>, StatusCode> {
    user.require(Role::Admin)?;
    let log = state.audit.lock().await;
    log.verify_chain()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(serde_json::json!({
        "ok": true,
        "events": log.events(),
        "verified": true,
    })))
}

async fn me_export(
    State(state): State<Arc<AppState>>,
    GatewayUser(user): GatewayUser,
) -> Result<Json<serde_json::Value>, StatusCode> {
    user.require(Role::Viewer)?;
    let bag = state.subjects.lock().await.get(&user.sub).cloned();
    let events = state.audit.lock().await.for_actor(&user.sub);
    Ok(Json(serde_json::json!({
        "sub": user.sub,
        "org": user.org,
        "role": format!("{:?}", user.role).to_ascii_lowercase(),
        "profile": bag,
        "audit": events,
    })))
}

async fn me_delete(
    State(state): State<Arc<AppState>>,
    GatewayUser(user): GatewayUser,
) -> Result<Json<serde_json::Value>, StatusCode> {
    user.require(Role::Viewer)?;
    state.subjects.lock().await.remove(&user.sub);
    state.audit.lock().await.purge_actor(&user.sub);
    // Re-append a system note that purge happened (by anonymous system).
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    state
        .audit
        .lock()
        .await
        .append("system", "gdpr.purge", &user.sub, ts, None);
    Ok(Json(serde_json::json!({ "ok": true, "purged": user.sub })))
}

async fn handle_socket(socket: WebSocket, room_id: String, state: Arc<AppState>) {
    let (mut sink, mut stream) = socket.split();
    {
        let mut rooms = state.rooms.lock().await;
        rooms.entry(room_id.clone()).or_default();
    }
    let tx = {
        let mut buses = state.buses.lock().await;
        buses
            .entry(room_id.clone())
            .or_insert_with(|| broadcast::channel(256).0)
            .clone()
    };
    let mut rx = tx.subscribe();
    let snapshot = {
        let rooms = state.rooms.lock().await;
        let room = rooms.get(&room_id).cloned().unwrap_or_default();
        WireMessage::Snapshot {
            ops: room.ops,
            comments: room.comments,
            versions: room.versions,
        }
    };
    if let Ok(text) = serde_json::to_string(&snapshot) {
        let _ = sink.send(Message::Text(text)).await;
    }
    loop {
        tokio::select! {
            msg = stream.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        handle_client_text(&state, &room_id, &tx, text.as_str()).await;
                    }
                    Some(Ok(Message::Ping(p))) => {
                        let _ = sink.send(Message::Pong(p)).await;
                    }
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Ok(_)) => {}
                    Some(Err(_)) => break,
                }
            }
            Ok(broadcasted) = rx.recv() => {
                if sink.send(Message::Text(broadcasted)).await.is_err() {
                    break;
                }
            }
        }
    }
}

async fn handle_client_text(
    state: &AppState,
    room_id: &str,
    tx: &broadcast::Sender<String>,
    text: &str,
) {
    let Ok(msg) = serde_json::from_str::<WireMessage>(text) else {
        return;
    };
    match msg {
        WireMessage::Op { envelope } => {
            {
                let mut rooms = state.rooms.lock().await;
                if let Some(room) = rooms.get_mut(room_id) {
                    if !room.ops.iter().any(|e| e.op_id == envelope.op_id) {
                        room.ops.push(envelope.clone());
                    }
                }
            }
            if let Ok(out) = serde_json::to_string(&WireMessage::Op { envelope }) {
                let _ = tx.send(out);
            }
        }
        WireMessage::Presence { state: presence } => {
            if let Ok(out) = serde_json::to_string(&WireMessage::Presence { state: presence }) {
                let _ = tx.send(out);
            }
        }
        WireMessage::CommentUpsert { comment } => {
            {
                let mut rooms = state.rooms.lock().await;
                if let Some(room) = rooms.get_mut(room_id) {
                    if let Some(pos) = room.comments.iter().position(|c| c.id == comment.id) {
                        room.comments[pos] = comment.clone();
                    } else {
                        room.comments.push(comment.clone());
                    }
                }
            }
            if let Ok(out) = serde_json::to_string(&WireMessage::CommentUpsert { comment }) {
                let _ = tx.send(out);
            }
        }
        WireMessage::CommentResolve {
            id,
            lamport,
            actor_id,
        } => {
            if let Ok(cid) = uuid::Uuid::parse_str(&id) {
                let mut rooms = state.rooms.lock().await;
                if let Some(room) = rooms.get_mut(room_id) {
                    if let Some(c) = room.comments.iter_mut().find(|c| c.id.0 == cid) {
                        c.resolved = true;
                        c.created.lamport = c.created.lamport.max(lamport);
                        let _ = actor_id;
                    }
                }
            }
            if let Ok(out) = serde_json::to_string(&WireMessage::CommentResolve {
                id,
                lamport,
                actor_id,
            }) {
                let _ = tx.send(out);
            }
        }
        WireMessage::VersionSave { point } => {
            {
                let mut rooms = state.rooms.lock().await;
                if let Some(room) = rooms.get_mut(room_id) {
                    room.versions.push(point.clone());
                }
            }
            if let Ok(out) = serde_json::to_string(&WireMessage::VersionSave { point }) {
                let _ = tx.send(out);
            }
        }
        WireMessage::Hello { state: presence } => {
            if let Ok(out) = serde_json::to_string(&WireMessage::Presence { state: presence }) {
                let _ = tx.send(out);
            }
        }
        WireMessage::Ping => {
            let _ = tx.send(r#"{"type":"ping"}"#.into());
        }
        WireMessage::Snapshot { .. } => {}
    }
}
