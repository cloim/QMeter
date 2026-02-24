import type { NormalizedRow } from "../types.js";

export type AlertLevel = "normal" | "warning" | "critical";

export type NotificationState = {
  eventKey: string;
  level: AlertLevel;
  lastNotifiedAt: string | null;
};

export type NotificationThresholds = {
  warningPercent: number;
  criticalPercent: number;
};

export function toAlertLevel(
  row: NormalizedRow,
  thresholds: NotificationThresholds
): AlertLevel {
  const p = row.usedPercent ?? 0;
  if (p >= thresholds.criticalPercent) return "critical";
  if (p >= thresholds.warningPercent) return "warning";
  return "normal";
}

export function makeEventKey(row: NormalizedRow): string {
  return `${row.provider}:${row.window}`;
}

export function shouldNotifyTransition(
  prev: NotificationState | null,
  next: NotificationState,
  cooldownMs: number,
  nowMs = Date.now()
): boolean {
  if (!prev) return next.level !== "normal";
  if (prev.level !== next.level) {
    return next.level !== "normal";
  }
  if (!prev.lastNotifiedAt) return false;
  const last = Date.parse(prev.lastNotifiedAt);
  if (!Number.isFinite(last)) return false;
  return nowMs - last >= cooldownMs;
}
