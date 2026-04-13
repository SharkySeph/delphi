use std::ops::{Deref, DerefMut};

use delphi_core::notation::compute_token_spans;
// Re-export types and functions for backward compatibility with existing GUI modules.
pub use delphi_core::{
    Cell, Project, TrackState, TokenSpan,
    gm_program_from_name, gm_program_from_name_checked, GM_INSTRUMENT_NAMES,
    parse_notation_to_events, parse_notation_with_diagnostics,
};

/// The top-level studio state: wraps `Project` and adds GUI-specific methods.
pub struct StudioState {
    pub project: Project,
}

impl Deref for StudioState {
    type Target = Project;
    fn deref(&self) -> &Project {
        &self.project
    }
}

impl DerefMut for StudioState {
    fn deref_mut(&mut self) -> &mut Project {
        &mut self.project
    }
}

impl StudioState {
    pub fn new() -> Self {
        Self {
            project: Project::new(),
        }
    }

    /// Compute token-to-tick spans for all cells (for playback highlighting).
    pub fn compute_all_token_spans(&self) -> Vec<Vec<TokenSpan>> {
        self.cells
            .iter()
            .map(|cell| {
                if cell.cell_type == "notation" && !cell.source.trim().is_empty() {
                    compute_token_spans(
                        &cell.source,
                        self.settings.time_sig_num,
                        self.settings.time_sig_den,
                    )
                } else {
                    Vec::new()
                }
            })
            .collect()
    }

    /// Render the track list in the side panel.
    pub fn tracks_ui(&mut self, ui: &mut egui::Ui) {
        ui.heading("Tracks");

        let mut to_remove: Option<usize> = None;
        for (idx, track) in self.tracks.iter_mut().enumerate() {
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    ui.text_edit_singleline(&mut track.name);
                    if ui.small_button("\u{1F5D1}").clicked() {
                        to_remove = Some(idx);
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("Instrument:");
                    let prev_instrument = track.instrument.clone();
                    egui::ComboBox::from_id_salt(format!("track_inst_{}", idx))
                        .selected_text(&track.instrument)
                        .width(140.0)
                        .show_ui(ui, |ui| {
                            for &name in GM_INSTRUMENT_NAMES {
                                ui.selectable_value(
                                    &mut track.instrument,
                                    name.to_string(),
                                    name,
                                );
                            }
                        });
                    if track.instrument != prev_instrument {
                        track.program = gm_program_from_name(&track.instrument);
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("Ch:");
                    let mut ch = track.channel as i32;
                    if ui.add(egui::DragValue::new(&mut ch).range(0..=15)).changed() {
                        track.channel = ch as u8;
                    }
                });
                ui.checkbox(&mut track.muted, "Mute");
                ui.checkbox(&mut track.solo, "Solo");
            });
        }

        if let Some(idx) = to_remove {
            self.tracks.remove(idx);
        }

        if ui.button("+ Add Track").clicked() {
            let n = self.tracks.len() + 1;
            self.tracks.push(TrackState::new(
                &format!("Track {}", n),
                "piano",
                0,
                (n - 1).min(15) as u8,
            ));
        }
    }
}
