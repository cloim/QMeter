import { app, BrowserWindow, ipcMain, Menu, Notification, Tray, nativeImage, screen } from "electron";
import path from "node:path";
import { fileURLToPath } from "node:url";

import { collectSnapshot } from "../core/snapshot.js";
import { evaluateNotificationPolicy } from "../core/notificationPolicy.js";
import { loadNotificationState, saveNotificationState } from "./notificationStore.js";
import { loadTraySettings, saveTraySettings, type TraySettings } from "./settings.js";
import type { NormalizedSnapshot, ProviderId } from "../types.js";

let tray: Tray | null = null;
let win: BrowserWindow | null = null;
let timer: NodeJS.Timeout | null = null;
let currentSettings: TraySettings | null = null;

let currentSnapshot: NormalizedSnapshot = {
  fetchedAt: new Date().toISOString(),
  rows: [],
  errors: [],
};

function estimateWindowHeight(snapshot: NormalizedSnapshot): number {
  // Deterministic sizing based on visible provider cards.
  const hasClaude = snapshot.rows.some((r) => r.provider === "claude");
  const hasCodex = snapshot.rows.some((r) => r.provider === "codex");
  const providerCount = (hasClaude ? 1 : 0) + (hasCodex ? 1 : 0);

  // header + content paddings + per-card fixed height + gaps
  // Tuned so the last card keeps the same bottom breathing room
  // as the first card's top breathing room.
  const headerHeight = 86;
  const bodyPaddingY = 60;
  const cardHeight = 238;
  const cardGap = 18;
  const n = Math.max(1, providerCount);
  const estimated = headerHeight + bodyPaddingY + cardHeight * n + cardGap * (n - 1);
  return Math.max(260, Math.min(820, estimated));
}

function applyWindowHeight(height: number): void {
  if (!win || win.isDestroyed()) return;

  const display = screen.getDisplayNearestPoint(
    tray?.getBounds() ?? { x: 0, y: 0, width: 0, height: 0 }
  );
  const maxByDisplay = Math.max(260, display.workArea.height - 40);
  const nextH = Math.max(220, Math.min(maxByDisplay, Math.round(height)));

  const size = win.getContentSize();
  const currW = size[0] ?? 560;
  const currH = size[1] ?? 400;
  if (Math.abs(currH - nextH) < 2) return;

  win.setContentSize(currW, nextH);
  if (win.isVisible()) positionWindow();
}

function selectedProvidersFromSettings(settings: TraySettings): ProviderId[] {
  const providers: ProviderId[] = [];
  if (settings.visibleProviders.claude) providers.push("claude");
  if (settings.visibleProviders.codex) providers.push("codex");
  return providers;
}

async function getSettings(): Promise<TraySettings> {
  if (currentSettings) return currentSettings;
  currentSettings = await loadTraySettings();
  return currentSettings;
}

function resetRefreshTimer(intervalMs: number): void {
  if (timer) clearInterval(timer);
  timer = setInterval(() => {
    void refreshSnapshot(false);
  }, intervalMs);
}

function resourceCandidates(fileName: string): string[] {
  const p = process as NodeJS.Process & { resourcesPath?: string };
  return [
    path.join(process.cwd(), "resources", fileName),
    path.join(process.cwd(), "dist", "resources", fileName),
    p.resourcesPath ? path.join(p.resourcesPath, "resources", fileName) : "",
    path.join(app.getAppPath(), "resources", fileName),
    fileURLToPath(new URL(`../../resources/${fileName}`, import.meta.url)),
  ].filter(Boolean);
}

function loadImageFromCandidates(fileName: string): Electron.NativeImage | null {
  for (const p of resourceCandidates(fileName)) {
    const img = nativeImage.createFromPath(p);
    if (!img.isEmpty()) return img;
  }
  return null;
}

function loadPngDataUrl(fileName: string): string {
  const img = loadImageFromCandidates(fileName);
  return img ? img.toDataURL() : "";
}

