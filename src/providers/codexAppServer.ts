import { spawn, type ChildProcessWithoutNullStreams } from "node:child_process";
import readline from "node:readline";

import { z } from "zod";

import type { NormalizedRow } from "../types.js";
import { toIsoFromEpochSeconds } from "../time.js";

const JsonRpcResponseSchema = z.object({
  id: z.union([z.number(), z.string()]),
  result: z.unknown().optional(),
  error: z
    .object({
      code: z.number(),
      message: z.string(),
      data: z.unknown().optional(),
    })
    .optional(),
});

const RateLimitWindowSchema = z.object({
  usedPercent: z.number().int(),
  windowDurationMins: z.number().int().nullable().optional(),
  resetsAt: z.number().int().nullable().optional(),
});

const RateLimitSnapshotSchema = z.object({
  limitId: z.string().nullable().optional(),
  limitName: z.string().nullable().optional(),
  planType: z.string().nullable().optional(),
  primary: RateLimitWindowSchema.nullable().optional(),
  secondary: RateLimitWindowSchema.nullable().optional(),
  credits: z
    .object({
      hasCredits: z.boolean(),
      unlimited: z.boolean(),
      balance: z.string().nullable().optional(),
    })
    .nullable()
    .optional(),
});

const GetAccountRateLimitsResponseSchema = z.object({
  rateLimits: RateLimitSnapshotSchema,
  rateLimitsByLimitId: z.record(z.string(), RateLimitSnapshotSchema).nullable().optional(),
});

function formatWindow(mins: number | null | undefined): string {
  if (!mins || mins <= 0) return "unknown";

  if (mins >= 295 && mins <= 305) return "5h";
  if (mins >= 10000 && mins <= 10100) return "weekly";
  if (mins % (60 * 24) === 0) return `${mins / (60 * 24)}d`;
  if (mins % 60 === 0) return `${mins / 60}h`;
  return `${mins}m`;
}

function snapshotToRows(snapshot: z.infer<typeof RateLimitSnapshotSchema>): NormalizedRow[] {
  const rows: NormalizedRow[] = [];

  const pushWindow = (labelPrefix: string, window: unknown) => {
    const w = RateLimitWindowSchema.safeParse(window);
    if (!w.success) return;

    const windowLabel = formatWindow(w.data.windowDurationMins);
    rows.push({
      provider: "codex",
      window: `${labelPrefix}:${windowLabel}`,
      used: null,
      limit: null,
      usedPercent: w.data.usedPercent,
      resetAt: toIsoFromEpochSeconds(w.data.resetsAt),
      source: "structured",
      confidence: "high",
      stale: false,
      notes: null,
    });
  };

  if (snapshot.primary) pushWindow("codex", snapshot.primary);
  if (snapshot.secondary) pushWindow("codex", snapshot.secondary);

  return rows;
}

export type CodexAppServerRateLimitsResult = {
  rows: NormalizedRow[];
  debug: {
    spawnCommand: string;
    limitId: string | null;
    limitName: string | null;
    planType: string | null;
    hadRateLimitsByLimitId: boolean;
  };
};

export async function getCodexRateLimitsViaAppServer(opts: {
  codexCommand?: string;
  timeoutMs?: number;
}): Promise<CodexAppServerRateLimitsResult> {
  const codexCommand = opts.codexCommand?.trim() || "codex";
  const timeoutMs = opts.timeoutMs ?? 10_000;

  const { proc, spawnCommand } = spawnPossiblyThroughBash(codexCommand, ["app-server"]);

  const rl = readline.createInterface({ input: proc.stdout });

  const send = (msg: unknown) => {
    proc.stdin.write(`${JSON.stringify(msg)}\n`);
  };

  const initializeId = 1;
  const rateLimitsId = 2;

  const responsePromise = new Promise<unknown>((resolve, reject) => {
    const t = setTimeout(() => {
      reject(new Error(`codex app-server timed out after ${timeoutMs}ms`));
    }, timeoutMs);

    const onProcError = (err: Error) => {
      clearTimeout(t);
      rl.removeAllListeners();
      reject(err);
    };

    proc.once("error", onProcError);

    const onLine = (line: string) => {
      let msg: unknown;
      try {
        msg = JSON.parse(line);
      } catch {
        return;
      }

      const parsed = JsonRpcResponseSchema.safeParse(msg);
      if (!parsed.success) return;
      if (parsed.data.id === initializeId && parsed.data.error) {
        clearTimeout(t);
        rl.off("line", onLine);
        reject(new Error(`codex initialize failed: ${parsed.data.error.message}`));
        return;
      }
      if (parsed.data.id === rateLimitsId) {
        clearTimeout(t);
        rl.off("line", onLine);
        proc.off("error", onProcError);
        if (parsed.data.error) {
          reject(new Error(`codex account/rateLimits/read failed: ${parsed.data.error.message}`));
          return;
        }
        resolve(parsed.data.result);
      }
    };

    rl.on("line", onLine);
  });

  // Minimal handshake.
  send({
    method: "initialize",
    id: initializeId,
    params: {
      clientInfo: {
        name: "usage_status_cli",
        title: "Usage Status CLI",
        version: "0.1.0",
      },
    },
  });
  send({ method: "initialized", params: {} });
  send({ method: "account/rateLimits/read", id: rateLimitsId });

  try {
    const result = await responsePromise;
    const parsed = GetAccountRateLimitsResponseSchema.parse(result);

    const byLimitId = parsed.rateLimitsByLimitId ?? null;
    const preferCodexSnapshot = byLimitId?.codex ?? null;
    const snapshot = preferCodexSnapshot ?? parsed.rateLimits;

    return {
      rows: snapshotToRows(snapshot),
      debug: {
        spawnCommand,
        limitId: snapshot.limitId ?? null,
        limitName: snapshot.limitName ?? null,
        planType: snapshot.planType ?? null,
        hadRateLimitsByLimitId: byLimitId != null,
      },
    };
  } finally {
    rl.close();
    proc.kill();
  }
}

function hasBashOnPath(): boolean {
  const path = process.env.PATH ?? "";
  // Very rough; used only as a hint.
  return /\\usr\\bin|\\Git\\bin|\\msys|\\mingw|\/usr\/bin|\/bin/i.test(path);
}

function bashEscape(s: string): string {
  // Wrap in single quotes, escape embedded single quotes.
  return `'${s.replaceAll("'", `'\\''`)}'`;
}

function spawnPossiblyThroughBash(
  command: string,
  args: string[]
): { proc: ChildProcessWithoutNullStreams; spawnCommand: string } {
  const isWin = process.platform === "win32";

  if (isWin) {
    const winCmd = command === "codex" ? "codex.cmd" : command;
    return {
      proc: spawn(winCmd, args, {
        shell: true,
        windowsHide: true,
        stdio: ["pipe", "pipe", "pipe"],
        env: process.env,
      }),
      spawnCommand: `${winCmd} ${args.join(" ")} (shell=true)`.trim(),
    };
  }

  const preferBash = hasBashOnPath();

  if (preferBash) {
    const cmd = [command, ...args].map(bashEscape).join(" ");
    return {
      proc: spawn("bash", ["-lc", cmd], {
        stdio: ["pipe", "pipe", "pipe"],
        env: process.env,
      }),
      spawnCommand: `bash -lc ${cmd}`,
    };
  }

  return {
    proc: spawn(command, args, {
      stdio: ["pipe", "pipe", "pipe"],
      env: process.env,
    }),
    spawnCommand: `${command} ${args.join(" ")}`.trim(),
  };
}
