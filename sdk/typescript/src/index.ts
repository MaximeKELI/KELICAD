/** Plugin permissions (fail closed on the host). */
export type Permissions = {
  doc_read?: boolean;
  doc_write_layers?: string[];
  net_hosts?: string[];
};

/** plugin.json shape. */
export type Manifest = {
  id: string;
  version: string;
  name?: string;
  permissions?: Permissions;
  max_hostcalls?: number;
};

/** One hostcall step for ScriptGuest. */
export type HostcallStep = {
  method: string;
  args?: Record<string, unknown>;
};

/** Client-shaped helper for future RPC bridges. */
export class PluginHostClient {
  constructor(private invoke: (method: string, args: Record<string, unknown>) => Promise<unknown>) {}

  ping() {
    return this.invoke("ping", {});
  }

  summarizeDocument() {
    return this.invoke("summarize_document", {});
  }

  listEntities() {
    return this.invoke("list_entities", {});
  }

  createLine(start: [number, number], end: [number, number], layer = "0") {
    return this.invoke("create_line", { start, end, layer });
  }
}
