#[cfg(windows)]
use crate::tray_app::UserEvent;
#[cfg(windows)]
use base64::{Engine as _, engine::general_purpose};
#[cfg(windows)]
use qmeter_core::settings::TraySettings;
#[cfg(windows)]
use qmeter_core::types::{NormalizedSnapshot, ProviderId};
#[cfg(windows)]
use std::time::{Duration, Instant};
#[cfg(windows)]
use winit::dpi::{LogicalPosition, LogicalSize, Position};
#[cfg(windows)]
use winit::event::WindowEvent;
#[cfg(windows)]
use winit::event_loop::{ActiveEventLoop, EventLoopProxy};
#[cfg(windows)]
use winit::monitor::MonitorHandle;
#[cfg(windows)]
use winit::platform::windows::WindowAttributesExtWindows;
#[cfg(windows)]
use winit::window::{Window, WindowId, WindowLevel};
#[cfg(windows)]
use wry::{WebView, WebViewBuilder};

#[cfg(windows)]
const POPUP_WIDTH: f64 = 560.0;
#[cfg(windows)]
const HEADER_HEIGHT: f64 = 86.0;
#[cfg(windows)]
const BODY_PADDING_Y: f64 = 60.0;
#[cfg(windows)]
const CARD_HEIGHT: f64 = 238.0;
#[cfg(windows)]
const CARD_GAP: f64 = 18.0;
#[cfg(windows)]
const TASKBAR_HEIGHT: f64 = 48.0;

#[cfg(windows)]
const QMETER_PNG: &[u8] = include_bytes!("../../../resources/QMeter.png");
#[cfg(windows)]
const CLAUDE_PNG: &[u8] = include_bytes!("../../../resources/Claude.png");
#[cfg(windows)]
const CODEX_PNG: &[u8] = include_bytes!("../../../resources/Codex.png");

#[cfg(windows)]
pub(crate) struct PopupOverlay {
    window: Option<Window>,
    webview: Option<WebView>,
    created_at: Option<Instant>,
    focused_once: bool,
    visible: bool,
}

#[cfg(windows)]
impl PopupOverlay {
    pub(crate) fn new() -> Self {
        Self {
            window: None,
            webview: None,
            created_at: None,
            focused_once: false,
            visible: false,
        }
    }

    pub(crate) fn is_visible(&self) -> bool {
        self.visible
    }

    pub(crate) fn toggle(
        &mut self,
        event_loop: &ActiveEventLoop,
        proxy: EventLoopProxy<UserEvent>,
        snapshot: &NormalizedSnapshot,
        settings: &TraySettings,
        anchor: Option<(f64, f64)>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if self.visible {
            self.hide();
            return Ok(());
        }

        self.show_or_create(event_loop, proxy, snapshot, settings, anchor)
    }

    pub(crate) fn show_or_create(
        &mut self,
        event_loop: &ActiveEventLoop,
        proxy: EventLoopProxy<UserEvent>,
        snapshot: &NormalizedSnapshot,
        settings: &TraySettings,
        anchor: Option<(f64, f64)>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let size = PopupSize {
            width: POPUP_WIDTH,
            height: estimate_window_height(snapshot),
        };
        let anchor = anchor.map(|(x, y)| PopupPoint { x, y });

        if let Some(window) = self.window.as_ref() {
            let _ = window.request_inner_size(LogicalSize::new(size.width, size.height));
            if let Some(position) = popup_position_for_display(event_loop, anchor, size) {
                window.set_outer_position(Position::Logical(LogicalPosition::new(
                    position.x, position.y,
                )));
            }
            self.created_at = Some(Instant::now());
            self.focused_once = false;
            window.set_visible(true);
            window.focus_window();
            self.visible = true;
            self.update_snapshot(snapshot);
            self.update_settings(settings);
            return Ok(());
        }

        let mut attrs = Window::default_attributes()
            .with_title("QMeter")
            .with_inner_size(LogicalSize::new(size.width, size.height))
            .with_decorations(false)
            .with_resizable(false)
            .with_window_level(WindowLevel::AlwaysOnTop)
            .with_skip_taskbar(true)
            .with_transparent(true)
            .with_undecorated_shadow(false)
            .with_visible(true);

        if let Some(position) = popup_position_for_display(event_loop, anchor, size) {
            attrs = attrs.with_position(LogicalPosition::new(position.x, position.y));
        }

        let window = event_loop.create_window(attrs)?;
        let webview = WebViewBuilder::new()
            .with_transparent(true)
            .with_html(render_popup_html(snapshot, settings))
            .with_ipc_handler(move |request| {
                let body = request.body();
                if body == "refresh" {
                    let _ = proxy.send_event(UserEvent::PopupRefresh);
                } else if let Some(json) = body.strip_prefix("settings:save:") {
                    let _ = proxy.send_event(UserEvent::PopupSaveSettings(json.to_string()));
                }
            })
            .build(&window)?;

        self.webview = Some(webview);
        self.window = Some(window);
        self.created_at = Some(Instant::now());
        self.focused_once = false;
        self.visible = true;
        if let Some(window) = self.window.as_ref() {
            window.focus_window();
        }
        Ok(())
    }

