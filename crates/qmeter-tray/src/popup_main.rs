#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

#[cfg(windows)]
use base64::{Engine as _, engine::general_purpose};
#[cfg(windows)]
use qmeter_core::settings::{TraySettingsConfig, load_tray_settings};
#[cfg(windows)]
use qmeter_core::snapshot::{CollectOptions, collect_fixture_snapshot, is_fixture_mode_from_env};
#[cfg(windows)]
use qmeter_core::types::{NormalizedSnapshot, ProviderId};
#[cfg(windows)]
use qmeter_providers::snapshot::collect_live_snapshot;
#[cfg(windows)]
use std::time::{Duration, Instant};
#[cfg(windows)]
use winit::application::ApplicationHandler;
#[cfg(windows)]
use winit::dpi::{LogicalPosition, LogicalSize};
#[cfg(windows)]
use winit::event::WindowEvent;
#[cfg(windows)]
use winit::event_loop::{ActiveEventLoop, EventLoop, EventLoopProxy};
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
const QMETER_PNG: &[u8] = include_bytes!("../../../resources/QMeter.png");
#[cfg(windows)]
const CLAUDE_PNG: &[u8] = include_bytes!("../../../resources/Claude.png");
#[cfg(windows)]
const CODEX_PNG: &[u8] = include_bytes!("../../../resources/Codex.png");

#[cfg(windows)]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let snapshot = collect_popup_snapshot(false)?;
    let size = PopupSize {
        width: POPUP_WIDTH,
        height: estimate_window_height(&snapshot),
    };
    let anchor = popup_anchor_from_env();
    let html = render_popup_html(&snapshot);

    let event_loop = EventLoop::<PopupEvent>::with_user_event().build()?;
    let proxy = event_loop.create_proxy();
    let mut app = PopupApp {
        html,
        size,
        anchor,
        proxy,
        window: None,
        webview: None,
        created_at: None,
        focused_once: false,
    };
    event_loop.run_app(&mut app)?;
    Ok(())
}

#[cfg(not(windows))]
fn main() {
    eprintln!("qmeter-popup is only supported on Windows");
    std::process::exit(1);
}

#[cfg(windows)]
#[derive(Clone, Debug)]
enum PopupEvent {
    Refresh,
}

#[cfg(windows)]
struct PopupApp {
    html: String,
    size: PopupSize,
    anchor: Option<PopupPoint>,
    proxy: EventLoopProxy<PopupEvent>,
    window: Option<Window>,
    webview: Option<WebView>,
    created_at: Option<Instant>,
    focused_once: bool,
}

#[cfg(windows)]
impl ApplicationHandler<PopupEvent> for PopupApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let mut attrs = Window::default_attributes()
            .with_title("QMeter")
            .with_inner_size(LogicalSize::new(self.size.width, self.size.height))
            .with_decorations(false)
            .with_resizable(false)
            .with_transparent(true)
            .with_window_level(WindowLevel::AlwaysOnTop)
            .with_skip_taskbar(true)
            .with_visible(true);

        if let Some(position) = popup_position_for_display(event_loop, self.anchor, self.size) {
            attrs = attrs.with_position(LogicalPosition::new(position.x, position.y));
        }

        let window = event_loop
            .create_window(attrs)
            .expect("create popup window");
        let proxy = self.proxy.clone();
        let webview = WebViewBuilder::new()
            .with_html(self.html.clone())
            .with_transparent(true)
            .with_ipc_handler(move |request| {
                if request.body() == "refresh" {
                    let _ = proxy.send_event(PopupEvent::Refresh);
                }
            })
            .build(&window)
            .expect("create popup webview");

        self.webview = Some(webview);
        self.window = Some(window);
        self.created_at = Some(Instant::now());
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Focused(true) => {
                self.focused_once = true;
            }
            WindowEvent::Focused(false) => {
                let age = self
                    .created_at
                    .map(|time| time.elapsed())
                    .unwrap_or_default();
                if should_close_on_focus_loss(self.focused_once, age) {
                    event_loop.exit();
                }
            }
            _ => {}
        }
    }

    fn user_event(&mut self, _event_loop: &ActiveEventLoop, event: PopupEvent) {
        match event {
            PopupEvent::Refresh => {
                if let (Some(webview), Ok(snapshot)) =
                    (self.webview.as_ref(), collect_popup_snapshot(true))
                {
                    let script = format!("window.__render({});", snapshot_json(&snapshot));
                    let _ = webview.evaluate_script(&script);
                }
            }
        }
    }
}

