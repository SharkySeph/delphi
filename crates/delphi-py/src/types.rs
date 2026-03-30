use pyo3::prelude::*;
use pyo3::exceptions::PyValueError;

use delphi_core::chord::Chord;
use delphi_core::note::Note;
use delphi_core::scale::{Key, Scale};

/// A musical note with pitch class, accidental, and octave.
#[pyclass(name = "Note", skip_from_py_object)]
#[derive(Clone)]
pub struct PyNote {
    pub inner: Note,
}

#[pymethods]
impl PyNote {
    #[new]
    fn new(s: &str) -> PyResult<Self> {
        let note: Note = s
            .parse()
            .map_err(|e: delphi_core::note::NoteParseError| PyValueError::new_err(e.to_string()))?;
        Ok(PyNote { inner: note })
    }

    /// MIDI note number (0-127).
    #[getter]
    fn midi(&self) -> u8 {
        self.inner.to_midi()
    }

    /// Note name as string.
    fn __repr__(&self) -> String {
        format!("Note('{}')", self.inner)
    }

    fn __str__(&self) -> String {
        self.inner.to_string()
    }

    /// Transpose by semitones.
    fn transpose(&self, semitones: i16) -> PyNote {
        PyNote {
            inner: self.inner.transpose(semitones),
        }
    }
}

/// A chord defined by root + quality, resolves to notes.
#[pyclass(name = "Chord", skip_from_py_object)]
#[derive(Clone)]
pub struct PyChord {
    pub inner: Chord,
}

#[pymethods]
impl PyChord {
    #[new]
    fn new(s: &str) -> PyResult<Self> {
        let chord: Chord = s
            .parse()
            .map_err(|e: delphi_core::chord::ChordParseError| {
                PyValueError::new_err(e.to_string())
            })?;
        Ok(PyChord { inner: chord })
    }

    /// Get constituent notes as a list of Note objects.
    fn notes(&self) -> Vec<PyNote> {
        self.inner
            .notes()
            .into_iter()
            .map(|n| PyNote { inner: n })
            .collect()
    }

    /// Get MIDI note numbers.
    fn midi_notes(&self) -> Vec<u8> {
        self.inner.to_midi()
    }

    fn __repr__(&self) -> String {
        format!("Chord('{}')", self.inner)
    }

    fn __str__(&self) -> String {
        self.inner.to_string()
    }
}

/// A musical scale: root + scale type.
#[pyclass(name = "Scale", skip_from_py_object)]
#[derive(Clone)]
pub struct PyScale {
    pub inner: Scale,
}

#[pymethods]
impl PyScale {
    #[new]
    fn new(root: &str, scale_type: &str) -> PyResult<Self> {
        let key_str = format!("{} {}", root, scale_type);
        let key: Key = key_str
            .parse()
            .map_err(|e: delphi_core::note::NoteParseError| PyValueError::new_err(e.to_string()))?;
        let scale = key.to_scale(4);
        Ok(PyScale { inner: scale })
    }

    /// Get notes in the scale.
    fn notes(&self) -> Vec<PyNote> {
        self.inner
            .notes()
            .into_iter()
            .map(|n| PyNote { inner: n })
            .collect()
    }

    fn __repr__(&self) -> String {
        format!(
            "Scale('{}', '{}')",
            self.inner.root, self.inner.scale_type
        )
    }
}

/// A musical key (root + scale type, used as tonal center).
#[pyclass(name = "Key", skip_from_py_object)]
#[derive(Clone)]
pub struct PyKey {
    pub inner: Key,
}

#[pymethods]
impl PyKey {
    #[new]
    fn new(s: &str) -> PyResult<Self> {
        let key: Key = s
            .parse()
            .map_err(|e: delphi_core::note::NoteParseError| PyValueError::new_err(e.to_string()))?;
        Ok(PyKey { inner: key })
    }

    fn __repr__(&self) -> String {
        format!("Key('{}')", self.inner)
    }

    fn __str__(&self) -> String {
        self.inner.to_string()
    }
}
