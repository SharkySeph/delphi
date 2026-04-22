use crate::studio::StudioState;

/// The mixer panel: per-track gain, pan, mute, solo.
/// Master gain is stored in `studio.settings.master_gain` so it persists with
/// the project and is honoured consistently by playback and export.
pub struct MixerPanel;

impl MixerPanel {
    pub fn new() -> Self {
        Self
    }

    pub fn ui(&mut self, ui: &mut egui::Ui, studio: &mut StudioState) {
        ui.horizontal(|ui| {
            ui.heading("Mixer");
            ui.separator();
            ui.label("Master:")
                .on_hover_text("Master output gain — saved with the project and applied to playback and export");
            ui.add(
                egui::Slider::new(&mut studio.settings.master_gain, 0.0..=1.5)
                    .text("vol")
                    .fixed_decimals(2),
            );
        });
        ui.separator();

        // Per-track mixer strips
        egui::ScrollArea::horizontal().show(ui, |ui| {
            ui.horizontal(|ui| {
                // Check if any track is solo'd
                let any_solo = studio.tracks.iter().any(|t| t.solo);

                for track in studio.tracks.iter_mut() {
                    ui.vertical(|ui| {
                        ui.set_min_width(80.0);

                        // Track name
                        ui.label(
                            egui::RichText::new(&track.name)
                                .strong()
                                .color(if track.muted || (any_solo && !track.solo) {
                                    egui::Color32::from_rgb(100, 100, 100)
                                } else {
                                    egui::Color32::from_rgb(200, 200, 200)
                                }),
                        );

                        // Gain fader (vertical slider)
                        ui.add(
                            egui::Slider::new(&mut track.gain, 0.0..=1.5)
                                .vertical()
                                .text(""),
                        )
                        .on_hover_text("Per-track gain");

                        // Pan knob
                        ui.horizontal(|ui| {
                            ui.label("L");
                            ui.add(
                                egui::Slider::new(&mut track.pan, 0.0..=1.0)
                                    .show_value(false)
                                    .text(""),
                            )
                            .on_hover_text(format!("Pan: {:.2}", track.pan));
                            ui.label("R");
                        });

                        // Mute / Solo buttons
                        ui.horizontal(|ui| {
                            let mute_color = if track.muted {
                                egui::Color32::from_rgb(224, 108, 117)
                            } else {
                                ui.style().visuals.text_color()
                            };
                            if ui
                                .button(egui::RichText::new("M").color(mute_color))
                                .on_hover_text("Mute this track")
                                .clicked()
                            {
                                track.muted = !track.muted;
                            }

                            let solo_color = if track.solo {
                                egui::Color32::from_rgb(229, 192, 123)
                            } else {
                                ui.style().visuals.text_color()
                            };
                            if ui
                                .button(egui::RichText::new("S").color(solo_color))
                                .on_hover_text("Solo this track (mutes all others)")
                                .clicked()
                            {
                                track.solo = !track.solo;
                            }
                        });

                        // Instrument label
                        ui.label(
                            egui::RichText::new(&track.instrument)
                                .small()
                                .color(egui::Color32::from_rgb(150, 150, 150)),
                        );
                    });

                    ui.separator();
                }

                // Empty state
                if studio.tracks.is_empty() {
                    ui.centered_and_justified(|ui| {
                        ui.label(
                            egui::RichText::new("No tracks — add cells in the Editor to populate the mixer")
                                .color(egui::Color32::from_rgb(120, 120, 130)),
                        );
                    });
                }
            });
        });
    }
}

