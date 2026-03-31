mod types;
mod engine;
mod midi;
mod sf;

use pyo3::prelude::*;

/// The native Rust engine exposed to Python as `delphi._engine`.
#[pymodule]
fn _engine(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Core types
    m.add_class::<types::PyNote>()?;
    m.add_class::<types::PyChord>()?;
    m.add_class::<types::PyScale>()?;
    m.add_class::<types::PyKey>()?;
    m.add_class::<engine::StopFlag>()?;

    // Engine functions
    m.add_function(wrap_pyfunction!(engine::play_events, m)?)?;

    // MIDI functions
    m.add_function(wrap_pyfunction!(midi::export_midi, m)?)?;

    // SoundFont functions
    m.add_function(wrap_pyfunction!(sf::play_sf, m)?)?;
    m.add_function(wrap_pyfunction!(sf::render_wav, m)?)?;

    Ok(())
}
