import { collectSnapshot } from "../core/snapshot.js";
import { evaluateNotificationPolicy } from "../core/notificationPolicy.js";
import { loadNotificationState, saveNotificationState } from "./notificationStore.js";
import { loadTraySettings } from "./settings.js";

async function main(): Promise<void> {
  if (!process.env.USAGE_STATUS_FIXTURE) {
    process.env.USAGE_STATUS_FIXTURE = "demo";
  }

  const providers = ["claude", "codex"] as const;
  const { snapshot } = await collectSnapshot({
    refresh: true,
    debug: false,
    providers: [...providers],
  });

  const settings = await loadTraySettings();
  const prevState = await loadNotificationState();
  const evalResult = evaluateNotificationPolicy(
    snapshot.rows,
    prevState,
    {
      thresholds: {
        warningPercent: settings.notification.warningPercent,
        criticalPercent: settings.notification.criticalPercent,
      },
      cooldownMs: settings.notification.cooldownMinutes * 60_000,
      hysteresisPercent: settings.notification.hysteresisPercent,
      quietHours: settings.notification.quietHours,
    },
    new Date()
  );
  await saveNotificationState(evalResult.nextState);

  // Machine-readable readiness marker for automation.
  process.stdout.write("TRAY_READY=1\n");
  process.stdout.write(`ROWS=${snapshot.rows.length}\n`);
  process.stdout.write(`ERRORS=${snapshot.errors.length}\n`);
  process.stdout.write(`NOTIFY_EVENTS=${evalResult.events.length}\n`);
}

void main();
