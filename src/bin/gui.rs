#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use archive_streamer::{AppMessage, extract_archive};
use eframe::egui;
use std::path::Path;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver};

struct ArchiveApp {
    archive_path: Option<PathBuf>,
    output_dir: Option<PathBuf>,
    running: bool,
    receiver: Option<Receiver<AppMessage>>,
    status: String,
    logs: Vec<String>,
}

impl Default for ArchiveApp {
    fn default() -> Self {
        Self {
            archive_path: None,
            output_dir: None,
            running: false,
            receiver: None,
            status: "Ready".into(),
            logs: Vec::new(),
        }
    }
}

impl eframe::App for ArchiveApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        if let Some(rx) = &self.receiver {
            let messages: Vec<AppMessage> = rx.try_iter().collect();

            for message in messages {
                match message {
                    AppMessage::Progress(message) => {
                        self.status = message.clone();
                        self.logs.push(message);

                        if self.logs.len() > 500 {
                            self.logs.remove(0);
                        }
                    }
                    AppMessage::Finished(result) => {
                        self.running = false;

                        match result {
                            Ok(()) => {
                                self.status = "Extraction completed.".into();
                            }
                            Err(error) => {
                                self.status = format!("Greška: {error}");
                            }
                        }
                    }
                }
            }
        }

        egui::CentralPanel::default().show(ui, |ui| {
            let visuals = ui.visuals_mut();
            visuals.panel_fill = egui::Color32::from_rgb(18, 22, 32);
            visuals.window_fill = egui::Color32::from_rgb(25, 31, 44);

            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.add_space(8.0);

                // Wide banner
                let banner_width = ui.available_width().min(650.0);
                let banner_height = 155.0;
                ui.vertical_centered(|ui| {
                    ui.add(
                        egui::Image::new(egui::include_image!("../../assets/banner.png"))
                            .fit_to_exact_size(egui::vec2(banner_width, banner_height)),
                    );
                });

                ui.add_space(14.0);

                ui.heading(
                    egui::RichText::new("Archive Streamer")
                        .size(28.0)
                        .strong()
                        .color(egui::Color32::from_rgb(235, 240, 255)),
                );

                ui.label(
                    egui::RichText::new(
                        "Extract large archives without requiring extra disk space",
                    )
                    .size(14.0)
                    .color(egui::Color32::from_rgb(155, 169, 190)),
                );

                ui.add_space(20.0);

                // Archive selection card
                egui::Frame::group(ui.style())
                    .fill(egui::Color32::from_rgb(27, 34, 48))
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());

                        ui.label(
                            egui::RichText::new("SOURCE ARCHIVE")
                                .strong()
                                .color(egui::Color32::from_rgb(130, 170, 255)),
                        );

                        ui.add_space(8.0);

                        ui.horizontal(|ui| {
                            if ui
                                .add_enabled(
                                    !self.running,
                                    egui::Button::new("  Browse archive...  "),
                                )
                                .clicked()
                            {
                                if let Some(path) = rfd::FileDialog::new()
                                    .add_filter(
                                        "Archives",
                                        &["zip", "tar", "gz", "bz2", "xz", "7z", "rar"],
                                    )
                                    .pick_file()
                                {
                                    if self.output_dir.is_none() {
                                        self.output_dir = path.parent().map(Path::to_path_buf);
                                    }

                                    self.archive_path = Some(path);
                                }
                            }

                            ui.add_space(6.0);

                            ui.label(
                                self.archive_path
                                    .as_ref()
                                    .map(|p| p.display().to_string())
                                    .unwrap_or_else(|| "No archive selected".into()),
                            );
                        });
                    });

                ui.add_space(12.0);

                // Destination selection card
                egui::Frame::group(ui.style())
                    .fill(egui::Color32::from_rgb(27, 34, 48))
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());

                        ui.label(
                            egui::RichText::new("DESTINATION")
                                .strong()
                                .color(egui::Color32::from_rgb(130, 210, 180)),
                        );

                        ui.add_space(8.0);

                        ui.horizontal(|ui| {
                            if ui
                                .add_enabled(
                                    !self.running,
                                    egui::Button::new("  Choose folder...  "),
                                )
                                .clicked()
                            {
                                if let Some(path) = rfd::FileDialog::new().pick_folder() {
                                    self.output_dir = Some(path);
                                }
                            }

                            ui.add_space(6.0);

                            ui.label(
                                self.output_dir
                                    .as_ref()
                                    .map(|p| p.display().to_string())
                                    .unwrap_or_else(|| "No destination selected".into()),
                            );
                        });
                    });

                ui.add_space(18.0);

                let can_start =
                    !self.running && self.archive_path.is_some() && self.output_dir.is_some();

                ui.horizontal(|ui| {
                    let button = egui::Button::new(
                        egui::RichText::new(if self.running {
                            "  EXTRACTING...  "
                        } else {
                            "  EXTRACT ARCHIVE  "
                        })
                        .size(16.0)
                        .strong(),
                    )
                    .min_size(egui::vec2(210.0, 44.0))
                    .fill(if can_start {
                        egui::Color32::from_rgb(46, 125, 90)
                    } else {
                        egui::Color32::from_rgb(65, 72, 86)
                    });

                    if ui.add_enabled(can_start, button).clicked() {
                        let archive = self.archive_path.clone().unwrap();
                        let output = self.output_dir.clone().unwrap();

                        let (tx, rx) = mpsc::channel();

                        self.receiver = Some(rx);
                        self.running = true;
                        self.logs.clear();
                        self.status = "Starting extraction...".into();

                        std::thread::spawn(move || {
                            let result = extract_archive(&archive, &output, &tx);
                            let _ = tx.send(AppMessage::Finished(result));
                        });
                    }

                    if self.running {
                        ui.spinner();
                    }
                });

                ui.add_space(18.0);
                ui.separator();
                ui.add_space(8.0);

                // Status panel
                egui::Frame::group(ui.style())
                    .fill(egui::Color32::from_rgb(27, 34, 48))
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());

                        ui.label(
                            egui::RichText::new("STATUS")
                                .strong()
                                .color(egui::Color32::from_rgb(170, 180, 200)),
                        );

                        let status_color = if self.status.starts_with("Extraction completed") {
                            egui::Color32::from_rgb(75, 220, 140)
                        } else if self.status.starts_with("Error:")
                            || self.status.starts_with("Greška:")
                        {
                            egui::Color32::from_rgb(255, 100, 110)
                        } else {
                            egui::Color32::from_rgb(220, 225, 235)
                        };

                        ui.label(
                            egui::RichText::new(&self.status)
                                .size(15.0)
                                .color(status_color),
                        );
                    });

                ui.add_space(12.0);

                ui.label(
                    egui::RichText::new("ACTIVITY LOG")
                        .strong()
                        .color(egui::Color32::from_rgb(170, 180, 200)),
                );

                egui::Frame::group(ui.style())
                    .fill(egui::Color32::from_rgb(13, 17, 25))
                    .show(ui, |ui| {
                        egui::ScrollArea::vertical()
                            .max_height(180.0)
                            .stick_to_bottom(true)
                            .show(ui, |ui| {
                                if self.logs.is_empty() {
                                    ui.label(
                                        egui::RichText::new("Waiting for activity...")
                                            .color(egui::Color32::from_rgb(110, 125, 145)),
                                    );
                                }

                                for line in &self.logs {
                                    ui.label(egui::RichText::new(line).monospace().size(12.0));
                                }
                            });
                    });

                ui.add_space(8.0);
            });
        });

        if self.running {
            ui.request_repaint_after(std::time::Duration::from_millis(100));
        }
    }
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([850.0, 700.0])
            .with_min_inner_size([750.0, 600.0]),
        ..Default::default()
    };
    egui_extras::install_image_loaders(&egui::Context::default());

    eframe::run_native(
        "Archive Streamer",
        options,
        Box::new(|cc| {
            egui_extras::install_image_loaders(&cc.egui_ctx);
            Ok(Box::new(ArchiveApp::default()))
        }),
    )
}
