import { getTool, toolMatchesActive } from "../cad/commandCatalog";
import { TOOL_RAIL_GROUPS } from "../cad/ribbonLayout";

type Props = {
  activeCmd?: string | null;
  onCommand: (cmd: string) => void;
};

export function ToolRail({ activeCmd, onCommand }: Props) {
  return (
    <aside className="cad-rail flex w-11 shrink-0 flex-col overflow-y-auto border-r border-[#2a2a2a] bg-[#1b1b1b]">
      {TOOL_RAIL_GROUPS.map((g) => (
        <div key={g.title} className="border-b border-[#2a2a2a] py-1">
          <p className="px-0.5 pb-0.5 text-center text-[8px] uppercase tracking-wide text-[#6a6a6a]">
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
                <span className="text-[13px] leading-none">{tool.glyph}</span>
              </button>
            );
          })}
        </div>
      ))}
    </aside>
  );
}
