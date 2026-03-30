use pyo3::prelude::*;
use pyo3::exceptions::PyRuntimeError;

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use delphi_core::duration::Tempo;
use delphi_engine::soundfont::{SfEvent, play_with_soundfont, render_to_wav};

/// Play multi-voice events through a SoundFont.
///
/// sf_path: path to a .sf2 SoundFont file
/// events: list of (midi_note, velocity, tick, duration_ticks, channel, program) tuples
/// bpm: tempo in BPM
#[pyfunction]
#[pyo3(signature = (sf_path, events, bpm = 120.0))]
pub fn play_sf(
    py: Python<'_>,
    sf_path: &str,
    events: Vec<(u8, u8, u32, u32, u8, u8)>,
    bpm: f64,
) -> PyResult<()> {
    let tempo = Tempo::new(bpm);
    let sf_events: Vec<SfEvent> = events
        .into_iter()
        .map(|(note, vel, tick, dur, ch, prog)| SfEvent {
            tick,
            midi_note: note,
            velocity: vel,
            duration_ticks: dur,
            channel: ch,
            program: prog,
        })
        .collect();

    let sf_path_owned = PathBuf::from(sf_path);
    let stop = Arc::new(AtomicBool::new(false));
    let stop_thread = Arc::clone(&stop);

    // Spawn playback in a separate thread so the main thread can check signals
    let handle = std::thread::spawn(move || {
        play_with_soundfont(&sf_path_owned, &sf_events, &tempo, &stop_thread)
    });

    // Poll for KeyboardInterrupt (Ctrl+C) while playback runs
    loop {
        std::thread::sleep(std::time::Duration::from_millis(100));
        if let Err(e) = py.check_signals() {
            // Signal received (e.g. Ctrl+C) — tell playback to stop
            stop.store(true, Ordering::Relaxed);
            let _ = handle.join();
            return Err(e);
        }
        if handle.is_finished() {
            break;
        }
    }

    match handle.join() {
        Ok(Ok(())) => Ok(()),
        Ok(Err(e)) => Err(PyRuntimeError::new_err(e.to_string())),
        Err(_) => Err(PyRuntimeError::new_err("Playback thread panicked")),
    }
}

/// Render multi-voice events to a WAV file through a SoundFont.
///
/// sf_path: path to a .sf2 SoundFont file
/// events: list of (midi_note, velocity, tick, duration_ticks, channel, program) tuples
/// output_path: path for the output .wav file
/// bpm: tempo in BPM
#[pyfunction]
#[pyo3(signature = (sf_path, events, output_path, bpm = 120.0))]
pub fn render_wav(
    sf_path: &str,
    events: Vec<(u8, u8, u32, u32, u8, u8)>,
    output_path: &str,
    bpm: f64,
) -> PyResult<()> {
    let tempo = Tempo::new(bpm);
    let sf_events: Vec<SfEvent> = events
        .into_iter()
        .map(|(note, vel, tick, dur, ch, prog)| SfEvent {
            tick,
            midi_note: note,
            velocity: vel,
            duration_ticks: dur,
            channel: ch,
            program: prog,
        })
        .collect();

    render_to_wav(
        Path::new(sf_path),
        &sf_events,
        &tempo,
        Path::new(output_path),
    )
    .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;

    Ok(())
}
