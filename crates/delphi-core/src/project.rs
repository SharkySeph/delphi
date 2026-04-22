use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::duration::{Tempo, TempoMap};
use crate::event::{MetaEvent, NoteEvent};
use crate::gm::gm_program_from_name;
use crate::notation::{apply_humanize, apply_swing, parse_notation_to_events_ts, parse_notation_full};

/// A single cell in the studio notebook.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cell {
    pub cell_type: String,
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
    pub pan: f32,
    pub muted: bool,
    pub solo: bool,
    #[serde(default)]
    pub reverb: f32,
    #[serde(default)]
    pub delay: f32,
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
            reverb: 0.0,
            delay: 0.0,
        }
    }
}

/// A global timeline event applied to all tracks at a specific bar position.
/// These are stored on the project and resolved to tick-based MetaEvents at export time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineEntry {
    /// Bar number (1-indexed) where this event occurs.
    pub bar: u32,
    /// The kind of timeline change.
    pub event: TimelineEventKind,
}

/// The kind of change in a timeline entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TimelineEventKind {
    Tempo(f64),
    TimeSig(u8, u8),
    Key(String),
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
    /// Master output gain (0.0–1.5). Persisted so reopening, playback, and
    /// export all use the same level. Default 0.8.
    #[serde(default = "default_master_gain")]
    pub master_gain: f32,
    /// Global timeline events (tempo, time_sig, key changes by bar number).
    #[serde(default)]
    pub timeline: Vec<TimelineEntry>,
}

fn default_master_gain() -> f32 {
    0.8
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
            master_gain: default_master_gain(),
            timeline: Vec::new(),
        }
    }
}

/// The project data: cells, tracks, and settings.
/// This is the shared, non-GUI project model used by both CLI and Studio.
pub struct Project {
    pub cells: Vec<Cell>,
    pub tracks: Vec<TrackState>,
    pub settings: ProjectSettings,
}

impl Project {
    pub fn new() -> Self {
        Self {
            cells: vec![Cell::new_notation()],
            tracks: vec![TrackState::new("Track 1", "piano", 0, 0)],
            settings: ProjectSettings::default(),
        }
    }

    pub fn tempo(&self) -> Tempo {
        Tempo { bpm: self.settings.bpm }
    }

    /// Build a TempoMap from the project's initial tempo + all meta events.
    pub fn tempo_map(&self) -> TempoMap {
        let tempo = self.tempo();
        let meta = self.collect_meta_events();
        TempoMap::from_meta_events(&tempo, &meta)
    }

    /// Save project state to a .dstudio JSON file.
    pub fn save(&self, path: &PathBuf) {
        #[derive(Serialize)]
        struct ProjectFile<'a> {
            settings: &'a ProjectSettings,
            cells: &'a [Cell],
            tracks: &'a [TrackState],
        }

        let project = ProjectFile {
            settings: &self.settings,
            cells: &self.cells,
            tracks: &self.tracks,
        };

