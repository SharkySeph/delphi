use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use delphi_core::{Duration, Key, Note, Scale, ScaleType, Tempo, TimeSignature};

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
