import type { NormalizedSnapshot } from "./types.js";

function padRight(s: string, width: number): string {
  if (s.length >= width) return s;
  return s + " ".repeat(width - s.length);
}

function clampText(s: string, width: number): string {
  if (s.length <= width) return s;
  if (width <= 1) return s.slice(0, width);
  return s.slice(0, width - 1) + "…";
}

function renderErrors(snapshot: NormalizedSnapshot): string[] {
  if (snapshot.errors.length === 0) return [];

  const lines: string[] = [];
  lines.push("");
  lines.push("Errors:");
  for (const e of snapshot.errors) {
    const action = e.actionable ? ` (next: ${e.actionable})` : "";
    lines.push(`- ${e.provider}: ${e.type}: ${e.message}${action}`);
  }
  return lines;
}

function makeBar(percent: number, width = 24): string {
  const p = Math.max(0, Math.min(100, Math.round(percent)));
  const filled = Math.round((p / 100) * width);
  return `${"█".repeat(filled)}${"░".repeat(width - filled)}`;
}

function prettyWindowTitle(provider: string, window: string): string {
  if (provider === "claude" && window === "claude:session") return "Claude Session limit";
  if (provider === "claude" && window === "claude:week(all-models)") {
    return "Claude Week limit";
  }
  if (provider === "codex" && window === "codex:5h") return "Codex Session limit";
  if (provider === "codex" && window === "codex:weekly") return "Codex Week limit";
  return window;
}

function prettyResetLabel(resetAt: string | null, notes: string | null): string {
  if (resetAt) {
    const ts = Date.parse(resetAt);
    if (Number.isFinite(ts)) {
      const formatted = new Intl.DateTimeFormat(undefined, {
        dateStyle: "medium",
        timeStyle: "short",
      }).format(new Date(ts));
      return `Resets ${formatted}`;
    }
  }
  if (notes) return `Resets ${notes.replace(/^Resets\s+/i, "")}`;
  return "Resets unknown";
}

export function renderGraph(snapshot: NormalizedSnapshot): string {
  const lines: string[] = [];
  lines.push(`Usage Snapshot @ ${snapshot.fetchedAt}`);
  lines.push("");

  if (snapshot.rows.length === 0) {
    lines.push("(no rows)");
  } else {
    for (const r of snapshot.rows) {
      const percent = r.usedPercent;
      const title = prettyWindowTitle(r.provider, r.window);
      const reset = prettyResetLabel(r.resetAt, r.notes);
      const meta = `${r.source}/${r.confidence}${r.stale ? "/stale" : ""}`;

      lines.push(title);
      if (percent != null) {
        lines.push(`  ${makeBar(percent, 32)}  ${Math.round(percent)}% used`);
      } else {
        const usageLabel =
          r.used != null && r.limit != null ? `${r.used}/${r.limit}` : "unknown";
        lines.push(`  ${usageLabel} used`);
      }
      lines.push(`  ${reset}`);
      lines.push(`  Source ${meta}`);
      lines.push("");
    }
    while (lines.length > 0 && lines[lines.length - 1] === "") lines.pop();
  }

  lines.push(...renderErrors(snapshot));
  return lines.join("\n");
}

export function renderTable(snapshot: NormalizedSnapshot): string {
  const rows = snapshot.rows.map((r) => {
    const usage =
      r.usedPercent != null
        ? `${Math.round(r.usedPercent)}%`
        : r.used != null && r.limit != null
          ? `${r.used}/${r.limit}`
          : "?";
    const reset = r.resetAt ?? "?";
    const meta = `${r.source}/${r.confidence}${r.stale ? "/stale" : ""}`;
    return {
      provider: r.provider,
      window: r.window,
      usage,
      reset,
      meta,
    };
  });

  const cols = {
    provider: 6,
    window: 16,
    usage: 10,
    reset: 25,
    meta: 18,
  };

  const header = [
    padRight("PROV", cols.provider),
    padRight("WINDOW", cols.window),
    padRight("USAGE", cols.usage),
    padRight("RESET_AT", cols.reset),
    padRight("META", cols.meta),
  ].join(" ");
  const sep = "-".repeat(header.length);

  const lines: string[] = [header, sep];
  if (rows.length === 0) {
    lines.push("(no rows)");
  } else {
    for (const r of rows) {
      lines.push(
        [
          padRight(clampText(r.provider, cols.provider), cols.provider),
          padRight(clampText(r.window, cols.window), cols.window),
          padRight(clampText(r.usage, cols.usage), cols.usage),
          padRight(clampText(r.reset, cols.reset), cols.reset),
          padRight(clampText(r.meta, cols.meta), cols.meta),
        ].join(" ")
      );
    }
  }

  lines.push(...renderErrors(snapshot));

  return lines.join("\n");
}