function makeIcon() {
  const icon = loadImageFromCandidates("QMeter.ico");
  if (icon) {
    return icon.resize({ width: 20, height: 20 });
  }

  const fallback = nativeImage.createFromPath(process.execPath);
  return fallback.resize({ width: 20, height: 20 });
}

function usagePercentLabel(snapshot: NormalizedSnapshot): string {
  const rows = snapshot.rows.filter((r) => r.usedPercent != null);
  if (rows.length === 0) return "QMeter";
  const avg = Math.round(rows.reduce((acc, r) => acc + (r.usedPercent ?? 0), 0) / rows.length);
  return `QMeter ${avg}%`;
}

function prettyTitle(provider: string, window: string): string {
  if (provider === "claude" && window === "claude:session") return "Claude Session limit";
  if (provider === "claude" && window === "claude:week(all-models)") return "Claude Week limit";
  if (provider === "codex" && window === "codex:5h") return "Codex Session limit";
  if (provider === "codex" && window === "codex:weekly") return "Codex Week limit";
  return window;
}

function fmtReset(iso: string | null, note: string | null): string {
  if (iso) return iso;
  if (note) return note.replace(/^Resets\s+/i, "");
  return "-";
}

function formatResetForWindow(window: string, raw: string): string {
  const t = Date.parse(raw);
  if (!Number.isFinite(t)) return raw;

  const d = new Date(t);
  const hh = String(d.getHours()).padStart(2, "0");
  const mm = String(d.getMinutes()).padStart(2, "0");
  const isWeek = /week/i.test(window);
  if (!isWeek) {
    return `~ ${hh}:${mm}`;
  }

  const yy = String(d.getFullYear() % 100).padStart(2, "0");
  const mo = String(d.getMonth() + 1).padStart(2, "0");
  const dd = String(d.getDate()).padStart(2, "0");
  const weekday = new Intl.DateTimeFormat(undefined, { weekday: "short" }).format(d);
  return `~ ${yy}.${mo}.${dd} (${weekday}) ${hh}:${mm}`;
}

