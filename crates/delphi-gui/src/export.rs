use std::path::PathBuf;

use delphi_core::duration::TimeSignature;
use delphi_engine::soundfont::{render_to_wav_panned};
use delphi_midi::export::{MidiExporter, MidiTrack};

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
    /// SoundFont path for WAV rendering.
    pub sf_path: Option<PathBuf>,
    /// Mixer master gain (synced from app when dialog opens).
    pub master_gain: f32,
}

impl ExportDialog {
    pub fn new() -> Self {
        Self {
            open: false,
            format: ExportFormat::Midi,
            path: String::new(),
            status: String::new(),
            sf_path: None,
            master_gain: 1.0,
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
                    ui.add_enabled(false, egui::SelectableLabel::new(false, "MusicXML (coming soon)"));
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
        ui.add_enabled(false, egui::RadioButton::new(false, "MusicXML (coming soon)"));

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

        // Auto-append correct extension if missing
        let expected_ext = match self.format {
            ExportFormat::Midi => ".mid",
            ExportFormat::Wav => ".wav",
            ExportFormat::MusicXml => ".xml",
        };
        if !self.path.ends_with(expected_ext) {
            self.path.push_str(expected_ext);
        }

        let path = PathBuf::from(&self.path);
        let events = studio.collect_events_mixed(None, self.master_gain);

        if events.is_empty() {
            self.status = "Error: no events to export (cells are empty)".into();
            return;
        }

        match self.format {
            ExportFormat::Midi => {
                let mut exporter = MidiExporter::new();
                exporter.set_tempo(studio.tempo());
                exporter.set_time_signature(TimeSignature {
                    numerator: studio.settings.time_sig_num,
                    denominator: studio.settings.time_sig_den,
                });

                // Group events by channel into tracks
                let mut channel_events: std::collections::HashMap<u8, Vec<&delphi_engine::SfEvent>> =
                    std::collections::HashMap::new();
                for ev in &events {
                    channel_events.entry(ev.channel).or_default().push(ev);
                }

                for (ch, evs) in &channel_events {
                    let program = evs.first().map(|e| e.program).unwrap_or(0);
                    let track_name = studio
                        .tracks
                        .iter()
                        .find(|t| t.channel == *ch)
                        .map(|t| t.name.clone())
                        .unwrap_or_else(|| format!("Channel {}", ch));
                    let mut track = MidiTrack::new(&track_name, *ch, program);
                    for ev in evs {
                        track.add_note(ev.tick, ev.midi_note, ev.velocity, ev.duration_ticks);
                    }
                    exporter.add_track(track);
                }

                match exporter.write_file(path.to_str().unwrap_or("")) {
                    Ok(()) => self.status = format!("Exported MIDI to {}", path.display()),
                    Err(e) => self.status = format!("Error: {}", e),
                }
            }
            ExportFormat::Wav => {
                let sf = match &self.sf_path {
                    Some(p) if p.is_file() => p.clone(),
                    _ => {
                        self.status = "Error: no SoundFont loaded — required for WAV export".into();
                        return;
                    }
                };
                let tempo = studio.tempo();
                let pan = studio.channel_pan_map();
                match render_to_wav_panned(&sf, &events, &tempo, &path, &pan) {
                    Ok(()) => self.status = format!("Exported WAV to {}", path.display()),
                    Err(e) => self.status = format!("Error: {}", e),
                }
            }
            ExportFormat::MusicXml => {
                // MusicXML is a stretch goal; provide a helpful message
                self.status = "MusicXML export is not yet implemented".into();
            }
        }
    }
}
