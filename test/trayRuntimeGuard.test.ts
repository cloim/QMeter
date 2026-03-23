import { describe, expect, test, vi } from "vitest";

import { formatRuntimeError, runGuardedTrayTask } from "../src/tray/runtimeGuard.js";

describe("tray runtime guard", () => {
  test("runGuardedTrayTask resolves true when task succeeds", async () => {
    const onError = vi.fn();

    const ok = await runGuardedTrayTask("refresh", async () => {}, onError);

    expect(ok).toBe(true);
    expect(onError).not.toHaveBeenCalled();
  });

  test("runGuardedTrayTask swallows task failure and reports it", async () => {
    const onError = vi.fn();

    const ok = await runGuardedTrayTask(
      "refresh",
      async () => {
        throw new Error("provider timed out");
      },
      onError
    );

    expect(ok).toBe(false);
    expect(onError).toHaveBeenCalledTimes(1);
    expect(onError).toHaveBeenCalledWith(
      "refresh",
      "provider timed out",
      expect.any(Error)
    );
  });

  test("formatRuntimeError handles non-Error throw values", () => {
    expect(formatRuntimeError("boom")).toBe("boom");
    expect(formatRuntimeError(null)).toBe("Unknown error");
  });
});
