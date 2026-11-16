import {
  TONE_CSS,
  getTool,
  toolMatchesActive,
} from "../cad/commandCatalog";
import { TOOL_RAIL_GROUPS } from "../cad/ribbonLayout";

type Props = {
  activeCmd?: string | null;
  onCommand: (cmd: string) => void;
};

export function ToolRail({ activeCmd, onCommand }: Props) {
  return (
    <aside className="cad-rail flex w-14 shrink-0 flex-col overflow-y-auto border-r border-[#2a2a2a] bg-[#1b1b1b]">
      {TOOL_RAIL_GROUPS.map((g) => (
        <div key={g.title} className="border-b border-[#2a2a2a] py-1.5">
          <p className="px-0.5 pb-1 text-center text-[9px] uppercase tracking-wide text-[#7a7a7a]">
            {g.title.slice(0, 3)}
          </p>
          {g.ids.map((id) => {
            const tool = getTool(id);
            if (!tool || tool.kind.type !== "cad") return null;
            if (tool.status === "unavailable") return null;
            const cmd = tool.kind.cmd;
            const active = toolMatchesActive(tool, activeCmd);
            return (
              <button
                key={id}
                type="button"
                title={tool.tip}
                className={`cad-tool ${active ? "is-active" : ""}`}
                onClick={() => onCommand(cmd)}
              >
                <span
                className="text-[13px] leading-none"
                style={{ color: TONE_CSS[tool.tone] ?? TONE_CSS.muted }}
              >
                {tool.glyph}
              </span>
              </button>
            );
          })}
        </div>
      ))}
    </aside>
  );
}
