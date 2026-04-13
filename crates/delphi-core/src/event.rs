use crate::duration::Tempo;

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
    pub fn start_seconds(&self, tempo: &Tempo) -> f64 {
        let beats = self.tick as f64 / 480.0;
        beats * 60.0 / tempo.bpm
    }

    pub fn duration_seconds(&self, tempo: &Tempo) -> f64 {
        let beats = self.duration_ticks as f64 / 480.0;
        beats * 60.0 / tempo.bpm
    }
}
