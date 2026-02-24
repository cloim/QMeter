import type { NormalizedError, NormalizedRow } from "../types.js";
import { parseLocalResetAt } from "../time.js";

function stripAnsi(input: string): string {
  // Roughly matches ANSI escape codes and OSC sequences.
  return input
    .replaceAll(/\x1b\[[0-?]*[ -/]*[@-~]/g, "")
    .replaceAll(/\x1b\][^\x07]*\x07/g, "")
    .replaceAll(/\x1b\][^\x1b]*\x1b\\/g, "");
}

function normalizeWhitespace(s: string): string {
  return s
    .replaceAll("\r\n", "\n")
    .replaceAll("\r", "\n")
    .replaceAll(/\u00a0/g, " ")
    .replaceAll(/[ \t]+/g, " ")
    .replaceAll(/\n{3,}/g, "\n\n");
}

export function cleanClaudeScreenText(raw: string): string {
  return normalizeWhitespace(stripAnsi(raw));
}

function parsePercentNear(text: string, anchor: RegExp): number | null {
  const idx = text.search(anchor);
  if (idx < 0) return null;
  const window = text.slice(idx, idx + 600);
  const m = window.match(/(\d{1,3})%\s*used/i);
  if (!m) return null;
  const n = Number(m[1]);
  if (!Number.isFinite(n) || n < 0 || n > 100) return null;
  return n;
}

function parseResetLineNear(text: string, anchor: RegExp): {
  raw: string | null;
  iso: string | null;
} {
  const idx = text.search(anchor);
  if (idx < 0) return { raw: null, iso: null };
  const window = text.slice(idx, idx + 1200);
  // Common patterns:
  // - "Resets 3am (Asia/Seoul)"
  // - "Resets Feb 28, 10am (Asia/Seoul)"
  const m = window.match(/Resets\s+([^\n]+?)(?:\s*\(([^)]+)\))?\s*(?:\n|$)/i);
  if (!m) return { raw: null, iso: null };

  const raw = m[0].trim();
  const body = (m[1] ?? "").trim();
  const tz = (m[2] ?? "").trim();
  const systemTz = Intl.DateTimeFormat().resolvedOptions().timeZone;

  // Only convert when timezone label matches the system time zone.
  if (tz && tz !== systemTz) return { raw, iso: null };

  const iso = parseLocalResetAt(body, new Date());
  return { raw, iso };
}

export function parseClaudeUsageFromScreen(screenText: string): {
  rows: NormalizedRow[];
  errors: NormalizedError[];
} {
  const rows: NormalizedRow[] = [];
  const errors: NormalizedError[] = [];

  const sessionAnchor = /Current session/i;
  const weekAnchor = /Current week \(all models\)/i;

  const sessionUsed = parsePercentNear(screenText, sessionAnchor);
  const sessionReset = parseResetLineNear(screenText, sessionAnchor);
  if (sessionUsed != null) {
    rows.push({
      provider: "claude",
      window: "claude:session",
      used: null,
      limit: null,
      usedPercent: sessionUsed,
      resetAt: sessionReset.iso,
      source: "parsed",
      confidence: "medium",
      stale: false,
      notes: sessionReset.raw,
    });
  }

  const weekUsed = parsePercentNear(screenText, weekAnchor);
  const weekReset = parseResetLineNear(screenText, weekAnchor);
  if (weekUsed != null) {
    rows.push({
      provider: "claude",
      window: "claude:week(all-models)",
      used: null,
      limit: null,
      usedPercent: weekUsed,
      resetAt: weekReset.iso,
      source: "parsed",
      confidence: "medium",
      stale: false,
      notes: weekReset.raw,
    });
  }

  if (rows.length === 0) {
    errors.push({
      provider: "claude",
      type: "parse-failed",
      message: "Failed to extract usage from /usage screen output",
      actionable: "run `claude`, run /usage, and ensure you are logged in",
    });
  }

  return { rows, errors };
}
