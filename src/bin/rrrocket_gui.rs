use std::fs;
use std::path::PathBuf;

use eframe::{egui, App};
use rrrocket::ReplayParser;

struct RrrocketGui {
    parser: ReplayParser,
    last_path: Option<PathBuf>,
    last_json: Option<String>,
    last_error: Option<String>,
    last_status: Option<String>,
}

impl RrrocketGui {
    fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        Self {
            parser: ReplayParser::default(),
            last_path: None,
            last_json: None,
            last_error: None,
            last_status: None,
        }
    }

    fn load_path(&mut self, path: PathBuf) {
        self.last_path = Some(path.clone());
        self.last_status = None;
        match self.parser.parse_file(&path) {
            Ok(replay) => match serde_json::to_string_pretty(&replay) {
                Ok(json) => {
                    self.last_json = Some(json);
                    self.last_error = None;
                }
                Err(err) => {
                    self.last_error = Some(format!("Failed to serialize replay: {err}"));
                    self.last_json = None;
                }
            },
            Err(err) => {
                self.last_error = Some(format!("{err:#}"));
                self.last_json = None;
            }
        }
    }

    fn reparse_last(&mut self) {
        if let Some(path) = self.last_path.clone() {
            self.load_path(path);
        }
    }

    fn save_json(&mut self) {
        let Some(json) = self.last_json.as_ref() else {
            return;
        };

        let mut dialog = rfd::FileDialog::new().add_filter("JSON", &["json"]);
        if let Some(path) = &self.last_path {
            if let Some(parent) = path.parent() {
                dialog = dialog.set_directory(parent);
            }
            if let Some(stem) = path.file_stem().and_then(|stem| stem.to_str()) {
                let default_name = format!("{stem}.json");
                dialog = dialog.set_file_name(default_name);
            }
        }

        if let Some(save_path) = dialog.save_file() {
            match fs::write(&save_path, json) {
                Ok(()) => {
                    self.last_status =
                        Some(format!("Saved replay JSON to {}", save_path.display()));
                    self.last_error = None;
                }
                Err(err) => {
                    self.last_error = Some(format!(
                        "Failed to save JSON to {}: {err}",
                        save_path.display()
                    ));
                    self.last_status = None;
                }
            }
        }
    }
}

impl App for RrrocketGui {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("rrrocket GUI");
            ui.label("Parse Rocket League replays and inspect their decoded JSON.");
            ui.add_space(8.0);

            let mut parser_changed = false;
            ui.horizontal(|ui| {
                let mut crc = self.parser.crc_check();
                if ui.checkbox(&mut crc, "Force CRC check").changed() {
                    self.parser.set_crc_check(crc);
                    parser_changed = true;
                }

                let mut network = self.parser.network_parse();
                if ui.checkbox(&mut network, "Parse network data").changed() {
                    self.parser.set_network_parse(network);
                    parser_changed = true;
                }
            });

            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui.button("Open Replay…").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("Rocket League Replay", &["replay"])
                        .pick_file()
                    {
                        self.load_path(path);
                    }
                }

                if ui.button("Reload").clicked() {
                    self.reparse_last();
                }
            });

            if parser_changed {
                self.reparse_last();
            }

            ui.add_space(8.0);
            if let Some(path) = &self.last_path {
                ui.label(format!("Selected replay: {}", path.display()));
            } else {
                ui.label("Select a replay file to begin.");
            }

            if let Some(error) = &self.last_error {
                ui.colored_label(egui::Color32::RED, error);
            }

            if let Some(status) = &self.last_status {
                ui.colored_label(egui::Color32::from_rgb(38, 166, 91), status);
            }

            if self.last_json.is_some() {
                ui.separator();
                ui.horizontal(|ui| {
                    ui.heading("Replay JSON");
                    if ui.button("Copy to clipboard").clicked() {
                        if let Some(json) = &self.last_json {
                            ctx.output_mut(|out| out.copied_text = json.clone());
                        }
                    }
                    if ui.button("Save to file…").clicked() {
                        self.save_json();
                    }
                });

                if let Some(json) = &mut self.last_json {
                    egui::ScrollArea::vertical()
                        .id_source("replay_json")
                        .show(ui, |ui| {
                            ui.add(
                                egui::TextEdit::multiline(json)
                                    .code_editor()
                                    .desired_rows(30)
                                    .desired_width(f32::INFINITY)
                                    .interactive(false),
                            );
                        });
                }
            }
        });
    }
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "rrrocket GUI",
        options,
        Box::new(|cc| Box::new(RrrocketGui::new(cc))),
    )
}
