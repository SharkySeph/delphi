"""
Composition tools for organizing large pieces.

Provides Section, Pattern, Voice, Arrangement, and PatternLibrary for
building structured compositions from reusable building blocks.

Usage:
    verse = Section("Verse")
    verse.add("Piano", "| C | Am | F | G |", program="piano")
    verse.add("Bass", "C2:h G2:h A2:h E2:h", program="acoustic bass")

    chorus = Section("Chorus")
    chorus.add("Piano", "| F | G | C | Am |", program="piano")
    chorus.add("Bass", "F2:h G2:h C2:h A2:h", program="acoustic bass")

    # Quick arrangement with timeline syntax:
    arr = Arrangement("My Song", tempo=120)
    arr.section(verse, repeat=2)
    arr.section(chorus)
    arr.section(verse)
    arr.section(chorus, repeat=2)
    arr.build().play()

    # Or timeline shorthand:
    arr = Arrangement("My Song", tempo=120)
    arr.timeline(verse, verse, chorus, verse, (chorus, 2))

Pattern:
    p = Pattern("Arpeggio", "C4:8 E4:8 G4:8 C5:8")
    p.repeat(4)  # play 4 times

Voice:
    lead = Voice("Lead", program="piano")
    lead.add(verse_pattern)
    lead.add(chorus_pattern)

Pattern library (global named patterns):
    lib = PatternLibrary()
    lib.register("motif_a", "C4:8 E4:8 G4:8 C5:8")
    lib.register("motif_b", "D4:8 F4:8 A4:8 D5:8")
    p = lib.get("motif_a")  # returns a Pattern

File includes:
    include("strings.delphi")  # imports notation/patterns from another file
"""

from __future__ import annotations

import os
from dataclasses import dataclass, field
from typing import Optional

from delphi.notation import parse, events_to_tuples, TICKS_PER_QUARTER


@dataclass
class Pattern:
    """
    A reusable musical pattern — a named snippet of notation.

    Can be repeated, transposed, and reversed before being placed
    into a Section or Voice.
    """

    name: str
    notation: str = ""
    _transpose: int = 0
    _reversed: bool = False
    _repeats: int = 1

    def repeat(self, n: int) -> "Pattern":
        """Repeat this pattern N times."""
        self._repeats = max(1, n)
        return self

    def transpose(self, semitones: int) -> "Pattern":
        """Transpose all notes by N semitones."""
        self._transpose += semitones
        return self

    def rev(self) -> "Pattern":
        """Reverse the pattern."""
        self._reversed = not self._reversed
        return self

    def get_events(self, default_velocity: int = 80) -> list:
        """Parse and return events (with transforms applied)."""
        events = parse(self.notation, default_velocity=default_velocity)

        # Transpose
        if self._transpose != 0:
            for evt in events:
                evt.midi_notes = [
                    max(0, min(127, n + self._transpose))
                    for n in evt.midi_notes
                ]

        # Reverse
        if self._reversed and events:
            max_tick = max(e.tick + e.duration_ticks for e in events)
            for evt in events:
                evt.tick = max_tick - evt.tick - evt.duration_ticks
            events.sort(key=lambda e: e.tick)

        # Repeat
        if self._repeats > 1:
            if events:
                pattern_len = max(e.tick + e.duration_ticks for e in events)
            else:
                pattern_len = 0
            all_events = list(events)
            for r in range(1, self._repeats):
                offset = pattern_len * r
                for evt in events:
                    from copy import copy
                    new_evt = copy(evt)
                    new_evt.tick = evt.tick + offset
                    all_events.append(new_evt)
            events = all_events
            events.sort(key=lambda e: e.tick)

        return events

    def duration_ticks(self) -> int:
        """Total duration in ticks."""
        events = self.get_events()
        if not events:
            return 0
        return max(e.tick + e.duration_ticks for e in events)

    def __repr__(self):
        return f'Pattern("{self.name}", {self._repeats}x)'


