use pyo3::prelude::*;
use pyo3::exceptions::PyRuntimeError;

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use delphi_core::duration::{Duration, Tempo};
use delphi_core::dynamics::Velocity;
use delphi_engine::output::AudioOutput;
use delphi_engine::scheduler::AudioEvent;

/// Play a list of (midi_note, velocity, tick, duration_ticks) tuples at a given BPM.
/// This is the low-level playback function called from Python.
#[pyfunction]
#[pyo3(signature = (events, bpm = 120.0))]
pub fn play_events(py: Python<'_>, events: Vec<(u8, u8, u32, u32)>, bpm: f64) -> PyResult<()> {
    let tempo = Tempo::new(bpm);
    let audio_events: Vec<AudioEvent> = events
        .into_iter()
        .map(|(note, vel, tick, dur)| {
            AudioEvent::new(tick, note, Velocity::new(vel), Duration::new(dur))
        })
        .collect();

    let stop = Arc::new(AtomicBool::new(false));
    let stop_thread = Arc::clone(&stop);

    let handle = std::thread::spawn(move || {
        let output = AudioOutput::new();
        output.play_events(&audio_events, &tempo, &stop_thread)
    });

    // Poll for KeyboardInterrupt (Ctrl+C) while playback runs
    loop {
        std::thread::sleep(std::time::Duration::from_millis(100));
        if let Err(e) = py.check_signals() {
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
