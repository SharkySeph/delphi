use pyo3::prelude::*;
use pyo3::exceptions::PyRuntimeError;

use delphi_core::duration::{Tempo, TimeSignature};
use delphi_midi::export::{MidiExporter, MidiTrack};

/// Export a list of note events to a MIDI file.
///
/// events: list of (midi_note, velocity, tick, duration_ticks) tuples
/// path: output file path
/// bpm: tempo in BPM
/// time_sig: tuple (numerator, denominator)
#[pyfunction]
#[pyo3(signature = (events, path, bpm = 120.0, time_sig = (4, 4), track_name = "Delphi", program = 0))]
pub fn export_midi(
    events: Vec<(u8, u8, u32, u32)>,
    path: &str,
    bpm: f64,
    time_sig: (u8, u8),
    track_name: &str,
    program: u8,
) -> PyResult<()> {
    let mut exporter = MidiExporter::new();
    exporter.set_tempo(Tempo::new(bpm));
    exporter.set_time_signature(TimeSignature::new(time_sig.0, time_sig.1));

    let mut track = MidiTrack::new(track_name, 0, program);
    for (note, vel, tick, dur) in events {
        track.add_note(tick, note, vel, dur);
    }

    exporter.add_track(track);
    exporter
        .write_file(path)
        .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;

    Ok(())
}