class Voice:
    """
    A single instrument voice across the entire piece.

    Holds an ordered list of Patterns that will be played sequentially.
    """

    def __init__(self, name: str, program: int | str = 0,
                 velocity: int = 80, channel: Optional[int] = None):
        self.name = name
        self.program = program
        self.velocity = velocity
        self.channel = channel
        self.patterns: list[Pattern] = []

    def add(self, pattern: Pattern | str) -> "Voice":
        """Add a pattern (or raw notation string) to this voice."""
        if isinstance(pattern, str):
            pattern = Pattern(f"_{len(self.patterns)}", pattern)
        self.patterns.append(pattern)
        return self

    def get_tuples(self) -> list[tuple[int, int, int, int]]:
        """Get all (midi, velocity, tick, duration) tuples across all patterns."""
        tuples = []
        offset = 0
        for pat in self.patterns:
            events = pat.get_events(default_velocity=self.velocity)
            for evt in events:
                if evt.kind != "rest":
                    for t in evt.to_tuples():
                        tuples.append((t[0], t[1], t[2] + offset, t[3]))
            offset += pat.duration_ticks()
        return tuples

    def __repr__(self):
        return f'Voice("{self.name}", {len(self.patterns)} patterns)'


class Section:
    """
    A named section of a composition (e.g., Verse, Chorus, Bridge).

    Contains multiple voices that play simultaneously within this section.
    """

    def __init__(self, name: str):
        self.name = name
        self.voices: list[Voice] = []
        self._repeat: int = 1

    def add(self, name: str, notation: str, program: int | str = 0,
            velocity: int = 80) -> "Section":
        """Add a voice to this section with a notation string."""
        voice = Voice(name, program=program, velocity=velocity)
        voice.add(Pattern(f"{self.name}_{name}", notation))
        self.voices.append(voice)
        return self

    def add_voice(self, voice: Voice) -> "Section":
        """Add an existing Voice object to this section."""
        self.voices.append(voice)
        return self

    def repeat(self, n: int) -> "Section":
        """Set repeat count for this section."""
        self._repeat = max(1, n)
        return self

    def duration_ticks(self) -> int:
        """Duration of one pass of this section in ticks."""
        if not self.voices:
            return 0
        maxdur = 0
        for v in self.voices:
            t = v.get_tuples()
            if t:
                maxdur = max(maxdur, t[-1][2] + t[-1][3])
        return maxdur

    def __repr__(self):
        voices = ", ".join(v.name for v in self.voices)
        rpt = f" x{self._repeat}" if self._repeat > 1 else ""
        return f'Section("{self.name}"{rpt}: [{voices}])'


