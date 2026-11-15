import { useCallback, useState } from "react";

import type { CollabClient, CollabStatus } from "../cad/collab";

type Props = {
  collab: CollabClient;
  status: CollabStatus;
  name: string;
  onName: (n: string) => void;
  onEcho: (line: string) => void;
};

export function CollabBar({ collab, status, name, onName, onEcho }: Props) {
  const [roomInput, setRoomInput] = useState("");

  const createAndJoin = useCallback(async () => {
    try {
      const id = await collab.createRoom();
      setRoomInput(id);
      collab.connect(id, name);
    } catch (e) {
      onEcho(`Salle: ${String(e)}`);
    }
  }, [collab, name, onEcho]);

  const join = useCallback(() => {
    const id = roomInput.trim();
    if (!id) return;
    collab.connect(id, name);
  }, [collab, name, roomInput]);

  const statusColor =
    status === "online"
      ? "text-emerald-300"
      : status === "offline"
        ? "text-amber-300"
        : status === "connecting"
          ? "text-sky-300"
          : "text-ink-500";

  return (
    <div className="flex flex-wrap items-center gap-2 border-b border-ink-700/50 bg-ink-950/80 px-3 py-1.5 text-xs">
      <span className="uppercase tracking-wider text-ink-500">Collab</span>
      <span className={statusColor}>{status}</span>
      <input
        className="w-28 rounded border border-ink-700 bg-ink-900 px-2 py-1 text-ink-100"
        value={name}
        onChange={(e) => onName(e.target.value)}
        placeholder="Nom"
      />
      <input
        className="min-w-[12rem] flex-1 rounded border border-ink-700 bg-ink-900 px-2 py-1 font-mono text-ink-100"
        value={roomInput}
        onChange={(e) => setRoomInput(e.target.value)}
        placeholder="room id"
      />
      <button
        type="button"
        className="rounded border border-ink-600 px-2 py-1 hover:border-accent-soft"
        onClick={() => void createAndJoin()}
      >
        Nouvelle salle
      </button>
      <button
        type="button"
        className="rounded border border-ink-600 px-2 py-1 hover:border-accent-soft"
        onClick={join}
      >
        Rejoindre
      </button>
      <button
        type="button"
        className="rounded border border-ink-600 px-2 py-1 hover:border-accent-soft"
        onClick={() => collab.goOffline()}
      >
        Hors-ligne
      </button>
      <button
        type="button"
        className="rounded border border-ink-600 px-2 py-1 hover:border-accent-soft"
        onClick={() => collab.reconnect(name)}
      >
        Reconnecter
      </button>
    </div>
  );
}
