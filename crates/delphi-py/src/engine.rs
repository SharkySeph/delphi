use pyo3::prelude::*;
use pyo3::exceptions::PyRuntimeError;

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use delphi_core::duration::{Duration, Tempo};
use delphi_core::dynamics::Velocity;
use delphi_engine::output::AudioOutput;
use delphi_engine::scheduler::AudioEvent;

/// A thread-safe stop flag that Python can use to cancel playback.
#[pyclass(from_py_object)]
#[derive(Clone)]
pub struct StopFlag {
    inner: Arc<AtomicBool>,
}

#[pymethods]
impl StopFlag {
    #[new]
    fn new() -> Self {
        Self {
            inner: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Signal playback to stop.
    fn stop(&self) {
        self.inner.store(true, Ordering::Relaxed);
    }

    /// Reset the flag for reuse.
    fn reset(&self) {
        self.inner.store(false, Ordering::Relaxed);
    }

    /// Check if stop has been requested.
    fn is_stopped(&self) -> bool {
        self.inner.load(Ordering::Relaxed)
    }
}

impl StopFlag {
    pub fn arc(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.inner)
    }
}

/// Play a list of (midi_note, velocity, tick, duration_ticks) tuples at a given BPM.
/// This is the low-level playback function called from Python.
#[pyfunction]
#[pyo3(signature = (events, bpm = 120.0, stop_flag = None))]
pub fn play_events(py: Python<'_>, events: Vec<(u8, u8, u32, u32)>, bpm: f64, stop_flag: Option<StopFlag>) -> PyResult<()> {
    let tempo = Tempo::new(bpm);
    let audio_events: Vec<AudioEvent> = events
        .into_iter()
        .map(|(note, vel, tick, dur)| {
            AudioEvent::new(tick, note, Velocity::new(vel), Duration::new(dur))
        })
        .collect();

    let stop = stop_flag.as_ref().map(|f| f.arc()).unwrap_or_else(|| Arc::new(AtomicBool::new(false)));
    let stop_thread = Arc::clone(&stop);

    let handle = std::thread::spawn(move || {
        let output = AudioOutput::new();
        output.play_events(&audio_events, &tempo, &stop_thread)
    });

    // Poll for stop flag / KeyboardInterrupt while playback runs.
    // Release the GIL during sleep so other Python threads (e.g. the
    // prompt_toolkit UI thread) can run and set the stop flag.
    loop {
        py.detach(|| std::thread::sleep(std::time::Duration::from_millis(50)));

        // Check if stop was requested externally (e.g. Studio Ctrl+C)
        if stop.load(Ordering::Relaxed) {
            let _ = handle.join();
            return Ok(());
        }

        // Check for KeyboardInterrupt (works on main thread)
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
