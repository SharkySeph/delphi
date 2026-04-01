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
    /// Supports both GUI format (settings/cells/tracks) and Python format
    /// (version/title/settings/cells with type+meta).
    pub fn load(&mut self, path: &PathBuf) {
        // GUI-native format
        #[derive(Deserialize)]
        struct NativeProject {
            settings: ProjectSettings,
            cells: Vec<Cell>,
            #[serde(default)]
            tracks: Vec<TrackState>,
        }

        // Python Studio format
        #[derive(Deserialize)]
        struct PyProject {
            #[serde(default)]
            version: u32,
            #[serde(default)]
            title: String,
            #[serde(default)]
            settings: Option<PySettings>,
            #[serde(default)]
            cells: Vec<PyCell>,
        }
        #[derive(Deserialize)]
        struct PySettings {
            #[serde(default)]
            tempo: Option<f64>,
            #[serde(default)]
            key: Option<String>,
            #[serde(default)]
            time_sig: Option<String>,
        }
        #[derive(Deserialize)]
        struct PyCell {
            #[serde(alias = "type", alias = "cell_type")]
            cell_type: Option<String>,
            #[serde(default)]
            source: String,
            #[serde(default)]
            meta: Option<PyCellMeta>,
        }
        #[derive(Deserialize)]
        struct PyCellMeta {
            #[serde(default)]
            label: Option<String>,
            #[serde(default)]
            program: Option<String>,
            #[serde(default)]
            channel: Option<u8>,
            #[serde(default)]
            velocity: Option<u8>,
        }

        if let Ok(contents) = std::fs::read_to_string(path) {
            // Try native format first
            if let Ok(project) = serde_json::from_str::<NativeProject>(&contents) {
                self.settings = project.settings;
                self.cells = project.cells;
                self.tracks = if project.tracks.is_empty() {
                    vec![TrackState::new("Track 1", "piano", 0, 0)]
                } else {
                    project.tracks
                };
                return;
            }

            // Try Python format
            if let Ok(py) = serde_json::from_str::<PyProject>(&contents) {
                // Map settings
                let mut settings = ProjectSettings::default();
                if !py.title.is_empty() {
                    settings.title = py.title;
                }
                if let Some(ps) = &py.settings {
                    if let Some(bpm) = ps.tempo {
                        settings.bpm = bpm;
                    }
                    if let Some(ref key) = ps.key {
                        settings.key_name = key.clone();
                    }
                    if let Some(ref ts) = ps.time_sig {
                        let parts: Vec<&str> = ts.split('/').collect();
                        if parts.len() == 2 {
                            if let (Ok(n), Ok(d)) = (parts[0].parse::<u8>(), parts[1].parse::<u8>()) {
                                settings.time_sig_num = n;
                                settings.time_sig_den = d;
                            }
                        }
                    }
                }
                self.settings = settings;

                // Map cells
                self.cells = py.cells.iter().map(|pc| {
                    let ct = pc.cell_type.as_deref().unwrap_or("notation");
                    let mut cell = match ct {
                        "markdown" => Cell::new_markdown(),
                        "code" => Cell::new_code(),
                        _ => Cell::new_notation(),
                    };
                    cell.source = pc.source.clone();
                    if let Some(ref meta) = pc.meta {
                        if let Some(ref label) = meta.label {
                            cell.label = label.clone();
                        }
                        if let Some(ref prog) = meta.program {
                            cell.instrument = prog.clone();
                        }
                        if let Some(ch) = meta.channel {
                            cell.channel = ch;
                        }
                        if let Some(vel) = meta.velocity {
                            cell.velocity = vel;
                        }
                    }
                    // Also extract pragmas from source comments
                    for line in cell.source.lines() {
                        let line = line.trim();
                        if let Some(rest) = line.strip_prefix("# @instrument ") {
                            cell.instrument = rest.trim().to_string();
                        } else if let Some(rest) = line.strip_prefix("# @channel ") {
                            if let Ok(ch) = rest.trim().parse::<u8>() {
                                cell.channel = ch;
                            }
                        } else if let Some(rest) = line.strip_prefix("# @velocity ") {
                            if let Ok(v) = rest.trim().parse::<u8>() {
                                cell.velocity = v;
                            }
                        } else if let Some(rest) = line.strip_prefix("# @track ") {
                            cell.label = rest.trim().to_string();
                        }
                    }
                    cell
                }).collect();

                // Build tracks from cells
                self.tracks = Vec::new();
                for cell in &self.cells {
                    if cell.cell_type == "markdown" || cell.cell_type == "code" {
                        continue;
                    }
                    let name = if cell.label.is_empty() { "Track".to_string() } else { cell.label.clone() };
                    let program = gm_program_from_name(&cell.instrument);
                    if !self.tracks.iter().any(|t| t.name == name) {
                        self.tracks.push(TrackState::new(&name, &cell.instrument, program, cell.channel));
                    }
                }
                if self.tracks.is_empty() {
                    self.tracks.push(TrackState::new("Track 1", "piano", 0, 0));
                }
                return;
            }

            // Try loading as plain .delphi notation — single cell
            if !contents.trim().is_empty() && !contents.trim_start().starts_with('{') {
                self.settings = ProjectSettings::default();
                let mut cell = Cell::new_notation();
                cell.source = contents;
                self.cells = vec![cell];
                self.tracks = vec![TrackState::new("Track 1", "piano", 0, 0)];
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
        self.collect_events_mixed(cell_index, 1.0)
    }

    /// Collect events with mixer applied (mute/solo/gain).
    pub fn collect_events_mixed(&self, cell_index: Option<usize>, master_gain: f32) -> Vec<SfEvent> {
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

        // Check if any track is solo'd
        let any_solo = self.tracks.iter().any(|t| t.solo);

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

            // Find matching track by channel
            let track = self.tracks.iter().find(|t| t.channel == channel);

            // Skip muted / non-solo'd tracks
            if let Some(trk) = track {
                if trk.muted {
                    continue;
                }
                if any_solo && !trk.solo {
                    continue;
                }
            }

            let track_gain = track.map(|t| t.gain).unwrap_or(1.0);

            let mut events = parse_notation_to_events(source, channel, program, velocity);
            // Apply gain to velocity
            let gain = master_gain * track_gain;
            if (gain - 1.0).abs() > 0.001 {
                for ev in &mut events {
                    ev.velocity = ((ev.velocity as f32 * gain).round() as u8).min(127);
                }
            }
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

/// Articulation modifiers applied to a single note/chord.
struct Articulation {
    vel_mult: f32,
    dur_mult: f32,
}

fn parse_articulation(suffix: &str) -> Option<Articulation> {
    match suffix {
        "stac" | "staccato" => Some(Articulation { vel_mult: 1.0, dur_mult: 0.5 }),
        "stacc" | "staccatissimo" => Some(Articulation { vel_mult: 1.0, dur_mult: 0.25 }),
        "ten" | "tenuto" => Some(Articulation { vel_mult: 1.0, dur_mult: 1.1 }),
        "port" | "portato" => Some(Articulation { vel_mult: 1.0, dur_mult: 0.75 }),
        "acc" | "accent" => Some(Articulation { vel_mult: 1.3, dur_mult: 1.0 }),
        "marc" | "marcato" => Some(Articulation { vel_mult: 1.4, dur_mult: 0.85 }),
        "ferm" | "fermata" => Some(Articulation { vel_mult: 1.0, dur_mult: 2.0 }),
        "ghost" => Some(Articulation { vel_mult: 0.4, dur_mult: 0.8 }),
        "leg" | "legato" => Some(Articulation { vel_mult: 1.0, dur_mult: 1.05 }),
        "pizz" | "pizzicato" => Some(Articulation { vel_mult: 1.0, dur_mult: 0.3 }),
        "mute" => Some(Articulation { vel_mult: 0.6, dur_mult: 0.5 }),
        _ => None,
    }
}

/// Result of stripping inline modifiers from a token.
struct TokenParts<'a> {
    /// Core token (note name, chord symbol, drum name, rest, etc.)
    core: &'a str,
    /// Inline duration override (if any)
    duration: Option<u32>,
    /// Inline dynamic override (if any)
    inline_velocity: Option<u8>,
    /// Articulation modifier (if any)
    articulation: Option<Articulation>,
    /// Repeat count from *N suffix
    repeat: u32,
    /// Elongation weight from @N suffix
    weight: f32,
    /// Tie flag — token ends with ~ (or started with ~)
    tie_next: bool,
    /// Random removal probability `?` or `?0.3`
    random_prob: Option<f32>,
}

/// Split a raw token into its core plus inline duration/dynamic/articulation/repeat/weight.
///
/// Grammar: `CORE[:DURATION][!DYNAMIC][.ARTIC][*N][@W][?P][~]`
///
/// Also handles ornaments like `.tr`, `.mord`, `.grace` etc.  Ornaments are returned as
/// articulations with special vel_mult sentinels (negative) so the caller can expand them.
fn split_token(raw: &str) -> TokenParts<'_> {
    let mut parts = TokenParts {
        core: raw,
        duration: None,
        inline_velocity: None,
        articulation: None,
        repeat: 1,
        weight: 1.0,
        tie_next: false,
        random_prob: None,
    };

    let mut s = raw;

    // Trailing tie marker
    if s.ends_with('~') {
        parts.tie_next = true;
        s = &s[..s.len() - 1];
    }

    // Random removal ?P
    if let Some(q) = s.find('?') {
        let prob_str = &s[q + 1..];
        parts.random_prob = Some(prob_str.parse::<f32>().unwrap_or(0.5));
        s = &s[..q];
    }

    // Elongation @W
    if let Some(at) = s.find('@') {
        let w_str = &s[at + 1..];
        parts.weight = w_str.parse::<f32>().unwrap_or(1.0);
        s = &s[..at];
    }

    // Repeat *N
    if let Some(star) = s.rfind('*') {
        if let Ok(n) = s[star + 1..].parse::<u32>() {
            parts.repeat = n.max(1);
            s = &s[..star];
        }
    }

    // Articulation .suffix (must be checked before we consume the remaining string)
    // Ornaments: .tr, .trill, .mord, .mordent, .lmord, .turn, .gruppetto,
    //   .grace, .acciaccatura, .appoggiatura, .trem, .tremolo, .gliss, .glissando,
    //   .arp, .arpeggio, .roll
    if let Some(dot) = s.rfind('.') {
        let suffix = &s[dot + 1..];
        if !suffix.is_empty() && !suffix.chars().next().unwrap().is_ascii_digit() {
            if let Some(a) = parse_articulation(suffix) {
                parts.articulation = Some(a);
                s = &s[..dot];
            } else {
                // Ornament sentinels — use negative vel_mult as tag
                let ornament_tag = match suffix {
                    "tr" | "trill" => Some(-1.0),
                    "mord" | "mordent" => Some(-2.0),
                    "lmord" => Some(-3.0),
                    "turn" | "gruppetto" => Some(-4.0),
                    "grace" | "acciaccatura" => Some(-5.0),
                    "appoggiatura" => Some(-6.0),
                    "trem" | "tremolo" => Some(-7.0),
                    "gliss" | "glissando" => Some(-8.0),
                    "arp" | "arpeggio" => Some(-9.0),
                    "roll" => Some(-10.0),
                    _ => None,
                };
                if let Some(tag) = ornament_tag {
                    parts.articulation = Some(Articulation { vel_mult: tag, dur_mult: 1.0 });
                    s = &s[..dot];
                }
            }
        }
    }

    // Dynamic !dyn
    if let Some(bang) = s.rfind('!') {
        let dyn_str = &s[bang + 1..];
        if let Some(v) = Dynamic::velocity_from_str(dyn_str) {
            parts.inline_velocity = Some(v);
            s = &s[..bang];
        }
    }

    // Duration :dur (must come after stripping articulation/dynamic)
    if let Some(colon) = s.rfind(':') {
        let dur_str = &s[colon + 1..];
        if let Some(d) = Duration::from_suffix(dur_str) {
            parts.duration = Some(d.ticks);
            s = &s[..colon];
        }
    }

    parts.core = s;
    parts
}

/// Expand an ornament tagged via negative vel_mult sentinel.
/// Returns a list of (midi_note, duration_ticks) pairs that replace the single note.
fn expand_ornament(tag: f32, midi_note: u8, total_dur: u32) -> Vec<(u8, u32)> {
    let tag = tag as i32;
    match tag {
        // Trill: alternate main note and +2 semitones
        -1 => {
            let sub = (total_dur / 8).max(30);
            let mut out = Vec::new();
            let mut t = 0;
            let mut upper = false;
            while t + sub <= total_dur {
                let n = if upper { midi_note.saturating_add(2).min(127) } else { midi_note };
                out.push((n, sub));
                t += sub;
                upper = !upper;
            }
            if out.is_empty() { out.push((midi_note, total_dur)); }
            out
        }
        // Mordent (upper): main → upper → main
        -2 => {
            let third = total_dur / 3;
            vec![
                (midi_note, third),
                (midi_note.saturating_add(2).min(127), third),
                (midi_note, total_dur - 2 * third),
            ]
        }
        // Lower mordent: main → lower → main
        -3 => {
            let third = total_dur / 3;
            vec![
                (midi_note, third),
                (midi_note.saturating_sub(1), third),
                (midi_note, total_dur - 2 * third),
            ]
        }
        // Turn: upper → main → lower → main
        -4 => {
            let q = total_dur / 4;
            vec![
                (midi_note.saturating_add(2).min(127), q),
                (midi_note, q),
                (midi_note.saturating_sub(1), q),
                (midi_note, total_dur - 3 * q),
            ]
        }
        // Grace / acciaccatura: quick lower → main
        -5 => {
            let grace_dur = (total_dur / 4).max(30);
            vec![
                (midi_note.saturating_sub(1), grace_dur),
                (midi_note, total_dur - grace_dur),
            ]
        }
        // Appoggiatura: upper → main (50/50 split)
        -6 => {
            let half = total_dur / 2;
            vec![
                (midi_note.saturating_add(2).min(127), half),
                (midi_note, total_dur - half),
            ]
        }
        // Tremolo: rapid repeated hits
        -7 => {
            let sub = (total_dur / 6).max(30);
            let mut out = Vec::new();
            let mut t = 0;
            while t + sub <= total_dur {
                out.push((midi_note, sub));
                t += sub;
            }
            if out.is_empty() { out.push((midi_note, total_dur)); }
            out
        }
        // Glissando: slide up by semitones to +7
        -8 => {
            let steps = 8u32;
            let sub = total_dur / steps;
            (0..steps).map(|i| {
                let n = midi_note.saturating_add(i as u8).min(127);
                let dur = if i == steps - 1 { total_dur - sub * (steps - 1) } else { sub };
                (n, dur)
            }).collect()
        }
        // Arpeggio: triad up
        -9 => {
            let third = total_dur / 3;
            vec![
                (midi_note, third),
                (midi_note.saturating_add(4).min(127), third),
                (midi_note.saturating_add(7).min(127), total_dur - 2 * third),
            ]
        }
        // Roll (drum): rapid repeated hits (same as tremolo)
        -10 => {
            let sub = (total_dur / 6).max(30);
            let mut out = Vec::new();
            let mut t = 0;
            while t + sub <= total_dur {
                out.push((midi_note, sub));
                t += sub;
            }
            if out.is_empty() { out.push((midi_note, total_dur)); }
            out
        }
        _ => vec![(midi_note, total_dur)],
    }
}

/// Parse .delphi notation text into SfEvents.
///
/// Supports the full notation grammar: notes (C4, D#5), chords (Cmaj7, Am/E),
/// inline durations (C4:q, G4:8.), dotted/double-dotted/triplet (:q., :8t),
/// dynamics (!p, !ff, !sfz), articulations (.stac, .acc, .ferm), ornaments
/// (.tr, .mord, .turn, .grace, .trem, .gliss, .arp), rests (., ~, r, _, rest),
/// ties (C4:h~C4:q), polyphony (C4,E4,G4), repeats (C4*4), elongation (C4@2),
/// subdivisions ([C4 E4 G4]), tuplets ((3 C4 E4 G4)), layer groups ({...}),
/// drum names, euclidean patterns (kick(3,8)), breath/caesura markers,
/// structural markers (DC, DS, segno, fine, coda), and bar notation.
pub fn parse_notation_to_events(
    source: &str,
    channel: u8,
    program: u8,
    default_velocity: u8,
) -> Vec<SfEvent> {
    let mut events: Vec<SfEvent> = Vec::new();
    let mut tick: u32 = 0;
    let mut current_duration: u32 = 480; // default quarter note
    let mut velocity = default_velocity;
    let mut tie_accum: u32 = 0; // accumulated tie duration

    // Flatten source into one big token stream, handling grouping constructs.
    // We process line-by-line to skip comments/pragmas, then tokenize.
    let mut all_tokens: Vec<String> = Vec::new();

    for line in source.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with("//") || line.starts_with('#') || line.starts_with('@') {
            continue;
        }
        // Collect characters, emitting tokens and handling grouping
        all_tokens.extend(
            line.split_whitespace()
                .flat_map(|t| {
                    // Split off leading/trailing bar lines but keep them as tokens
                    let mut parts = Vec::new();
                    let t = t.trim_matches(|c: char| {
                        if c == '|' { true } else { false }
                    });
                    if !t.is_empty() {
                        parts.push(t.to_string());
                    }
                    parts
                })
        );
    }

    let len = all_tokens.len();
    let mut i = 0;

    while i < len {
        let raw = &all_tokens[i];

        // ── Bar-line (skip) ──
        if raw == "|" {
            i += 1;
            continue;
        }

        // ── Structural markers (skip) ──
        if matches!(raw.as_str(), "DC" | "D.C." | "DS" | "D.S." | "segno" | "fine" | "coda") {
            i += 1;
            continue;
        }

        // ── Breath / Caesura ──
        if raw == "breath" {
            tick += 120; // 1/4 beat
            i += 1;
            continue;
        }
        if raw == "caesura" {
            tick += 240; // 1/2 beat
            i += 1;
            continue;
        }

        // ── Standalone duration change: :q, :h etc. ──
        if raw.starts_with(':') && raw.len() > 1 {
            if let Some(d) = Duration::from_suffix(&raw[1..]) {
                current_duration = d.ticks;
            }
            i += 1;
            continue;
        }

        // ── Standalone dynamic: !ff etc. ──
        if raw.starts_with('!') && raw.len() > 1 {
            if let Some(v) = Dynamic::velocity_from_str(&raw[1..]) {
                velocity = v;
            }
            i += 1;
            continue;
        }

        // ── Subdivision: [C4 E4 G4] — everything inside shares one beat ──
        if raw.starts_with('[') || raw == "[" {
            let mut sub_tokens: Vec<String> = Vec::new();
            // Collect until ']'
            let first = raw.trim_start_matches('[');
            if !first.is_empty() {
                let first = first.trim_end_matches(']');
                if !first.is_empty() {
                    sub_tokens.push(first.to_string());
                }
            }
            // Check if closing bracket was on this token
            if !raw.contains(']') {
                i += 1;
                while i < len {
                    let t = &all_tokens[i];
                    let is_end = t.contains(']');
                    let clean = t.trim_end_matches(']');
                    if !clean.is_empty() {
                        sub_tokens.push(clean.to_string());
                    }
                    i += 1;
                    if is_end { break; }
                }
            } else {
                i += 1;
            }

            if !sub_tokens.is_empty() {
                let sub_dur = current_duration / sub_tokens.len() as u32;
                for st in &sub_tokens {
                    emit_token(st, &mut events, &mut tick, sub_dur, velocity, channel, program, &mut tie_accum);
                }
            }
            continue;
        }

        // ── Tuplet: (3 C4 E4 G4) — n notes in the space of (n-1) ──
        if raw.starts_with('(') || raw == "(" {
            let first = raw.trim_start_matches('(');
            // First element should be the count
            let mut tuplet_count: Option<u32> = None;
            let mut sub_tokens: Vec<String> = Vec::new();

            if let Ok(n) = first.parse::<u32>() {
                tuplet_count = Some(n);
            } else if !first.is_empty() {
                let first = first.trim_end_matches(')');
                if !first.is_empty() {
                    sub_tokens.push(first.to_string());
                }
            }

            if !raw.contains(')') {
                i += 1;
                while i < len {
                    let t = &all_tokens[i];
                    let is_end = t.contains(')');
                    let clean = t.trim_end_matches(')');
                    if !clean.is_empty() {
                        if tuplet_count.is_none() {
                            if let Ok(n) = clean.parse::<u32>() {
                                tuplet_count = Some(n);
                                i += 1;
                                if is_end { break; }
                                continue;
                            }
                        }
                        sub_tokens.push(clean.to_string());
                    }
                    i += 1;
                    if is_end { break; }
                }
            } else {
                i += 1;
            }

            let n = tuplet_count.unwrap_or(3);
            let total_time = current_duration * (n.saturating_sub(1)).max(1);
            let note_count = sub_tokens.len().max(1) as u32;
            let sub_dur = total_time / note_count;

            for st in &sub_tokens {
                emit_token(st, &mut events, &mut tick, sub_dur, velocity, channel, program, &mut tie_accum);
            }
            continue;
        }

        // ── Layer group: {bd(3,8) sd(2,8) hh(5,8)} — simultaneous patterns ──
        if raw.starts_with('{') || raw == "{" {
            let mut sub_tokens: Vec<String> = Vec::new();
            let first = raw.trim_start_matches('{');
            if !first.is_empty() {
                let first = first.trim_end_matches('}');
                if !first.is_empty() {
                    sub_tokens.push(first.to_string());
                }
            }
            if !raw.contains('}') {
                i += 1;
                while i < len {
                    let t = &all_tokens[i];
                    let is_end = t.contains('}');
                    let clean = t.trim_end_matches('}');
                    if !clean.is_empty() {
                        sub_tokens.push(clean.to_string());
                    }
                    i += 1;
                    if is_end { break; }
                }
            } else {
                i += 1;
            }

            // All sub-patterns start at the same tick
            let base_tick = tick;
            let mut max_advance: u32 = 0;
            for st in &sub_tokens {
                let save_tick = tick;
                tick = base_tick;
                emit_token(st, &mut events, &mut tick, current_duration, velocity, channel, program, &mut tie_accum);
                let advance = tick - base_tick;
                if advance > max_advance {
                    max_advance = advance;
                }
                tick = save_tick;
            }
            tick = base_tick + max_advance;
            continue;
        }

        // ── Regular token ──
        emit_token(raw, &mut events, &mut tick, current_duration, velocity, channel, program, &mut tie_accum);
        i += 1;
    }

    events
}

