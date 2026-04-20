use delphi_core::duration::{Duration, Tempo, TimeSignature};
use std::io::Write;

/// A single MIDI note event at a specific tick position.
#[derive(Debug, Clone)]
pub struct MidiNoteEvent {
    pub tick: u32,
    pub channel: u8,
    pub note: u8,
    pub velocity: u8,
    pub duration_ticks: u32,
}

/// A track of MIDI events with a name and instrument.
#[derive(Debug, Clone)]
pub struct MidiTrack {
    pub name: String,
    pub channel: u8,
    pub program: u8, // General MIDI program number (0-127)
    pub events: Vec<MidiNoteEvent>,
}

impl MidiTrack {
    pub fn new(name: &str, channel: u8, program: u8) -> Self {
        Self {
            name: name.to_string(),
            channel,
            program,
            events: Vec::new(),
        }
    }

    pub fn add_note(&mut self, tick: u32, note: u8, velocity: u8, duration_ticks: u32) {
        self.events.push(MidiNoteEvent {
            tick,
            channel: self.channel,
            note,
            velocity,
            duration_ticks,
        });
    }
}

/// Exports musical events to a Standard MIDI File (SMF).
pub struct MidiExporter {
    pub tempo: Tempo,
    pub time_signature: TimeSignature,
    pub tracks: Vec<MidiTrack>,
    /// Mid-song tempo changes (tick, new tempo).
    pub tempo_changes: Vec<(u32, Tempo)>,
    /// Mid-song time signature changes (tick, new time_sig).
    pub time_sig_changes: Vec<(u32, TimeSignature)>,
}

impl MidiExporter {
    pub fn new() -> Self {
        Self {
            tempo: Tempo::default(),
            time_signature: TimeSignature::default(),
            tracks: Vec::new(),
            tempo_changes: Vec::new(),
            time_sig_changes: Vec::new(),
        }
    }

    pub fn set_tempo(&mut self, tempo: Tempo) {
        self.tempo = tempo;
    }

    pub fn set_time_signature(&mut self, ts: TimeSignature) {
        self.time_signature = ts;
    }

    pub fn add_track(&mut self, track: MidiTrack) {
        self.tracks.push(track);
    }

    /// Write a complete MIDI file to the given writer.
    /// Uses SMF Format 1 (multi-track) with the standard 480 PPQ resolution.
    pub fn write<W: Write>(&self, writer: &mut W) -> Result<(), MidiExportError> {
        let ppq = Duration::TICKS_PER_QUARTER as u16;
        let num_tracks = self.tracks.len() as u16 + 1; // +1 for tempo track

        // --- Header chunk ---
        writer.write_all(b"MThd")?;
        writer.write_all(&6u32.to_be_bytes())?; // chunk length
        writer.write_all(&1u16.to_be_bytes())?; // format 1
        writer.write_all(&num_tracks.to_be_bytes())?;
        writer.write_all(&ppq.to_be_bytes())?;

        // --- Tempo track (track 0) ---
        let mut tempo_data = Vec::new();

        // Build a sorted list of all tempo track meta events (tick, raw bytes)
        let mut meta_events: Vec<(u32, Vec<u8>)> = Vec::new();

        // Initial tempo event at tick 0
        {
            let uspqn = self.tempo.to_midi_tempo();
            let mut bytes = Vec::new();
            bytes.push(0xFF);
            bytes.push(0x51);
            bytes.push(0x03);
            bytes.push((uspqn >> 16) as u8);
            bytes.push((uspqn >> 8) as u8);
            bytes.push(uspqn as u8);
            meta_events.push((0, bytes));
        }

        // Initial time signature event at tick 0
        {
            let mut bytes = Vec::new();
            bytes.push(0xFF);
            bytes.push(0x58);
            bytes.push(0x04);
            bytes.push(self.time_signature.numerator);
            let denom_pow = (self.time_signature.denominator as f32).log2() as u8;
            bytes.push(denom_pow);
            bytes.push(24); // MIDI clocks per metronome tick
            bytes.push(8);  // 32nd notes per MIDI quarter note
            meta_events.push((0, bytes));
        }

        // Mid-song tempo changes
        for (tick, tempo) in &self.tempo_changes {
            if *tick == 0 { continue; } // already handled above
            let uspqn = tempo.to_midi_tempo();
            let mut bytes = Vec::new();
            bytes.push(0xFF);
            bytes.push(0x51);
            bytes.push(0x03);
            bytes.push((uspqn >> 16) as u8);
            bytes.push((uspqn >> 8) as u8);
            bytes.push(uspqn as u8);
            meta_events.push((*tick, bytes));
        }

        // Mid-song time signature changes
        for (tick, ts) in &self.time_sig_changes {
            if *tick == 0 { continue; } // already handled above
            let mut bytes = Vec::new();
            bytes.push(0xFF);
            bytes.push(0x58);
            bytes.push(0x04);
            bytes.push(ts.numerator);
            let denom_pow = (ts.denominator as f32).log2() as u8;
            bytes.push(denom_pow);
            bytes.push(24);
            bytes.push(8);
            meta_events.push((*tick, bytes));
        }

        // Sort by tick (stable sort preserves order of same-tick events)
        meta_events.sort_by_key(|(tick, _)| *tick);

        // Write all meta events with delta times
        let mut current_tick = 0u32;
        for (abs_tick, bytes) in &meta_events {
            let delta = abs_tick - current_tick;
            write_variable_length(&mut tempo_data, delta);
            tempo_data.extend_from_slice(bytes);
            current_tick = *abs_tick;
        }

        // End of track
        write_variable_length(&mut tempo_data, 0);
        tempo_data.extend_from_slice(&[0xFF, 0x2F, 0x00]);

        writer.write_all(b"MTrk")?;
        writer.write_all(&(tempo_data.len() as u32).to_be_bytes())?;
        writer.write_all(&tempo_data)?;

        // --- Note tracks ---
        for track in &self.tracks {
            let track_data = self.encode_track(track);
            writer.write_all(b"MTrk")?;
            writer.write_all(&(track_data.len() as u32).to_be_bytes())?;
            writer.write_all(&track_data)?;
        }

        Ok(())
    }

