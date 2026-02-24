import type { NormalizedRow } from "../types.js";
import {
  makeEventKey,
  shouldNotifyTransition,
  type AlertLevel,
  type NotificationState,
  type NotificationThresholds,
} from "./notificationState.js";

export type QuietHours = {
  enabled: boolean;
  startHour: number;
  endHour: number;
};

export type NotificationPolicyConfig = {
  thresholds: NotificationThresholds;
  cooldownMs: number;
  hysteresisPercent: number;
  quietHours: QuietHours;
};

export type NotificationEvent = {
  eventKey: string;
  level: Exclude<AlertLevel, "normal">;
  row: NormalizedRow;
  reason: "transition" | "cooldown";
};

export type NotificationEvaluation = {
  events: NotificationEvent[];
  nextState: Record<string, NotificationState>;
};

function clampHour(h: number): number {
  if (!Number.isFinite(h)) return 0;
  const n = Math.floor(h);
  if (n < 0) return 0;
  if (n > 23) return 23;
  return n;
}

export function isInQuietHours(q: QuietHours, now = new Date()): boolean {
  if (!q.enabled) return false;
  const start = clampHour(q.startHour);
  const end = clampHour(q.endHour);
  const h = now.getHours();

  if (start === end) return true;
  if (start < end) {
    return h >= start && h < end;
  }
  return h >= start || h < end;
}

function levelWithHysteresis(
  usedPercent: number,
  prev: AlertLevel,
  cfg: NotificationPolicyConfig
): AlertLevel {
  const p = Math.max(0, Math.min(100, usedPercent));
  const warning = cfg.thresholds.warningPercent;
  const critical = cfg.thresholds.criticalPercent;
  const h = Math.max(0, cfg.hysteresisPercent);

  if (prev === "critical") {
    if (p >= critical - h) return "critical";
    if (p >= warning) return "warning";
    return "normal";
  }

  if (prev === "warning") {
    if (p >= critical) return "critical";
    if (p >= warning - h) return "warning";
    return "normal";
  }

  if (p >= critical) return "critical";
  if (p >= warning) return "warning";
  return "normal";
}

export function evaluateNotificationPolicy(
  rows: NormalizedRow[],
  prevState: Record<string, NotificationState>,
  cfg: NotificationPolicyConfig,
  now = new Date()
): NotificationEvaluation {
  const nextState: Record<string, NotificationState> = { ...prevState };
  const events: NotificationEvent[] = [];
  const quiet = isInQuietHours(cfg.quietHours, now);

  for (const row of rows) {
    if (row.usedPercent == null) continue;
    const eventKey = makeEventKey(row);
    const prev = prevState[eventKey] ?? null;
    const prevLevel: AlertLevel = prev?.level ?? "normal";
    const level = levelWithHysteresis(row.usedPercent, prevLevel, cfg);

    const candidate: NotificationState = {
      eventKey,
      level,
      lastNotifiedAt: prev?.lastNotifiedAt ?? null,
    };

    const shouldNotify =
      !quiet &&
      shouldNotifyTransition(
        prev,
        candidate,
        cfg.cooldownMs,
        now.getTime()
      );

    if (shouldNotify && level !== "normal") {
      const reason = prev && prev.level === level ? "cooldown" : "transition";
      events.push({
        eventKey,
        level,
        row,
        reason,
      });
      candidate.lastNotifiedAt = now.toISOString();
    }

    nextState[eventKey] = candidate;
  }

  return { events, nextState };
}