function renderHtml(_snapshot: NormalizedSnapshot): string {
  const qmeterLogo = loadPngDataUrl("QMeter.png");
  const claudeLogo = loadPngDataUrl("Claude.png");
  const codexLogo = loadPngDataUrl("Codex.png");

  return `<!doctype html>
<html><head><meta charset="utf-8" />
<style>
  html,body{margin:0;padding:0;overflow:hidden;background:#05070A;color:#fff;font-family:"Segoe UI",sans-serif}
  body{box-sizing:border-box;width:100%;height:100vh;background:radial-gradient(600px 360px at 50% 10%,rgba(79,70,229,.18),transparent 60%),#05070A}
  .panel{position:relative;width:100vw;height:100vh;background:rgba(11,15,25,.86);backdrop-filter:blur(8px);border:1px solid rgba(255,255,255,.10);border-radius:0;overflow:hidden;box-shadow:0 28px 48px rgba(0,0,0,.45)}
  .header{padding:18px 22px;border-bottom:1px solid rgba(255,255,255,.06);display:flex;justify-content:space-between;align-items:center;background:linear-gradient(to bottom,rgba(255,255,255,.06),transparent)}
  .title{font-size:20px;font-weight:800;letter-spacing:.2px;display:flex;align-items:center;gap:8px}
  .titleLogo{width:18px;height:18px;object-fit:contain;display:block}
  .sub{font-size:11px;color:#9ca3af;margin-top:5px}
  .sub .mono{font-family:Consolas,monospace;color:#cfd6e6}
  .btn{border:1px solid rgba(255,255,255,.14);background:rgba(255,255,255,.06);color:#d1d5db;border-radius:12px;padding:9px 14px;font-weight:600;cursor:pointer;transition:all .2s;display:flex;align-items:center;gap:7px}
  .btn:hover{background:rgba(255,255,255,.11);color:#fff;border-color:rgba(255,255,255,.24)}
  .btn.secondary{padding:9px 11px}
  .btnSpin{display:inline-block;transition:transform .6s linear}
  .btn.spinning .btnSpin{transform:rotate(360deg)}
  .btnRow{display:flex;gap:8px;align-items:center}
  .content{padding:20px;display:grid;gap:18px}
  .provider{position:relative;background:#121827;border:1px solid rgba(255,255,255,.07);border-radius:18px;padding:18px;overflow:hidden}
  .provider::after{content:"";position:absolute;right:-18px;top:-18px;width:150px;height:150px;border-radius:999px;filter:blur(55px);opacity:.2;pointer-events:none}
  .provider.claude::after{background:#f97316}
  .provider.codex::after{background:#3b82f6}
  .providerHead{display:flex;justify-content:space-between;align-items:center;margin-bottom:12px}
  .providerName{display:flex;gap:10px;align-items:center;font-size:18px;font-weight:800;letter-spacing:.2px}
  .iconBox{width:26px;height:26px;border-radius:10px;display:inline-flex;align-items:center;justify-content:center;border:1px solid rgba(255,255,255,.2);font-size:15px;overflow:hidden}
  .iconBox img{width:16px;height:16px;object-fit:contain;display:block}
  .iconBox.claude{background:rgba(249,115,22,.12);color:#fb923c;border-color:rgba(249,115,22,.25)}
  .iconBox.codex{background:rgba(59,130,246,.12);color:#60a5fa;border-color:rgba(59,130,246,.25)}
  .warn{font-size:11px;padding:3px 8px;border-radius:999px;border:1px solid rgba(239,68,68,.3);background:rgba(239,68,68,.12);color:#fca5a5;font-weight:700}
  .progress{margin-bottom:16px}
  .progress:last-child{margin-bottom:0}
  .rowTop{display:flex;justify-content:space-between;align-items:flex-end;margin-bottom:9px}
  .labelWrap{display:flex;align-items:center;gap:8px}
  .miniIcon{width:18px;height:18px;border-radius:7px;border:1px solid rgba(255,255,255,.15);display:inline-flex;align-items:center;justify-content:center;font-size:11px;color:#9ca3af;background:rgba(255,255,255,.04)}
  .label{font-size:13px;font-weight:700;color:#d1d5db;letter-spacing:.2px}
  .pctWrap{display:flex;align-items:baseline;gap:2px}
  .pctNum{font-size:28px;line-height:1;font-weight:800;letter-spacing:-.3px}
  .pctSign{font-size:13px;color:#9ca3af;font-weight:700}
  .barTrack{height:10px;width:100%;background:#0f1423;border-radius:999px;overflow:hidden;border:1px solid rgba(255,255,255,.08);box-shadow:inset 0 1px 2px rgba(0,0,0,.45)}
  .barFill{height:100%;transition:width .55s ease;position:relative}
  .barFill::before{content:"";position:absolute;left:0;right:0;top:0;height:1px;background:rgba(255,255,255,.35)}
  .fillClaude{background:linear-gradient(90deg,#fbbf24,#f97316)}
  .fillCodex{background:linear-gradient(90deg,#60a5fa,#6366f1)}
  .fillWarn{background:linear-gradient(90deg,#f59e0b,#f97316)}
  .fillCritical{background:linear-gradient(90deg,#ef4444,#fb7185)}
  .rowBottom{display:flex;justify-content:space-between;align-items:center;margin-top:9px}
  .metaLabel{font-size:11px;color:#9ca3af;font-weight:700}
  .reset{font-family:Consolas,"Malgun Gothic",monospace;font-size:11px;color:#cbd5e1;padding:2px 0;min-width:165px;text-align:right}
  .empty{padding:16px;background:#121827;border:1px solid rgba(255,255,255,.08);border-radius:14px;color:#9ca3af}
  .settingsBackdrop{position:fixed;inset:0;background:rgba(2,5,12,.66);display:none;align-items:center;justify-content:center;z-index:50}
  .settingsBackdrop.show{display:flex}
  .settingsModal{width:380px;max-width:92vw;background:#101726;border:1px solid rgba(255,255,255,.14);border-radius:16px;padding:16px;box-shadow:0 20px 40px rgba(0,0,0,.45)}
  .settingsTitle{font-size:16px;font-weight:800;margin-bottom:12px}
  .field{margin-bottom:12px}
  .field label{display:block;font-size:12px;color:#a9b5cc;margin-bottom:6px}
  .select{width:100%;background:#0f1423;color:#e6ebf7;border:1px solid rgba(255,255,255,.14);border-radius:10px;padding:8px}
  .chk{accent-color:#6366f1}
  .checks{display:grid;gap:8px}
  .checkRow{display:flex;align-items:center;gap:8px;color:#dbe4ff;font-size:13px}
  .actions{display:flex;justify-content:flex-end;gap:8px;margin-top:10px}
  .saveHint{margin-top:8px;min-height:18px;font-size:12px;color:#a9b5cc}
  .saveHint.ok{color:#86efac}
  .saveHint.err{color:#fca5a5}
  .btn[disabled]{opacity:.6;cursor:not-allowed}
  .skeleton{position:relative;overflow:hidden}
  .skeleton::after{content:"";position:absolute;inset:0;transform:translateX(-100%);background:linear-gradient(90deg,transparent,rgba(255,255,255,.08),transparent);animation:shimmer 1.4s infinite}
  .sk-card{background:#121827;border:1px solid rgba(255,255,255,.08);border-radius:18px;padding:16px}
  .sk-line{height:12px;border-radius:8px;background:rgba(255,255,255,.08);margin-bottom:10px}
  .sk-line.sm{width:42%}
  .sk-line.md{width:70%}
  .sk-line.lg{width:100%}
  @keyframes shimmer { 100% { transform:translateX(100%); } }
</style>
</head>
<body>
  <div class="panel" id="panel">
    <div class="header">
      <div>
        <div class="title">${qmeterLogo ? `<img class="titleLogo" src="${qmeterLogo}" alt="QMeter" />` : ""}QMeter</div>
        <div class="sub">마지막 확인: <span class="mono" id="lastChecked">-</span></div>
      </div>
      <div class="btnRow">
        <button class="btn secondary" id="openSettings" title="환경설정">⚙</button>
        <button class="btn" id="refresh"><span class="btnSpin">↻</span><span>새로고침</span></button>
      </div>
    </div>
    <div class="content" id="cards">
      <div class="sk-card skeleton">
        <div class="sk-line sm"></div><div class="sk-line md"></div><div class="sk-line lg"></div><div class="sk-line md"></div>
      </div>
      <div class="sk-card skeleton">
        <div class="sk-line sm"></div><div class="sk-line md"></div><div class="sk-line lg"></div><div class="sk-line md"></div>
      </div>
    </div>
  </div>
  <div class="settingsBackdrop" id="settingsBackdrop">
    <div class="settingsModal">
      <div class="settingsTitle">환경설정</div>
      <div class="field">
        <label for="refreshInterval">새로고침 주기</label>
        <select class="select" id="refreshInterval">
          <option value="10000">10초</option>
          <option value="30000">30초</option>
          <option value="60000">1분</option>
          <option value="300000">5분</option>
          <option value="600000">10분</option>
        </select>
      </div>
      <div class="field">
        <label>보여줄 카드</label>
        <div class="checks">
          <label class="checkRow"><input class="chk" type="checkbox" id="showClaude" /> Claude</label>
          <label class="checkRow"><input class="chk" type="checkbox" id="showCodex" /> Codex</label>
        </div>
      </div>
      <div class="actions">
        <button class="btn secondary" id="cancelSettings">취소</button>
        <button class="btn" id="saveSettings">저장</button>
      </div>
      <div class="saveHint" id="saveHint"></div>
    </div>
  </div>
  <script>
    const settingsEls = {
      backdrop: null,
      refreshInterval: null,
      showClaude: null,
      showCodex: null,
      openBtn: null,
      cancelBtn: null,
      saveBtn: null,
      saveHint: null,
    };

    let bridgeRef = null;

    const showSkeleton = () => {
      document.getElementById('cards').innerHTML =
        '<div class="sk-card skeleton">' +
        '<div class="sk-line sm"></div><div class="sk-line md"></div><div class="sk-line lg"></div><div class="sk-line md"></div>' +
        '</div>' +
        '<div class="sk-card skeleton">' +
        '<div class="sk-line sm"></div><div class="sk-line md"></div><div class="sk-line lg"></div><div class="sk-line md"></div>' +
        '</div>';
    };

    const wait = (ms) => new Promise((r) => setTimeout(r, ms));

    const openSettings = async () => {
      if (!bridgeRef) return;
      const s = await bridgeRef.getSettings();
      settingsEls.refreshInterval.value = String(s.refreshIntervalMs);
      settingsEls.showClaude.checked = !!s.visibleProviders.claude;
      settingsEls.showCodex.checked = !!s.visibleProviders.codex;
      settingsEls.backdrop.classList.add('show');
    };

    const closeSettings = () => {
      settingsEls.backdrop.classList.remove('show');
      if (settingsEls.saveHint) {
        settingsEls.saveHint.textContent = '';
        settingsEls.saveHint.className = 'saveHint';
      }
    };

    const setSaving = (saving, hintText) => {
      if (settingsEls.saveBtn) {
        settingsEls.saveBtn.disabled = !!saving;
        settingsEls.saveBtn.textContent = saving ? '저장 중...' : '저장';
      }
      if (settingsEls.cancelBtn) settingsEls.cancelBtn.disabled = !!saving;
      if (settingsEls.openBtn) settingsEls.openBtn.disabled = !!saving;
      if (settingsEls.saveHint) {
        settingsEls.saveHint.textContent = hintText || '';
        settingsEls.saveHint.className = 'saveHint';
      }
    };

    const saveSettings = async () => {
      if (!bridgeRef) return;
      try {
        setSaving(true, '설정을 저장하고 있습니다...');
        const next = {
          refreshIntervalMs: Number(settingsEls.refreshInterval.value || 60000),
          visibleProviders: {
            claude: !!settingsEls.showClaude.checked,
            codex: !!settingsEls.showCodex.checked,
          },
        };
        const saved = await bridgeRef.saveSettings(next);
        settingsEls.refreshInterval.value = String(saved.refreshIntervalMs);
        settingsEls.showClaude.checked = !!saved.visibleProviders.claude;
        settingsEls.showCodex.checked = !!saved.visibleProviders.codex;
        closeSettings();
        showSkeleton();
        const [snapshot] = await Promise.all([bridgeRef.refresh(), wait(220)]);
        window.__render(snapshot);
        setSaving(false, '');
      } catch (e) {
        setSaving(false, '');
        if (settingsEls.saveHint) {
          settingsEls.saveHint.textContent = '저장 실패. 다시 시도해주세요.';
          settingsEls.saveHint.className = 'saveHint err';
        }
      }
    };

    window.__render = (snapshot) => {
      if (!snapshot || !Array.isArray(snapshot.rows)) {
        document.getElementById('cards').innerHTML = "<div class='empty'>No data</div>";
        return;
      }

      const label = (p) => p === 'claude' ? 'Claude Code' : p === 'codex' ? 'Codex CLI' : p;
      const limitLabel = (row) => /week/i.test(row.window) ? 'Week Limit' : 'Session Limit';
      const isRowFor = (row, kind) => kind === 'session' ? /session|:5h/i.test(row.window) : /week|weekly/i.test(row.window);
      const pctClass = (n) => n >= 95 ? 'fillCritical' : n >= 80 ? 'fillWarn' : '';
      const barClass = (provider, n) => {
        const warn = pctClass(n);
        if (warn) return warn;
        return provider === 'claude' ? 'fillClaude' : 'fillCodex';
      };
      const fmt = (windowName, raw) => {
        const d = new Date(raw);
        if (Number.isNaN(d.getTime())) return '-';
        const now = new Date();
        const mo = String(d.getMonth() + 1).padStart(2, '0');
        const dd = String(d.getDate()).padStart(2, '0');
        const wd = new Intl.DateTimeFormat(undefined,{weekday:'short'}).format(d);
        const hh = String(d.getHours()).padStart(2, '0');
        const mm = String(d.getMinutes()).padStart(2, '0');
        if (/week|weekly/i.test(windowName)) return mo + '.' + dd + ' (' + wd + ') ' + hh + ':' + mm;
        const day0 = new Date(now.getFullYear(), now.getMonth(), now.getDate());
        const day1 = new Date(d.getFullYear(), d.getMonth(), d.getDate());
        const diff = Math.round((day1.getTime() - day0.getTime()) / 86400000);
        if (diff === 0) return '오늘 ' + hh + ':' + mm;
        if (diff === 1) return '내일 ' + hh + ':' + mm;
        return mo + '.' + dd + ' ' + hh + ':' + mm;
      };
      const progress = (row) => {
        const p = Math.max(0, Math.min(100, Math.round(row.usedPercent || 0)));
        const resetRaw = row.resetAt || (row.notes ? row.notes.replace(/^Resets\s+/i, '') : '');
        const reset = fmt(row.window, resetRaw);
        const warn = p >= 80;
        const pctColor = p >= 95 ? '#f87171' : p >= 80 ? '#fbbf24' : '#ffffff';
        return '<div class="progress">' +
          '<div class="rowTop">' +
            '<div class="labelWrap"><span class="miniIcon">◷</span><span class="label">' + limitLabel(row) + '</span></div>' +
            '<div class="pctWrap"><span class="pctNum" style="color:' + pctColor + '">' + p + '</span><span class="pctSign">%</span></div>' +
          '</div>' +
          '<div class="barTrack"><div class="barFill ' + barClass(row.provider, p) + '" style="width:' + p + '%"></div></div>' +
          '<div class="rowBottom"><span class="metaLabel">초기화 일시</span><span class="reset">' + reset + '</span></div>' +
        '</div>';
      };

      const providers = ['claude','codex'];
      const cards = providers.map((provider) => {
        const rows = (snapshot.rows || []).filter(r => r.provider === provider);
        if (!rows.length) return '';
        const s = rows.find(r => isRowFor(r,'session'));
        const w = rows.find(r => isRowFor(r,'week'));
        const warning = rows.some(r => Math.round(r.usedPercent || 0) >= 80);
        let html = '<div class="provider ' + provider + '">';
        const providerIcon = provider === 'claude' ? ${JSON.stringify(claudeLogo)} : ${JSON.stringify(codexLogo)};
        const iconHtml = providerIcon ? ('<img src="' + providerIcon + '" alt="" />') : (provider === 'claude' ? '✦' : '⌘');
        html += '<div class="providerHead"><div class="providerName"><span class="iconBox ' + provider + '">' + iconHtml + '</span>' + label(provider) + '</div>';
        if (warning) html += '<span class="warn">Limit Warning</span>';
        html += '</div>';
        if (s) html += progress(s);
        if (w) html += progress(w);
        html += '</div>';
        return html;
      }).join('');

      document.getElementById('cards').innerHTML = cards || "<div class='empty'>No available providers</div>";

      const fetched = snapshot.fetchedAt ? new Date(snapshot.fetchedAt) : null;
      if (fetched && !Number.isNaN(fetched.getTime())) {
        document.getElementById('lastChecked').textContent =
          fetched.getFullYear() + '. ' +
          String(fetched.getMonth() + 1).padStart(2,'0') + '. ' +
          String(fetched.getDate()).padStart(2,'0') + ' ' +
          String(fetched.getHours()).padStart(2,'0') + ':' +
          String(fetched.getMinutes()).padStart(2,'0') + ':' +
          String(fetched.getSeconds()).padStart(2,'0');
      }

    };

    const setFatal = (msg) => {
      document.getElementById('cards').innerHTML = '<div class="empty">' + msg + '</div>';
    };

    (async () => {
      try {
        let bridge = window.usageTray;
        if (!bridge && typeof window.require === 'function') {
          const { ipcRenderer } = window.require('electron');
          bridge = {
            getSnapshot: () => ipcRenderer.invoke('tray:get-snapshot'),
            refresh: () => ipcRenderer.invoke('tray:refresh'),
            getSettings: () => ipcRenderer.invoke('tray:get-settings'),
            saveSettings: (settings) => ipcRenderer.invoke('tray:save-settings', settings),
            setHeight: (height) => ipcRenderer.invoke('tray:set-height', height),
            onSnapshot: (handler) => {
              const listener = (_evt, payload) => handler(payload);
              ipcRenderer.on('tray:snapshot-updated', listener);
              return () => ipcRenderer.off('tray:snapshot-updated', listener);
            },
          };
        }
        if (!bridge) {
          setFatal('Bridge unavailable');
          return;
        }
        bridgeRef = bridge;

        settingsEls.backdrop = document.getElementById('settingsBackdrop');
        settingsEls.refreshInterval = document.getElementById('refreshInterval');
        settingsEls.showClaude = document.getElementById('showClaude');
        settingsEls.showCodex = document.getElementById('showCodex');
        settingsEls.openBtn = document.getElementById('openSettings');
        settingsEls.cancelBtn = document.getElementById('cancelSettings');
        settingsEls.saveBtn = document.getElementById('saveSettings');
        settingsEls.saveHint = document.getElementById('saveHint');

        settingsEls.openBtn.addEventListener('click', () => { void openSettings(); });
        settingsEls.cancelBtn.addEventListener('click', closeSettings);
        settingsEls.saveBtn.addEventListener('click', () => { void saveSettings(); });
        settingsEls.backdrop.addEventListener('click', (e) => {
          if (e.target === settingsEls.backdrop) closeSettings();
        });

        showSkeleton();
        const [initial] = await Promise.all([bridge.getSnapshot(), wait(420)]);
        window.__render(initial);

        const refreshBtn = document.getElementById('refresh');
        refreshBtn.addEventListener('click', async () => {
          refreshBtn.classList.add('spinning');
          showSkeleton();
          const [s] = await Promise.all([bridge.refresh(), wait(260)]);
          window.__render(s);
          setTimeout(() => refreshBtn.classList.remove('spinning'), 220);
        });

        bridge.onSnapshot((s) => window.__render(s));
      } catch (err) {
        setFatal('Failed to load snapshot');
      }
    })();
  </script>
</body></html>`;
}

