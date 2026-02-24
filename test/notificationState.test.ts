import { describe, expect, test } from "vitest";

import {
  makeEventKey,
  shouldNotifyTransition,
  toAlertLevel,
  type NotificationState,
} from "../src/core/notificationState.js";

describe("notification state", () => {
  const row = {
    provider: "codex" as const,
    window: "codex:5h",
    used: null,
    limit: null,
    usedPercent: 85,
    resetAt: null,
    source: "structured" as const,
    confidence: "high" as const,
    stale: false,
    notes: null,
  };

  test("maps alert level by thresholds", () => {
    expect(
      toAlertLevel(
        {
          ...row,
          usedPercent: 30,
        },
        { warningPercent: 80, criticalPercent: 95 }
      )
    ).toBe("normal");
    expect(toAlertLevel(row, { warningPercent: 80, criticalPercent: 95 })).toBe(
      "warning"
    );
  });

  test("event key stable", () => {
    expect(makeEventKey(row)).toBe("codex:codex:5h");
  });

  test("notifies on transition with cooldown", () => {
    const prev: NotificationState = {
      eventKey: makeEventKey(row),
      level: "warning",
      lastNotifiedAt: "2026-02-24T00:00:00.000Z",
    };
    const next: NotificationState = {
      eventKey: makeEventKey(row),
      level: "critical",
      lastNotifiedAt: null,
    };
    expect(
      shouldNotifyTransition(prev, next, 60_000, Date.parse("2026-02-24T00:00:10.000Z"))
    ).toBe(true);
  });
});