        if let Ok(json) = serde_json::to_string_pretty(&project) {
            let _ = std::fs::write(path, json);
        }
    }

    /// Load project state from a .dstudio JSON file.
    pub fn load(&mut self, path: &PathBuf) -> Result<(), String> {
        #[derive(Deserialize)]
        struct NativeProject {
            settings: ProjectSettings,
            cells: Vec<Cell>,
            #[serde(default)]
            tracks: Vec<TrackState>,
        }

        #[derive(Deserialize)]
        #[allow(dead_code)]
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

        let contents = std::fs::read_to_string(path)
            .map_err(|e| format!("Could not read file: {}", e))?;

        // Try native format first
        if let Ok(project) = serde_json::from_str::<NativeProject>(&contents) {
            self.settings = project.settings;
            self.cells = project.cells;
            self.tracks = if project.tracks.is_empty() {
                vec![TrackState::new("Track 1", "piano", 0, 0)]
            } else {
                project.tracks
            };
            self.auto_assign_channels();
            return Ok(());
        }

        // Try Python format
        if let Ok(py) = serde_json::from_str::<PyProject>(&contents) {
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

            let mut next_auto_channel: u8 = 0;
            let mut label_channel: Vec<(String, u8)> = Vec::new();

            self.cells = py.cells.iter().map(|pc| {
                let ct = pc.cell_type.as_deref().unwrap_or("notation");
                let mut cell = match ct {
                    "markdown" => Cell::new_markdown(),
                    "code" => Cell::new_code(),
                    _ => Cell::new_notation(),
                };
                cell.source = pc.source.clone();
                let mut has_explicit_channel = false;
                if let Some(ref meta) = pc.meta {
                    if let Some(ref label) = meta.label {
                        cell.label = label.clone();
                    }
                    if let Some(ref prog) = meta.program {
                        cell.instrument = prog.clone();
                    }
                    if let Some(ch) = meta.channel {
                        cell.channel = ch;
                        has_explicit_channel = true;
                    }
                    if let Some(vel) = meta.velocity {
                        cell.velocity = vel;
                    }
                }
                // Extract pragmas from source comments
                for line in cell.source.lines() {
                    let line = line.trim();
                    if let Some(rest) = line.strip_prefix("# @instrument ") {
                        cell.instrument = rest.trim().to_string();
                    } else if let Some(rest) = line.strip_prefix("# @channel ") {
                        if let Ok(ch) = rest.trim().parse::<u8>() {
                            cell.channel = ch;
                            has_explicit_channel = true;
                        }
                    } else if let Some(rest) = line.strip_prefix("# @velocity ") {
                        if let Ok(v) = rest.trim().parse::<u8>() {
                            cell.velocity = v;
                        }
                    } else if let Some(rest) = line.strip_prefix("# @track ") {
                        cell.label = rest.trim().to_string();
                    }
                }
                if ct != "markdown" && ct != "code" && !has_explicit_channel {
                    let key = if cell.label.is_empty() {
                        cell.instrument.to_lowercase()
                    } else {
                        cell.label.clone()
                    };
                    if let Some((_name, ch)) = label_channel.iter().find(|(n, _)| *n == key) {
                        cell.channel = *ch;
                    } else {
                        if next_auto_channel == 9 {
                            next_auto_channel = 10;
                        }
                        cell.channel = next_auto_channel;
                        label_channel.push((key, next_auto_channel));
                        next_auto_channel = (next_auto_channel + 1).min(15);
                    }
                }
                cell
            }).collect();

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

            // Extract directive overrides from code cells
            for cell in &self.cells {
                if cell.cell_type == "code" {
                    for line in cell.source.lines() {
                        let line = line.trim();
                        if let Some(rest) = line.strip_prefix("swing(").and_then(|r| r.strip_suffix(')')) {
                            if let Ok(v) = rest.trim().parse::<f32>() {
                                self.settings.swing = v;
                            }
                        }
                        if let Some(rest) = line.strip_prefix("humanize(").and_then(|r| r.strip_suffix(')')) {
                            if let Ok(v) = rest.trim().parse::<f32>() {
                                self.settings.humanize = v;
                            }
                        }
                        if let Some(rest) = line.strip_prefix("tempo(").and_then(|r| r.strip_suffix(')')) {
                            if let Ok(v) = rest.trim().parse::<f64>() {
                                self.settings.bpm = v;
                            }
                        }
                        if let Some(rest) = line.strip_prefix("key(").and_then(|r| r.strip_suffix(')')) {
                            let k = rest.trim().trim_matches('"').trim_matches('\'');
                            if !k.is_empty() {
                                self.settings.key_name = k.to_string();
                            }
                        }
                        if let Some(rest) = line.strip_prefix("time_sig(").and_then(|r| r.strip_suffix(')')) {
                            let parts: Vec<&str> = rest.split(',').collect();
                            if parts.len() == 2 {
                                if let (Ok(n), Ok(d)) = (parts[0].trim().parse::<u8>(), parts[1].trim().parse::<u8>()) {
                                    self.settings.time_sig_num = n;
                                    self.settings.time_sig_den = d;
                                }
                            }
                        }
                    }
                }
            }

            self.auto_assign_channels();
            return Ok(());
        }

        // Try loading as plain .delphi notation
        if !contents.trim().is_empty() && !contents.trim_start().starts_with('{') {
            self.settings = ProjectSettings::default();
            let mut cell = Cell::new_notation();
            cell.source = contents;
            self.cells = vec![cell];
            self.tracks = vec![TrackState::new("Track 1", "piano", 0, 0)];
            self.auto_assign_channels();
            return Ok(());
        }

        Err("Unrecognized file format".into())
    }

    /// Auto-assign unique MIDI channels to cells/tracks.
    pub fn auto_assign_channels(&mut self) {
        let mut label_channel: Vec<(String, u8)> = Vec::new();
        let mut next_auto: u8 = 0;

        for cell in &mut self.cells {
            if cell.cell_type == "markdown" || cell.cell_type == "code" {
                continue;
            }
            if cell.channel == 9 {
                continue;
            }
            let key = if cell.label.is_empty() {
                cell.instrument.to_lowercase()
            } else {
                cell.label.clone()
            };
            if let Some((_, ch)) = label_channel.iter().find(|(k, _)| *k == key) {
                cell.channel = *ch;
            } else {
                if next_auto == 9 { next_auto = 10; }
                cell.channel = next_auto;
                label_channel.push((key, next_auto));
                next_auto = (next_auto + 1).min(15);
            }
        }

        for track in &mut self.tracks {
            if track.channel == 9 {
                continue;
            }
            let key = track.name.clone();
            if let Some((_, ch)) = label_channel.iter().find(|(k, _)| *k == key) {
                track.channel = *ch;
            }
        }
    }

    pub fn add_cell(&mut self) {
        self.cells.push(Cell::new_code());
        self.auto_assign_channels();
    }

    pub fn add_notation_cell(&mut self) {
        self.cells.push(Cell::new_notation());
        self.auto_assign_channels();
    }

    pub fn add_markdown_cell(&mut self) {
        self.cells.push(Cell::new_markdown());
    }

    pub fn move_cell_up(&mut self, idx: usize) -> bool {
        if idx > 0 && idx < self.cells.len() {
            self.cells.swap(idx, idx - 1);
            true
        } else {
            false
        }
    }

    pub fn move_cell_down(&mut self, idx: usize) -> bool {
        if idx + 1 < self.cells.len() {
            self.cells.swap(idx, idx + 1);
            true
        } else {
            false
        }
    }

    pub fn channel_pan_map(&self) -> [f32; 16] {
        let mut pan = [0.5_f32; 16];
        for track in &self.tracks {
            let ch = track.channel as usize;
            if ch < 16 { pan[ch] = track.pan; }
        }
        pan
    }

    pub fn channel_reverb_map(&self) -> [f32; 16] {
        let mut reverb = [0.0_f32; 16];
        for track in &self.tracks {
            let ch = track.channel as usize;
            if ch < 16 { reverb[ch] = track.reverb; }
        }
        reverb
    }

    pub fn channel_delay_map(&self) -> [f32; 16] {
        let mut delay = [0.0_f32; 16];
        for track in &self.tracks {
            let ch = track.channel as usize;
            if ch < 16 { delay[ch] = track.delay; }
        }
        delay
    }

    pub fn channel_volume_map(&self) -> [f32; 16] {
        let mut vol = [1.0_f32; 16];
        for track in &self.tracks {
            let ch = track.channel as usize;
            if ch < 16 { vol[ch] = track.gain.clamp(0.0, 1.5); }
        }
        vol
    }

    /// Collect NoteEvents from all cells (or a single cell).
    #[allow(dead_code)]
    pub fn collect_events(&self, cell_index: Option<usize>) -> Vec<NoteEvent> {
        self.collect_events_mixed(cell_index, 1.0)
    }

    /// Collect events with mixer applied (mute/solo/gain).
    pub fn collect_events_mixed(&self, cell_index: Option<usize>, master_gain: f32) -> Vec<NoteEvent> {
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

        let any_solo = self.tracks.iter().any(|t| t.solo);

        // Parse directive overrides from code cells
        let mut swing = self.settings.swing;
        let mut humanize = self.settings.humanize;
        for cell in &self.cells {
            if cell.cell_type == "code" {
                for line in cell.source.lines() {
                    let line = line.trim();
                    if let Some(rest) = line.strip_prefix("swing(").and_then(|r| r.strip_suffix(')')) {
                        if let Ok(v) = rest.trim().parse::<f32>() {
                            swing = v;
                        }
                    }
                    if let Some(rest) = line.strip_prefix("humanize(").and_then(|r| r.strip_suffix(')')) {
                        if let Ok(v) = rest.trim().parse::<f32>() {
                            humanize = v;
                        }
                    }
                }
            }
        }

        for (_cell_idx, cell) in cells {
            if cell.cell_type == "markdown" || cell.cell_type == "code" {
                continue;
            }
            let source = &cell.source;
            if source.trim().is_empty() {
                continue;
            }
            let mut program = gm_program_from_name(&cell.instrument);
            let velocity = cell.velocity;
            let channel = if cell.channel == 9 { 9 } else { cell.channel };

            let track = self.tracks.iter().find(|t| {
                if !cell.label.is_empty() {
                    t.name == cell.label
                } else {
                    t.instrument.eq_ignore_ascii_case(&cell.instrument)
                }
            });

            if let Some(trk) = track {
                if trk.muted { continue; }
                if any_solo && !trk.solo { continue; }
                // Track voice selection is authoritative in Studio.
                program = trk.program;
            }

            let key_str = &self.settings.key_name;
            let key_opt = if key_str.is_empty() { None } else { Some(key_str.as_str()) };
            let mut events = parse_notation_to_events_ts(
                source, channel, program, velocity,
                self.settings.time_sig_num, self.settings.time_sig_den,
                key_opt,
            );
            if (master_gain - 1.0).abs() > 0.001 {
                for ev in &mut events {
                    ev.velocity = ((ev.velocity as f32 * master_gain).round() as u8).min(127);
                }
            }
            apply_swing(&mut events, swing);
            apply_humanize(&mut events, humanize);
            all_events.extend(events);
        }

        all_events.retain(|ev| {
            if let Some(trk) = self.tracks.iter().find(|t| t.channel == ev.channel) {
                if trk.muted { return false; }
                if any_solo && !trk.solo { return false; }
            }
            true
        });

        all_events
    }

    /// Collect meta events from all cells and the project timeline.
    ///
    /// Returns a sorted list of MetaEvents (tempo, time_sig, key changes) with
    /// tick positions. This merges:
    /// 1. Cell-level pragmas (`# @tempo`, `# @time_sig`, `# @key` within notation)
    /// 2. Project-level timeline entries (bar-based, converted to ticks)
    pub fn collect_meta_events(&self) -> Vec<MetaEvent> {
        let mut all_meta: Vec<MetaEvent> = Vec::new();

        // Collect from cell-level pragmas
        for cell in &self.cells {
            if cell.cell_type != "notation" || cell.source.trim().is_empty() {
                continue;
            }
            let program = gm_program_from_name(&cell.instrument);
            let key_str = &self.settings.key_name;
            let key_opt = if key_str.is_empty() { None } else { Some(key_str.as_str()) };
            let (_events, meta) = parse_notation_full(
                &cell.source, cell.channel, program, cell.velocity,
                self.settings.time_sig_num, self.settings.time_sig_den,
                key_opt,
            );
            all_meta.extend(meta);
        }

        // Convert project-level timeline entries (bar-based) to tick-based MetaEvents.
        // Walk through the timeline sorted by bar, tracking time signature changes
        // to correctly compute tick offsets.
        let mut sorted_timeline = self.settings.timeline.clone();
        sorted_timeline.sort_by_key(|e| e.bar);

        let mut current_num = self.settings.time_sig_num;
        let mut current_den = self.settings.time_sig_den;
        let mut current_bar: u32 = 1;
        let mut current_tick: u32 = 0;

        for entry in &sorted_timeline {
            // Advance ticks from current_bar to entry.bar using current time signature
            if entry.bar > current_bar {
                let bars_to_advance = entry.bar - current_bar;
                let beat_ticks = 480u32 * 4 / current_den.max(1) as u32;
                let measure_ticks = beat_ticks * current_num as u32;
                current_tick += bars_to_advance * measure_ticks;
                current_bar = entry.bar;
            }

            match &entry.event {
                TimelineEventKind::Tempo(bpm) => {
                    all_meta.push(MetaEvent::TempoChange { tick: current_tick, bpm: *bpm });
                }
                TimelineEventKind::TimeSig(n, d) => {
                    all_meta.push(MetaEvent::TimeSigChange { tick: current_tick, numerator: *n, denominator: *d });
                    current_num = *n;
                    current_den = *d;
                }
                TimelineEventKind::Key(key_name) => {
                    all_meta.push(MetaEvent::KeyChange { tick: current_tick, key_name: key_name.clone() });
                }
            }
        }

        // Deduplicate: prefer the latest event at each tick for each type
        all_meta.sort_by_key(|e| match e {
            MetaEvent::TempoChange { tick, .. } => (*tick, 0),
            MetaEvent::TimeSigChange { tick, .. } => (*tick, 1),
            MetaEvent::KeyChange { tick, .. } => (*tick, 2),
        });
        all_meta.dedup_by(|a, b| {
            let tick_a = match a { MetaEvent::TempoChange { tick, .. } | MetaEvent::TimeSigChange { tick, .. } | MetaEvent::KeyChange { tick, .. } => *tick };
            let tick_b = match b { MetaEvent::TempoChange { tick, .. } | MetaEvent::TimeSigChange { tick, .. } | MetaEvent::KeyChange { tick, .. } => *tick };
            let type_a = match a { MetaEvent::TempoChange { .. } => 0, MetaEvent::TimeSigChange { .. } => 1, MetaEvent::KeyChange { .. } => 2 };
            let type_b = match b { MetaEvent::TempoChange { .. } => 0, MetaEvent::TimeSigChange { .. } => 1, MetaEvent::KeyChange { .. } => 2 };
            tick_a == tick_b && type_a == type_b
        });

        all_meta
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── ProjectSettings defaults ──────────────────────────────────────────────

    #[test]
    fn default_master_gain_is_0_8() {
        let settings = ProjectSettings::default();
        assert!(
            (settings.master_gain - 0.8).abs() < f32::EPSILON,
            "default master_gain should be 0.8, got {}",
            settings.master_gain
        );
    }

    #[test]
    fn master_gain_round_trips_through_json() {
        let mut settings = ProjectSettings::default();
        settings.master_gain = 0.65;
        let json = serde_json::to_string(&settings).expect("serialize");
        let loaded: ProjectSettings = serde_json::from_str(&json).expect("deserialize");
        assert!(
            (loaded.master_gain - 0.65).abs() < 0.001,
            "master_gain should survive JSON round-trip"
        );
    }

    #[test]
    fn old_json_without_master_gain_defaults_to_0_8() {
        // Simulate a .dstudio file saved before master_gain was added.
        let json = r#"{
            "title": "Legacy",
            "bpm": 120.0,
            "key_name": "C major",
            "time_sig_num": 4,
            "time_sig_den": 4,
            "swing": 0.0,
            "humanize": 0.0,
            "soundfont_path": null,
            "timeline": []
        }"#;
        let settings: ProjectSettings = serde_json::from_str(json).expect("deserialize");
        assert!(
            (settings.master_gain - 0.8).abs() < f32::EPSILON,
            "missing master_gain field should default to 0.8"
        );
    }

    // ── Mixer state — mute/solo affect collect_events_mixed ───────────────────

    fn project_with_two_tracks() -> Project {
        let mut p = Project::new();
        p.settings.bpm = 120.0;
        p.settings.time_sig_num = 4;
        p.settings.time_sig_den = 4;

        // Two notation cells on separate channels
        p.cells.clear();
        let mut c0 = Cell::new_notation();
        c0.source = "C4:q".into();
        c0.instrument = "piano".into();
        c0.label = "Piano".into();
        c0.channel = 0;
        c0.velocity = 80;

        let mut c1 = Cell::new_notation();
        c1.source = "G3:q".into();
        c1.instrument = "bass".into();
        c1.label = "Bass".into();
        c1.channel = 1;
        c1.velocity = 80;

        p.cells.push(c0);
        p.cells.push(c1);

        p.tracks.clear();
        p.tracks.push(TrackState::new("Piano", "piano", 0, 0));
        p.tracks.push(TrackState::new("Bass", "bass", 32, 1));
        p
    }

    #[test]
    fn muted_track_produces_no_events() {
        let mut p = project_with_two_tracks();
        p.tracks[0].muted = true;

        let events = p.collect_events_mixed(None, 1.0);
        // All events should be from channel 1 (bass) only
        assert!(
            events.iter().all(|e| e.channel == 1),
            "muted piano track should produce no events"
        );
    }

    #[test]
    fn solo_track_silences_others() {
        let mut p = project_with_two_tracks();
        p.tracks[0].solo = true; // solo Piano

        let events = p.collect_events_mixed(None, 1.0);
        assert!(
            events.iter().all(|e| e.channel == 0),
            "soloed piano track should be the only source of events"
        );
        assert!(
            !events.is_empty(),
            "soloed track should still produce events"
        );
    }

    #[test]
    fn master_gain_scales_velocity() {
        let p = project_with_two_tracks();
        let events_full = p.collect_events_mixed(None, 1.0);
        let events_half = p.collect_events_mixed(None, 0.5);

        assert!(!events_full.is_empty());
        for (full, half) in events_full.iter().zip(events_half.iter()) {
            let expected = ((full.velocity as f32 * 0.5).round() as u8).min(127);
            assert_eq!(
                half.velocity, expected,
                "master_gain=0.5 should halve velocity"
            );
        }
    }

    #[test]
    fn track_program_overrides_cell_instrument_program() {
        let mut p = Project::new();
        p.cells.clear();
        p.tracks.clear();

        let mut cell = Cell::new_notation();
        cell.label = "Lead".into();
        cell.instrument = "piano".into();
        cell.channel = 0;
        cell.source = "C4:q".into();
        p.cells.push(cell);

        let mut track = TrackState::new("Lead", "piano", 0, 0);
        track.program = 40; // violin
        p.tracks.push(track);

        let events = p.collect_events_mixed(None, 1.0);
        assert!(!events.is_empty());
        assert!(events.iter().all(|ev| ev.program == 40));
    }

    // ── Channel maps ──────────────────────────────────────────────────────────

    #[test]
    fn channel_pan_map_reflects_track_state() {
        let mut p = project_with_two_tracks();
        p.tracks[0].pan = 0.25;
        p.tracks[1].pan = 0.75;

        let pan = p.channel_pan_map();
        assert!((pan[0] - 0.25).abs() < f32::EPSILON);
        assert!((pan[1] - 0.75).abs() < f32::EPSILON);
    }

    #[test]
    fn channel_volume_map_reflects_track_gain() {
        let mut p = project_with_two_tracks();
        p.tracks[0].gain = 1.2;

        let vol = p.channel_volume_map();
        assert!((vol[0] - 1.2).abs() < f32::EPSILON);
    }
}

