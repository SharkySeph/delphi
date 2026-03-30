"""Tests for the notation parser."""
import sys
import os

sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "..", "python"))

from delphi.notation import parse, events_to_tuples, TICKS_PER_QUARTER


def test_parse_single_notes():
    events = parse("C4 E4 G4")
    assert len(events) == 3
    assert events[0].midi_notes == [60]
    assert events[1].midi_notes == [64]
    assert events[2].midi_notes == [67]


def test_parse_notes_sequential_timing():
    events = parse("C4 E4 G4")
    assert events[0].tick == 0
    assert events[1].tick == TICKS_PER_QUARTER
    assert events[2].tick == TICKS_PER_QUARTER * 2


def test_parse_bar_notation():
    events = parse("| Cmaj7 | Am7 |")
    assert len(events) == 2
    # Cmaj7 = C E G B
    assert events[0].kind == "chord"
    assert 60 in events[0].midi_notes  # C4
    # Each bar takes 4 beats (1920 ticks)
    assert events[0].tick == 0
    assert events[1].tick == TICKS_PER_QUARTER * 4


def test_parse_rests():
    events = parse("C4 . E4 .")
    assert len(events) == 4
    assert events[0].kind == "note"
    assert events[1].kind == "rest"
    assert events[2].kind == "note"
    assert events[3].kind == "rest"


def test_parse_drum_names():
    events = parse("kick snare hihat")
    assert len(events) == 3
    assert events[0].kind == "drum"
    assert events[0].midi_notes == [36]  # kick = 36
    assert events[1].midi_notes == [38]  # snare = 38
    assert events[2].midi_notes == [42]  # hihat = 42


def test_parse_duration_suffix():
    events = parse("C4:h E4:8")
    assert events[0].duration_ticks == TICKS_PER_QUARTER * 2  # half
    assert events[1].duration_ticks == TICKS_PER_QUARTER // 2  # eighth


def test_parse_dynamics():
    events = parse("C4!ff")
    assert events[0].velocity == 112


def test_parse_subdivision():
    events = parse("[C4 E4 G4]")
    assert len(events) == 3
    # Each should be 1/3 of a quarter note
    expected_dur = TICKS_PER_QUARTER // 3
    assert events[0].duration_ticks == expected_dur
    assert events[1].duration_ticks == expected_dur
    assert events[2].duration_ticks == expected_dur


def test_events_to_tuples():
    events = parse("C4 E4")
    tuples = events_to_tuples(events)
    assert len(tuples) == 2
    # Each tuple: (midi_note, velocity, tick, duration)
    assert tuples[0][0] == 60  # C4
    assert tuples[1][0] == 64  # E4


def test_parse_accidentals():
    events = parse("C#4 Db4 Ebb4")
    assert events[0].midi_notes == [61]  # C#4
    assert events[1].midi_notes == [61]  # Db4
    assert events[2].midi_notes == [62]  # Ebb4 = D4


def test_parse_empty():
    events = parse("")
    assert events == []


def test_chord_quality_varieties():
    for chord_str in ["| C |", "| Cm |", "| C7 |", "| Cmaj7 |", "| Cm7 |",
                       "| Cdim |", "| Csus4 |", "| C5 |"]:
        events = parse(chord_str)
        assert len(events) == 1, f"Failed to parse: {chord_str}"
        assert events[0].kind == "chord"


def test_multi_bar_progression():
    events = parse("| C | Am | F | G |")
    assert len(events) == 4
    # Each bar is measure_ticks = 1920 apart
    for i, evt in enumerate(events):
        assert evt.tick == i * TICKS_PER_QUARTER * 4