/// Emit events for a single token, handling inline modifiers, ties, repeats, etc.
fn emit_token(
    raw: &str,
    events: &mut Vec<SfEvent>,
    tick: &mut u32,
    default_dur: u32,
    base_velocity: u8,
    channel: u8,
    program: u8,
    tie_accum: &mut u32,
) {
    // Rest tokens
    if matches!(raw, "." | "~" | "r" | "rest" | "_") {
        *tick += default_dur;
        return;
    }

    let parts = split_token(raw);

    let core = parts.core;
    if core.is_empty() { return; }

    let dur = parts.duration.unwrap_or(default_dur);
    let vel_raw = parts.inline_velocity.unwrap_or(base_velocity);
    let weight = parts.weight;
    let repeat = parts.repeat;

    let effective_dur = (dur as f64 * weight as f64) as u32;

    // Apply articulation to duration and velocity
    let (art_dur, art_vel) = if let Some(ref art) = parts.articulation {
        if art.vel_mult >= 0.0 {
            // Normal articulation
            let d = (effective_dur as f32 * art.dur_mult) as u32;
            let v = ((vel_raw as f32 * art.vel_mult).round() as u8).min(127);
            (d, v)
        } else {
            // Ornament — we'll expand later
            (effective_dur, vel_raw)
        }
    } else {
        (effective_dur, vel_raw)
    };

    let is_ornament = parts.articulation.as_ref().map_or(false, |a| a.vel_mult < 0.0);
    let ornament_tag = parts.articulation.as_ref().map(|a| a.vel_mult).unwrap_or(0.0);

    // Handle tie: if we have accumulated tie duration from a previous note, add it
    let tie_next = parts.tie_next;

    // Euclidean pattern: name(hits,steps[,offset])
    if core.contains('(') && core.ends_with(')') {
        if let Some(paren) = core.find('(') {
            let name = &core[..paren];
            let args = &core[paren + 1..core.len() - 1];
            let arg_parts: Vec<&str> = args.split(',').collect();
            if arg_parts.len() >= 2 {
                if let (Ok(hits), Ok(steps)) = (
                    arg_parts[0].trim().parse::<u32>(),
                    arg_parts[1].trim().parse::<u32>(),
                ) {
                    let offset = if arg_parts.len() >= 3 {
                        arg_parts[2].trim().parse::<usize>().unwrap_or(0)
                    } else {
                        0
                    };
                    let midi = drum_name_to_midi(name);
                    if midi > 0 {
                        let mut pattern = euclidean(hits as usize, steps as usize);
                        // Apply offset rotation
                        if offset > 0 && !pattern.is_empty() {
                            let off = offset % pattern.len();
                            pattern.rotate_left(off);
                        }
                        let step_dur = dur;
                        for rep in 0..repeat {
                            let rep_offset = rep * steps * step_dur;
                            for (j, &hit) in pattern.iter().enumerate() {
                                if hit {
                                    events.push(SfEvent {
                                        tick: *tick + rep_offset + j as u32 * step_dur,
                                        midi_note: midi,
                                        velocity: art_vel,
                                        duration_ticks: step_dur / 2,
                                        channel: 9,
                                        program: 0,
                                    });
                                }
                            }
                        }
                        *tick += repeat * steps * step_dur;
                        return;
                    }
                }
            }
        }
    }

    // Drum name (possibly with inline modifiers already stripped)
    let drum_midi = drum_name_to_midi(core);
    if drum_midi > 0 {
        for _ in 0..repeat {
            if is_ornament {
                let expanded = expand_ornament(ornament_tag, drum_midi, art_dur / 2);
                let mut t = *tick;
                for (n, d) in expanded {
                    events.push(SfEvent { tick: t, midi_note: n, velocity: art_vel, duration_ticks: d, channel: 9, program: 0 });
                    t += d;
                }
            } else {
                events.push(SfEvent {
                    tick: *tick,
                    midi_note: drum_midi,
                    velocity: art_vel,
                    duration_ticks: art_dur / 2,
                    channel: 9,
                    program: 0,
                });
            }
            *tick += effective_dur;
        }
        return;
    }

    // Polyphony: C4,E4,G4 (comma-separated)
    if core.contains(',') {
        let parts_list: Vec<&str> = core.split(',').collect();
        for _ in 0..repeat {
            for part in &parts_list {
                let sub = split_token(part);
                let sub_core = sub.core;
                let sub_dur = sub.duration.unwrap_or(art_dur);
                let sub_vel = sub.inline_velocity.unwrap_or(art_vel);
                if let Ok(note) = sub_core.parse::<Note>() {
                    events.push(SfEvent {
                        tick: *tick,
                        midi_note: note.to_midi(),
                        velocity: sub_vel,
                        duration_ticks: sub_dur,
                        channel,
                        program,
                    });
                } else if let Ok(chord) = sub_core.parse::<delphi_core::Chord>() {
                    for midi in chord.to_midi() {
                        events.push(SfEvent {
                            tick: *tick,
                            midi_note: midi,
                            velocity: sub_vel,
                            duration_ticks: sub_dur,
                            channel,
                            program,
                        });
                    }
                }
            }
            *tick += effective_dur;
        }
        return;
    }

    // Slash chord: Am/E, C/G
    if core.contains('/') {
        if let Some(slash) = core.find('/') {
            let chord_part = &core[..slash];
            let bass_part = &core[slash + 1..];
            // Try to parse chord + bass
            if let Ok(chord) = chord_part.parse::<delphi_core::Chord>() {
                let chord_midis = chord.to_midi();
                let bass_midi = if let Ok(note) = bass_part.parse::<Note>() {
                    Some(note.to_midi())
                } else {
                    // Bass as note name without octave — default octave 3
                    let with_octave = format!("{}3", bass_part);
                    with_octave.parse::<Note>().ok().map(|n| n.to_midi())
                };
                for _ in 0..repeat {
                    if let Some(bm) = bass_midi {
                        events.push(SfEvent {
                            tick: *tick, midi_note: bm, velocity: art_vel,
                            duration_ticks: art_dur, channel, program,
                        });
                    }
                    for midi in &chord_midis {
                        events.push(SfEvent {
                            tick: *tick, midi_note: *midi, velocity: art_vel,
                            duration_ticks: art_dur, channel, program,
                        });
                    }
                    *tick += effective_dur;
                }
                return;
            }
        }
    }

    // Try chord symbol (Cmaj7, Am, G7)
    if let Ok(chord) = core.parse::<delphi_core::Chord>() {
        for _ in 0..repeat {
            if is_ornament {
                // Apply ornament to root note of chord
                let midis = chord.to_midi();
                if let Some(&root) = midis.first() {
                    let expanded = expand_ornament(ornament_tag, root, art_dur);
                    let mut t = *tick;
                    for (n, d) in expanded {
                        // Play root ornament + other chord tones sustain
                        events.push(SfEvent { tick: t, midi_note: n, velocity: art_vel, duration_ticks: d, channel, program });
                        t += d;
                    }
                    // Sustain non-root chord tones for full duration
                    for &midi in midis.iter().skip(1) {
                        events.push(SfEvent {
                            tick: *tick, midi_note: midi, velocity: art_vel,
                            duration_ticks: art_dur, channel, program,
                        });
                    }
                }
            } else {
                for midi in chord.to_midi() {
                    events.push(SfEvent {
                        tick: *tick, midi_note: midi, velocity: art_vel,
                        duration_ticks: art_dur, channel, program,
                    });
                }
            }
            *tick += effective_dur;
        }
        return;
    }

    // Try note (C4, D#5, Bb3)
    if let Ok(note) = core.parse::<Note>() {
        let midi = note.to_midi();

        // Handle tie accumulation
        if *tie_accum > 0 {
            // Extend the last event with this duration
            if let Some(last) = events.last_mut() {
                if last.midi_note == midi {
                    last.duration_ticks += art_dur;
                    *tie_accum = 0;
                    if tie_next {
                        *tie_accum = art_dur;
                    }
                    *tick += effective_dur;
                    return;
                }
            }
            *tie_accum = 0;
        }

        for rep in 0..repeat {
            if is_ornament {
                let expanded = expand_ornament(ornament_tag, midi, art_dur);
                let mut t = *tick;
                for (n, d) in expanded {
                    events.push(SfEvent { tick: t, midi_note: n, velocity: art_vel, duration_ticks: d, channel, program });
                    t += d;
                }
            } else {
                events.push(SfEvent {
                    tick: *tick, midi_note: midi, velocity: art_vel,
                    duration_ticks: art_dur, channel, program,
                });
            }
            // Only advance tick on non-last repetition or if not tie_next
            if rep < repeat - 1 || !tie_next {
                *tick += effective_dur;
            } else {
                // Last rep with tie: advance tick but mark tie accumulator
                *tick += effective_dur;
                *tie_accum = art_dur;
            }
        }
        return;
    }

    // Unknown token — skip
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

/// Parse notation and return events PLUS a list of warning strings for unknown tokens.
pub fn parse_notation_with_diagnostics(
    source: &str,
    channel: u8,
    program: u8,
    default_velocity: u8,
) -> (Vec<SfEvent>, Vec<String>) {
    let events = parse_notation_to_events(source, channel, program, default_velocity);
    let mut warnings = Vec::new();

    for (line_no, line) in source.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with("//") || line.starts_with('#') || line.starts_with('@') {
            continue;
        }
        for raw in line.split_whitespace() {
            // Strip grouping chars
            let token = raw.trim_matches(|c: char| matches!(c, '|' | '[' | ']' | '(' | ')' | '{' | '}'));
            if token.is_empty() { continue; }

            // Skip known structural markers
            if matches!(token, "DC" | "D.C." | "DS" | "D.S." | "segno" | "fine" | "coda" | "breath" | "caesura") {
                continue;
            }

            // Standalone duration / dynamic
            if token.starts_with(':') || token.starts_with('!') {
                continue;
            }

            // Rest
            if matches!(token, "." | "~" | "r" | "rest" | "_") {
                continue;
            }

            // Strip modifiers to get core
            let parts = split_token(token);
            let core = parts.core;
            if core.is_empty() { continue; }

            // Check if it's a recognizable token
            if core.parse::<u32>().is_ok() { continue; } // tuplet count
            if drum_name_to_midi(core) > 0 { continue; }
            if core.contains('(') && core.ends_with(')') { continue; } // euclidean
            if core.contains(',') { continue; } // polyphony
            if core.contains('/') {
                // slash chord
                if let Some(slash) = core.find('/') {
                    if core[..slash].parse::<delphi_core::Chord>().is_ok() { continue; }
                }
            }
            if core.parse::<delphi_core::Chord>().is_ok() { continue; }
            if core.parse::<Note>().is_ok() { continue; }

            // Unknown
            warnings.push(format!("line {}: unknown token '{}'", line_no + 1, core));
        }
    }

    (events, warnings)
}

/// Map drum names to General MIDI percussion note numbers.
fn drum_name_to_midi(name: &str) -> u8 {
    match name.to_lowercase().as_str() {
        "kick" | "bd" => 36,
        "snare" | "sd" => 38,
        "rimshot" | "rim" => 37,
        "clap" | "cp" => 39,
        "hihat" | "hh" | "closehat" | "ch" => 42,
        "openhat" | "oh" => 46,
        "pedal" => 44,
        "ride" | "rd" => 51,
        "crash" | "cr" => 49,
        "tom1" => 50,
        "tom2" => 47,
        "tom3" => 45,
        "cowbell" | "cb" => 56,
        "tambourine" | "tamb" => 54,
        "cabasa" => 69,
        "maracas" => 70,
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
