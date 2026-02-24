import { describe, expect, test } from "vitest";

import { computeBackoffDelayMs } from "../src/core/scheduler.js";

describe("scheduler backoff", () => {
  test("increases with failures and respects max", () => {
    const r = () => 0.5;
    const d0 = computeBackoffDelayMs(0, undefined, r);
    const d1 = computeBackoffDelayMs(1, undefined, r);
    const d4 = computeBackoffDelayMs(4, undefined, r);
    expect(d1).toBeGreaterThanOrEqual(d0);
    expect(d4).toBeGreaterThanOrEqual(d1);
    expect(d4).toBeLessThanOrEqual(5 * 60_000);
  });
});
