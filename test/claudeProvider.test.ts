import { beforeEach, describe, expect, test, vi } from "vitest";

const spawnMock = vi.fn();

vi.mock("node-pty", () => ({
  default: {
    spawn: spawnMock,
  },
}));

function makeUsageScreen(): string {
  return [
    "Settings: Usage",
    "Current session",
    "  90% used",
    "  Resets 3am",
    "",
    "Current week (all models)",
    "  21% used",
    "  Resets Feb 28, 10am",
    "",
  ].join("\n");
}

describe("ClaudeProvider", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    vi.resetModules();
    spawnMock.mockReset();
  });

  test("cleans up PTY listeners and handles after a successful acquire", async () => {
    let onDataListener: ((chunk: string) => void) | undefined;
    let onExitListener: (() => void) | undefined;
    const onDataDispose = vi.fn();
    const onExitDispose = vi.fn();
    const kill = vi.fn();
    const destroy = vi.fn(() => {
      onExitListener?.();
    });
    const write = vi.fn((chunk: string | Buffer) => {
      if (String(chunk) === "\r" && onDataListener) {
        onDataListener(makeUsageScreen());
      }
    });

    spawnMock.mockReturnValue({
      onData: vi.fn((listener: (chunk: string) => void) => {
        onDataListener = listener;
        return { dispose: onDataDispose };
      }),
      onExit: vi.fn((listener: () => void) => {
        onExitListener = listener;
        return { dispose: onExitDispose };
      }),
      write,
      kill,
      destroy,
      removeAllListeners: vi.fn(),
    });

    const { ClaudeProvider } = await import("../src/providers/claudeProvider.js");
    const provider = new ClaudeProvider();

    const acquirePromise = provider.acquire({ refresh: true, debug: false });

    await vi.advanceTimersByTimeAsync(5_000);
    const result = await acquirePromise;

    expect(result.errors).toHaveLength(0);
    expect(result.rows).toHaveLength(2);
    expect(onDataDispose).toHaveBeenCalledTimes(1);
    expect(onExitDispose).toHaveBeenCalled();
    expect(kill).toHaveBeenCalledTimes(1);
    expect(destroy).toHaveBeenCalledTimes(1);
  });

  test("forces the legacy Windows PTY backend on Windows", async () => {
    let onDataListener: ((chunk: string) => void) | undefined;

    spawnMock.mockReturnValue({
      onData: vi.fn((listener: (chunk: string) => void) => {
        onDataListener = listener;
        return { dispose: vi.fn() };
      }),
      onExit: vi.fn(() => ({ dispose: vi.fn() })),
      write: vi.fn((chunk: string | Buffer) => {
        if (String(chunk) === "\r" && onDataListener) {
          onDataListener(makeUsageScreen());
        }
      }),
      kill: vi.fn(),
      destroy: vi.fn(),
      removeAllListeners: vi.fn(),
    });

    const originalPlatform = process.platform;
    Object.defineProperty(process, "platform", {
      value: "win32",
    });

    try {
      const { ClaudeProvider } = await import("../src/providers/claudeProvider.js");
      const provider = new ClaudeProvider();
      const acquirePromise = provider.acquire({ refresh: true, debug: false });
      await vi.advanceTimersByTimeAsync(7_000);
      await acquirePromise;
    } finally {
      Object.defineProperty(process, "platform", {
        value: originalPlatform,
      });
    }

    const options = spawnMock.mock.calls[0]?.[2];
    expect(options?.useConpty).toBe(false);
    expect(options?.useConptyDll).toBe(false);
  });
});
