use delphi_core::{ChordQuality, Note, Scale, ScaleType};
use egui::Color32;

/// Interactive music theory explorer panel.
pub struct TheoryPanel {
    pub root_note: String,
    pub chord_quality: String,
    pub scale_type: String,
    pub mode: TheoryMode,
    pub result_text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TheoryMode {
    Chords,
    Scales,
    Intervals,
    CircleOfFifths,
}

impl TheoryPanel {
    pub fn new() -> Self {
        Self {
            root_note: "C4".into(),
            chord_quality: "Major".to_string(),
            scale_type: "Major".to_string(),
            mode: TheoryMode::Chords,
            result_text: String::new(),
        }
    }

    pub fn ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.mode, TheoryMode::Chords, "Chords");
            ui.selectable_value(&mut self.mode, TheoryMode::Scales, "Scales");
            ui.selectable_value(&mut self.mode, TheoryMode::Intervals, "Intervals");
            ui.selectable_value(&mut self.mode, TheoryMode::CircleOfFifths, "Circle of 5ths");
        });
        ui.separator();

        match self.mode {
            TheoryMode::Chords => self.chords_ui(ui),
            TheoryMode::Scales => self.scales_ui(ui),
            TheoryMode::Intervals => self.intervals_ui(ui),
            TheoryMode::CircleOfFifths => self.circle_of_fifths_ui(ui),
        }
    }

    fn chords_ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Root:");
            ui.text_edit_singleline(&mut self.root_note);
        });

        ui.horizontal(|ui| {
            ui.label("Quality:");
            egui::ComboBox::from_id_salt("chord_quality")
                .selected_text(&self.chord_quality)
                .show_ui(ui, |ui| {
                    for q in &[
                        "Major", "Minor", "Diminished", "Augmented",
                        "Major7", "Minor7", "Dominant7", "Diminished7",
                        "Sus2", "Sus4", "Add9", "Power",
                    ] {
                        ui.selectable_value(&mut self.chord_quality, q.to_string(), *q);
                    }
                });
        });

        if ui.button("Show Chord").clicked() {
            self.result_text = format!(
                "{} {} — intervals from delphi-core ChordQuality",
                self.root_note, self.chord_quality
            );
            // TODO: use delphi_core::Chord to compute actual notes
        }

        if !self.result_text.is_empty() {
            ui.separator();
            ui.label(
                egui::RichText::new(&self.result_text)
                    .monospace()
                    .color(Color32::from_rgb(229, 192, 123)),
            );
        }

        // Piano keyboard visualization
        ui.separator();
        self.draw_keyboard(ui, &[]);
    }

    fn scales_ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Root:");
            ui.text_edit_singleline(&mut self.root_note);
        });

        ui.horizontal(|ui| {
            ui.label("Scale:");
            egui::ComboBox::from_id_salt("scale_type")
                .selected_text(&self.scale_type)
                .show_ui(ui, |ui| {
                    for s in &[
                        "Major", "Natural Minor", "Harmonic Minor", "Melodic Minor",
                        "Dorian", "Phrygian", "Lydian", "Mixolydian",
                        "Major Pentatonic", "Minor Pentatonic", "Blues",
                        "Whole Tone", "Chromatic",
                    ] {
                        ui.selectable_value(&mut self.scale_type, s.to_string(), *s);
                    }
                });
        });

        if ui.button("Show Scale").clicked() {
            self.result_text = format!(
                "{} {} — scale degrees from delphi-core ScaleType",
                self.root_note, self.scale_type
            );
            // TODO: use delphi_core::Scale to compute actual notes
        }

        if !self.result_text.is_empty() {
            ui.separator();
            ui.label(
                egui::RichText::new(&self.result_text)
                    .monospace()
                    .color(Color32::from_rgb(152, 195, 121)),
            );
        }

        ui.separator();
        self.draw_keyboard(ui, &[]);
    }

    fn intervals_ui(&mut self, ui: &mut egui::Ui) {
        ui.label("Interval reference:");
        ui.separator();

        let intervals = [
            ("m2", "Minor 2nd", 1),
            ("M2", "Major 2nd", 2),
            ("m3", "Minor 3rd", 3),
            ("M3", "Major 3rd", 4),
            ("P4", "Perfect 4th", 5),
            ("TT", "Tritone", 6),
            ("P5", "Perfect 5th", 7),
            ("m6", "Minor 6th", 8),
            ("M6", "Major 6th", 9),
            ("m7", "Minor 7th", 10),
            ("M7", "Major 7th", 11),
            ("P8", "Octave", 12),
        ];

        egui::Grid::new("intervals_grid")
            .striped(true)
            .show(ui, |ui| {
                ui.label(egui::RichText::new("Symbol").strong());
                ui.label(egui::RichText::new("Name").strong());
                ui.label(egui::RichText::new("Semitones").strong());
                ui.end_row();

                for (sym, name, semi) in &intervals {
                    ui.label(
                        egui::RichText::new(*sym)
                            .monospace()
                            .color(Color32::from_rgb(86, 182, 194)),
                    );
                    ui.label(*name);
                    ui.label(format!("{}", semi));
                    ui.end_row();
                }
            });
    }

    fn circle_of_fifths_ui(&mut self, ui: &mut egui::Ui) {
        let available = ui.available_size();
        let size = available.x.min(available.y).min(300.0);
        let (response, painter) = ui.allocate_painter(
            egui::Vec2::new(size, size),
            egui::Sense::click(),
        );
        let center = response.rect.center();
        let radius = size * 0.4;

        let keys = [
            "C", "G", "D", "A", "E", "B", "F#/Gb", "Db", "Ab", "Eb", "Bb", "F",
        ];
        let minor_keys = [
            "Am", "Em", "Bm", "F#m", "C#m", "G#m", "D#m/Ebm", "Bbm", "Fm", "Cm", "Gm", "Dm",
        ];

        for (i, (major, minor)) in keys.iter().zip(minor_keys.iter()).enumerate() {
            let angle = std::f32::consts::TAU * (i as f32 / 12.0) - std::f32::consts::FRAC_PI_2;
            let x = center.x + angle.cos() * radius;
            let y = center.y + angle.sin() * radius;

            // Major key (outer)
            painter.text(
                egui::Pos2::new(x, y),
                egui::Align2::CENTER_CENTER,
                *major,
                egui::FontId::proportional(14.0),
                Color32::from_rgb(229, 192, 123),
            );

            // Minor key (inner)
            let inner_r = radius * 0.65;
            let ix = center.x + angle.cos() * inner_r;
            let iy = center.y + angle.sin() * inner_r;
            painter.text(
                egui::Pos2::new(ix, iy),
                egui::Align2::CENTER_CENTER,
                *minor,
                egui::FontId::proportional(10.0),
                Color32::from_rgb(150, 150, 150),
            );
        }

        // Center circle
        painter.circle_stroke(
            center,
            radius * 0.5,
            egui::Stroke::new(1.0, Color32::from_rgb(60, 60, 65)),
        );
        painter.circle_stroke(
            center,
            radius * 0.85,
            egui::Stroke::new(1.0, Color32::from_rgb(60, 60, 65)),
        );
    }

    /// Draw a small piano keyboard with highlighted notes.
    fn draw_keyboard(&self, ui: &mut egui::Ui, highlighted_midi: &[u8]) {
        let (response, painter) = ui.allocate_painter(
            egui::Vec2::new(ui.available_width(), 50.0),
            egui::Sense::click(),
        );
        let rect = response.rect;

        // Draw 2 octaves of piano keys (C3–B4)
        let white_notes = [0, 2, 4, 5, 7, 9, 11]; // C D E F G A B
        let start_midi: u8 = 48; // C3
        let octaves = 2;
        let total_whites = octaves * 7;
        let key_w = rect.width() / total_whites as f32;
        let key_h = rect.height();

        // White keys
        for i in 0..total_whites {
            let oct = i / 7;
            let degree = i % 7;
            let midi = start_midi + (oct * 12) as u8 + white_notes[degree];
            let x = rect.left() + i as f32 * key_w;
            let is_highlighted = highlighted_midi.contains(&midi);

            let color = if is_highlighted {
                Color32::from_rgb(86, 182, 194)
            } else {
                Color32::from_rgb(220, 220, 220)
            };

            let key_rect = egui::Rect::from_min_size(
                egui::Pos2::new(x, rect.top()),
                egui::Vec2::new(key_w - 1.0, key_h),
            );
            painter.rect_filled(key_rect, 1.0, color);
            painter.rect_stroke(
                key_rect,
                1.0,
                egui::Stroke::new(0.5, Color32::from_rgb(100, 100, 100)),
            );
        }

        // Black keys
        let black_offsets = [1, 3, 6, 8, 10]; // C# D# F# G# A#
        let black_positions = [0.7, 1.7, 3.7, 4.7, 5.7]; // relative positions between white keys
        for oct in 0..octaves {
            for (j, &offset) in black_offsets.iter().enumerate() {
                let midi = start_midi + (oct * 12) as u8 + offset;
                let x = rect.left() + (oct * 7) as f32 * key_w + black_positions[j] * key_w;
                let is_highlighted = highlighted_midi.contains(&midi);

                let color = if is_highlighted {
                    Color32::from_rgb(229, 192, 123)
                } else {
                    Color32::from_rgb(30, 30, 35)
                };

                let bkey_rect = egui::Rect::from_min_size(
                    egui::Pos2::new(x, rect.top()),
                    egui::Vec2::new(key_w * 0.6, key_h * 0.6),
                );
                painter.rect_filled(bkey_rect, 1.0, color);
            }
        }
    }
}
