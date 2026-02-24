import fs from "node:fs/promises";
import os from "node:os";
import path from "node:path";

import { z } from "zod";

import {
  NormalizedRowSchema,
  type NormalizedRow,
  type ProviderId,
} from "./types.js";

const CacheProviderEntrySchema = z.object({
  fetchedAt: z.string().datetime({ offset: true }),
  rows: z.array(NormalizedRowSchema),
});

const CacheFileSchema = z.object({
  version: z.literal(1),
  savedAt: z.string().datetime({ offset: true }),
  providers: z.object({
    claude: CacheProviderEntrySchema.nullable().optional(),
    codex: CacheProviderEntrySchema.nullable().optional(),
  }),
});

export type CacheProviderEntry = z.infer<typeof CacheProviderEntrySchema>;

export type CacheState = {
  path: string;
  ttlMs: number;
  providers: Partial<Record<ProviderId, CacheProviderEntry>>;
};

function defaultCacheDir(): string {
  if (process.platform === "win32") {
    const base = process.env.LOCALAPPDATA;
    if (base && base.trim()) return base;
  }

  const xdg = process.env.XDG_CACHE_HOME;
  if (xdg && xdg.trim()) return xdg;
  return path.join(os.homedir(), ".cache");
}

export function getCachePath(): string {
  const override = process.env.USAGE_STATUS_CACHE_PATH;
  if (override && override.trim()) return override;
  return path.join(defaultCacheDir(), "qmeter", "cache.v1.json");
}

export function getCacheTtlMs(): number {
  const raw = process.env.USAGE_STATUS_CACHE_TTL_SECS;
  if (!raw || !raw.trim()) return 60_000;
  const n = Number(raw);
  if (!Number.isFinite(n) || n < 0) return 60_000;
  return Math.floor(n * 1000);
}

export async function loadCache(): Promise<CacheState> {
  const cachePath = getCachePath();
  const ttlMs = getCacheTtlMs();

  try {
    const raw = await fs.readFile(cachePath, "utf8");
    const parsed = CacheFileSchema.parse(JSON.parse(raw));

    const providers: CacheState["providers"] = {};
    if (parsed.providers.claude) providers.claude = parsed.providers.claude;
    if (parsed.providers.codex) providers.codex = parsed.providers.codex;

    return { path: cachePath, ttlMs, providers };
  } catch {
    return { path: cachePath, ttlMs, providers: {} };
  }
}

export function isEntryFresh(entry: CacheProviderEntry, ttlMs: number, now = Date.now()): boolean {
  if (ttlMs <= 0) return false;
  const t = Date.parse(entry.fetchedAt);
  if (!Number.isFinite(t)) return false;
  return now - t <= ttlMs;
}

export function asCacheRows(rows: NormalizedRow[], stale: boolean, note: string | null): NormalizedRow[] {
  return rows.map((r) => {
    const mergedNote = note
      ? r.notes
        ? `${r.notes}; ${note}`
        : note
      : r.notes;
    return {
      ...r,
      source: "cache",
      stale,
      notes: mergedNote,
    };
  });
}

export async function saveCache(cache: CacheState): Promise<void> {
  const dir = path.dirname(cache.path);
  await fs.mkdir(dir, { recursive: true });

  const file = {
    version: 1 as const,
    savedAt: new Date().toISOString(),
    providers: {
      claude: cache.providers.claude ?? null,
      codex: cache.providers.codex ?? null,
    },
  };

  const json = JSON.stringify(file, null, 2);
  await fs.writeFile(cache.path, json, "utf8");
}
