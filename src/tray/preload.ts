import { contextBridge, ipcRenderer } from "electron";

type SnapshotPayload = {
  fetchedAt: string;
  rows: Array<{
    provider: string;
    window: string;
    usedPercent: number | null;
    resetAt: string | null;
    source: string;
    confidence: string;
    stale: boolean;
    notes: string | null;
  }>;
  errors: Array<{
    provider: string;
    type: string;
    message: string;
    actionable: string | null;
  }>;
};

type TraySettingsPayload = {
  refreshIntervalMs: number;
  visibleProviders: {
    claude: boolean;
    codex: boolean;
  };
};

contextBridge.exposeInMainWorld("usageTray", {
  getSnapshot: (): Promise<SnapshotPayload> => ipcRenderer.invoke("tray:get-snapshot"),
  refresh: (): Promise<SnapshotPayload> => ipcRenderer.invoke("tray:refresh"),
  getSettings: (): Promise<TraySettingsPayload> => ipcRenderer.invoke("tray:get-settings"),
  saveSettings: (settings: TraySettingsPayload): Promise<TraySettingsPayload> =>
    ipcRenderer.invoke("tray:save-settings", settings),
  setHeight: (height: number): Promise<void> => ipcRenderer.invoke("tray:set-height", height),
  onSnapshot: (handler: (s: SnapshotPayload) => void) => {
    const listener = (_: unknown, payload: SnapshotPayload) => handler(payload);
    ipcRenderer.on("tray:snapshot-updated", listener);
    return () => ipcRenderer.off("tray:snapshot-updated", listener);
  },
});
