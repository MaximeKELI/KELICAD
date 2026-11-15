import { useCallback, useEffect, useState } from "react";

import type { CadSession } from "../cad/types";
import type { CollabClient } from "../cad/collab";

type Comment = {
  id: string;
  body: string;
  resolved: boolean;
  anchor: { Point?: { position: { x: number; y: number } } } | { Entity?: { id: string } };
};

type Version = { name: string; oplog_fingerprint: string };

type Props = {
  cad: CadSession;
  collab: CollabClient;
  revision: number;
  onEcho: (line: string) => void;
};

export function CommentsPanel({ cad, collab, revision, onEcho }: Props) {
  const [body, setBody] = useState("À vérifier");
  const [comments, setComments] = useState<Comment[]>([]);
  const [versions, setVersions] = useState<Version[]>([]);
  const [diffText, setDiffText] = useState("");

  const refresh = useCallback(() => {
    try {
      setComments(JSON.parse(cad.sync_comments_json()) as Comment[]);
      setVersions(JSON.parse(cad.sync_versions_json()) as Version[]);
    } catch {
      setComments([]);
      setVersions([]);
    }
  }, [cad]);

  useEffect(() => {
    refresh();
  }, [refresh, revision]);

  const add = () => {
    const json = cad.sync_add_comment_at(0, 0, body.trim() || "note");
    collab.sendComment(json);
    refresh();
    onEcho("Commentaire ajouté");
  };

  const resolve = (id: string) => {
    cad.sync_resolve_comment(id);
    refresh();
  };

  const saveVer = () => {
    const name = `v${versions.length + 1}`;
    const point = cad.sync_save_version(name);
    collab.sendVersion(point);
    refresh();
    onEcho(`Point de restauration ${name}`);
  };

  const compare = () => {
    if (versions.length < 2) {
      setDiffText("Il faut ≥2 versions");
      return;
    }
    const a = versions[versions.length - 2].name;
    const b = versions[versions.length - 1].name;
    const d = JSON.parse(cad.sync_diff_versions(a, b)) as {
      added: string[];
      removed: string[];
      changed: string[];
    };
    setDiffText(
      `${a}→${b}: +${d.added.length} −${d.removed.length} ~${d.changed.length}`,
    );
  };

  return (
    <aside className="flex w-64 shrink-0 flex-col border-l border-ink-700/60 bg-ink-900 text-xs">
      <div className="border-b border-ink-700/60 px-3 py-2">
        <p className="uppercase tracking-wider text-ink-500">Commentaires</p>
      </div>
      <div className="flex flex-col gap-2 p-2">
        <textarea
          className="min-h-[3rem] rounded border border-ink-700 bg-ink-950 px-2 py-1 text-ink-100"
          value={body}
          onChange={(e) => setBody(e.target.value)}
        />
        <button
          type="button"
          className="rounded border border-accent-soft/40 px-2 py-1 text-accent-soft"
          onClick={add}
        >
          Ajouter (0,0)
        </button>
        <ul className="max-h-40 space-y-1 overflow-auto">
          {comments.map((c) => (
            <li key={c.id} className="border-b border-ink-800 pb-1">
              <span className={c.resolved ? "text-ink-600 line-through" : "text-ink-200"}>
                {c.body}
              </span>
              {!c.resolved && (
                <button
                  type="button"
                  className="ml-2 text-emerald-400"
                  onClick={() => resolve(c.id)}
                >
                  résoudre
                </button>
              )}
            </li>
          ))}
        </ul>
      </div>
      <div className="mt-auto border-t border-ink-700/60 p-2">
        <p className="mb-1 uppercase tracking-wider text-ink-500">Versions</p>
        <div className="mb-2 flex gap-1">
          <button
            type="button"
            className="rounded border border-ink-600 px-2 py-1"
            onClick={saveVer}
          >
            Sauver
          </button>
          <button
            type="button"
            className="rounded border border-ink-600 px-2 py-1"
            onClick={compare}
          >
            Comparer
          </button>
        </div>
        {diffText && <p className="mb-1 text-ink-400">{diffText}</p>}
        <ul className="space-y-0.5 text-ink-400">
          {versions.map((v) => (
            <li key={v.name} className="font-mono">
              {v.name}
            </li>
          ))}
        </ul>
      </div>
    </aside>
  );
}
