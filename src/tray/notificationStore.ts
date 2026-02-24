import fs from "node:fs/promises";
import os from "node:os";
import path from "node:path";

import { z } from "zod";

import type { NotificationState } from "../core/notificationState.js";

const NotificationStateSchema = z.object({
  eventKey: z.string().min(1),
  level: z.enum(["normal", "warning", "critical"]),
  lastNotifiedAt: z.string().datetime({ offset: true }).nullable(),
});

const NotificationStoreSchema = z.object({
  version: z.literal(1),
  items: z.record(z.string(), NotificationStateSchema),
});

function defaultStateDir(): string {
  if (process.platform === "win32") {
    const base = process.env.LOCALAPPDATA;
    if (base && base.trim()) return base;
  }
  const xdg = process.env.XDG_STATE_HOME;
  if (xdg && xdg.trim()) return xdg;
  return path.join(os.homedir(), ".local", "state");
}

export function getNotificationStatePath(): string {
  const override = process.env.USAGE_STATUS_NOTIFICATION_STATE_PATH;
  if (override && override.trim()) return override;
  return path.join(defaultStateDir(), "qmeter", "notification-state.v1.json");
}

export async function loadNotificationState(): Promise<Record<string, NotificationState>> {
  const p = getNotificationStatePath();
  try {
    const raw = await fs.readFile(p, "utf8");
    const parsed = NotificationStoreSchema.parse(JSON.parse(raw));
    return parsed.items;
  } catch {
    return {};
  }
}

export async function saveNotificationState(
  state: Record<string, NotificationState>
): Promise<void> {
  const parsed = NotificationStoreSchema.parse({ version: 1, items: state });
  const p = getNotificationStatePath();
  await fs.mkdir(path.dirname(p), { recursive: true });
  await fs.writeFile(p, JSON.stringify(parsed, null, 2), "utf8");
}
