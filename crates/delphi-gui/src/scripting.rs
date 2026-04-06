use rhai::{Engine, Scope};

use crate::studio::StudioState;

/// Rhai-based scripting console for programmable composition.
pub struct ScriptEngine {
    engine: Engine,
    input: String,
    history: Vec<String>,
    output: Vec<String>,
    /// Pending commands that affect studio state (processed by caller).
    pub pending_commands: Vec<ScriptCommand>,
}

/// Commands the script console can issue to mutate studio state.
#[derive(Debug, Clone)]
pub enum ScriptCommand {
    SetTempo(f64),
    SetKey(String),
    SetTimeSig(u8, u8),
    AddCell { source: String, instrument: String, channel: u8 },
    SetCellInstrument { cell: usize, instrument: String },
    SetCellVelocity { cell: usize, velocity: u8 },
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
            pending_commands: Vec::new(),
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
        ui.label(
            egui::RichText::new("Rhai scripting + studio commands")
                .small()
                .color(egui::Color32::from_rgb(150, 150, 150)),
        );
        ui.separator();

        // Quick-reference for studio commands
        ui.collapsing("Commands", |ui| {
            ui.label(egui::RichText::new(
                "set_tempo(120.0)\n\
                 set_key(\"C\")\n\
                 set_time_sig(4, 4)\n\
                 add_cell(\"C4:q E4:q G4:q\", \"piano\", 0)\n\
                 set_instrument(0, \"violin\")\n\
                 set_velocity(0, 90)\n\
                 info()  — show project info\n\
                 cells() — list cells\n\
                 tracks() — list tracks\n\
                 note_to_midi(\"C4\")\n\
                 midi_to_note(60)\n\
                 chord_notes(\"C4\", \"maj7\")\n\
                 scale_notes(\"C4\", \"major\")"
            ).monospace().small());
        });
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
                    .hint_text("e.g. set_tempo(140.0) or info()"),
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
                let result = self.eval_with_studio(&script, studio);
                self.output.push(result);
                self.input.clear();
            }
        }

        // Process any pending commands
        for cmd in self.pending_commands.drain(..) {
            match cmd {
                ScriptCommand::SetTempo(bpm) => studio.settings.bpm = bpm,
                ScriptCommand::SetKey(key) => studio.settings.key_name = key,
                ScriptCommand::SetTimeSig(num, den) => {
                    studio.settings.time_sig_num = num;
                    studio.settings.time_sig_den = den;
                }
                ScriptCommand::AddCell { source, instrument, channel } => {
                    let mut cell = crate::studio::Cell::new_notation();
                    cell.source = source;
                    cell.instrument = instrument;
                    cell.channel = channel;
                    studio.cells.push(cell);
                }
                ScriptCommand::SetCellInstrument { cell, instrument } => {
                    if cell < studio.cells.len() {
                        studio.cells[cell].instrument = instrument;
                    }
                }
                ScriptCommand::SetCellVelocity { cell, velocity } => {
                    if cell < studio.cells.len() {
                        studio.cells[cell].velocity = velocity;
                    }
                }
            }
        }
    }

    /// Evaluate a script, handling studio-specific commands explicitly.
    fn eval_with_studio(&mut self, script: &str, studio: &StudioState) -> String {
        let trimmed = script.trim();
        let lower = trimmed.to_lowercase();

        // Handle studio built-in commands that read/modify state
        if lower == "info()" || lower == "info" {
            return format!(
                "Title: {}\nTempo: {} BPM\nKey: {}\nTime: {}/{}\nCells: {}\nTracks: {}",
                studio.settings.title,
                studio.settings.bpm,
                studio.settings.key_name,
                studio.settings.time_sig_num,
                studio.settings.time_sig_den,
                studio.cells.len(),
                studio.tracks.len(),
            );
        }

        if lower == "cells()" || lower == "cells" {
            if studio.cells.is_empty() {
                return "No cells".into();
            }
            return studio.cells.iter().enumerate().map(|(i, c)| {
                let preview: String = c.source.chars().take(40).collect();
                let preview = preview.replace('\n', " ");
                format!("[{}] {} — {} — {}", i, c.cell_type, if c.instrument.is_empty() { "—" } else { &c.instrument }, preview)
            }).collect::<Vec<_>>().join("\n");
        }

        if lower == "tracks()" || lower == "tracks" {
            if studio.tracks.is_empty() {
                return "No tracks".into();
            }
            return studio.tracks.iter().enumerate().map(|(i, t)| {
                format!("[{}] {} — {} ch:{} gain:{:.2}{}{}", i, t.name, t.instrument, t.channel, t.gain,
                    if t.muted { " [M]" } else { "" },
                    if t.solo { " [S]" } else { "" })
            }).collect::<Vec<_>>().join("\n");
        }

        // set_tempo(N)
        if let Some(inner) = extract_call(trimmed, "set_tempo") {
            if let Ok(bpm) = inner.parse::<f64>() {
                if (20.0..=300.0).contains(&bpm) {
                    self.pending_commands.push(ScriptCommand::SetTempo(bpm));
                    return format!("Tempo set to {} BPM", bpm);
                } else {
                    return "Error: BPM must be between 20 and 300".into();
                }
            }
            return format!("Error: invalid tempo value '{}'", inner);
        }

        // set_key("X")
        if let Some(inner) = extract_call(trimmed, "set_key") {
            let key = inner.trim_matches('"').trim_matches('\'').to_string();
            if !key.is_empty() {
                self.pending_commands.push(ScriptCommand::SetKey(key.clone()));
                return format!("Key set to {}", key);
            }
            return "Error: empty key name".into();
        }

        // set_time_sig(N, N)
        if let Some(inner) = extract_call(trimmed, "set_time_sig") {
            let parts: Vec<&str> = inner.split(',').map(|s| s.trim()).collect();
            if parts.len() == 2 {
                if let (Ok(num), Ok(den)) = (parts[0].parse::<u8>(), parts[1].parse::<u8>()) {
                    if num > 0 && den > 0 && den.is_power_of_two() {
                        self.pending_commands.push(ScriptCommand::SetTimeSig(num, den));
                        return format!("Time signature set to {}/{}", num, den);
                    }
                }
            }
            return "Error: usage: set_time_sig(4, 4)".into();
        }

        // add_cell("notation", "instrument", channel)
        if let Some(inner) = extract_call(trimmed, "add_cell") {
            let parts = split_quoted_args(&inner);
            if !parts.is_empty() {
                let source = parts[0].trim_matches('"').trim_matches('\'').to_string();
                let instrument = parts.get(1).map(|s| s.trim_matches('"').trim_matches('\'')).unwrap_or("piano").to_string();
                let channel = parts.get(2).and_then(|s| s.trim().parse::<u8>().ok()).unwrap_or(0);
                self.pending_commands.push(ScriptCommand::AddCell { source: source.clone(), instrument, channel });
                return format!("Added cell: {}", &source[..source.len().min(40)]);
            }
            return "Error: usage: add_cell(\"C4:q E4:q\", \"piano\", 0)".into();
        }

        // set_instrument(cell_idx, "name")
        if let Some(inner) = extract_call(trimmed, "set_instrument") {
            let parts = split_quoted_args(&inner);
            if parts.len() == 2 {
                if let Ok(cell) = parts[0].trim().parse::<usize>() {
                    let instrument = parts[1].trim_matches('"').trim_matches('\'').to_string();
                    self.pending_commands.push(ScriptCommand::SetCellInstrument { cell, instrument: instrument.clone() });
                    return format!("Cell {} instrument set to {}", cell, instrument);
                }
            }
            return "Error: usage: set_instrument(0, \"violin\")".into();
        }

        // set_velocity(cell_idx, vel)
        if let Some(inner) = extract_call(trimmed, "set_velocity") {
            let parts: Vec<&str> = inner.split(',').map(|s| s.trim()).collect();
            if parts.len() == 2 {
                if let (Ok(cell), Ok(vel)) = (parts[0].parse::<usize>(), parts[1].parse::<u8>()) {
                    self.pending_commands.push(ScriptCommand::SetCellVelocity { cell, velocity: vel.min(127) });
                    return format!("Cell {} velocity set to {}", cell, vel.min(127));
                }
            }
            return "Error: usage: set_velocity(0, 90)".into();
        }

        // Fall back to Rhai evaluation for math/music functions
        self.eval(script)
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

/// Extract the inner arguments from a function call like "set_tempo(120.0)".
/// Returns Some("120.0") for input "set_tempo(120.0)".
fn extract_call<'a>(input: &'a str, func_name: &str) -> Option<&'a str> {
    let trimmed = input.trim().trim_end_matches(';');
    // Case-insensitive prefix match
    if trimmed.len() >= func_name.len()
        && trimmed[..func_name.len()].eq_ignore_ascii_case(func_name)
    {
        let rest = &trimmed[func_name.len()..];
        if rest.starts_with('(') && rest.ends_with(')') {
            return Some(&rest[1..rest.len() - 1]);
        }
    }
    None
}

/// Split comma-separated arguments, respecting quoted strings.
fn split_quoted_args(input: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut start = 0;
    let mut in_quotes = false;
    let mut quote_char = '"';
    let bytes = input.as_bytes();
    for i in 0..bytes.len() {
        let ch = bytes[i] as char;
        if !in_quotes && (ch == '"' || ch == '\'') {
            in_quotes = true;
            quote_char = ch;
        } else if in_quotes && ch == quote_char {
            in_quotes = false;
        } else if !in_quotes && ch == ',' {
            parts.push(input[start..i].trim());
            start = i + 1;
        }
    }
    if start < input.len() {
        parts.push(input[start..].trim());
    }
    parts
}
