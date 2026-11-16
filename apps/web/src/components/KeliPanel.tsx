import { useCallback, useEffect, useState } from "react";

import type {
  CadSession,
  KeliAskResult,
  KeliImpulse,
  KeliJournalEntry,
} from "../cad/types";

type Props = {
  cad: CadSession;
  onChanged: () => void;
  onEcho: (lines: string[]) => void;
  revision: number;
};

export function KeliPanel({ cad, onChanged, onEcho, revision }: Props) {
  const [input, setInput] = useState(
    "dessine un rectangle de 4 m × 3 m, cote-le et déplace-le de 2 m vers la droite",
  );
  const [mode, setMode] = useState<"propose" | "apply">("propose");
  const [ask, setAsk] = useState<KeliAskResult | null>(null);
  const [journal, setJournal] = useState<KeliJournalEntry[]>([]);
  const [impulses, setImpulses] = useState<KeliImpulse[]>([]);
  const [busy, setBusy] = useState(false);

  const refreshJournal = useCallback(() => {
    try {
      setJournal(JSON.parse(cad.keli_journal_json()) as KeliJournalEntry[]);
    } catch {
      setJournal([]);
    }
  }, [cad]);

  const refreshImpulses = useCallback(() => {
    try {
      setImpulses(JSON.parse(cad.keli_impulses_json()) as KeliImpulse[]);
    } catch {
      setImpulses([]);
    }
  }, [cad]);

  useEffect(() => {
    refreshImpulses();
  }, [refreshImpulses, revision]);

  const runAsk = useCallback(
    (text: string) => {
      const utterance = text.trim();
      if (!utterance || busy) {
        return;
      }
      setBusy(true);
      try {
        cad.keli_set_mode(mode);
        const raw = cad.keli_ask(utterance);
        const result = JSON.parse(raw) as KeliAskResult;
        setAsk(result);
        onEcho(result.echoes ?? []);
        if (result.applied) {
          onChanged();
        }
        refreshJournal();
        refreshImpulses();
      } catch (e) {
        onEcho([`KELI: ${String(e)}`]);
      } finally {
        setBusy(false);
      }
    },
    [busy, cad, mode, onChanged, onEcho, refreshImpulses, refreshJournal],
  );

  const submit = useCallback(() => runAsk(input), [input, runAsk]);

  const accept = useCallback(() => {
    try {
      cad.keli_accept();
      setAsk(null);
      onEcho(["KELI — plan validé (un Undo annule tout)."]);
      onChanged();
      refreshJournal();
      refreshImpulses();
    } catch (e) {
      onEcho([`KELI accept: ${String(e)}`]);
    }
  }, [cad, onChanged, onEcho, refreshImpulses, refreshJournal]);

  const reject = useCallback(() => {
    try {
      cad.keli_reject();
      setAsk(null);
      onEcho(["KELI — plan rejeté."]);
      refreshJournal();
    } catch (e) {
      onEcho([`KELI reject: ${String(e)}`]);
    }
  }, [cad, onEcho, refreshJournal]);

  const pending = ask && !ask.applied && ask.proposal.has_mutations;

  return (
    <aside className="flex h-full w-80 shrink-0 flex-col border-l border-ink-700/60 bg-ink-900">
      <div className="border-b border-ink-700/60 px-3 py-2">
        <p className="text-xs uppercase tracking-[0.2em] text-accent-soft">KELI</p>
        <p className="text-sm text-ink-100">Impulsions & propositions</p>
      </div>

      <div className="flex items-center gap-2 border-b border-ink-700/40 px-3 py-2 text-xs">
        <span className="text-ink-500">Mode</span>
        <button
          type="button"
          className={`rounded px-2 py-1 ${mode === "propose" ? "bg-accent-soft/20 text-accent-soft" : "text-ink-300"}`}
          onClick={() => setMode("propose")}
        >
          Proposer
        </button>
        <button
          type="button"
          className={`rounded px-2 py-1 ${mode === "apply" ? "bg-accent-soft/20 text-accent-soft" : "text-ink-300"}`}
          onClick={() => setMode("apply")}
        >
          Appliquer
        </button>
      </div>

      <div className="border-b border-ink-700/40 px-3 py-2">
        <p className="mb-1.5 text-[10px] uppercase tracking-wider text-ink-500">
          Impulsions
        </p>
        <div className="flex flex-wrap gap-1">
          {impulses.slice(0, 10).map((imp) => (
            <button
              key={`${imp.category}-${imp.utterance}`}
              type="button"
              title={imp.utterance}
              disabled={busy}
              className="rounded border border-ink-700/70 bg-ink-950/70 px-2 py-1 text-[11px] text-ink-200 hover:border-accent-soft/50 hover:text-accent-soft disabled:opacity-50"
              onClick={() => {
                setInput(imp.utterance);
                runAsk(imp.utterance);
              }}
            >
              {imp.label_fr}
            </button>
          ))}
          {impulses.length === 0 && (
            <span className="text-[11px] text-ink-600">Chargement…</span>
          )}
        </div>
      </div>

      <div className="flex min-h-0 flex-1 flex-col gap-2 p-3">
        <label className="text-xs text-ink-500" htmlFor="keli-input">
          Demande
        </label>
        <textarea
          id="keli-input"
          className="min-h-[5.5rem] resize-y rounded border border-ink-700 bg-ink-950 px-2 py-1.5 text-sm text-ink-100 outline-none focus:border-accent-soft"
          value={input}
          onChange={(e) => setInput(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter" && (e.metaKey || e.ctrlKey)) {
              e.preventDefault();
              submit();
            }
          }}
        />
        <button
          type="button"
          disabled={busy}
          className="rounded border border-accent-soft/50 bg-accent-soft/10 px-3 py-1.5 text-sm text-accent-soft hover:bg-accent-soft/20 disabled:opacity-50"
          onClick={submit}
        >
          Demander
        </button>

        {ask && (
          <div className="rounded border border-ink-700/60 bg-ink-950/80 p-2 text-xs">
            <p className="mb-1 font-medium text-ink-100">{ask.proposal.summary_fr}</p>
            <ul className="mb-2 space-y-0.5 text-ink-400">
              {ask.proposal.steps.map((s, i) => (
                <li key={`${s.tool}-${i}`}>
                  <span className="font-mono text-accent-soft">{s.tool}</span> —{" "}
                  {s.summary_fr}
                </li>
              ))}
            </ul>
            {ask.proposal.ghost_count > 0 && (
              <p className="mb-2 text-ink-500">
                Aperçu fantôme : {ask.proposal.ghost_count} géométrie(s)
              </p>
            )}
            {pending && (
              <div className="flex gap-2">
                <button
                  type="button"
                  className="flex-1 rounded border border-emerald-700/60 px-2 py-1 text-emerald-300 hover:bg-emerald-900/30"
                  onClick={accept}
                >
                  Valider
                </button>
                <button
                  type="button"
                  className="flex-1 rounded border border-ink-600 px-2 py-1 text-ink-300 hover:bg-ink-800"
                  onClick={reject}
                >
                  Rejeter
                </button>
              </div>
            )}
          </div>
        )}

        <div className="min-h-0 flex-1 overflow-auto">
          <p className="mb-1 text-xs text-ink-500">Journal IA</p>
          <ul className="space-y-1 text-[11px] text-ink-400">
            {[...journal].reverse().slice(0, 20).map((e) => (
              <li key={e.index} className="border-b border-ink-800 pb-1">
                <span className="font-mono text-ink-500">#{e.index}</span>{" "}
                <span className="text-accent-soft">{e.outcome}</span> —{" "}
                {e.utterance.slice(0, 48)}
                {e.utterance.length > 48 ? "…" : ""}
              </li>
            ))}
            {journal.length === 0 && <li className="text-ink-600">Vide</li>}
          </ul>
        </div>
      </div>
    </aside>
  );
}
