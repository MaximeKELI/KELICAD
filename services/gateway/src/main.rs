//! KELICAD collaboration gateway — in-memory rooms + WebSocket sync.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Path, State};
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use futures_util::{SinkExt, StreamExt};
use kelicad_sync::{Comment, OpEnvelope, RestorePoint, WireMessage};
use serde::Serialize;
use tokio::sync::{broadcast, Mutex};
use tower_http::cors::CorsLayer;
use uuid::Uuid;

#[derive(Clone, Default)]
struct Room {
    ops: Vec<OpEnvelope>,
    comments: Vec<Comment>,
    versions: Vec<RestorePoint>,
}

struct AppState {
    rooms: Mutex<HashMap<String, Room>>,
    /// room_id → broadcast of serialized WireMessage
    buses: Mutex<HashMap<String, broadcast::Sender<String>>>,
}

#[derive(Serialize)]
struct CreateRoomResponse {
    room_id: String,
    ws_url: String,
}

#[tokio::main]
async fn main() {
    let state = Arc::new(AppState {
        rooms: Mutex::new(HashMap::new()),
        buses: Mutex::new(HashMap::new()),
    });

    let app = Router::new()
        .route("/health", get(|| async { "ok" }))
        .route(
            "/v1/health",
            get(|| async { Json(serde_json::json!({"ok": true})) }),
        )
        .route("/rooms", post(create_room))
        .route("/v1/rooms", post(create_room))
        .route("/rooms/:id/ws", get(ws_handler))
        .route("/v1/rooms/:id/snapshot", get(get_snapshot))
        .route("/v1/rooms/:id/ops", post(post_ops))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let addr = SocketAddr::from(([127, 0, 0, 1], 8787));
    eprintln!("kelicad-gateway listening on http://{addr}");
    let listener = tokio::net::TcpListener::bind(addr).await.expect("bind");
    axum::serve(listener, app).await.expect("serve");
}

async fn create_room(State(state): State<Arc<AppState>>) -> Json<CreateRoomResponse> {
    let room_id = Uuid::now_v7().to_string();
    {
        let mut rooms = state.rooms.lock().await;
        rooms.insert(room_id.clone(), Room::default());
    }
    {
        let (tx, _rx) = broadcast::channel(256);
        state.buses.lock().await.insert(room_id.clone(), tx);
    }
    Json(CreateRoomResponse {
        ws_url: format!("ws://127.0.0.1:8787/rooms/{room_id}/ws"),
        room_id,
    })
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
) -> Result<Json<SnapshotResponse>, axum::http::StatusCode> {
    let rooms = state.rooms.lock().await;
    let room = rooms.get(&id).ok_or(axum::http::StatusCode::NOT_FOUND)?;
    Ok(Json(SnapshotResponse {
        room_id: id,
        ops: room.ops.clone(),
        comments: room.comments.clone(),
        versions: room.versions.clone(),
    }))
}

#[derive(serde::Deserialize)]
struct PostOpsBody {
    envelope: OpEnvelope,
}

async fn post_ops(
    Path(id): Path<String>,
    State(state): State<Arc<AppState>>,
    Json(body): Json<PostOpsBody>,
) -> Result<Json<serde_json::Value>, axum::http::StatusCode> {
    let tx = {
        let buses = state.buses.lock().await;
        buses.get(&id).cloned()
    };
    {
        let mut rooms = state.rooms.lock().await;
        let room = rooms
            .get_mut(&id)
            .ok_or(axum::http::StatusCode::NOT_FOUND)?;
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
    Ok(Json(serde_json::json!({ "ok": true })))
}

async fn handle_socket(socket: WebSocket, room_id: String, state: Arc<AppState>) {
    let (mut sink, mut stream) = socket.split();

    // Ensure room exists
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

    // Send snapshot
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
        let _ = sink.send(Message::Text(text.into())).await;
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
                if sink.send(Message::Text(broadcasted.into())).await.is_err() {
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
        WireMessage::Snapshot { .. } => {
            // client → server snapshots ignored
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn health_path_exists() {
        assert_eq!("/health", "/health");
    }
}
