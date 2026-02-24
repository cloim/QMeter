import fs from "node:fs/promises";
import os from "node:os";
import path from "node:path";

import { z } from "zod";

const TraySettingsSchema = z.object({
  startupEnabled: z.boolean(),
  refreshIntervalMs: z.number().int().min(5000).max(60 * 60 * 1000),
  visibleProviders: z.object({
    claude: z.boolean(),
    codex: z.boolean(),
  }),
  notification: z.object({
    warningPercent: z.number().min(0).max(100),
    criticalPercent: z.number().min(0).max(100),
    cooldownMinutes: z.number().int().min(1).max(24 * 60),
    hysteresisPercent: z.number().min(0).max(30),
    quietHours: z.object({
      enabled: z.boolean(),
      startHour: z.number().int().min(0).max(23),
      endHour: z.number().int().min(0).max(23),
    }),
  }),
});

export type TraySettings = z.infer<typeof TraySettingsSchema>;

export const DEFAULT_TRAY_SETTINGS: TraySettings = {
  startupEnabled: false,
  refreshIntervalMs: 60_000,
  visibleProviders: {
    claude: true,
    codex: true,
  },
  notification: {
    warningPercent: 80,
    criticalPercent: 95,
    cooldownMinutes: 60,
    hysteresisPercent: 2,
    quietHours: {
      enabled: false,
      startHour: 22,
      endHour: 8,
    },
  },
};

function defaultConfigDir(): string {
  if (process.platform === "win32") {
    const base = process.env.APPDATA;
    if (base && base.trim()) return base;
  }
  const xdg = process.env.XDG_CONFIG_HOME;
  if (xdg && xdg.trim()) return xdg;
  return path.join(os.homedir(), ".config");
}

export function getTraySettingsPath(): string {
  const override = process.env.USAGE_STATUS_TRAY_SETTINGS_PATH;
  if (override && override.trim()) return override;
  return path.join(defaultConfigDir(), "qmeter", "tray-settings.v1.json");
}

export async function loadTraySettings(): Promise<TraySettings> {
  const p = getTraySettingsPath();
  try {
    const raw = await fs.readFile(p, "utf8");
    const parsed = TraySettingsSchema.parse(JSON.parse(raw));
    return parsed;
  } catch {
    return DEFAULT_TRAY_SETTINGS;
  }
}

export async function saveTraySettings(settings: TraySettings): Promise<void> {
  const parsed = TraySettingsSchema.parse(settings);
  const p = getTraySettingsPath();
  await fs.mkdir(path.dirname(p), { recursive: true });
  await fs.writeFile(p, JSON.stringify(parsed, null, 2), "utf8");
}
