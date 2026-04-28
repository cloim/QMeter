#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

#[cfg(windows)]
mod popup_model;

#[cfg(windows)]
use eframe::egui;
#[cfg(windows)]
use popup_model::{PopupModel, popup_model_from_snapshot};
#[cfg(windows)]
use qmeter_core::settings::{TraySettingsConfig, load_tray_settings};
#[cfg(windows)]
use qmeter_core::snapshot::{CollectOptions, collect_fixture_snapshot, is_fixture_mode_from_env};
#[cfg(windows)]
use qmeter_core::types::{NormalizedError, NormalizedSnapshot, ProviderId};
#[cfg(windows)]
use qmeter_providers::snapshot::collect_live_snapshot;

const POPUP_WIDTH: f32 = 420.0;
const POPUP_HEIGHT: f32 = 560.0;

#[cfg(windows)]
fn main() -> eframe::Result {
    let size = PopupSize {
        width: POPUP_WIDTH,
        height: POPUP_HEIGHT,
    };
    let mut viewport = egui::ViewportBuilder::default()
        .with_inner_size([POPUP_WIDTH, POPUP_HEIGHT])
        .with_min_inner_size([380.0, 420.0])
        .with_resizable(true)
        .with_active(true)
        .with_title("QMeter");
    if let Some(anchor) = popup_anchor_from_env() {
        let position = popup_position_for_anchor(anchor, size);
        viewport = viewport.with_position([position.x, position.y]);
    }

    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    eframe::run_native(
        "QMeter",
        options,
        Box::new(|cc| {
            cc.egui_ctx.set_visuals(egui::Visuals::dark());
            Ok(Box::new(QMeterPopupApp::new()))
        }),
    )
}

#[cfg(not(windows))]
fn main() {
    eprintln!("qmeter-popup is only supported on Windows");
    std::process::exit(1);
}

#[cfg(windows)]
struct QMeterPopupApp {
    model: PopupModel,
    last_error: Option<String>,
}

#[cfg(windows)]
impl QMeterPopupApp {
    fn new() -> Self {
        let mut app = Self {
            model: popup_model_from_snapshot(&empty_snapshot()),
            last_error: None,
        };
        app.refresh(false);
        app
    }

    fn refresh(&mut self, force_refresh: bool) {
        match collect_popup_snapshot(force_refresh) {
            Ok(snapshot) => {
                self.model = popup_model_from_snapshot(&snapshot);
                self.last_error = None;
            }
            Err(err) => {
                self.last_error = Some(err);
            }
        }
    }
}

#[cfg(windows)]
impl eframe::App for QMeterPopupApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("QMeter");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Refresh").clicked() {
                        self.refresh(true);
                    }
                });
            });
            ui.label(format!("Last checked: {}", self.model.fetched_at));
            ui.add_space(8.0);

            if let Some(error) = &self.last_error {
                ui.colored_label(egui::Color32::from_rgb(255, 170, 90), error);
                ui.add_space(8.0);
            }

            egui::ScrollArea::vertical().show(ui, |ui| {
                if self.model.rows.is_empty() {
                    ui.label("No provider rows available.");
                }

                for row in &self.model.rows {
                    egui::Frame::group(ui.style())
                        .inner_margin(egui::Margin::symmetric(12, 10))
                        .show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            ui.horizontal_wrapped(|ui| {
                                ui.heading(&row.title);
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        ui.label(&row.provider);
                                    },
                                );
                            });
                            ui.add_space(6.0);
                            let percent = row.used_percent.clamp(0.0, 100.0);
                            if percent <= 0.0 {
                                ui.label("0% used");
                            } else {
                                ui.add(
                                    egui::ProgressBar::new((percent / 100.0) as f32)
                                        .text(format!("{percent:.0}% used"))
                                        .desired_width(ui.available_width()),
                                );
                            }
                            ui.add_space(6.0);
                            ui.horizontal_wrapped(|ui| {
                                ui.add(
                                    egui::Label::new(
                                        row.reset_at.as_deref().unwrap_or("Reset: unknown"),
                                    )
                                    .wrap(),
                                );
                                if row.stale {
                                    ui.colored_label(
                                        egui::Color32::from_rgb(255, 190, 80),
                                        "stale",
                                    );
                                }
                                ui.add(egui::Label::new(&row.meta).wrap());
                            });
                        });
                    ui.add_space(8.0);
                }

                if !self.model.errors.is_empty() {
                    ui.separator();
                    ui.heading("Errors");
                    for error in &self.model.errors {
                        ui.add(
                            egui::Label::new(
                                egui::RichText::new(format!(
                                    "{}: {}",
                                    error.provider.as_str(),
                                    error.message
                                ))
                                .color(egui::Color32::from_rgb(255, 140, 120)),
                            )
                            .wrap(),
                        );
                    }
                }
            });
        });
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
fn empty_snapshot() -> NormalizedSnapshot {
    NormalizedSnapshot {
        fetched_at: "pending".to_string(),
        rows: Vec::new(),
        errors: Vec::<NormalizedError>::new(),
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct PopupPoint {
    x: f32,
    y: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct PopupSize {
    width: f32,
    height: f32,
}

fn popup_anchor_from_env() -> Option<PopupPoint> {
    let x = std::env::var("QMETER_POPUP_ANCHOR_X")
        .ok()?
        .parse::<f32>()
        .ok()?;
    let y = std::env::var("QMETER_POPUP_ANCHOR_Y")
        .ok()?
        .parse::<f32>()
        .ok()?;
    Some(PopupPoint { x, y })
}

fn popup_position_for_anchor(anchor: PopupPoint, size: PopupSize) -> PopupPoint {
    PopupPoint {
        x: (anchor.x - size.width + 16.0).max(0.0),
        y: (anchor.y - size.height - 8.0).max(0.0),
    }
}

#[cfg(all(test, windows))]
mod tests {
    use super::{PopupPoint, PopupSize, popup_position_for_anchor};

    #[test]
    fn popup_position_places_window_above_anchor() {
        let pos = popup_position_for_anchor(
            PopupPoint {
                x: 1900.0,
                y: 1030.0,
            },
            PopupSize {
                width: 420.0,
                height: 560.0,
            },
        );

        assert_eq!(pos.x, 1496.0);
        assert_eq!(pos.y, 462.0);
    }
}
