import { run } from "./main.js";

type CliOptions = {
  json: boolean;
  refresh: boolean;
  debug: boolean;
  view: "table" | "graph";
  providers: "all" | Set<"claude" | "codex">;
};

function printHelp(): void {
  // Keep help stable for parsing/tests.
  process.stdout.write(`qmeter

Unified usage + reset status for Claude Code and Codex.

Usage:
  qmeter [options]

Options:
  --json                 Print machine-readable JSON
  --refresh              Bypass cache
  --debug                Print debug diagnostics (no secrets)
  --view <mode>          View mode: table,graph (default: table)
  --providers <list>     Providers to query: claude,codex,all (default: all)
  -h, --help             Show help
`);
}

function parseArgs(argv: string[]): CliOptions {
  const opts: CliOptions = {
    json: false,
    refresh: false,
    debug: false,
    view: "table",
    providers: "all",
  };

  for (let i = 0; i < argv.length; i++) {
    const a = argv[i];
    if (a === "-h" || a === "--help") {
      printHelp();
      process.exit(0);
    }
    if (a === "--json") {
      opts.json = true;
      continue;
    }
    if (a === "--refresh") {
      opts.refresh = true;
      continue;
    }
    if (a === "--debug") {
      opts.debug = true;
      continue;
    }
    if (a === "--providers") {
      const raw = argv[i + 1];
      if (!raw || raw.startsWith("-")) {
        throw new Error("--providers requires a value (claude,codex,all)");
      }
      i++;
      if (raw === "all") {
        opts.providers = "all";
        continue;
      }
      const set = new Set<"claude" | "codex">();
      for (const part of raw.split(",").map((s) => s.trim()).filter(Boolean)) {
        if (part === "claude" || part === "codex") {
          set.add(part);
        } else {
          throw new Error(`Unknown provider: ${part}`);
        }
      }
      if (set.size === 0) {
        throw new Error("--providers must include at least one of: claude,codex,all");
      }
      opts.providers = set;
      continue;
    }

    if (a === "--view") {
      const raw = argv[i + 1];
      if (!raw || raw.startsWith("-")) {
        throw new Error("--view requires a value (table,graph)");
      }
      i++;
      if (raw === "table" || raw === "graph") {
        opts.view = raw;
      } else {
        throw new Error(`Unknown view: ${raw}`);
      }
      continue;
    }

    throw new Error(`Unknown argument: ${a}`);
  }

  return opts;
}

async function main(): Promise<void> {
  try {
    const opts = parseArgs(process.argv.slice(2));
    const exitCode = await run(opts);
    process.exit(exitCode);
  } catch (err) {
    const msg = err instanceof Error ? err.message : String(err);
    process.stderr.write(`${msg}\n\n`);
    printHelp();
    process.exit(2);
  }
}

void main();
