import {
  asCacheRows,
  isEntryFresh,
  loadCache,
  saveCache,
  type CacheState,
} from "../cache.js";
import { ClaudeProvider } from "../providers/claudeProvider.js";
import { CodexProvider } from "../providers/codexProvider.js";
import type { Provider, ProviderResult } from "../providers/provider.js";
import type { NormalizedSnapshot, ProviderId } from "../types.js";

export type CollectOptions = {
  refresh: boolean;
  debug: boolean;
  providers: ProviderId[];
};

export type CollectResult = {
  snapshot: NormalizedSnapshot;
  debugByProvider: Partial<Record<ProviderId, Record<string, unknown>>>;
};

export type ProviderFactory = (id: ProviderId) => Provider;

function defaultProviderFactory(id: ProviderId): Provider {
  switch (id) {
    case "claude":
      return new ClaudeProvider();
    case "codex":
      return new CodexProvider();
  }
}

function fixtureProviderFactory(id: ProviderId): Provider {
  const fixedNow = new Date("2026-02-24T00:00:00.000Z").toISOString();
  return {
    id,
    async acquire(): Promise<ProviderResult> {
      if (id === "claude") {
        return {
          rows: [
            {
              provider: "claude",
              window: "claude:session",
              used: null,
              limit: null,
              usedPercent: 79,
              resetAt: fixedNow,
              source: "parsed",
              confidence: "medium",
              stale: false,
              notes: "fixture",
            },
            {
              provider: "claude",
              window: "claude:week(all-models)",
              used: null,
              limit: null,
              usedPercent: 22,
              resetAt: fixedNow,
              source: "parsed",
              confidence: "medium",
              stale: false,
              notes: "fixture",
            },
          ],
          errors: [],
          debug: { fixture: true },
        };
      }

      return {
        rows: [
          {
            provider: "codex",
            window: "codex:5h",
            used: null,
            limit: null,
            usedPercent: 81,
            resetAt: fixedNow,
            source: "structured",
            confidence: "high",
            stale: false,
            notes: "fixture",
          },
          {
            provider: "codex",
            window: "codex:weekly",
            used: null,
            limit: null,
            usedPercent: 30,
            resetAt: fixedNow,
            source: "structured",
            confidence: "high",
            stale: false,
            notes: "fixture",
          },
        ],
        errors: [],
        debug: { fixture: true },
      };
    },
  };
}

export function getProviderFactoryFromEnv(): ProviderFactory {
  const mode = process.env.USAGE_STATUS_FIXTURE?.trim().toLowerCase();
  if (mode === "demo") return fixtureProviderFactory;
  return defaultProviderFactory;
}

function isFixtureMode(): boolean {
  return (process.env.USAGE_STATUS_FIXTURE?.trim().toLowerCase() ?? "") === "demo";
}

function makeUnexpectedError(id: ProviderId, err: unknown): ProviderResult {
  const msg = err instanceof Error ? err.message : String(err);
  return {
    rows: [],
    errors: [
      {
        provider: id,
        type: "unexpected",
        message: msg,
        actionable: null,
      },
    ],
  };
}

function applyRowsAndCache(
  cache: CacheState,
  snapshot: NormalizedSnapshot,
  id: ProviderId,
  res: ProviderResult
): boolean {
  let cacheDirty = false;
  snapshot.rows.push(...res.rows);
  snapshot.errors.push(...res.errors);

  const cached = cache.providers[id];
  if (res.rows.length > 0) {
    cache.providers[id] = {
      fetchedAt: new Date().toISOString(),
      rows: res.rows,
    };
    cacheDirty = true;
  } else if (cached?.rows?.length) {
    snapshot.rows.push(
      ...asCacheRows(cached.rows, true, `stale cache from ${cached.fetchedAt}`)
    );
  }

  return cacheDirty;
}

export async function collectSnapshot(
  opts: CollectOptions,
  providerFactory: ProviderFactory = getProviderFactoryFromEnv()
): Promise<CollectResult> {
  const snapshot: NormalizedSnapshot = {
    fetchedAt: new Date().toISOString(),
    rows: [],
    errors: [],
  };

  const fixtureMode = isFixtureMode();
  const cache = fixtureMode
    ? ({ path: "", ttlMs: 0, providers: {} } as CacheState)
    : await loadCache();
  const nowMs = Date.now();
  let cacheDirty = false;
  const debugByProvider: CollectResult["debugByProvider"] = {};

  for (const id of opts.providers) {
    const cached = cache.providers[id];
    if (!fixtureMode && !opts.refresh && cached && isEntryFresh(cached, cache.ttlMs, nowMs)) {
      snapshot.rows.push(
        ...asCacheRows(cached.rows, false, `cached at ${cached.fetchedAt}`)
      );
      continue;
    }

    const provider = providerFactory(id);
    let res: ProviderResult;
    try {
      res = await provider.acquire({ refresh: opts.refresh, debug: opts.debug });
    } catch (err) {
      res = makeUnexpectedError(id, err);
    }

    cacheDirty = applyRowsAndCache(cache, snapshot, id, res) || cacheDirty;
    if (opts.debug && res.debug) {
      debugByProvider[id] = res.debug;
    }
  }

  if (!fixtureMode && cacheDirty) {
    await saveCache(cache);
  }

  return { snapshot, debugByProvider };
}
