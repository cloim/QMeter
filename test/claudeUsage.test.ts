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

  test("returns parse-failed error when no rows found", () => {
    const parsed = parseClaudeUsageFromScreen("hello world");
    expect(parsed.rows).toHaveLength(0);
    expect(parsed.errors).toHaveLength(1);
    expect(parsed.errors[0]?.type).toBe("parse-failed");
  });
});
