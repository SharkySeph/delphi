use serde::{Deserialize, Serialize};

use crate::duration::{Tempo, TempoMap};

/// A mid-song meta event (tempo change, time signature change, key change).
///
/// These are emitted by the notation parser when `# @tempo`, `# @time_sig`,
/// or `# @key` pragmas are encountered, and also produced from project-level
/// timeline entries. They are used by the MIDI exporter to write meta events
/// at the correct tick positions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MetaEvent {
    TempoChange { tick: u32, bpm: f64 },
    TimeSigChange { tick: u32, numerator: u8, denominator: u8 },
    KeyChange { tick: u32, key_name: String },
}

/// A scheduled note event for playback, MIDI export, or WAV rendering.
///
/// This is the universal event type used throughout Delphi. Each event
/// represents a single note (or percussion hit) at a specific tick position.
#[derive(Debug, Clone)]
pub struct NoteEvent {
    pub tick: u32,
    pub midi_note: u8,
    pub velocity: u8,
    pub duration_ticks: u32,
    pub channel: u8,
    pub program: u8,
}

impl NoteEvent {
    pub fn start_seconds(&self, tempo: &TempoMap) -> f64 {
        tempo.tick_to_seconds(self.tick)
    }

    pub fn duration_seconds(&self, tempo: &TempoMap) -> f64 {
        tempo.tick_range_to_seconds(self.tick, self.duration_ticks)
    }

    /// Convenience: use a single constant Tempo (no mid-song changes).
    pub fn start_seconds_const(&self, tempo: &Tempo) -> f64 {
        let beats = self.tick as f64 / 480.0;
        beats * 60.0 / tempo.bpm
    }

    /// Convenience: use a single constant Tempo (no mid-song changes).
    pub fn duration_seconds_const(&self, tempo: &Tempo) -> f64 {
        let beats = self.duration_ticks as f64 / 480.0;
        beats * 60.0 / tempo.bpm
    }
}
