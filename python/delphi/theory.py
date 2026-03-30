"""
High-level music theory helpers that return Python objects.
These use the Rust engine when available, with pure-Python fallbacks.
"""

from typing import Optional


def _get_engine():
    """Try to import the Rust engine; return None if not built yet."""
    try:
        from delphi import _engine
        return _engine
    except ImportError:
        return None


class NoteObj:
    """A note object with method chaining and playback."""

    def __init__(self, name: str):
        self.name = name
        self._midi = _note_name_to_midi(name)

    @property
    def midi(self) -> int:
        return self._midi

    def transpose(self, semitones: int) -> "NoteObj":
        new_midi = max(0, min(127, self._midi + semitones))
        return NoteObj(_midi_to_name(new_midi))

    def play(self):
        from delphi.playback import play_notes
        play_notes([(self._midi, 80, 0, 480)])

    def __repr__(self):
        return f"note('{self.name}')"

    def __str__(self):
        return self.name


class ChordObj:
    """A chord object with method chaining, arpeggio, and playback."""

    def __init__(self, name: str):
        self.name = name
        self._parse()

    def _parse(self):
        from delphi.notation import CHORD_RE, _note_to_midi, _chord_intervals
        token = self.name.strip()
        m = CHORD_RE.match(token)
        if m:
            root = m.group(1).upper()
            acc = m.group(2) or ""
            quality = m.group(3) or ""
            root_midi = _note_to_midi(root, acc, 4)
            intervals = _chord_intervals(quality)
            self._midi_notes = [root_midi + iv for iv in intervals]
        else:
            raise ValueError(f"Cannot parse chord: '{self.name}'")

    @property
    def midi_notes(self) -> list[int]:
        return list(self._midi_notes)

    def notes(self) -> list[NoteObj]:
        return [NoteObj(_midi_to_name(m)) for m in self._midi_notes]

    def arpeggio(self, direction: str = "up", duration: str = "eighth") -> "ArpeggioObj":
        return ArpeggioObj(self, direction, duration)

    def play(self):
        from delphi.playback import play_notes
        tuples = [(n, 80, 0, 480 * 2) for n in self._midi_notes]
        play_notes(tuples)

    def __repr__(self):
        return f"chord('{self.name}')"

    def __str__(self):
        return self.name