    /// Write MIDI file to a path.
    pub fn write_file(&self, path: &str) -> Result<(), MidiExportError> {
        let mut file = std::fs::File::create(path)?;
        self.write(&mut file)
    }

    fn encode_track(&self, track: &MidiTrack) -> Vec<u8> {
        let mut data = Vec::new();

        // Track name meta event
        write_variable_length(&mut data, 0);
        data.push(0xFF);
        data.push(0x03);
        let name_bytes = track.name.as_bytes();
        write_variable_length(&mut data, name_bytes.len() as u32);
        data.extend_from_slice(name_bytes);

        // Program change
        write_variable_length(&mut data, 0);
        data.push(0xC0 | (track.channel & 0x0F));
        data.push(track.program & 0x7F);

        // Sort events by tick, then build note-on/note-off pairs
        let mut sorted_events = track.events.clone();
        sorted_events.sort_by_key(|e| e.tick);

        // Build a list of (absolute_tick, event_byte_1, byte_2, byte_3)
        // Note-on and note-off events
        let mut raw_events: Vec<(u32, u8, u8, u8)> = Vec::new();
        for evt in &sorted_events {
            let ch = evt.channel & 0x0F;
            // Note on
            raw_events.push((evt.tick, 0x90 | ch, evt.note & 0x7F, evt.velocity & 0x7F));
            // Note off
            raw_events.push((
                evt.tick + evt.duration_ticks,
                0x80 | ch,
                evt.note & 0x7F,
                0,
            ));
        }

        // Sort by absolute tick (note-offs before note-ons at same tick)
        raw_events.sort_by(|a, b| {
            a.0.cmp(&b.0)
                .then_with(|| {
                    let a_is_off = (a.1 & 0xF0) == 0x80;
                    let b_is_off = (b.1 & 0xF0) == 0x80;
                    b_is_off.cmp(&a_is_off) // note-offs first
                })
        });

        // Write events with delta times
        let mut current_tick = 0u32;
        for (abs_tick, status, d1, d2) in &raw_events {
            let delta = abs_tick - current_tick;
            write_variable_length(&mut data, delta);
            data.push(*status);
            data.push(*d1);
            data.push(*d2);
            current_tick = *abs_tick;
        }

        // End of track
        write_variable_length(&mut data, 0);
        data.extend_from_slice(&[0xFF, 0x2F, 0x00]);

        data
    }
}

/// Write a MIDI variable-length quantity.
fn write_variable_length(buf: &mut Vec<u8>, mut value: u32) {
    if value == 0 {
        buf.push(0);
        return;
    }

    let mut bytes = Vec::new();
    while value > 0 {
        bytes.push((value & 0x7F) as u8);
        value >>= 7;
    }
    bytes.reverse();

    for (i, byte) in bytes.iter().enumerate() {
        if i < bytes.len() - 1 {
            buf.push(byte | 0x80);
        } else {
            buf.push(*byte);
        }
    }
}

#[derive(Debug)]
pub enum MidiExportError {
    Io(std::io::Error),
}

impl From<std::io::Error> for MidiExportError {
    fn from(e: std::io::Error) -> Self {
        MidiExportError::Io(e)
    }
}

impl std::fmt::Display for MidiExportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MidiExportError::Io(e) => write!(f, "MIDI I/O error: {}", e),
        }
    }
}

impl std::error::Error for MidiExportError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_variable_length_encoding() {
        let mut buf = Vec::new();
        write_variable_length(&mut buf, 0);
        assert_eq!(buf, vec![0x00]);

        buf.clear();
        write_variable_length(&mut buf, 127);
        assert_eq!(buf, vec![0x7F]);

        buf.clear();
        write_variable_length(&mut buf, 128);
        assert_eq!(buf, vec![0x81, 0x00]);

        buf.clear();
        write_variable_length(&mut buf, 480);
        assert_eq!(buf, vec![0x83, 0x60]);
    }

    #[test]
    fn test_basic_midi_export() {
        let mut exporter = MidiExporter::new();
        exporter.set_tempo(Tempo::new(120.0));

        let mut track = MidiTrack::new("Piano", 0, 0);
        // C major chord
        track.add_note(0, 60, 80, 480); // C4
        track.add_note(0, 64, 80, 480); // E4
        track.add_note(0, 67, 80, 480); // G4

        exporter.add_track(track);

        let mut output = Vec::new();
        exporter.write(&mut output).unwrap();

        // Verify MIDI header
        assert_eq!(&output[0..4], b"MThd");
        assert!(output.len() > 14); // at least header + some track data
    }
}