#[cfg(windows)]
fn collect_popup_snapshot(force_refresh: bool) -> Result<NormalizedSnapshot, String> {
    let settings = load_tray_settings(&TraySettingsConfig::from_env())
        .map_err(|err| format!("Failed to load settings: {err}"))?;
    let mut providers = Vec::new();
    if settings.visible_providers.claude {
        providers.push(ProviderId::Claude);
    }
    if settings.visible_providers.codex {
        providers.push(ProviderId::Codex);
    }
    let opts = CollectOptions {
        refresh: force_refresh,
        debug: false,
        providers,
    };

    Ok(if is_fixture_mode_from_env() {
        collect_fixture_snapshot(&opts)
    } else {
        collect_live_snapshot(&opts).snapshot
    })
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

#[cfg(windows)]
fn popup_anchor_from_env() -> Option<PopupPoint> {
    let x = std::env::var("QMETER_POPUP_ANCHOR_X")
        .ok()?
        .parse::<f64>()
        .ok()?;
    let y = std::env::var("QMETER_POPUP_ANCHOR_Y")
        .ok()?
        .parse::<f64>()
        .ok()?;
    Some(PopupPoint { x, y })
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
        x: (screen.x + screen.width - popup.width - 16.0).max(screen.x),
        y: (screen.y + screen.height - popup.height - 64.0).max(screen.y),
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
fn render_popup_html(snapshot: &NormalizedSnapshot) -> String {
    let snapshot = snapshot_json(snapshot);
    let qmeter_logo = image_data_url(QMETER_PNG);
    let claude_logo = image_data_url(CLAUDE_PNG);
    let codex_logo = image_data_url(CODEX_PNG);
    format!(
        r##"<!doctype html>
<html><head><meta charset="utf-8" />
<title>QMeter</title>
<style>
  html,body{{margin:0;padding:0;overflow:hidden;background:transparent;color:#fff;font-family:"Segoe UI",sans-serif}}
  body{{box-sizing:border-box;width:100vw;height:100vh;padding:10px;background:transparent}}
  .panel{{position:relative;width:calc(100vw - 20px);height:calc(100vh - 20px);background:radial-gradient(600px 360px at 50% 10%,rgba(79,70,229,.18),transparent 60%),rgba(11,15,25,.93);backdrop-filter:blur(8px);border:1px solid rgba(255,255,255,.10);border-radius:18px;overflow:hidden;box-shadow:0 28px 48px rgba(0,0,0,.45)}}
  .header{{padding:18px 22px;border-bottom:1px solid rgba(255,255,255,.06);display:flex;justify-content:space-between;align-items:center;background:linear-gradient(to bottom,rgba(255,255,255,.06),transparent)}}
  .title{{font-size:20px;font-weight:800;letter-spacing:.2px;display:flex;align-items:center;gap:8px}}
  .titleLogo{{width:18px;height:18px;object-fit:contain;display:block}}
  .sub{{font-size:11px;color:#9ca3af;margin-top:5px}}
  .sub .mono{{font-family:Consolas,monospace;color:#cfd6e6}}
  .btn{{border:1px solid rgba(255,255,255,.14);background:rgba(255,255,255,.06);color:#d1d5db;border-radius:12px;padding:9px 14px;font-weight:600;cursor:pointer;transition:all .2s;display:flex;align-items:center;gap:7px}}
  .btn:hover{{background:rgba(255,255,255,.11);color:#fff;border-color:rgba(255,255,255,.24)}}
  .btnSpin{{display:inline-block;transition:transform .6s linear}}
  .btn.spinning .btnSpin{{transform:rotate(360deg)}}
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
</style></head>
<body><div class="panel">
  <div class="header">
    <div><div class="title"><img class="titleLogo" src="{qmeter_logo}" alt="" />QMeter <span style="font-size:12px;color:#9ca3af;font-weight:700">v0.1.8</span></div>
    <div class="sub">마지막 확인: <span class="mono" id="lastChecked">-</span></div></div>
    <button class="btn" id="refresh"><span class="btnSpin">↻</span><span>새로고침</span></button>
  </div>
  <div class="content" id="cards"></div>
</div>
<script>
const CLAUDE_LOGO = "{claude_logo}";
const CODEX_LOGO = "{codex_logo}";
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
window.__render = (snapshot) => {{
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
  window.ipc.postMessage("refresh");
  setTimeout(() => btn.classList.remove("spinning"), 700);
}});
window.__render({snapshot});
</script></body></html>"##
    )
}

#[cfg(windows)]
fn snapshot_json(snapshot: &NormalizedSnapshot) -> String {
    serde_json::to_string(snapshot).unwrap_or_else(|_| "{}".to_string())
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
        popup_position_for_screen, should_close_on_focus_loss,
    };
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

        assert_eq!(pos.x, 1344.0);
        assert_eq!(pos.y, 376.0);
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
