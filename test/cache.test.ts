import { describe, expect, test } from "vitest";

import { asCacheRows, isEntryFresh } from "../src/cache.js";

describe("cache helpers", () => {
  test("isEntryFresh respects ttl and timestamps", () => {
    const entry = { fetchedAt: "2026-02-23T00:00:00.000Z", rows: [] };
    expect(isEntryFresh(entry, 60_000, Date.parse(entry.fetchedAt) + 59_000)).toBe(
      true
    );
    expect(isEntryFresh(entry, 60_000, Date.parse(entry.fetchedAt) + 61_000)).toBe(
      false
    );
    expect(isEntryFresh(entry, 0, Date.parse(entry.fetchedAt) + 1)).toBe(false);
  });

  test("asCacheRows rewrites source/stale and merges notes", () => {
    const rows = [
      {
        provider: "codex" as const,
        window: "codex:5h",
        used: null,
        limit: null,
        usedPercent: 10,
        resetAt: "2026-02-23T00:00:00.000Z",
        source: "structured" as const,
        confidence: "high" as const,
        stale: false,
        notes: "original",
      },
    ];

    const cached = asCacheRows(rows, true, "stale cache");
    expect(cached[0]?.source).toBe("cache");
    expect(cached[0]?.stale).toBe(true);
    expect(cached[0]?.notes).toContain("original");
    expect(cached[0]?.notes).toContain("stale cache");
  });
});
