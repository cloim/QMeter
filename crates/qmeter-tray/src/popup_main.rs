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

#[cfg(windows)]
fn main() -> eframe::Result {
    let viewport = egui::ViewportBuilder::default()
        .with_inner_size([420.0, 560.0])
        .with_min_inner_size([380.0, 420.0])
        .with_resizable(true)
        .with_title("QMeter");

    let options = eframe::NativeOptions {
        viewport,
        centered: true,
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
                            ui.horizontal(|ui| {
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
                            ui.add(
                                egui::ProgressBar::new((percent / 100.0) as f32)
                                    .text(format!("{percent:.0}% used"))
                                    .desired_width(f32::INFINITY),
                            );
                            ui.add_space(6.0);
                            ui.horizontal(|ui| {
                                ui.label(row.reset_at.as_deref().unwrap_or("Reset: unknown"));
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        if row.stale {
                                            ui.colored_label(
                                                egui::Color32::from_rgb(255, 190, 80),
                                                "stale",
                                            );
                                        }
                                        ui.label(&row.meta);
                                    },
                                );
                            });
                        });
                    ui.add_space(8.0);
                }

                if !self.model.errors.is_empty() {
                    ui.separator();
                    ui.heading("Errors");
                    for error in &self.model.errors {
                        ui.colored_label(
                            egui::Color32::from_rgb(255, 140, 120),
                            format!("{}: {}", error.provider.as_str(), error.message),
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
