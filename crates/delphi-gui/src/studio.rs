use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use delphi_core::{Duration, Dynamic, Note, Tempo};
use delphi_engine::SfEvent;

/// A single cell in the studio notebook.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cell {
    pub cell_type: String, // "code", "notation", "markdown"
    pub source: String,
    pub output: String,
    pub label: String,
    pub instrument: String,
    pub channel: u8,
    pub velocity: u8,
    #[serde(default)]
    pub collapsed: bool,
}

impl Cell {
    pub fn new_code() -> Self {
        Self {
            cell_type: "code".into(),
            source: String::new(),
            output: String::new(),
            label: String::new(),
            instrument: "piano".into(),
            channel: 0,
            velocity: 80,
            collapsed: false,
        }
    }

    pub fn new_notation() -> Self {
        Self {
            cell_type: "notation".into(),
            source: String::new(),
            output: String::new(),
            label: String::new(),
            instrument: "piano".into(),
            channel: 0,
            velocity: 80,
            collapsed: false,
        }
    }

    pub fn new_markdown() -> Self {
        Self {
            cell_type: "markdown".into(),
            source: String::new(),
            output: String::new(),
            label: String::new(),
            instrument: String::new(),
            channel: 0,
            velocity: 80,
            collapsed: false,
        }
    }
}

/// A track in the arrangement.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackState {
    pub name: String,
    pub instrument: String,
    pub program: u8,
    pub channel: u8,
    pub gain: f32,
    pub pan: f32,     // 0.0 = left, 0.5 = center, 1.0 = right
    pub muted: bool,
    pub solo: bool,
}

impl TrackState {
    pub fn new(name: &str, instrument: &str, program: u8, channel: u8) -> Self {
        Self {
            name: name.into(),
            instrument: instrument.into(),
            program,
            channel,
            gain: 1.0,
            pan: 0.5,
            muted: false,
            solo: false,
        }
    }
}

/// Project settings persisted with the .dstudio file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectSettings {
    pub title: String,
    pub bpm: f64,
    pub key_name: String,
    pub time_sig_num: u8,
    pub time_sig_den: u8,
    pub swing: f32,
    pub humanize: f32,
    pub soundfont_path: Option<String>,
}

impl Default for ProjectSettings {
    fn default() -> Self {
        Self {
            title: "Untitled".into(),
            bpm: 120.0,
            key_name: "C major".into(),
            time_sig_num: 4,
            time_sig_den: 4,
            swing: 0.0,
            humanize: 0.0,
            soundfont_path: None,
        }
    }
}

/// The top-level studio state: cells, tracks, and project settings.
pub struct StudioState {
    pub cells: Vec<Cell>,
    pub tracks: Vec<TrackState>,
    pub settings: ProjectSettings,
}

impl StudioState {
    pub fn new() -> Self {
        Self {
            cells: vec![Cell::new_notation()],
            tracks: vec![TrackState::new("Track 1", "piano", 0, 0)],
            settings: ProjectSettings::default(),
        }
    }

    pub fn add_cell(&mut self) {
        self.cells.push(Cell::new_code());
    }

    pub fn add_notation_cell(&mut self) {
        self.cells.push(Cell::new_notation());
    }

    pub fn add_markdown_cell(&mut self) {
        self.cells.push(Cell::new_markdown());
    }

    pub fn tempo(&self) -> Tempo {
        Tempo { bpm: self.settings.bpm }
    }

    /// Save project state to a .dstudio JSON file.
    pub fn save(&self, path: &PathBuf) {
        #[derive(Serialize)]
        struct Project<'a> {
            settings: &'a ProjectSettings,
            cells: &'a [Cell],
            tracks: &'a [TrackState],
        }

        let project = Project {
            settings: &self.settings,
            cells: &self.cells,
            tracks: &self.tracks,
        };

