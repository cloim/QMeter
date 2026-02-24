import pty from "node-pty";

import type { NormalizedError, NormalizedRow } from "../types.js";
import {
  cleanClaudeScreenText,
  parseClaudeUsageFromScreen,
} from "../parsers/claudeUsage.js";
import type { AcquireContext, Provider, ProviderResult } from "./provider.js";

const DEFAULT_GIT_BASH_EXE = "C:/Program Files/Git/usr/bin/bash.exe";

export class ClaudeProvider implements Provider {
  public readonly id = "claude" as const;

  async acquire(ctx: AcquireContext): Promise<ProviderResult> {
    // Claude Code is a TUI; we drive it via a PTY and parse the rendered text.
    const cols = 140;
    const rows = 40;
    const cwd = process.cwd();

    const bashExe =
      process.env.USAGE_STATUS_BASH_EXE ??
      (process.platform === "win32" ? DEFAULT_GIT_BASH_EXE : "bash");

    const spawnFile = process.platform === "win32" ? bashExe : "claude";
    const spawnArgs =
      process.platform === "win32" ? ["-lc", "claude"] : ([] as string[]);

    let p: ReturnType<typeof pty.spawn>;
    try {
      p = pty.spawn(spawnFile, spawnArgs, {
        name: "xterm-color",
        cols,
        rows,
        cwd,
        env: process.env,
        ...(process.platform === "win32"
          ? {
              useConpty: false,
              useConptyDll: false,
            }
          : {}),
      });
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      const lowered = msg.toLowerCase();
      const type: NormalizedError["type"] =
        lowered.includes("enoent") || lowered.includes("not found")
          ? "not-installed"
          : "tty-unavailable";
      return {
        rows: [],
        errors: [
          {
            provider: "claude",
            type,
            message: msg,
            actionable:
              process.platform === "win32"
                ? `set USAGE_STATUS_BASH_EXE (default: ${DEFAULT_GIT_BASH_EXE})`
                : "ensure `claude` is on PATH",
          },
        ],
      };
    }

    let buf = "";
    const maxBuf = 600_000;
    const append = (d: string) => {
      buf += d;
      if (buf.length > maxBuf) buf = buf.slice(-maxBuf);
    };

    const start = Date.now();
    const timeoutMs = 25_000;

    const done = new Promise<ProviderResult>((resolve) => {
      let finished = false;

       const finish = (reason: "success" | "timeout") => {
         if (finished) return;
         finished = true;

         const clean = cleanClaudeScreenText(buf);
         const parsed = parseClaudeUsageFromScreen(clean);

         let errors = parsed.errors;
         if (reason === "timeout") {
           const timeoutErr: NormalizedError = {
             provider: "claude",
             type: "timeout",
             message: `claude /usage timed out after ${timeoutMs}ms`,
             actionable: "run `claude` and verify /usage is available and you are logged in",
           };
           errors = parsed.rows.length === 0 ? [timeoutErr] : [...errors, timeoutErr];
         }

        const debugLines = clean
          .split("\n")
          .map((l) => l.trimEnd())
          .filter((l) =>
            /(Current session|Current week \(all models\)|Resets\s+|%\s*used)/i.test(l)
          )
          .slice(-50)
          .join("\n");

         const out: ProviderResult = {
           rows: parsed.rows,
           errors,
         };
        if (ctx.debug) {
          out.debug = {
            reason,
            systemTimeZone: Intl.DateTimeFormat().resolvedOptions().timeZone,
            extractedLines: debugLines,
          };
        }

        try {
          // Close overlay + exit, best-effort.
          p.write("\x1b");
          setTimeout(() => {
            p.write("/exit");
            setTimeout(() => p.write("\r"), 300);
            setTimeout(() => p.kill(), 800);
          }, 300);
        } finally {
          resolve(out);
        }
      };

      const interval = setInterval(() => {
        if (Date.now() - start > timeoutMs) {
          clearInterval(interval);
          finish("timeout");
          return;
        }

        // Success condition: can extract both session + week rows.
         const clean = cleanClaudeScreenText(buf);
         const parsed = parseClaudeUsageFromScreen(clean);
        const hasSession = parsed.rows.some((r) => r.window === "claude:session");
        const hasWeek = parsed.rows.some((r) => r.window.startsWith("claude:week"));
        if (hasSession && hasWeek) {
          clearInterval(interval);
          finish("success");
        }
      }, 300);

      p.onData((d) => {
        append(d);
      });

      // Drive /usage.
      setTimeout(() => p.write("/usage"), 2500);
      setTimeout(() => p.write("\r"), 4000);
    });

    // Add some environment-aware diagnostics if startup itself fails.
    try {
      return await done;
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      const e: NormalizedError = {
        provider: "claude",
        type: "acquire-failed",
        message: msg,
        actionable:
          process.platform === "win32"
            ? `set USAGE_STATUS_BASH_EXE (default: ${DEFAULT_GIT_BASH_EXE})`
            : "ensure `claude` is on PATH",
      };
      return { rows: [], errors: [e] };
    }
  }
}
