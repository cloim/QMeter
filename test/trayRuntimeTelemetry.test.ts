import { describe, expect, test } from "vitest";

import {
  formatChildProcessGoneDetail,
  formatMemoryUsageSummary,
  resolveWindowAction,
} from "../src/tray/runtimeTelemetry.js";

describe("tray runtime telemetry", () => {
  test("resolveWindowAction lazily creates the popup on first toggle", () => {
    expect(resolveWindowAction({ hasWindow: false, isVisible: false }, "toggle")).toBe(
      "create-show"
    );
  });

  test("resolveWindowAction destroys the popup after blur to release renderer resources", () => {
    expect(resolveWindowAction({ hasWindow: true, isVisible: true }, "blur")).toBe(
      "hide-destroy"
    );
  });

  test("resolveWindowAction keeps hidden windows torn down", () => {
    expect(resolveWindowAction({ hasWindow: false, isVisible: false }, "blur")).toBe("noop");
  });

  test("formatChildProcessGoneDetail includes critical diagnostic fields", () => {
    expect(
      formatChildProcessGoneDetail({
        type: "GPU",
        reason: "crashed",
        exitCode: 133,
        serviceName: "gpu-process",
        name: "QMeter Helper (GPU)",
      })
    ).toBe("type=GPU reason=crashed exit=133 service=gpu-process name=QMeter Helper (GPU)");
  });

  test("formatMemoryUsageSummary reports rss and heap sizes in MB", () => {
    expect(
      formatMemoryUsageSummary({
        rss: 210 * 1024 * 1024,
        heapTotal: 64 * 1024 * 1024,
        heapUsed: 32 * 1024 * 1024,
        external: 8 * 1024 * 1024,
      })
    ).toBe("rss=210.0MB heap=32.0/64.0MB external=8.0MB");
  });
});
