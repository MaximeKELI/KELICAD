import { useEffect, useRef, useState } from "react";

type Props = {
  prompt: string;
  echoes: string[];
  onSubmit: (text: string) => void;
  /** Cursor hint for dynamic X/Y fields */
  cursor?: { x: number; y: number } | null;
  showZ?: boolean;
};

/** Detect AutoCAD-style point / radius / angle prompts. */
function expectKind(
  prompt: string,
): "point" | "radius" | "angle" | "number" | "text" | null {
  const p = prompt.toLowerCase();
  if (!p.trim()) return null;
  if (
    /point|coin|centre|center|sommet|base|insertion|passage|axe|emplacement|vertex|apex|direction/.test(
      p,
    )
  ) {
    return "point";
  }
  if (/rayon|radius|diamètre|diameter/.test(p)) return "radius";
  if (/angle|degré|degrees|rotation/.test(p)) return "angle";
  if (/distance|facteur|largeur|hauteur|côtés|nombre|espacement/.test(p)) {
    return "number";
  }
  if (/texte|saisissez|nom/.test(p)) return "text";
  return null;
}

export function CommandLine({
  prompt,
  echoes,
  onSubmit,
  cursor,
  showZ = false,
}: Props) {
  const [value, setValue] = useState("");
  const [x, setX] = useState("");
  const [y, setY] = useState("");
  const [z, setZ] = useState("0");
  const [scalar, setScalar] = useState("");
  const inputRef = useRef<HTMLInputElement>(null);
  const kind = expectKind(prompt);

  useEffect(() => {
    inputRef.current?.focus();
    setX("");
    setY("");
    setZ("0");
    setScalar("");
  }, [prompt, echoes.length]);

  // Live-fill dynamic fields from cursor while empty (AutoCAD dyninput feel).
  useEffect(() => {
    if (!cursor || kind !== "point") return;
    if (x === "" && y === "") {
      setX(cursor.x.toFixed(4));
      setY(cursor.y.toFixed(4));
    }
  }, [cursor, kind, x, y]);

  const submitPoint = () => {
    const xs = x.trim();
    const ys = y.trim();
    if (!xs || !ys) return;
    const token =
      showZ && z.trim() !== ""
        ? `${xs},${ys},${z.trim()}`
        : `${xs},${ys}`;
    onSubmit(token);
    setValue("");
    setX("");
    setY("");
    setZ("0");
  };

  const submitScalar = () => {
    if (!scalar.trim()) return;
    onSubmit(scalar.trim());
    setScalar("");
    setValue("");
  };

  return (
    <div className="cad-cmdline">
      <div className="cad-echo font-mono">
        {echoes.slice(-5).map((line, i) => (
          <div key={`${i}-${line.slice(0, 24)}`}>{line}</div>
        ))}
      </div>
      <div className="cad-cmdrow">
        <span className="cad-prompt font-mono">
          {prompt || "Entrez une commande"}
        </span>

        {kind === "point" && (
          <div className="cad-dynfields">
            <label>
              X
              <input
                className="cad-dyninput font-mono"
                value={x}
                onChange={(e) => setX(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === "Enter") {
                    e.preventDefault();
                    submitPoint();
                  }
                }}
                spellCheck={false}
                aria-label="X"
              />
            </label>
            <label>
              Y
              <input
                className="cad-dyninput font-mono"
                value={y}
                onChange={(e) => setY(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === "Enter") {
                    e.preventDefault();
                    submitPoint();
                  }
                }}
                spellCheck={false}
                aria-label="Y"
              />
            </label>
            {showZ && (
              <label>
                Z
                <input
                  className="cad-dyninput font-mono"
                  value={z}
                  onChange={(e) => setZ(e.target.value)}
                  onKeyDown={(e) => {
                    if (e.key === "Enter") {
                      e.preventDefault();
                      submitPoint();
                    }
                  }}
                  spellCheck={false}
                  aria-label="Z"
                />
              </label>
            )}
            <button
              type="button"
              className="cad-dyn-go"
              title="Valider le point"
              onClick={submitPoint}
            >
              ↵
            </button>
          </div>
        )}

        {(kind === "radius" || kind === "angle" || kind === "number") && (
          <div className="cad-dynfields">
            <label>
              {kind === "radius"
                ? "R"
                : kind === "angle"
                  ? "∠"
                  : "n"}
              <input
                className="cad-dyninput font-mono"
                value={scalar}
                onChange={(e) => setScalar(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === "Enter") {
                    e.preventDefault();
                    submitScalar();
                  }
                }}
                placeholder={
                  kind === "radius"
                    ? "rayon"
                    : kind === "angle"
                      ? "degrés"
                      : "valeur"
                }
                spellCheck={false}
              />
            </label>
            <button
              type="button"
              className="cad-dyn-go"
              onClick={submitScalar}
            >
              ↵
            </button>
          </div>
        )}

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
              return;
            }
            if (e.code === "Space" && value.trim() === "") {
              e.preventDefault();
              e.stopPropagation();
              onSubmit("");
            }
          }}
          spellCheck={false}
          autoComplete="off"
          aria-label="Ligne de commande KELICAD"
          placeholder={
            kind === "point"
              ? "ou 10,20  ·  @5,0  ·  @10<45"
              : "L, C, M, REC…"
          }
        />
      </div>
    </div>
  );
}
