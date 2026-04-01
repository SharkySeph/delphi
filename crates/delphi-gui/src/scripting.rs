use rhai::{Engine, Scope};

use crate::studio::StudioState;

/// Rhai-based scripting console for programmable composition.
pub struct ScriptEngine {
    engine: Engine,
    input: String,
    history: Vec<String>,
    output: Vec<String>,
}

impl ScriptEngine {
    pub fn new() -> Self {
        let mut engine = Engine::new();

        // Register Delphi-specific functions into Rhai.
        // These map to the same API as the Python DSL.

        engine.register_fn("note_to_midi", |name: &str| -> i64 {
            note_name_to_midi(name).unwrap_or(-1) as i64
        });

        engine.register_fn("midi_to_note", |midi: i64| -> String {
            let note_names = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];
            if midi < 0 || midi > 127 {
                return "?".into();
            }
            let octave = (midi / 12) - 1;
            let name = note_names[midi as usize % 12];
            format!("{}{}", name, octave)
        });

        engine.register_fn("chord_notes", |root: &str, quality: &str| -> Vec<i64> {
            let root_midi = note_name_to_midi(root).unwrap_or(60) as i64;
            let intervals = match quality {
                "major" | "maj" | "" => vec![0, 4, 7],
                "minor" | "min" | "m" => vec![0, 3, 7],
                "dim" => vec![0, 3, 6],
                "aug" => vec![0, 4, 8],
                "7" | "dom7" => vec![0, 4, 7, 10],
                "maj7" => vec![0, 4, 7, 11],
                "m7" | "min7" => vec![0, 3, 7, 10],
                "dim7" => vec![0, 3, 6, 9],
                "sus2" => vec![0, 2, 7],
                "sus4" => vec![0, 5, 7],
                _ => vec![0, 4, 7],
            };
            intervals.iter().map(|i| root_midi + i).collect()
        });

        engine.register_fn("scale_notes", |root: &str, scale_type: &str| -> Vec<i64> {
            let root_midi = note_name_to_midi(root).unwrap_or(60) as i64;
            let intervals: Vec<i64> = match scale_type {
                "major" => vec![0, 2, 4, 5, 7, 9, 11],
                "minor" | "natural_minor" => vec![0, 2, 3, 5, 7, 8, 10],
                "harmonic_minor" => vec![0, 2, 3, 5, 7, 8, 11],
                "melodic_minor" => vec![0, 2, 3, 5, 7, 9, 11],
                "dorian" => vec![0, 2, 3, 5, 7, 9, 10],
                "phrygian" => vec![0, 1, 3, 5, 7, 8, 10],
                "lydian" => vec![0, 2, 4, 6, 7, 9, 11],
                "mixolydian" => vec![0, 2, 4, 5, 7, 9, 10],
                "blues" => vec![0, 3, 5, 6, 7, 10],
                "pentatonic" | "major_pentatonic" => vec![0, 2, 4, 7, 9],
                "minor_pentatonic" => vec![0, 3, 5, 7, 10],
                "chromatic" => (0..12).collect(),
                _ => vec![0, 2, 4, 5, 7, 9, 11],
            };
            intervals.iter().map(|i| root_midi + i).collect()
        });

        Self {
            engine,
            input: String::new(),
            history: Vec::new(),
            output: Vec::new(),
        }
    }

    /// Evaluate a script and return the output (or error).
    pub fn eval(&mut self, script: &str) -> String {
        let mut scope = Scope::new();
        match self.engine.eval_with_scope::<rhai::Dynamic>(&mut scope, script) {
            Ok(result) => format!("{}", result),
            Err(err) => format!("Error: {}", err),
        }
    }

    /// Render the scripting console UI.
    pub fn ui(&mut self, ui: &mut egui::Ui, studio: &mut StudioState) {
        ui.heading("Script Console");
        ui.label("Rhai scripting — compose with code");
        ui.separator();

        // Output history
        egui::ScrollArea::vertical()
            .max_height(ui.available_height() - 60.0)
            .stick_to_bottom(true)
            .show(ui, |ui| {
                for line in &self.output {
                    if line.starts_with("Error:") {
                        ui.label(
                            egui::RichText::new(line)
                                .monospace()
                                .color(egui::Color32::from_rgb(224, 108, 117)),
                        );
                    } else if line.starts_with(">>>") {
                        ui.label(
                            egui::RichText::new(line)
                                .monospace()
                                .color(egui::Color32::from_rgb(86, 182, 194)),
                        );
                    } else {
                        ui.label(egui::RichText::new(line).monospace());
                    }
                }
            });

        ui.separator();

        // Input line
        let response = ui.horizontal(|ui| {
            ui.label(">>>");
            let r = ui.add(
                egui::TextEdit::singleline(&mut self.input)
                    .code_editor()
                    .desired_width(ui.available_width() - 60.0)
                    .hint_text("e.g. chord_notes(\"C4\", \"maj7\")"),
            );
            if ui.button("Run").clicked() || (r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter))) {
                true
            } else {
                false
            }
        });

        if response.inner {
            let script = self.input.clone();
            if !script.trim().is_empty() {
                self.output.push(format!(">>> {}", script));
                self.history.push(script.clone());
                let result = self.eval(&script);
                self.output.push(result);
                self.input.clear();
            }
        }
    }
}

/// Parse a note name like "C4", "D#5", "Bb3" into a MIDI number.
fn note_name_to_midi(name: &str) -> Option<i32> {
    let bytes = name.as_bytes();
    if bytes.is_empty() {
        return None;
    }

    let pitch = match (bytes[0] as char).to_ascii_uppercase() {
        'C' => 0,
        'D' => 2,
        'E' => 4,
        'F' => 5,
        'G' => 7,
        'A' => 9,
        'B' => 11,
        _ => return None,
    };

    let mut offset = 1;
    let mut accidental = 0i32;
    while offset < bytes.len() {
        match bytes[offset] as char {
            '#' => {
                accidental += 1;
                offset += 1;
            }
            'b' => {
                accidental -= 1;
                offset += 1;
            }
            _ => break,
        }
    }

    let octave_str = &name[offset..];
    let octave: i32 = octave_str.parse().ok()?;
    let midi = (octave + 1) * 12 + pitch + accidental;

    if (0..=127).contains(&midi) {
        Some(midi)
    } else {
        None
    }
}
