use delphi_core::duration::{Duration, Tempo};
use delphi_core::dynamics::Velocity;

/// A scheduled audio event: play a note at a specific tick with a duration.
#[derive(Debug, Clone)]
pub struct AudioEvent {
    pub tick: u32,
    pub midi_note: u8,
    pub velocity: Velocity,
    pub duration: Duration,
}

impl AudioEvent {
    pub fn new(tick: u32, midi_note: u8, velocity: Velocity, duration: Duration) -> Self {
        Self {
            tick,
            midi_note,
            velocity,
            duration,
        }
    }

    /// Get the start time in seconds for a given tempo.
    pub fn start_seconds(&self, tempo: &Tempo) -> f64 {
        let beats = self.tick as f64 / Duration::TICKS_PER_QUARTER as f64;
        beats * 60.0 / tempo.bpm
    }

    /// Get the duration in seconds for a given tempo.
    pub fn duration_seconds(&self, tempo: &Tempo) -> f64 {
        self.duration.to_seconds(tempo)
    }
}

/// Collects and sorts audio events, then provides them in time order.
pub struct Scheduler {
    events: Vec<AudioEvent>,
}

impl Scheduler {
    pub fn new() -> Self {
        Self { events: Vec::new() }
    }

    pub fn add_event(&mut self, event: AudioEvent) {
        self.events.push(event);
    }

    pub fn add_events(&mut self, events: impl IntoIterator<Item = AudioEvent>) {
        self.events.extend(events);
    }

    /// Get all events sorted by tick.
    pub fn events(&self) -> &[AudioEvent] {
        &self.events
    }

    /// Sort events by tick position.
    pub fn sort(&mut self) {
        self.events.sort_by_key(|e| e.tick);
    }

    /// Total duration in ticks.
    pub fn total_ticks(&self) -> u32 {
        self.events
            .iter()
            .map(|e| e.tick + e.duration.ticks)
            .max()
            .unwrap_or(0)
    }

    /// Total duration in seconds at a given tempo.
    pub fn total_seconds(&self, tempo: &Tempo) -> f64 {
        let ticks = self.total_ticks();
        let beats = ticks as f64 / Duration::TICKS_PER_QUARTER as f64;
        beats * 60.0 / tempo.bpm
    }

    pub fn clear(&mut self) {
        self.events.clear();
    }

    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }
}
