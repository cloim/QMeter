import { z } from "zod";

import {
  NormalizedSnapshotSchema,
  type ProviderId,
} from "./types.js";
import { renderGraph, renderTable } from "./output.js";
import { collectSnapshot } from "./core/snapshot.js";

export type RunOptions = {
  json: boolean;
  refresh: boolean;
  debug: boolean;
  view: "table" | "graph";
  providers: "all" | Set<ProviderId>;
};

const ProviderIdSchema = z.enum(["claude", "codex"]);

function selectedProviders(opts: RunOptions): ProviderId[] {
  if (opts.providers === "all") return ["claude", "codex"];
  const arr = [...opts.providers];
  // Deterministic order.
  arr.sort();
  for (const id of arr) ProviderIdSchema.parse(id);
  return arr;
}

export async function run(opts: RunOptions): Promise<number> {
  const ids = selectedProviders(opts);
  const { snapshot, debugByProvider } = await collectSnapshot({
    refresh: opts.refresh,
    debug: opts.debug,
    providers: ids,
  });

  if (opts.debug) {
    for (const id of ids) {
      const dbg = debugByProvider[id];
      if (dbg) {
        process.stderr.write(`[debug] ${id}: ${JSON.stringify(dbg, null, 2)}\n`);
      }
    }
  }

  // Validate final shape.
  NormalizedSnapshotSchema.parse(snapshot);

  if (opts.json) {
    process.stdout.write(`${JSON.stringify(snapshot, null, 2)}\n`);
  } else {
    const out = opts.view === "graph" ? renderGraph(snapshot) : renderTable(snapshot);
    process.stdout.write(`${out}\n`);
  }

  // Exit codes:
  // 0: full success
  // 1: partial success (some provider failed)
  // 2: usage/argument error (handled in cli.ts)
  // 3: total failure
  if (snapshot.rows.length > 0 && snapshot.errors.length === 0) return 0;
  if (snapshot.rows.length > 0 && snapshot.errors.length > 0) return 1;
  return 3;
}
