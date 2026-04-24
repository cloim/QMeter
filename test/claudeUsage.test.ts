import { describe, expect, test } from "vitest";

import {
  cleanClaudeScreenText,
  parseClaudeUsageFromScreen,
} from "../src/parsers/claudeUsage.js";

describe("claude usage parser", () => {
  test("cleans ANSI and normalizes whitespace", () => {
    const raw = "\x1b[31mCurrent session\x1b[0m\r\n  90%   used\r\n";
    const clean = cleanClaudeScreenText(raw);
    expect(clean).toContain("Current session");
    expect(clean).toContain("90% used");
    expect(clean).not.toContain("\x1b");
  });

  test("extracts session + week rows when present", () => {
    const screen = cleanClaudeScreenText(`
Settings: Usage
Current session
  90% used
  Resets 3am

Current week (all models)
  21% used
  Resets Feb 28, 10am
`);

    const parsed = parseClaudeUsageFromScreen(screen);
    expect(parsed.errors).toHaveLength(0);

    const session = parsed.rows.find((r) => r.window === "claude:session");
    const week = parsed.rows.find((r) => r.window === "claude:week(all-models)");
    expect(session?.usedPercent).toBe(90);
    expect(week?.usedPercent).toBe(21);
    expect(session?.source).toBe("parsed");
    expect(week?.source).toBe("parsed");
    const reset = session?.resetAt;
    expect(reset == null || Number.isFinite(Date.parse(reset))).toBe(true);
  });

  test("anchors session parsing to the actual section header", () => {
    const filler = "Context line with unrelated details.\n".repeat(40);
    const screen = cleanClaudeScreenText(`
/subagent-driven-development (superpowers) Use when executing implementation plans with independent tasks in the current session.
Choose a command to run.
${filler}

Current session
  7% used
  Resets 12pm (Asia/Seoul)

Current week (all models)
  1% used
  Resets Apr 26, 6pm (Asia/Seoul)
`);

    const parsed = parseClaudeUsageFromScreen(screen);
    const session = parsed.rows.find((r) => r.window === "claude:session");
    const week = parsed.rows.find((r) => r.window === "claude:week(all-models)");

    expect(parsed.errors).toHaveLength(0);
    expect(session?.usedPercent).toBe(7);
    expect(session?.notes).toBe("Resets 12pm (Asia/Seoul)");
    expect(week?.usedPercent).toBe(1);
  });

  test("returns parse-failed error when no rows found", () => {
    const parsed = parseClaudeUsageFromScreen("hello world");
    expect(parsed.rows).toHaveLength(0);
    expect(parsed.errors).toHaveLength(1);
    expect(parsed.errors[0]?.type).toBe("parse-failed");
  });
});