def build_song_from_sections(title: str, sections: list[Section | tuple[Section, int]],
                             tempo: float = 120, key: str = "C major") -> "Song":
    """
    Build a Song from a list of Sections.

    sections can contain Section objects or (Section, repeat_count) tuples.

    Example:
        song = build_song_from_sections("My Song", [
            (verse, 2),
            chorus,
            (verse, 1),
            (chorus, 2),
        ], tempo=120)
    """
    from delphi.song import Song, Track, GM_INSTRUMENTS

    song = Song(title, tempo=tempo, key=key)

    # Collect all unique voice names + programs across sections
    voice_map: dict[str, dict] = {}
    for item in sections:
        if isinstance(item, tuple):
            section, _ = item
        else:
            section = item
        for voice in section.voices:
            if voice.name not in voice_map:
                voice_map[voice.name] = {
                    "program": voice.program,
                    "velocity": voice.velocity,
                    "channel": voice.channel,
                    "notations": [],
                }

    # Build notation for each voice by concatenating sections
    for item in sections:
        if isinstance(item, tuple):
            section, repeat = item
        else:
            section, repeat = item, 1

        # Find notation for each voice in this section
        section_voices = {v.name: v for v in section.voices}

        for vname, vdata in voice_map.items():
            if vname in section_voices:
                voice = section_voices[vname]
                for pat in voice.patterns:
                    notation = pat.notation
                    for _ in range(repeat * pat._repeats):
                        vdata["notations"].append(notation)
            else:
                # This voice has a rest for this section
                dur = section.duration_ticks()
                rest_count = max(1, dur // TICKS_PER_QUARTER)
                vdata["notations"].append(" ".join(["r"] * rest_count))

    # Create tracks
    for vname, vdata in voice_map.items():
        full_notation = " ".join(vdata["notations"])
        song.track(
            vname, full_notation,
            program=vdata["program"],
            channel=vdata["channel"],
            velocity=vdata["velocity"],
        )

    return song


# ── Pattern Library (global named patterns) ───────────────────

class PatternLibrary:
    """A registry of named reusable patterns.

    Usage:
        lib = PatternLibrary()
        lib.register("motif_a", "C4:8 E4:8 G4:8 C5:8")
        lib.register("motif_b", "D4:8 F4:8 A4:8 D5:8")
        p = lib.get("motif_a")           # returns a new Pattern
        p = lib.get("motif_a").repeat(4)  # chainable
        lib.names()                       # list all registered names
    """

    def __init__(self):
        self._patterns: dict[str, str] = {}

    def register(self, name: str, notation: str) -> None:
        """Register a named pattern."""
        self._patterns[name] = notation

    def get(self, name: str) -> Pattern:
        """Get a new Pattern instance by name."""
        if name not in self._patterns:
            available = ", ".join(sorted(self._patterns.keys()))
            raise KeyError(f"Pattern '{name}' not found. Available: {available}")
        return Pattern(name, self._patterns[name])

    def names(self) -> list[str]:
        """List all registered pattern names."""
        return sorted(self._patterns.keys())

    def remove(self, name: str) -> None:
        """Remove a pattern from the library."""
        self._patterns.pop(name, None)

    def __contains__(self, name: str) -> bool:
        return name in self._patterns

    def __repr__(self):
        return f"PatternLibrary({len(self._patterns)} patterns: {self.names()})"


# Global shared library instance
_global_library = PatternLibrary()


def register_pattern(name: str, notation: str) -> None:
    """Register a pattern in the global library."""
    _global_library.register(name, notation)


def get_pattern(name: str) -> Pattern:
    """Get a pattern from the global library."""
    return _global_library.get(name)


def list_patterns() -> list[str]:
    """List all patterns in the global library."""
    return _global_library.names()


# ── Arrangement (timeline-based composition) ──────────────────

class Arrangement:
    """Build a Song from a timeline of sections with rehearsal marks.

    Usage:
        arr = Arrangement("My Song", tempo=120)
        arr.section(intro)
        arr.section(verse, repeat=2)
        arr.mark("Chorus")
        arr.section(chorus)
        arr.section(verse)
        arr.section(chorus, repeat=2)
        arr.section(outro)

        song = arr.build()
        song.play()

        # Play from a specific rehearsal mark:
        song = arr.build(start_from="Chorus")

        # Extract a single part:
        piano_song = arr.extract("Piano")

        # View the arrangement:
        arr.show()
    """

    def __init__(self, title: str = "Untitled", *, tempo: float = 120,
                 key: str = "C major"):
        self.title = title
        self.tempo = tempo
        self.key = key
        self._entries: list[dict] = []  # [{type: "section"/"mark", ...}]

    def section(self, sec: Section, repeat: int = 1) -> "Arrangement":
        """Add a section to the timeline."""
        self._entries.append({
            "type": "section",
            "section": sec,
            "repeat": repeat,
        })
        return self

    def mark(self, name: str) -> "Arrangement":
        """Insert a rehearsal mark at this point in the timeline."""
        self._entries.append({
            "type": "mark",
            "name": name,
        })
        return self

    def timeline(self, *items) -> "Arrangement":
        """Add multiple sections at once.

        Items can be Section objects or (Section, repeat_count) tuples.
        String items are treated as rehearsal marks.

        Example:
            arr.timeline(intro, "Verse", verse, verse, "Chorus", chorus,
                         verse, (chorus, 2), outro)
        """
        for item in items:
            if isinstance(item, str):
                self.mark(item)
            elif isinstance(item, tuple):
                sec, repeat = item
                self.section(sec, repeat=repeat)
            elif isinstance(item, Section):
                self.section(item)
        return self

    def build(self, start_from: str | None = None) -> "Song":
        """Build a Song from the arrangement.

        Args:
            start_from: Optional rehearsal mark name to start playback from.
        """
        # Filter entries if start_from is specified
        entries = self._entries
        if start_from:
            start_idx = None
            for i, entry in enumerate(entries):
                if entry["type"] == "mark" and entry["name"] == start_from:
                    start_idx = i
                    break
            if start_idx is not None:
                entries = entries[start_idx:]
            else:
                raise ValueError(
                    f"Rehearsal mark '{start_from}' not found. "
                    f"Available: {self.marks()}"
                )

        sections = []
        for entry in entries:
            if entry["type"] == "section":
                sec = entry["section"]
                repeat = entry["repeat"]
                if repeat > 1:
                    sections.append((sec, repeat))
                else:
                    sections.append(sec)

        return build_song_from_sections(
            self.title, sections, tempo=self.tempo, key=self.key
        )

    def extract(self, voice_name: str) -> "Song":
        """Build a Song containing only the specified voice/part.

        Useful for practice parts or debugging a single instrument.
        """
        song = self.build()

        from delphi.song import Song, Track
        extracted = Song(f"{self.title} — {voice_name}",
                         tempo=self.tempo, key=self.key)
        for track in song.tracks:
            if track.name == voice_name:
                extracted.add_track(track)
                break
        else:
            names = [t.name for t in song.tracks]
            raise ValueError(
                f"Voice '{voice_name}' not found. Available: {names}"
            )
        return extracted

    def marks(self) -> list[str]:
        """List all rehearsal marks."""
        return [e["name"] for e in self._entries if e["type"] == "mark"]

    def show(self) -> None:
        """Print a visual timeline of the arrangement."""
        print(f"\n  \033[1m{self.title}\033[0m ({self.tempo} BPM, {self.key})\n")
        section_num = 0
        for entry in self._entries:
            if entry["type"] == "mark":
                print(f"  \033[1;33m── {entry['name']} ──\033[0m")
            elif entry["type"] == "section":
                section_num += 1
                sec = entry["section"]
                repeat = entry["repeat"]
                rpt = f" ×{repeat}" if repeat > 1 else ""
                voices = ", ".join(v.name for v in sec.voices)
                print(f"  {section_num:2d}. {sec.name}{rpt}  [{voices}]")
        print()

    def __repr__(self):
        sec_count = sum(1 for e in self._entries if e["type"] == "section")
        return f'Arrangement("{self.title}", {sec_count} sections)'


# ── File Include ──────────────────────────────────────────────

def include(path: str, namespace: dict | None = None) -> dict:
    """Execute a .delphi/.py file and return its namespace.

    This lets you split large compositions across multiple files:

        # In your main file:
        ns = include("strings.delphi")
        violin_part = ns["violin"]

        # Or inject into current namespace:
        include("patterns.py", namespace=globals())

    Args:
        path: Path to the .delphi or .py file.
        namespace: Optional dict to merge results into.

    Returns:
        The namespace dict from the executed file.
    """
    import delphi

    resolved = os.path.expanduser(path)
    if not os.path.isabs(resolved):
        # Try relative to caller's directory
        import inspect
        frame = inspect.stack()[1]
        caller_dir = os.path.dirname(os.path.abspath(frame.filename))
        candidate = os.path.join(caller_dir, resolved)
        if os.path.exists(candidate):
            resolved = candidate

    if not os.path.exists(resolved):
        raise FileNotFoundError(f"Cannot find file: {path}")

    file_ns: dict = {
        "__builtins__": __builtins__,
        "play": delphi.play,
        "play_notes": delphi.play_notes,
        "export": delphi.export,
        "tempo": delphi.tempo,
        "key": delphi.key,
        "time_sig": delphi.time_sig,
        "swing": delphi.swing,
        "humanize": delphi.humanize,
        "note": delphi.note,
        "chord": delphi.chord,
        "scale": delphi.scale,
        "Song": delphi.Song,
        "Track": delphi.Track,
        "GM_INSTRUMENTS": delphi.GM_INSTRUMENTS,
        "Section": Section,
        "Pattern": Pattern,
        "Voice": Voice,
        "Arrangement": Arrangement,
        "PatternLibrary": PatternLibrary,
        "build_song_from_sections": build_song_from_sections,
        "register_pattern": register_pattern,
        "get_pattern": get_pattern,
        "list_patterns": list_patterns,
        "include": include,
    }

    with open(resolved) as f:
        code = f.read()

    exec(compile(code, resolved, "exec"), file_ns)

    # Merge into caller namespace if provided
    if namespace is not None:
        for k, v in file_ns.items():
            if not k.startswith("_"):
                namespace[k] = v

    return file_ns
