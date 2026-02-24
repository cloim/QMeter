import type { NormalizedError } from "../types.js";

import type { AcquireContext, Provider, ProviderResult } from "./provider.js";
import { getCodexRateLimitsViaAppServer } from "./codexAppServer.js";

export class CodexProvider implements Provider {
  public readonly id = "codex" as const;

  async acquire(_ctx: AcquireContext): Promise<ProviderResult> {
    try {
      const codexCommand = process.env.USAGE_STATUS_CODEX_COMMAND?.trim();
      const res = await getCodexRateLimitsViaAppServer({
        ...(codexCommand ? { codexCommand } : {}),
      });
      return {
        rows: res.rows,
        errors: [],
        debug: res.debug,
      };
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);

       const lowered = msg.toLowerCase();
       const type: NormalizedError["type"] =
         lowered.includes("enoent") || lowered.includes("not found")
           ? "not-installed"
           : lowered.includes("timed out") || lowered.includes("timeout")
             ? "timeout"
             : lowered.includes("unauthorized") || lowered.includes("forbidden")
               ? "auth-required"
               : "acquire-failed";

      const e: NormalizedError = {
        provider: "codex",
        type,
        message: msg,
        actionable:
          type === "not-installed"
            ? "install `codex` and ensure it is on PATH"
            : "run `codex` once and ensure you are logged in",
      };
      return { rows: [], errors: [e] };
    }
  }
}
