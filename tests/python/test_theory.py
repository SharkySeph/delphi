"""Tests for the theory module."""
import sys
import os

sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "..", "python"))

from delphi.theory import note, chord, scale


def test_note_creation():
    n = note("C4")
    assert n.midi == 60
    assert str(n) == "C4"


def test_note_transpose():
    n = note("C4").transpose(4)
    assert n.midi == 64  # E4


def test_chord_creation():
    c = chord("Cmaj7")
    assert c.midi_notes == [60, 64, 67, 71]


def test_chord_minor():
    c = chord("Am")
    assert c.midi_notes == [69, 72, 76]


def test_chord_arpeggio():
    arp = chord("C").arpeggio("up")
    tuples = arp._build_tuples()
    assert len(tuples) == 3
    assert tuples[0][0] == 60  # C4
    assert tuples[1][0] == 64  # E4
    assert tuples[2][0] == 67  # G4


def test_chord_arpeggio_down():
    arp = chord("C").arpeggio("down")
    tuples = arp._build_tuples()
    assert tuples[0][0] == 67  # G4 first
    assert tuples[-1][0] == 60  # C4 last


def test_scale_creation():
    s = scale("C", "major")
    midis = s.midi_notes
    assert midis == [60, 62, 64, 65, 67, 69, 71]


def test_scale_minor():
    s = scale("A", "minor")
    midis = s.midi_notes
    assert midis == [69, 71, 72, 74, 76, 77, 79]


def test_scale_blues():
    s = scale("C", "blues")
    midis = s.midi_notes
    assert midis == [60, 63, 65, 66, 67, 70]
