import { describe, expect, test } from "vitest";

import {
  evaluateNotificationPolicy,
  isInQuietHours,
  type NotificationPolicyConfig,
} from "../src/core/notificationPolicy.js";

const cfg: NotificationPolicyConfig = {
  thresholds: { warningPercent: 80, criticalPercent: 95 },
  cooldownMs: 60_000,
  hysteresisPercent: 2,
  quietHours: {
    enabled: false,
    startHour: 22,
    endHour: 8,
  },
};

function row(percent: number) {
  return {
    provider: "codex" as const,
    window: "codex:5h",
    used: null,
    limit: null,
    usedPercent: percent,
    resetAt: null,
    source: "structured" as const,
    confidence: "high" as const,
    stale: false,
    notes: null,
  };
}

describe("notification policy", () => {
  function localAt(hour: number): Date {
    const d = new Date(2026, 1, 24, 0, 0, 0, 0);
    d.setHours(hour, 0, 0, 0);
    return d;
  }

  test("alerts on threshold crossing", () => {
    const t0 = new Date("2026-02-24T00:00:00.000Z");
    const s1 = evaluateNotificationPolicy([row(79)], {}, cfg, t0);
    expect(s1.events).toHaveLength(0);

    const s2 = evaluateNotificationPolicy(
      [row(81)],
      s1.nextState,
      cfg,
      new Date("2026-02-24T00:00:10.000Z")
    );
    expect(s2.events).toHaveLength(1);
    expect(s2.events[0]?.level).toBe("warning");
    expect(s2.events[0]?.reason).toBe("transition");
  });

  test("suppresses during cooldown and re-notifies after cooldown", () => {
    const keyState = {
      "codex:codex:5h": {
        eventKey: "codex:codex:5h",
        level: "warning" as const,
        lastNotifiedAt: "2026-02-24T00:00:00.000Z",
      },
    };

    const early = evaluateNotificationPolicy(
      [row(85)],
      keyState,
      cfg,
      new Date("2026-02-24T00:00:30.000Z")
    );
    expect(early.events).toHaveLength(0);

    const late = evaluateNotificationPolicy(
      [row(85)],
      keyState,
      cfg,
      new Date("2026-02-24T00:02:00.000Z")
    );
    expect(late.events).toHaveLength(1);
    expect(late.events[0]?.reason).toBe("cooldown");
  });

  test("hysteresis prevents warning drop near threshold", () => {
    const prev = {
      "codex:codex:5h": {
        eventKey: "codex:codex:5h",
        level: "warning" as const,
        lastNotifiedAt: null,
      },
    };

    // warning=80, hysteresis=2 => remain warning while >=78
    const keep = evaluateNotificationPolicy([row(79)], prev, cfg, new Date());
    expect(keep.nextState["codex:codex:5h"]?.level).toBe("warning");

    const drop = evaluateNotificationPolicy([row(77)], prev, cfg, new Date());
    expect(drop.nextState["codex:codex:5h"]?.level).toBe("normal");
  });

  test("quiet hours across midnight", () => {
    const q = { enabled: true, startHour: 22, endHour: 8 };
    expect(isInQuietHours(q, localAt(23))).toBe(true);
    expect(isInQuietHours(q, localAt(7))).toBe(true);
    expect(isInQuietHours(q, localAt(12))).toBe(false);
  });
});