    pub(crate) fn show_settings(
        &mut self,
        event_loop: &ActiveEventLoop,
        proxy: EventLoopProxy<UserEvent>,
        snapshot: &NormalizedSnapshot,
        settings: &TraySettings,
        anchor: Option<(f64, f64)>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.show_or_create(event_loop, proxy, snapshot, settings, anchor)?;
        self.open_settings(settings);
        Ok(())
    }

    pub(crate) fn update_snapshot(&self, snapshot: &NormalizedSnapshot) {
        if let Some(webview) = self.webview.as_ref() {
            let script = format!("window.__render({});", snapshot_json(snapshot));
            let _ = webview.evaluate_script(&script);
        }
    }

    pub(crate) fn show_loading(&self) {
        if let Some(webview) = self.webview.as_ref() {
            let _ = webview.evaluate_script("window.__setLoading && window.__setLoading();");
        }
    }

    pub(crate) fn update_settings(&self, settings: &TraySettings) {
        if let Some(webview) = self.webview.as_ref() {
            let script = format!(
                "window.__setSettings && window.__setSettings({});",
                settings_json(settings)
            );
            let _ = webview.evaluate_script(&script);
        }
    }

    pub(crate) fn open_settings(&self, settings: &TraySettings) {
        if let Some(webview) = self.webview.as_ref() {
            let script = format!(
                "window.__openSettings && window.__openSettings({});",
                settings_json(settings)
            );
            let _ = webview.evaluate_script(&script);
        }
    }

    pub(crate) fn show_settings_saved(&self, settings: &TraySettings) {
        if let Some(webview) = self.webview.as_ref() {
            let script = format!(
                "window.__settingsSaved && window.__settingsSaved({});",
                settings_json(settings)
            );
            let _ = webview.evaluate_script(&script);
        }
    }

    pub(crate) fn show_settings_error(&self, message: &str) {
        if let Some(webview) = self.webview.as_ref() {
            let msg = serde_json::to_string(message).unwrap_or_else(|_| "\"error\"".to_string());
            let script = format!("window.__settingsError && window.__settingsError({msg});");
            let _ = webview.evaluate_script(&script);
        }
    }

    pub(crate) fn handle_window_event(&mut self, window_id: WindowId, event: &WindowEvent) -> bool {
        if self.window.as_ref().map(Window::id) != Some(window_id) {
            return false;
        }

        match event {
            WindowEvent::CloseRequested => self.hide(),
            WindowEvent::Focused(true) => {
                self.focused_once = true;
            }
            WindowEvent::Focused(false) => {
                let age = self
                    .created_at
                    .map(|time| time.elapsed())
                    .unwrap_or_default();
                if should_close_on_focus_loss(self.focused_once, age) {
                    self.hide();
                }
            }
            _ => {}
        }
        true
    }

    fn hide(&mut self) {
        if let Some(window) = self.window.as_ref() {
            window.set_visible(false);
        }
        self.visible = false;
    }
}

#[cfg(windows)]
#[derive(Clone, Copy, Debug, PartialEq)]
struct PopupPoint {
    x: f64,
    y: f64,
}

#[cfg(windows)]
#[derive(Clone, Copy, Debug, PartialEq)]
struct PopupSize {
    width: f64,
    height: f64,
}

#[cfg(all(test, windows))]
fn popup_position_for_anchor(anchor: PopupPoint, size: PopupSize) -> PopupPoint {
    PopupPoint {
        x: (anchor.x - size.width / 2.0).max(0.0),
        y: (anchor.y - size.height - 8.0).max(0.0),
    }
}

