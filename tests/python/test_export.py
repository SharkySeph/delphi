"""Tests for export (MIDI fallback)."""
import sys
import os
import tempfile

sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "..", "python"))

from delphi.context import reset_context, tempo
from delphi.export import export


def test_midi_export_creates_file():
    reset_context()
    tempo(120)

    with tempfile.NamedTemporaryFile(suffix=".mid", delete=False) as f:
        path = f.name

    try:
        export(path, "| C | Am | F | G |")
        assert os.path.exists(path)
        with open(path, "rb") as f:
            header = f.read(4)
        assert header == b"MThd", "Not a valid MIDI file"
    finally:
        os.unlink(path)


def test_midi_export_with_notes():
    reset_context()
    tempo(100)

    with tempfile.NamedTemporaryFile(suffix=".mid", delete=False) as f:
        path = f.name

    try:
        export(path, "C4 E4 G4")
        assert os.path.exists(path)
        size = os.path.getsize(path)
        assert size > 14, "MIDI file too small"
    finally:
        os.unlink(path)
