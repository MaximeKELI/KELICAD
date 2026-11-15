import { useEffect, useRef, useState } from "react";

type Props = {
  prompt: string;
  echoes: string[];
  onSubmit: (text: string) => void;
};

export function CommandLine({ prompt, echoes, onSubmit }: Props) {
  const [value, setValue] = useState("");
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    inputRef.current?.focus();
  }, [prompt]);

  return (
    <div className="border-t border-ink-700/60 bg-ink-900 px-3 py-2 font-mono text-sm">
      <div className="mb-1 max-h-16 overflow-y-auto text-ink-500">
        {echoes.slice(-4).map((line, i) => (
          <div key={`${i}-${line}`}>{line}</div>
        ))}
      </div>
      <div className="flex items-center gap-2">
        <span className="shrink-0 text-accent-soft">
          {prompt || "Commande :"}
        </span>
        <input
          ref={inputRef}
          className="w-full bg-transparent text-ink-100 outline-none"
          value={value}
          onChange={(e) => setValue(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter") {
              onSubmit(value);
              setValue("");
            }
          }}
          spellCheck={false}
          autoComplete="off"
          aria-label="Ligne de commande KELICAD"
        />
      </div>
    </div>
  );
}
