import type { CadSession } from "./types";

export type CollabStatus = "disconnected" | "connecting" | "online" | "offline";

export type PeerPresence = {
  peer_id: string;
  name: string;
  color: [number, number, number];
  cursor: { x: number; y: number } | null;
  selection: string[];
};

const DEFAULT_WS_BASE = "ws://127.0.0.1:8787";
const DEFAULT_HTTP = "http://127.0.0.1:8787";

export class CollabClient {
  private ws: WebSocket | null = null;
  private roomId: string | null = null;
  status: CollabStatus = "disconnected";
  onStatus?: (s: CollabStatus) => void;
  onPeers?: () => void;
  onDoc?: () => void;
  onEcho?: (line: string) => void;

  constructor(private cad: CadSession) {}

  private setStatus(s: CollabStatus) {
    this.status = s;
    this.onStatus?.(s);
  }

  async createRoom(): Promise<string> {
    const res = await fetch(`${DEFAULT_HTTP}/rooms`, { method: "POST" });
    if (!res.ok) {
      throw new Error(`create room HTTP ${res.status}`);
    }
    const body = (await res.json()) as { room_id: string; ws_url: string };
    return body.room_id;
  }

  connect(roomId: string, name: string) {
    this.disconnect();
    this.roomId = roomId;
    this.cad.sync_set_identity(name, 80, 180, 220);
    this.setStatus("connecting");
    const url = `${DEFAULT_WS_BASE}/rooms/${roomId}/ws`;
    const ws = new WebSocket(url);
    this.ws = ws;

    ws.onopen = () => {
      this.setStatus("online");
      ws.send(this.cad.sync_hello_json());
      // Flush any ops buffered while offline.
      const envelopes = JSON.parse(this.cad.sync_publish_local()) as unknown[];
      for (const env of envelopes) {
        ws.send(JSON.stringify({ type: "op", envelope: env }));
      }
      const drained = JSON.parse(this.cad.sync_drain_outbox()) as unknown[];
      for (const env of drained) {
        ws.send(JSON.stringify({ type: "op", envelope: env }));
      }
      this.onEcho?.(`Collab connecté — salle ${roomId.slice(0, 8)}…`);
    };
    ws.onclose = () => {
      if (this.status !== "offline") {
        this.setStatus("disconnected");
      }
      this.onEcho?.("Collab déconnecté");
    };
    ws.onerror = () => {
      this.onEcho?.("Collab erreur WebSocket (gateway lancé ?)");
    };
    ws.onmessage = (ev) => {
      try {
        const result = this.cad.sync_handle_wire(String(ev.data));
        if (result.startsWith("snapshot") || result === "Applied" || result.includes("Applied")) {
          this.onDoc?.();
        }
        if (result === "presence" || result === "hello") {
          this.onPeers?.();
        }
        if (result === "comment" || result === "resolve") {
          this.onPeers?.();
        }
      } catch (e) {
        this.onEcho?.(`Collab message: ${String(e)}`);
      }
    };
  }

  /** Simulate offline while keeping room id. */
  goOffline() {
    this.setStatus("offline");
    this.ws?.close();
    this.ws = null;
    this.onEcho?.("Mode hors-ligne (buffer local)");
  }

  reconnect(name: string) {
    if (!this.roomId) {
      return;
    }
    this.connect(this.roomId, name);
  }

  disconnect() {
    this.ws?.close();
    this.ws = null;
    this.roomId = null;
    this.setStatus("disconnected");
  }

  /** Publish local CAD ops + presence. */
  flush(cursor?: { x: number; y: number }) {
    if (this.status !== "online" || !this.ws || this.ws.readyState !== WebSocket.OPEN) {
      // Still record locally for later reconnect outbox
      this.cad.sync_publish_local();
      return;
    }
    const envelopes = JSON.parse(this.cad.sync_publish_local()) as unknown[];
    for (const env of envelopes) {
      this.ws.send(JSON.stringify({ type: "op", envelope: env }));
    }
    if (cursor) {
      const presenceMsg = this.cad.sync_set_presence(cursor.x, cursor.y, true);
      this.ws.send(presenceMsg);
    }
  }

  sendComment(json: string) {
    if (this.ws?.readyState === WebSocket.OPEN) {
      this.ws.send(JSON.stringify({ type: "comment_upsert", comment: JSON.parse(json) }));
    }
  }

  sendVersion(pointJson: string) {
    if (this.ws?.readyState === WebSocket.OPEN) {
      this.ws.send(JSON.stringify({ type: "version_save", point: JSON.parse(pointJson) }));
    }
  }

  sendPresence(x: number, y: number) {
    if (this.ws?.readyState === WebSocket.OPEN) {
      this.ws.send(this.cad.sync_set_presence(x, y, true));
    }
  }

  getRoomId() {
    return this.roomId;
  }
}
