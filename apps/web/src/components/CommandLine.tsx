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
  }, [prompt, echoes.length]);

  return (
    <div className="cad-cmdline">
      <div className="cad-echo font-mono">
        {echoes.slice(-5).map((line, i) => (
          <div key={`${i}-${line.slice(0, 24)}`}>{line}</div>
        ))}
      </div>
      <div className="cad-cmdrow">
        <span className="cad-prompt font-mono">
          {prompt || "Type a command"}
        </span>
        <input
          ref={inputRef}
          className="cad-cmdinput font-mono"
          value={value}
          onChange={(e) => setValue(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter") {
              e.preventDefault();
              e.stopPropagation();
              onSubmit(value);
              setValue("");
            }
          }}
          spellCheck={false}
          autoComplete="off"
          aria-label="Ligne de commande KELICAD"
          placeholder="L, C, M, REC…"
        />
      </div>
    </div>
  );
}