#[cfg(windows)]
fn popup_position_for_display(
    event_loop: &ActiveEventLoop,
    anchor: Option<PopupPoint>,
    popup: PopupSize,
) -> Option<PopupPoint> {
    let monitor = anchor
        .and_then(|anchor| monitor_for_anchor(event_loop, anchor))
        .or_else(|| event_loop.primary_monitor());
    monitor.map(|monitor| {
        let position = monitor.position();
        let size = monitor.size();
        popup_position_for_screen(
            PopupScreen {
                x: f64::from(position.x),
                y: f64::from(position.y),
                width: f64::from(size.width),
                height: f64::from(size.height),
            },
            popup,
        )
    })
}

#[cfg(windows)]
fn monitor_for_anchor(event_loop: &ActiveEventLoop, anchor: PopupPoint) -> Option<MonitorHandle> {
    event_loop.available_monitors().find(|monitor| {
        let position = monitor.position();
        let size = monitor.size();
        let left = f64::from(position.x);
        let top = f64::from(position.y);
        let right = left + f64::from(size.width);
        let bottom = top + f64::from(size.height);
        anchor.x >= left && anchor.x <= right && anchor.y >= top && anchor.y <= bottom
    })
}

#[cfg(windows)]
#[derive(Clone, Copy, Debug, PartialEq)]
struct PopupScreen {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

#[cfg(windows)]
fn popup_position_for_screen(screen: PopupScreen, popup: PopupSize) -> PopupPoint {
    PopupPoint {
        x: (screen.x + screen.width - popup.width).max(screen.x),
        y: (screen.y + screen.height - TASKBAR_HEIGHT - popup.height).max(screen.y),
    }
}

#[cfg(windows)]
fn should_close_on_focus_loss(focused_once: bool, age: Duration) -> bool {
    focused_once && age >= Duration::from_millis(500)
}

#[cfg(windows)]
fn estimate_window_height(snapshot: &NormalizedSnapshot) -> f64 {
    let has_claude = snapshot
        .rows
        .iter()
        .any(|row| row.provider == ProviderId::Claude);
    let has_codex = snapshot
        .rows
        .iter()
        .any(|row| row.provider == ProviderId::Codex);
    let provider_count = usize::from(has_claude) + usize::from(has_codex);
    let count = provider_count.max(1) as f64;
    (HEADER_HEIGHT + BODY_PADDING_Y + CARD_HEIGHT * count + CARD_GAP * (count - 1.0))
        .clamp(260.0, 820.0)
}

#[cfg(windows)]
fn render_popup_html(snapshot: &NormalizedSnapshot, settings: &TraySettings) -> String {
    let snapshot = snapshot_json(snapshot);
    let settings = settings_json(settings);
    let qmeter_logo = image_data_url(QMETER_PNG);
    let claude_logo = image_data_url(CLAUDE_PNG);
    let codex_logo = image_data_url(CODEX_PNG);
    format!(
        r##"<!doctype html>
<html><head><meta charset="utf-8" />
<title>QMeter</title>
<style>
  html,body{{margin:0;padding:0;overflow:hidden;background:transparent;color:#fff;font-family:"Segoe UI",sans-serif}}
  body{{box-sizing:border-box;width:100vw;height:100vh;background:transparent}}
  .panel{{position:relative;width:100vw;height:100vh;background:radial-gradient(600px 360px at 50% 10%,rgba(79,70,229,.18),transparent 60%),rgba(11,15,25,.96);backdrop-filter:blur(8px);border:1px solid rgba(255,255,255,.10);border-radius:18px;overflow:hidden;box-shadow:0 28px 48px rgba(0,0,0,.45)}}
  .header{{padding:18px 22px;border-bottom:1px solid rgba(255,255,255,.06);display:flex;justify-content:space-between;align-items:center;background:linear-gradient(to bottom,rgba(255,255,255,.06),transparent)}}
  .title{{font-size:20px;font-weight:800;letter-spacing:.2px;display:flex;align-items:center;gap:8px}}
  .titleLogo{{width:18px;height:18px;object-fit:contain;display:block}}
  .sub{{font-size:11px;color:#9ca3af;margin-top:5px}}
  .sub .mono{{font-family:Consolas,monospace;color:#cfd6e6}}
  .btn{{border:1px solid rgba(255,255,255,.14);background:rgba(255,255,255,.06);color:#d1d5db;border-radius:12px;padding:9px 14px;font-weight:600;cursor:pointer;transition:all .2s;display:flex;align-items:center;gap:7px}}
  .btn:hover{{background:rgba(255,255,255,.11);color:#fff;border-color:rgba(255,255,255,.24)}}
  .btn.secondary{{padding:9px 11px}}
  .btnSpin{{display:inline-block;transition:transform .6s linear}}
  .btn.spinning .btnSpin{{transform:rotate(360deg)}}
  .btn.spinning .btnSpin{{animation:spin .7s linear infinite}}
  .btn[disabled]{{opacity:.62;cursor:not-allowed}}
  .btnRow{{display:flex;align-items:center;gap:8px}}
  .content{{padding:20px;display:grid;gap:18px}}
  .provider{{position:relative;background:#121827;border:1px solid rgba(255,255,255,.07);border-radius:18px;padding:18px;overflow:hidden}}
  .provider::after{{content:"";position:absolute;right:-18px;top:-18px;width:150px;height:150px;border-radius:999px;filter:blur(55px);opacity:.2;pointer-events:none}}
  .provider.claude::after{{background:#f97316}}
  .provider.codex::after{{background:#3b82f6}}
  .providerHead{{display:flex;justify-content:space-between;align-items:center;margin-bottom:12px}}
  .providerName{{display:flex;gap:10px;align-items:center;font-size:18px;font-weight:800;letter-spacing:.2px}}
  .iconBox{{width:26px;height:26px;border-radius:10px;display:inline-flex;align-items:center;justify-content:center;border:1px solid rgba(255,255,255,.2);overflow:hidden}}
  .iconBox img{{width:16px;height:16px;object-fit:contain;display:block}}
  .iconBox.claude{{background:rgba(249,115,22,.12);border-color:rgba(249,115,22,.25)}}
  .iconBox.codex{{background:rgba(59,130,246,.12);border-color:rgba(59,130,246,.25)}}
  .warn{{font-size:11px;padding:3px 8px;border-radius:999px;border:1px solid rgba(239,68,68,.3);background:rgba(239,68,68,.12);color:#fca5a5;font-weight:700}}
  .progress{{margin-bottom:16px}}
  .progress:last-child{{margin-bottom:0}}
  .rowTop{{display:flex;justify-content:space-between;align-items:flex-end;margin-bottom:9px}}
  .labelWrap{{display:flex;align-items:center;gap:8px}}
  .miniIcon{{width:18px;height:18px;border-radius:7px;border:1px solid rgba(255,255,255,.15);display:inline-flex;align-items:center;justify-content:center;font-size:11px;color:#9ca3af;background:rgba(255,255,255,.04)}}
  .label{{font-size:13px;font-weight:700;color:#d1d5db;letter-spacing:.2px}}
  .pctWrap{{display:flex;align-items:baseline;gap:2px}}
  .pctNum{{font-size:28px;line-height:1;font-weight:800;letter-spacing:-.3px}}
  .pctSign{{font-size:13px;color:#9ca3af;font-weight:700}}
  .barTrack{{height:10px;width:100%;background:#0f1423;border-radius:999px;overflow:hidden;border:1px solid rgba(255,255,255,.08);box-shadow:inset 0 1px 2px rgba(0,0,0,.45)}}
  .barFill{{height:100%;transition:width .55s ease;position:relative}}
  .barFill::before{{content:"";position:absolute;left:0;right:0;top:0;height:1px;background:rgba(255,255,255,.35)}}
  .fillClaude{{background:linear-gradient(90deg,#fbbf24,#f97316)}}
  .fillCodex{{background:linear-gradient(90deg,#60a5fa,#6366f1)}}
  .fillWarn{{background:linear-gradient(90deg,#f59e0b,#f97316)}}
  .fillCritical{{background:linear-gradient(90deg,#ef4444,#fb7185)}}
  .rowBottom{{display:flex;justify-content:space-between;align-items:center;margin-top:9px}}
  .metaLabel{{font-size:11px;color:#9ca3af;font-weight:700}}
  .reset{{font-family:Consolas,"Malgun Gothic",monospace;font-size:11px;color:#cbd5e1;padding:2px 0;text-align:right}}
  .empty{{padding:16px;background:#121827;border:1px solid rgba(255,255,255,.08);border-radius:14px;color:#9ca3af}}
  .skeleton{{position:relative;overflow:hidden;background:#121827;border:1px solid rgba(255,255,255,.07);border-radius:18px;padding:18px}}
  .skeleton::after{{content:"";position:absolute;inset:0;transform:translateX(-100%);background:linear-gradient(90deg,transparent,rgba(255,255,255,.09),transparent);animation:shimmer 1.25s infinite}}
  .skLine{{height:12px;background:rgba(255,255,255,.08);border-radius:999px;margin-bottom:12px}}
  .skLine.title{{width:42%;height:18px;margin-bottom:20px}}
  .skLine.mid{{width:72%}}
  .skLine.full{{width:100%;height:10px}}
  .skLine.short{{width:35%;margin-left:auto;margin-bottom:0}}
  .settingsBackdrop{{position:fixed;inset:0;background:rgba(2,5,12,.66);display:none;align-items:center;justify-content:center;z-index:50}}
  .settingsBackdrop.show{{display:flex}}
  .settingsModal{{width:380px;max-width:92vw;background:#101726;border:1px solid rgba(255,255,255,.14);border-radius:16px;padding:16px;box-shadow:0 20px 40px rgba(0,0,0,.45)}}
  .settingsTitle{{font-size:16px;font-weight:800;margin-bottom:12px}}
  .field{{margin-bottom:12px}}
  .field label{{display:block;font-size:12px;color:#a9b5cc;margin-bottom:6px}}
  .select{{width:100%;background:#0f1423;color:#e6ebf7;border:1px solid rgba(255,255,255,.14);border-radius:10px;padding:8px}}
  .checks{{display:grid;gap:8px}}
  .checkRow{{display:flex!important;align-items:center;gap:8px;color:#dbe4ff;font-size:13px;margin:0!important}}
  .chk{{accent-color:#6366f1}}
  .actions{{display:flex;justify-content:flex-end;gap:8px;margin-top:10px}}
  .saveHint{{margin-top:8px;min-height:18px;font-size:12px;color:#a9b5cc}}
  .saveHint.ok{{color:#86efac}}
  .saveHint.err{{color:#fca5a5}}
  @keyframes shimmer{{100%{{transform:translateX(100%)}}}}
  @keyframes spin{{to{{transform:rotate(360deg)}}}}
</style></head>
<body><div class="panel">
  <div class="header">
    <div><div class="title"><img class="titleLogo" src="{qmeter_logo}" alt="" />QMeter <span style="font-size:12px;color:#9ca3af;font-weight:700">v0.1.9</span></div>
    <div class="sub">마지막 확인: <span class="mono" id="lastChecked">-</span></div></div>
    <div class="btnRow">
      <button class="btn secondary" id="openSettings" title="환경설정">⚙</button>
      <button class="btn" id="refresh"><span class="btnSpin">↻</span><span>새로고침</span></button>
    </div>
  </div>
  <div class="content" id="cards"></div>
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
const CLAUDE_LOGO = "{claude_logo}";
const CODEX_LOGO = "{codex_logo}";
let currentSettings = {settings};
const fmt = (raw) => {{
  const d = new Date(raw);
  if (Number.isNaN(d.getTime())) return raw || "-";
  return d.getFullYear() + "-" + String(d.getMonth()+1).padStart(2,"0") + "-" + String(d.getDate()).padStart(2,"0") + " " +
    String(d.getHours()).padStart(2,"0") + ":" + String(d.getMinutes()).padStart(2,"0") + ":" + String(d.getSeconds()).padStart(2,"0");
}};
const label = (p) => p === "claude" ? "Claude Code" : p === "codex" ? "Codex CLI" : p;
const limitLabel = (row) => /week|weekly|7d/i.test(row.window) ? "Week Limit" : "Session Limit";
const isRowFor = (row, kind) => kind === "session" ? /session|:5h/i.test(row.window) : /week|weekly|7d/i.test(row.window);
const barClass = (provider, n) => n >= 95 ? "fillCritical" : n >= 80 ? "fillWarn" : provider === "claude" ? "fillClaude" : "fillCodex";
function progress(row) {{
  const p = Math.max(0, Math.min(100, Math.round(row.usedPercent || 0)));
  const pctColor = p >= 95 ? "#f87171" : p >= 80 ? "#fbbf24" : "#ffffff";
  return `<div class="progress">
    <div class="rowTop"><div class="labelWrap"><span class="miniIcon">◷</span><span class="label">${{limitLabel(row)}}</span></div>
    <div class="pctWrap"><span class="pctNum" style="color:${{pctColor}}">${{p}}</span><span class="pctSign">%</span></div></div>
    <div class="barTrack"><div class="barFill ${{barClass(row.provider, p)}}" style="width:${{p}}%"></div></div>
    <div class="rowBottom"><span class="metaLabel">초기화 일시</span><span class="reset">${{fmt(row.resetAt)}}</span></div>
  </div>`;
}}
function skeletonCard() {{
  return `<div class="skeleton">
    <div class="skLine title"></div>
    <div class="skLine mid"></div>
    <div class="skLine full"></div>
    <div class="skLine short"></div>
  </div>`;
}}
window.__setLoading = () => {{
  document.getElementById("cards").innerHTML = skeletonCard() + skeletonCard();
}};
function applySettings(settings) {{
  currentSettings = settings || currentSettings;
  const interval = document.getElementById("refreshInterval");
  const claude = document.getElementById("showClaude");
  const codex = document.getElementById("showCodex");
  if (interval) interval.value = String(currentSettings.refreshIntervalMs || 60000);
  if (claude) claude.checked = !!(currentSettings.visibleProviders && currentSettings.visibleProviders.claude);
  if (codex) codex.checked = !!(currentSettings.visibleProviders && currentSettings.visibleProviders.codex);
}}
function setSaving(saving, message, cls) {{
  const save = document.getElementById("saveSettings");
  const cancel = document.getElementById("cancelSettings");
  const hint = document.getElementById("saveHint");
  if (save) {{
    save.disabled = saving;
    save.textContent = saving ? "저장 중..." : "저장";
  }}
  if (cancel) cancel.disabled = saving;
  if (hint) {{
    hint.textContent = message || "";
    hint.className = cls ? `saveHint ${{cls}}` : "saveHint";
  }}
}}
window.__setSettings = applySettings;
window.__openSettings = (settings) => {{
  applySettings(settings);
  setSaving(false, "", "");
  document.getElementById("settingsBackdrop").classList.add("show");
}};
window.__settingsSaved = (settings) => {{
  applySettings(settings);
  setSaving(false, "저장되었습니다.", "ok");
  document.getElementById("settingsBackdrop").classList.remove("show");
}};
window.__settingsError = (message) => {{
  setSaving(false, message || "저장 실패. 다시 시도해주세요.", "err");
}};
window.__render = (snapshot) => {{
  const refreshBtn = document.getElementById("refresh");
  if (refreshBtn) refreshBtn.classList.remove("spinning");
  const providers = ["claude","codex"];
  const cards = providers.map((provider) => {{
    const rows = (snapshot.rows || []).filter(r => r.provider === provider);
    if (!rows.length) return "";
    const session = rows.find(r => isRowFor(r, "session"));
    const week = rows.find(r => isRowFor(r, "week"));
    const warning = rows.some(r => Math.round(r.usedPercent || 0) >= 80);
    const logo = provider === "claude" ? CLAUDE_LOGO : CODEX_LOGO;
    return `<div class="provider ${{provider}}">
      <div class="providerHead"><div class="providerName"><span class="iconBox ${{provider}}"><img src="${{logo}}" alt="" /></span>${{label(provider)}}</div>${{warning ? '<span class="warn">Limit Warning</span>' : ''}}</div>
      ${{session ? progress(session) : ""}}${{week ? progress(week) : ""}}
    </div>`;
  }}).join("");
  document.getElementById("cards").innerHTML = cards || "<div class='empty'>No available providers</div>";
  document.getElementById("lastChecked").textContent = fmt(snapshot.fetchedAt);
}};
document.getElementById("refresh").addEventListener("click", () => {{
  const btn = document.getElementById("refresh");
  btn.classList.add("spinning");
  window.__setLoading();
  window.ipc.postMessage("refresh");
}});
document.getElementById("openSettings").addEventListener("click", () => {{
  window.__openSettings(currentSettings);
}});
document.getElementById("cancelSettings").addEventListener("click", () => {{
  document.getElementById("settingsBackdrop").classList.remove("show");
  setSaving(false, "", "");
}});
document.getElementById("settingsBackdrop").addEventListener("click", (event) => {{
  if (event.target === document.getElementById("settingsBackdrop")) {{
    document.getElementById("settingsBackdrop").classList.remove("show");
    setSaving(false, "", "");
  }}
}});
document.getElementById("saveSettings").addEventListener("click", () => {{
  const payload = {{
    refreshIntervalMs: Number(document.getElementById("refreshInterval").value || 60000),
    visibleProviders: {{
      claude: document.getElementById("showClaude").checked,
      codex: document.getElementById("showCodex").checked,
    }},
  }};
  setSaving(true, "설정을 저장하고 있습니다...", "");
  window.__setLoading();
  window.ipc.postMessage("settings:save:" + JSON.stringify(payload));
}});
applySettings(currentSettings);
window.__render({snapshot});
</script></body></html>"##
    )
}

#[cfg(windows)]
fn snapshot_json(snapshot: &NormalizedSnapshot) -> String {
    serde_json::to_string(snapshot).unwrap_or_else(|_| "{}".to_string())
}

#[cfg(windows)]
fn settings_json(settings: &TraySettings) -> String {
    serde_json::to_string(settings).unwrap_or_else(|_| "{}".to_string())
}

#[cfg(windows)]
fn image_data_url(bytes: &[u8]) -> String {
    format!(
        "data:image/png;base64,{}",
        general_purpose::STANDARD.encode(bytes)
    )
}

#[cfg(all(test, windows))]
mod tests {
    use super::{
        PopupPoint, PopupScreen, PopupSize, estimate_window_height, popup_position_for_anchor,
        popup_position_for_screen, render_popup_html, should_close_on_focus_loss,
    };
    use qmeter_core::settings::default_tray_settings;
    use qmeter_core::types::{
        Confidence, NormalizedRow, NormalizedSnapshot, ProviderId, SourceKind,
    };
    use std::time::Duration;

    #[test]
    fn popup_position_centers_window_above_anchor() {
        let pos = popup_position_for_anchor(
            PopupPoint {
                x: 1900.0,
                y: 1030.0,
            },
            PopupSize {
                width: 560.0,
                height: 640.0,
            },
        );

        assert_eq!(pos.x, 1620.0);
        assert_eq!(pos.y, 382.0);
    }

    #[test]
    fn popup_position_uses_bottom_right_screen_area() {
        let pos = popup_position_for_screen(
            PopupScreen {
                x: 0.0,
                y: 0.0,
                width: 1920.0,
                height: 1080.0,
            },
            PopupSize {
                width: 560.0,
                height: 640.0,
            },
        );

        assert_eq!(pos.x, 1360.0);
        assert_eq!(pos.y, 392.0);
    }

    #[test]
    fn popup_ignores_initial_focus_loss_before_window_is_ready() {
        assert!(!should_close_on_focus_loss(
            false,
            Duration::from_millis(100)
        ));
    }

    #[test]
    fn popup_closes_after_real_focus_loss() {
        assert!(should_close_on_focus_loss(true, Duration::from_millis(900)));
    }

    #[test]
    fn popup_height_matches_legacy_two_provider_layout() {
        let snapshot = NormalizedSnapshot {
            fetched_at: "2026-04-29T00:00:00Z".to_string(),
            rows: vec![
                row(ProviderId::Claude, "claude:5h"),
                row(ProviderId::Codex, "codex:5h"),
            ],
            errors: Vec::new(),
        };

        assert_eq!(estimate_window_height(&snapshot), 640.0);
    }

    #[test]
    fn popup_html_exposes_skeleton_and_settings_contract() {
        let snapshot = NormalizedSnapshot {
            fetched_at: "2026-04-29T00:00:00Z".to_string(),
            rows: vec![row(ProviderId::Claude, "claude:5h")],
            errors: Vec::new(),
        };
        let html = render_popup_html(&snapshot, &default_tray_settings());

        assert!(html.contains("window.__setLoading"));
        assert!(html.contains("skeleton"));
        assert!(html.contains("id=\"settingsBackdrop\""));
        assert!(html.contains("id=\"refreshInterval\""));
        assert!(html.contains("id=\"showClaude\""));
        assert!(html.contains("settings:save:"));
    }

    fn row(provider: ProviderId, window: &str) -> NormalizedRow {
        NormalizedRow {
            provider,
            window: window.to_string(),
            used: None,
            limit: None,
            used_percent: Some(1.0),
            reset_at: None,
            source: SourceKind::Structured,
            confidence: Confidence::High,
            stale: false,
            notes: None,
        }
    }
}
