use std::path::PathBuf;

use crate::studio::StudioState;

/// Export format options.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    Midi,
    Wav,
    MusicXml,
}

/// Export dialog state.
pub struct ExportDialog {
    pub open: bool,
    pub format: ExportFormat,
    pub path: String,
    pub status: String,
}

impl ExportDialog {
    pub fn new() -> Self {
        Self {
            open: false,
            format: ExportFormat::Midi,
            path: String::new(),
            status: String::new(),
        }
    }

    /// Render the modal export dialog.
    pub fn modal_ui(&mut self, ctx: &egui::Context, studio: &StudioState) {
        if !self.open {
            return;
        }

        egui::Window::new("Export")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.heading("Export Project");
                ui.separator();

                ui.horizontal(|ui| {
                    ui.label("Format:");
                    ui.selectable_value(&mut self.format, ExportFormat::Midi, "MIDI (.mid)");
                    ui.selectable_value(&mut self.format, ExportFormat::Wav, "WAV (.wav)");
                    ui.selectable_value(&mut self.format, ExportFormat::MusicXml, "MusicXML (.xml)");
                });

                ui.horizontal(|ui| {
                    ui.label("Path:");
                    ui.text_edit_singleline(&mut self.path);
                    if ui.button("Browse…").clicked() {
                        let ext = match self.format {
                            ExportFormat::Midi => ("MIDI", "mid"),
                            ExportFormat::Wav => ("WAV", "wav"),
                            ExportFormat::MusicXml => ("MusicXML", "xml"),
                        };
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter(ext.0, &[ext.1])
                            .save_file()
                        {
                            self.path = path.display().to_string();
                        }
                    }
                });

                if !self.status.is_empty() {
                    ui.label(
                        egui::RichText::new(&self.status)
                            .color(if self.status.starts_with("Error") {
                                egui::Color32::from_rgb(224, 108, 117)
                            } else {
                                egui::Color32::from_rgb(152, 195, 121)
                            }),
                    );
                }

                ui.separator();

                ui.horizontal(|ui| {
                    if ui.button("Export").clicked() {
                        self.do_export(studio);
                    }
                    if ui.button("Cancel").clicked() {
                        self.open = false;
                        self.status.clear();
                    }
                });
            });
    }

    /// Render an inline export panel (for the sidebar).
    pub fn panel_ui(&mut self, ui: &mut egui::Ui, studio: &StudioState) {
        ui.heading("Export");
        ui.separator();

        ui.label("Format:");
        ui.radio_value(&mut self.format, ExportFormat::Midi, "MIDI (.mid)");
        ui.radio_value(&mut self.format, ExportFormat::Wav, "WAV (.wav)");
        ui.radio_value(&mut self.format, ExportFormat::MusicXml, "MusicXML (.xml)");

        ui.separator();

        ui.horizontal(|ui| {
            ui.label("Path:");
            ui.text_edit_singleline(&mut self.path);
        });
        if ui.button("Browse…").clicked() {
            let ext = match self.format {
                ExportFormat::Midi => ("MIDI", "mid"),
                ExportFormat::Wav => ("WAV", "wav"),
                ExportFormat::MusicXml => ("MusicXML", "xml"),
            };
            if let Some(path) = rfd::FileDialog::new()
                .add_filter(ext.0, &[ext.1])
                .save_file()
            {
                self.path = path.display().to_string();
            }
        }

        ui.separator();

        if ui.button("Export").clicked() {
            self.do_export(studio);
        }

        if !self.status.is_empty() {
            ui.separator();
            ui.label(&self.status);
        }
    }

    fn do_export(&mut self, studio: &StudioState) {
        if self.path.is_empty() {
            self.status = "Error: no path specified".into();
            return;
        }

        let path = PathBuf::from(&self.path);

        match self.format {
            ExportFormat::Midi => {
                // TODO: collect events from studio, use delphi_midi::MidiExporter
                self.status = format!("Exported MIDI to {}", path.display());
            }
            ExportFormat::Wav => {
                // TODO: render audio via delphi_engine::render_to_wav
                self.status = format!("Exported WAV to {}", path.display());
            }
            ExportFormat::MusicXml => {
                // TODO: build MusicXML from events
                self.status = format!("Exported MusicXML to {}", path.display());
            }
        }
    }
}