async function refreshSnapshot(forceRefresh: boolean): Promise<void> {
  const settings = await getSettings();
  const providers = selectedProvidersFromSettings(settings);
  const prevState = await loadNotificationState();

  const { snapshot } = await collectSnapshot({
    refresh: forceRefresh,
    debug: false,
    providers,
  });

  currentSnapshot = snapshot;
  applyWindowHeight(estimateWindowHeight(snapshot));

  const evaluation = evaluateNotificationPolicy(snapshot.rows, prevState, {
    thresholds: {
      warningPercent: settings.notification.warningPercent,
      criticalPercent: settings.notification.criticalPercent,
    },
    cooldownMs: settings.notification.cooldownMinutes * 60_000,
    hysteresisPercent: settings.notification.hysteresisPercent,
    quietHours: settings.notification.quietHours,
  });

  await saveNotificationState(evaluation.nextState);

  for (const evt of evaluation.events) {
    new Notification({
      title: `${evt.level.toUpperCase()} - ${prettyTitle(evt.row.provider, evt.row.window)}`,
      body: `${Math.round(evt.row.usedPercent ?? 0)}% used`,
    }).show();
  }

  tray?.setToolTip(usagePercentLabel(snapshot));
  if (win && !win.isDestroyed()) {
    win.webContents.send("tray:snapshot-updated", snapshot);
  }
}