class ArpeggioObj:
    """An arpeggiated chord, ready for playback or export."""

    def __init__(self, chord_obj: ChordObj, direction: str, duration: str):
        self.chord_obj = chord_obj
        self.direction = direction
        self.duration = duration

    def _build_tuples(self) -> list[tuple[int, int, int, int]]:
        from delphi.notation import DURATION_MAP, TICKS_PER_QUARTER
        dur_ticks = DURATION_MAP.get(self.duration, TICKS_PER_QUARTER // 2)
        notes = list(self.chord_obj._midi_notes)
        if self.direction == "down":
            notes = list(reversed(notes))
        elif self.direction == "updown":
            notes = notes + list(reversed(notes[1:-1]))

        tuples = []
        tick = 0
        for n in notes:
            tuples.append((n, 80, tick, dur_ticks))
            tick += dur_ticks
        return tuples

    def play(self):
        from delphi.playback import play_notes
        play_notes(self._build_tuples())

    def __repr__(self):
        return f"chord('{self.chord_obj.name}').arpeggio('{self.direction}')"


class ScaleObj:
    """A scale object."""

    def __init__(self, root: str, scale_type: str):
        self.root = root
        self.scale_type = scale_type
        self._compute()

    def _compute(self):
        from delphi.notation import _note_to_midi

        # Parse root
        r = self.root.strip()
        pitch = r[0].upper()
        acc = r[1:] if len(r) > 1 else ""

        root_midi = _note_to_midi(pitch, acc, 4)

        scale_intervals = {
            # Standard modes (church modes)
            "major": [0, 2, 4, 5, 7, 9, 11],
            "ionian": [0, 2, 4, 5, 7, 9, 11],
            "minor": [0, 2, 3, 5, 7, 8, 10],
            "aeolian": [0, 2, 3, 5, 7, 8, 10],
            "natural minor": [0, 2, 3, 5, 7, 8, 10],
            "harmonic minor": [0, 2, 3, 5, 7, 8, 11],
            "melodic minor": [0, 2, 3, 5, 7, 9, 11],
            "dorian": [0, 2, 3, 5, 7, 9, 10],
            "phrygian": [0, 1, 3, 5, 7, 8, 10],
            "lydian": [0, 2, 4, 6, 7, 9, 11],
            "mixolydian": [0, 2, 4, 5, 7, 9, 10],
            "locrian": [0, 1, 3, 5, 6, 8, 10],

            # Pentatonic & blues
            "pentatonic": [0, 2, 4, 7, 9],
            "major pentatonic": [0, 2, 4, 7, 9],
            "minor pentatonic": [0, 3, 5, 7, 10],
            "blues": [0, 3, 5, 6, 7, 10],
            "major blues": [0, 2, 3, 4, 7, 9],

            # Symmetric scales
            "whole tone": [0, 2, 4, 6, 8, 10],
            "chromatic": [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11],
            "diminished": [0, 2, 3, 5, 6, 8, 9, 11],       # whole-half
            "half-whole diminished": [0, 1, 3, 4, 6, 7, 9, 10],
            "augmented": [0, 3, 4, 7, 8, 11],

            # Bebop scales
            "bebop dominant": [0, 2, 4, 5, 7, 9, 10, 11],
            "bebop major": [0, 2, 4, 5, 7, 8, 9, 11],
            "bebop minor": [0, 2, 3, 5, 7, 8, 9, 10],
            "bebop dorian": [0, 2, 3, 4, 5, 7, 9, 10],

            # Altered / exotic
            "altered": [0, 1, 3, 4, 6, 8, 10],              # super locrian
            "super locrian": [0, 1, 3, 4, 6, 8, 10],
            "lydian dominant": [0, 2, 4, 6, 7, 9, 10],      # lydian b7
            "lydian augmented": [0, 2, 4, 6, 8, 9, 11],
            "phrygian dominant": [0, 1, 4, 5, 7, 8, 10],    # Spanish / Jewish
            "spanish": [0, 1, 4, 5, 7, 8, 10],
            "double harmonic": [0, 1, 4, 5, 7, 8, 11],      # Byzantine / Arabic
            "hungarian minor": [0, 2, 3, 6, 7, 8, 11],
            "hungarian major": [0, 3, 4, 6, 7, 9, 10],
            "neapolitan minor": [0, 1, 3, 5, 7, 8, 11],
            "neapolitan major": [0, 1, 3, 5, 7, 9, 11],
            "enigmatic": [0, 1, 4, 6, 8, 10, 11],
            "persian": [0, 1, 4, 5, 6, 8, 11],
            "arabian": [0, 2, 4, 5, 6, 8, 10],
            "japanese": [0, 1, 5, 7, 8],                    # in-sen
            "hirajoshi": [0, 4, 6, 7, 11],
            "kumoi": [0, 2, 3, 7, 9],
            "iwato": [0, 1, 5, 6, 10],
            "egyptian": [0, 2, 5, 7, 10],

            # Jazz / modal interchange
            "dorian b2": [0, 1, 3, 5, 7, 9, 10],
            "mixolydian b6": [0, 2, 4, 5, 7, 8, 10],       # Hindu / Aeolian dominant
            "locrian #2": [0, 2, 3, 5, 6, 8, 10],
        }

        intervals = scale_intervals.get(self.scale_type.lower(), [0, 2, 4, 5, 7, 9, 11])
        self._midi_notes = [root_midi + iv for iv in intervals]

    @property
    def midi_notes(self) -> list[int]:
        return list(self._midi_notes)

    def notes(self) -> list[NoteObj]:
        return [NoteObj(_midi_to_name(m)) for m in self._midi_notes]

    def play(self):
        from delphi.playback import play_notes
        tuples = [(n, 80, i * 240, 240) for i, n in enumerate(self._midi_notes)]
        play_notes(tuples)

    def __repr__(self):
        return f"scale('{self.root}', '{self.scale_type}')"


# --- Public API functions ---

def note(name: str) -> NoteObj:
    """Create a note object. e.g. note('C4')"""
    return NoteObj(name)


def chord(name: str) -> ChordObj:
    """Create a chord object. e.g. chord('Cmaj7')"""
    return ChordObj(name)


def scale(root: str, scale_type: str = "major") -> ScaleObj:
    """Create a scale object. e.g. scale('C', 'major')"""
    return ScaleObj(root, scale_type)


# --- Utilities ---

_NOTE_NAMES = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"]


def _midi_to_name(midi: int) -> str:
    """Convert MIDI number to note name."""
    octave = (midi // 12) - 1
    note_idx = midi % 12
    return f"{_NOTE_NAMES[note_idx]}{octave}"


def _note_name_to_midi(name: str) -> int:
    """Convert note name to MIDI number."""
    from delphi.notation import _note_to_midi
    import re
    m = re.match(r'^([A-Ga-g])(##?|bb?)?(-?\d+)$', name.strip())
    if not m:
        raise ValueError(f"Invalid note: '{name}'")
    return _note_to_midi(m.group(1).upper(), m.group(2) or "", int(m.group(3)))
