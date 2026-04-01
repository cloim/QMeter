export type TrayWindowState = {
  hasWindow: boolean;
  isVisible: boolean;
};

export type TrayWindowTrigger = "toggle" | "blur";

export type TrayWindowAction = "create-show" | "show" | "hide-destroy" | "noop";

export type ChildProcessGoneLike = {
  type?: string;
  reason?: string;
  exitCode?: number;
  serviceName?: string;
  name?: string;
};

export type MemoryUsageLike = {
  rss: number;
  heapTotal: number;
  heapUsed: number;
  external: number;
};

function toMb(bytes: number): string {
  return `${(bytes / (1024 * 1024)).toFixed(1)}MB`;
}

export function resolveWindowAction(
  state: TrayWindowState,
  trigger: TrayWindowTrigger
): TrayWindowAction {
  if (trigger === "blur") {
    return state.hasWindow && state.isVisible ? "hide-destroy" : "noop";
  }

  if (!state.hasWindow) return "create-show";
  if (!state.isVisible) return "show";
  return "hide-destroy";
}

export function formatChildProcessGoneDetail(detail: ChildProcessGoneLike): string {
  const parts = [
    `type=${detail.type ?? "unknown"}`,
    `reason=${detail.reason ?? "unknown"}`,
  ];

  if (typeof detail.exitCode === "number") {
    parts.push(`exit=${detail.exitCode}`);
  }
  if (detail.serviceName) {
    parts.push(`service=${detail.serviceName}`);
  }
  if (detail.name) {
    parts.push(`name=${detail.name}`);
  }

  return parts.join(" ");
}

export function formatMemoryUsageSummary(memory: MemoryUsageLike): string {
  const heapUsedMb = (memory.heapUsed / (1024 * 1024)).toFixed(1);
  const heapTotalMb = (memory.heapTotal / (1024 * 1024)).toFixed(1);
  return `rss=${toMb(memory.rss)} heap=${heapUsedMb}/${heapTotalMb}MB external=${toMb(memory.external)}`;
}