function toggleWindow() {
  if (!win || !tray) return;
  if (win.isVisible()) {
    win.hide();
    return;
  }
  // Ensure stable first-open height even if renderer resize hasn't fired yet.
  applyWindowHeight(estimateWindowHeight(currentSnapshot));
  positionWindow();
  win.show();
  win.focus();
}

function positionWindow() {
  if (!win || !tray) return;
  const trayBounds = tray.getBounds();
  const winBounds = win.getBounds();
  const x = Math.round(trayBounds.x - winBounds.width / 2);
  const y = Math.round(trayBounds.y - winBounds.height - 8);
  win.setPosition(Math.max(0, x), Math.max(0, y));
}

function createWindow() {
  const preloadPath = fileURLToPath(new URL("./preload.js", import.meta.url));

  win = new BrowserWindow({
    width: 560,
    height: 380,
    show: false,
    frame: false,
    resizable: false,
    useContentSize: true,
    icon: makeIcon(),
    webPreferences: {
      preload: preloadPath,
      nodeIntegration: true,
      contextIsolation: false,
      sandbox: false,
    },
  });

  win.on("blur", () => {
    if (win && win.isVisible()) win.hide();
  });

  win.loadURL(`data:text/html;charset=utf-8,${encodeURIComponent(renderHtml(currentSnapshot))}`);
}