        if let Ok(json) = serde_json::to_string_pretty(&project) {
            let _ = std::fs::write(path, json);
        }
    }

    /// Load project state from a .dstudio JSON file.
    pub fn load(&mut self, path: &PathBuf) {
        #[derive(Deserialize)]
        struct Project {
            settings: ProjectSettings,
            cells: Vec<Cell>,
            tracks: Vec<TrackState>,
        }

        if let Ok(contents) = std::fs::read_to_string(path) {
            if let Ok(project) = serde_json::from_str::<Project>(&contents) {
                self.settings = project.settings;
                self.cells = project.cells;
                self.tracks = project.tracks;
            }
        }
    }

    /// Move a cell up by one position. Returns true if moved.
    pub fn move_cell_up(&mut self, idx: usize) -> bool {
        if idx > 0 && idx < self.cells.len() {
            self.cells.swap(idx, idx - 1);
            true
        } else {
            false
        }
    }

    /// Move a cell down by one position. Returns true if moved.
    pub fn move_cell_down(&mut self, idx: usize) -> bool {
        if idx + 1 < self.cells.len() {
            self.cells.swap(idx, idx + 1);
            true
        } else {
            false
        }
    }

    /// Collect SfEvents from all cells (or a single cell) by parsing notation.
    pub fn collect_events(&self, cell_index: Option<usize>) -> Vec<SfEvent> {
        let mut all_events = Vec::new();
        let cells: Vec<(usize, &Cell)> = match cell_index {
            Some(idx) => {
                if idx < self.cells.len() {
                    vec![(idx, &self.cells[idx])]
                } else {
                    return all_events;
                }
            }
            None => self.cells.iter().enumerate().collect(),
        };

        for (_cell_idx, cell) in cells {
            if cell.cell_type == "markdown" {
                continue;
            }
            let source = &cell.source;
            if source.trim().is_empty() {
                continue;
            }
            let channel = cell.channel;
            let program = gm_program_from_name(&cell.instrument);
            let velocity = cell.velocity;
            let events = parse_notation_to_events(source, channel, program, velocity);
            all_events.extend(events);
        }

        all_events
    }

    /// Render the track list in the side panel.
    pub fn tracks_ui(&mut self, ui: &mut egui::Ui) {
        ui.heading("Tracks");

        let mut to_remove: Option<usize> = None;
        for (idx, track) in self.tracks.iter_mut().enumerate() {
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    ui.text_edit_singleline(&mut track.name);
                    if ui.small_button("🗑").clicked() {
                        to_remove = Some(idx);
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("Program:");
                    ui.text_edit_singleline(&mut track.instrument);
                });
                ui.horizontal(|ui| {
                    ui.label("Ch:");
                    let mut ch = track.channel as i32;
                    if ui.add(egui::DragValue::new(&mut ch).range(0..=15)).changed() {
                        track.channel = ch as u8;
                    }
                    ui.label("Vel:");
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

// ─── Notation Parser ─────────────────────────────────────────────────

/// Parse .delphi notation text into SfEvents.
/// Supports: notes (C4, D#5), chords (Cmaj7, Am7), durations (:q, :h, :8),
/// rests (., ~, r), bar lines, ties (~), polyphony (C4,E4,G4),
/// dynamics (!p, !ff), drum names, repeats (*N), Euclidean patterns.
pub fn parse_notation_to_events(
    source: &str,
    channel: u8,
    program: u8,
    default_velocity: u8,
) -> Vec<SfEvent> {
    let mut events = Vec::new();
    let mut tick: u32 = 0;
    let mut current_duration: u32 = 480; // default quarter note
    let mut velocity = default_velocity;

    for line in source.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with("//") || line.starts_with('#') {
            continue;
        }
        // Handle @pragma lines
        if line.starts_with('@') {
            continue;
        }

        // Split by whitespace and bar lines
        let tokens: Vec<&str> = line
            .split(|c: char| c.is_whitespace() || c == '|')
            .filter(|s| !s.is_empty())
            .collect();

        for token in tokens {
            // Duration suffix: :q, :h, :8, :w, :16, :32, with optional dot
            if let Some(rest) = token.strip_prefix(':') {
                if let Some(d) = Duration::from_suffix(rest) {
                    current_duration = d.ticks;
                }
                continue;
            }

            // Dynamic marking: !p, !ff, !mf, etc.
            if let Some(rest) = token.strip_prefix('!') {
                if let Some(dyn_val) = Dynamic::from_str_dynamic(rest) {
                    velocity = dyn_val.velocity();
                }
                continue;
            }

            // Rest: ., ~, r, rest
            if matches!(token, "." | "~" | "r" | "rest") {
                tick += current_duration;
                continue;
            }

            // Repeat: *N suffix on previous events (simplified: just advance time)
            if token.starts_with('*') {
                if let Ok(n) = token[1..].parse::<u32>() {
                    // Repeat last duration n-1 more times
                    tick += current_duration * (n.saturating_sub(1));
                }
                continue;
            }

            // Euclidean pattern: name(hits,steps) e.g. kick(3,8)
            if token.contains('(') && token.ends_with(')') {
                if let Some(paren) = token.find('(') {
                    let name = &token[..paren];
                    let args = &token[paren + 1..token.len() - 1];
                    if let Some((hits_s, steps_s)) = args.split_once(',') {
                        if let (Ok(hits), Ok(steps)) =
                            (hits_s.trim().parse::<u32>(), steps_s.trim().parse::<u32>())
                        {
                            let midi = drum_name_to_midi(name);
                            if midi > 0 {
                                let pattern = euclidean(hits as usize, steps as usize);
                                let step_dur = current_duration;
                                for (i, &hit) in pattern.iter().enumerate() {
                                    if hit {
                                        events.push(SfEvent {
                                            tick: tick + i as u32 * step_dur,
                                            midi_note: midi,
                                            velocity,
                                            duration_ticks: step_dur / 2,
                                            channel: 9, // drums on channel 10
                                            program: 0,
                                        });
                                    }
                                }
                                tick += steps * step_dur;
                                continue;
                            }
                        }
                    }
                }
            }

            // Drum name (standalone)
            let drum_midi = drum_name_to_midi(token);
            if drum_midi > 0 {
                events.push(SfEvent {
                    tick,
                    midi_note: drum_midi,
                    velocity,
                    duration_ticks: current_duration / 2,
                    channel: 9,
                    program: 0,
                });
                tick += current_duration;
                continue;
            }

            // Polyphony: C4,E4,G4 (comma-separated notes at same tick)
            if token.contains(',') {
                let parts: Vec<&str> = token.split(',').collect();
                for part in &parts {
                    if let Ok(note) = part.parse::<Note>() {
                        events.push(SfEvent {
                            tick,
                            midi_note: note.to_midi(),
                            velocity,
                            duration_ticks: current_duration,
                            channel,
                            program,
                        });
                    }
                }
                tick += current_duration;
                continue;
            }

            // Try chord symbol (Cmaj7, Am, G7, etc.)
            if let Ok(chord) = token.parse::<delphi_core::Chord>() {
                for midi in chord.to_midi() {
                    events.push(SfEvent {
                        tick,
                        midi_note: midi,
                        velocity,
                        duration_ticks: current_duration,
                        channel,
                        program,
                    });
                }
                tick += current_duration;
                continue;
            }

            // Try note (C4, D#5, Bb3)
            if let Ok(note) = token.parse::<Note>() {
                events.push(SfEvent {
                    tick,
                    midi_note: note.to_midi(),
                    velocity,
                    duration_ticks: current_duration,
                    channel,
                    program,
                });
                tick += current_duration;
                continue;
            }

            // Unknown token — skip silently
        }
    }

    events
}

/// Euclidean rhythm generator.
fn euclidean(hits: usize, steps: usize) -> Vec<bool> {
    if steps == 0 {
        return vec![];
    }
    let mut pattern = vec![false; steps];
    if hits == 0 {
        return pattern;
    }
    for i in 0..hits.min(steps) {
        let pos = (i * steps) / hits;
        pattern[pos] = true;
    }
    pattern
}

/// Map drum names to General MIDI percussion note numbers.
fn drum_name_to_midi(name: &str) -> u8 {
    match name.to_lowercase().as_str() {
        "kick" | "bd" => 36,
        "snare" | "sd" => 38,
        "rimshot" | "rim" => 37,
        "clap" | "cp" => 39,
        "hihat" | "hh" => 42,
        "openhat" | "oh" => 46,
        "ride" | "rd" => 51,
        "crash" | "cr" => 49,
        "tom1" => 50,
        "tom2" => 47,
        "tom3" => 43,
        "cowbell" | "cb" => 56,
        "tambourine" | "tamb" => 54,
        "shaker" => 70,
        "clave" => 75,
        "woodblock" | "wb" => 76,
        "triangle" | "tri" => 81,
        "guiro" => 73,
        _ => 0, // 0 = not a drum
    }
}

/// Map instrument name to General MIDI program number.
pub fn gm_program_from_name(name: &str) -> u8 {
    match name.to_lowercase().as_str() {
        "piano" | "acoustic grand" | "grand piano" => 0,
        "bright piano" => 1,
        "electric piano" | "epiano" | "rhodes" => 4,
        "harpsichord" => 6,
        "clavinet" | "clav" => 7,
        "celesta" => 8,
        "glockenspiel" => 9,
        "music box" => 10,
        "vibraphone" | "vibes" => 11,
        "marimba" => 12,
        "xylophone" => 13,
        "organ" | "church organ" => 19,
        "accordion" => 21,
        "harmonica" => 22,
        "nylon guitar" | "classical guitar" => 24,
        "acoustic guitar" | "steel guitar" => 25,
        "electric guitar" | "clean guitar" => 27,
        "overdriven guitar" | "distortion" => 29,
        "bass" | "acoustic bass" => 32,
        "electric bass" | "finger bass" => 33,
        "pick bass" => 34,
        "slap bass" => 36,
        "synth bass" => 38,
        "violin" => 40,
        "viola" => 41,
        "cello" => 42,
        "contrabass" | "double bass" => 43,
        "strings" | "string ensemble" => 48,
        "synth strings" => 50,
        "choir" | "voice" | "aah" => 52,
        "orchestra hit" => 55,
        "trumpet" => 56,
        "trombone" => 57,
        "tuba" => 58,
        "french horn" | "horn" => 60,
        "brass" | "brass section" => 61,
        "synth brass" => 62,
        "soprano sax" => 64,
        "alto sax" | "sax" => 65,
        "tenor sax" => 66,
        "baritone sax" => 67,
        "oboe" => 68,
        "english horn" => 69,
        "bassoon" => 70,
        "clarinet" => 71,
        "piccolo" => 72,
        "flute" => 73,
        "recorder" => 74,
        "pan flute" => 75,
        "sitar" => 104,
        "banjo" => 105,
        "shamisen" => 106,
        "koto" => 107,
        "steel drum" => 114,
        "drums" | "percussion" | "drum kit" => 0, // channel 9 handles this
        _ => 0,
    }
}
