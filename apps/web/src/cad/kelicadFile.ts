/**
 * Build a minimal SQLite `.kelicad` file in the browser via sql.js.
 * Schema matches formats/kelicad-spec.md v0 (materialized snapshot).
 */

import initSqlJs from "sql.js";

type Snapshot = {
  schema_version: number;
  current_layer: string;
  layers: { id: string; name: string; color: [number, number, number] }[];
  entities: {
    id: string;
    kind: string;
    layer_id: string;
    geometry: Record<string, unknown>;
  }[];
};

function uuidToBytes(id: string): Uint8Array {
  const hex = id.replace(/-/g, "");
  const out = new Uint8Array(16);
  for (let i = 0; i < 16; i += 1) {
    out[i] = parseInt(hex.slice(i * 2, i * 2 + 2), 16);
  }
  return out;
}

export async function snapshotToKelicadBytes(snapshotJson: string): Promise<Uint8Array> {
  const snap = JSON.parse(snapshotJson) as Snapshot;
  const SQL = await initSqlJs({
    locateFile: (file) => `https://sql.js.org/dist/${file}`,
  });
  const db = new SQL.Database();
  db.run(`
    CREATE TABLE meta (key TEXT PRIMARY KEY NOT NULL, value TEXT NOT NULL);
    CREATE TABLE layers (
      id BLOB PRIMARY KEY NOT NULL,
      name TEXT NOT NULL,
      color_r INTEGER NOT NULL,
      color_g INTEGER NOT NULL,
      color_b INTEGER NOT NULL,
      lineweight INTEGER NOT NULL DEFAULT 0,
      frozen INTEGER NOT NULL DEFAULT 0,
      locked INTEGER NOT NULL DEFAULT 0,
      no_plot INTEGER NOT NULL DEFAULT 0
    );
    CREATE TABLE entities (
      id BLOB PRIMARY KEY NOT NULL,
      kind TEXT NOT NULL,
      layer_id BLOB NOT NULL,
      geometry_json TEXT NOT NULL,
      deleted INTEGER NOT NULL DEFAULT 0
    );
  `);
  db.run("INSERT INTO meta(key,value) VALUES(?,?)", ["format", "kelicad"]);
  db.run("INSERT INTO meta(key,value) VALUES(?,?)", [
    "schema_version",
    String(snap.schema_version ?? 0),
  ]);
  db.run("INSERT INTO meta(key,value) VALUES(?,?)", [
    "current_layer",
    snap.current_layer,
  ]);
  db.run("INSERT INTO meta(key,value) VALUES(?,?)", ["tolerance_abs", "1e-9"]);

  for (const layer of snap.layers) {
    db.run(
      "INSERT INTO layers(id,name,color_r,color_g,color_b,lineweight,frozen,locked,no_plot) VALUES(?,?,?,?,?,?,0,0,0)",
      [
        uuidToBytes(layer.id),
        layer.name,
        layer.color[0],
        layer.color[1],
        layer.color[2],
        0,
      ],
    );
  }

  for (const ent of snap.entities) {
    db.run(
      "INSERT INTO entities(id,kind,layer_id,geometry_json,deleted) VALUES(?,?,?,?,0)",
      [
        uuidToBytes(ent.id),
        ent.kind,
        uuidToBytes(ent.layer_id),
        JSON.stringify(ent.geometry),
      ],
    );
  }

  const data = db.export();
  db.close();
  return data;
}

export function downloadBytes(filename: string, data: Uint8Array): void {
  const blob = new Blob([data], { type: "application/vnd.kelicad+sqlite3" });
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = filename;
  a.click();
  URL.revokeObjectURL(url);
}