function createTray() {
  tray = new Tray(makeIcon());
  const menu = Menu.buildFromTemplate([
    {
      label: "Open Usage",
      click: () => toggleWindow(),
    },
    {
      label: "Refresh",
      click: async () => {
        await refreshSnapshot(true);
      },
    },
    { type: "separator" },
    {
      label: "Quit",
      click: () => app.quit(),
    },
  ]);

  tray.setContextMenu(menu);
  tray.setToolTip("QMeter");
  tray.on("click", () => toggleWindow());
}

function registerIpc() {
  ipcMain.handle("tray:get-snapshot", async () => currentSnapshot);
  ipcMain.handle("tray:refresh", async () => {
    await refreshSnapshot(true);
    return currentSnapshot;
  });
  ipcMain.handle("tray:get-settings", async () => {
    const s = await getSettings();
    return {
      refreshIntervalMs: s.refreshIntervalMs,
      visibleProviders: s.visibleProviders,
    };
  });
  ipcMain.handle(
    "tray:save-settings",
    async (
      _evt,
      partial: { refreshIntervalMs: number; visibleProviders: { claude: boolean; codex: boolean } }
    ) => {
      const prev = await getSettings();
      const next: TraySettings = {
        ...prev,
        refreshIntervalMs: partial.refreshIntervalMs,
        visibleProviders: {
          claude: partial.visibleProviders?.claude ?? prev.visibleProviders.claude,
          codex: partial.visibleProviders?.codex ?? prev.visibleProviders.codex,
        },
      };
      await saveTraySettings(next);
      currentSettings = next;
      resetRefreshTimer(next.refreshIntervalMs);
      await refreshSnapshot(true);
      return {
        refreshIntervalMs: next.refreshIntervalMs,
        visibleProviders: next.visibleProviders,
      };
    }
  );
  ipcMain.handle("tray:set-height", async (_evt, heightRaw: unknown) => {
    const h = typeof heightRaw === "number" ? heightRaw : Number(heightRaw);
    if (!Number.isFinite(h)) return;
    applyWindowHeight(h);
  });
}

async function boot() {
  currentSettings = await loadTraySettings();
  registerIpc();
  createWindow();
  createTray();

  await refreshSnapshot(true);
  resetRefreshTimer(currentSettings.refreshIntervalMs);
}

const gotLock = app.requestSingleInstanceLock();
if (!gotLock) {
  app.quit();
} else {
  app.on("second-instance", () => {
    if (win && !win.isVisible()) win.show();
    win?.focus();
  });

  app.whenReady().then(() => {
    void boot();
  });
}

app.on("window-all-closed", () => {
  // Keep background tray app alive on Windows.
});

app.on("before-quit", () => {
  if (timer) clearInterval(timer);
  timer = null;
});
