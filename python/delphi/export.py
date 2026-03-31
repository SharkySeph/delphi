"""Export interface -- write notation to MIDI, MusicXML (sheet music), and more."""

import xml.etree.ElementTree as _ET
from datetime import date as _date

from delphi.context import get_context
from delphi.notation import parse, events_to_tuples, TICKS_PER_QUARTER


def export(path: str, notation: str = "", events: list | None = None,
           track_name: str = "Delphi", program: int = 0) -> None:
    """
    Export notation or events to a file.

    Supports:
        export("out.mid", "| Cmaj7 | Am7 | Fmaj7 | G7 |")
        export("out.xml", "C4 E4 G4")
        export("out.mid", events=my_tuples)

    The format is inferred from the file extension.
    Supported: .mid, .midi, .xml, .musicxml
    """
    ctx = get_context()

    if not notation and events is None:
        raise ValueError("Provide either notation string or events list")

    if path.endswith((".mid", ".midi")):
        if events is not None:
            tuples = events
        else:
            tuples = events_to_tuples(parse(notation, default_velocity=80))
        if not tuples:
            print("(nothing to export)")
            return
        _export_midi(tuples, path, ctx, track_name, program)
    elif path.endswith((".xml", ".musicxml")):
        if notation:
            parsed = _parse_for_sheet(notation)
        elif events is not None:
            parsed = _tuples_to_events(events)
        else:
            parsed = []
        if not parsed:
            print("(nothing to export)")
            return
        _export_musicxml(path, parsed, ctx, track_name, program)
    else:
        raise ValueError(f"Unsupported export format: {path}")


def _export_midi(tuples, path, ctx, track_name, program):
    """Export to MIDI file."""
    try:
        from delphi._engine import export_midi
        export_midi(
            tuples,
            path,
            bpm=ctx.bpm,
            time_sig=(ctx.time_sig_num, ctx.time_sig_den),
            track_name=track_name,
            program=program,
        )
        print(f"Exported MIDI: {path}")
    except ImportError:
        _fallback_midi(tuples, path, ctx)


def _fallback_midi(tuples, path, ctx):
    """Pure-Python MIDI export fallback (minimal, no Rust needed)."""
    print(f"[Delphi] Rust engine not built. Writing minimal MIDI to {path}")
    # Write a bare-minimum MIDI file using pure Python
    ppq = 480
    data = bytearray()

    # Header
    data.extend(b"MThd")
    data.extend((6).to_bytes(4, "big"))
    data.extend((0).to_bytes(2, "big"))  # format 0
    data.extend((1).to_bytes(2, "big"))  # 1 track
    data.extend(ppq.to_bytes(2, "big"))

    # Track
    track_data = bytearray()

    # Tempo meta event
    uspqn = int(60_000_000 / ctx.bpm)
    track_data.extend(b"\x00\xff\x51\x03")
    track_data.extend(uspqn.to_bytes(3, "big"))

    # Sort events
    sorted_events = sorted(tuples, key=lambda t: t[2])

    # Build note-on / note-off
    raw = []
    for midi, vel, tick, dur in sorted_events:
        raw.append((tick, 0x90, midi, vel))
        raw.append((tick + dur, 0x80, midi, 0))
    raw.sort(key=lambda r: (r[0], r[1] & 0xF0))

    current_tick = 0
    for abs_tick, status, d1, d2 in raw:
        delta = abs_tick - current_tick
        _write_vlq(track_data, delta)
        track_data.extend([status, d1, d2])
        current_tick = abs_tick

    # End of track
    track_data.extend(b"\x00\xff\x2f\x00")

    data.extend(b"MTrk")
    data.extend(len(track_data).to_bytes(4, "big"))
    data.extend(track_data)

    with open(path, "wb") as f:
        f.write(data)
    print(f"Exported MIDI (fallback): {path}")


def _write_vlq(buf: bytearray, value: int):
    """Write a MIDI variable-length quantity."""
    if value == 0:
        buf.append(0)
        return
    bytes_list = []
    while value > 0:
        bytes_list.append(value & 0x7F)
        value >>= 7
    bytes_list.reverse()
    for i, b in enumerate(bytes_list):
        if i < len(bytes_list) - 1:
            buf.append(b | 0x80)
        else:
            buf.append(b)


# ── MusicXML sheet-music export ──────────────────────────────

# MusicXML uses "divisions" as the duration unit per quarter note.
# We pick 4 so that a 16th note = 1, eighth = 2, quarter = 4, half = 8, whole = 16.
_DIVISIONS = 4

# Map ticks → (MusicXML type, dots) for standard durations.
# Built from TICKS_PER_QUARTER = 480.
_TICK_TYPES: list[tuple[int, str, int]] = [
    (TICKS_PER_QUARTER * 8 + TICKS_PER_QUARTER * 4 + TICKS_PER_QUARTER * 2,
     "breve", 2),                             # breve double-dotted
    (TICKS_PER_QUARTER * 8 + TICKS_PER_QUARTER * 4,
     "breve", 1),                             # breve dotted
    (TICKS_PER_QUARTER * 8, "breve", 0),      # breve  (3840)
    (TICKS_PER_QUARTER * 4 + TICKS_PER_QUARTER * 2 + TICKS_PER_QUARTER,
     "whole", 2),                             # whole double-dotted
    (TICKS_PER_QUARTER * 4 + TICKS_PER_QUARTER * 2,
     "whole", 1),                             # whole dotted
    (TICKS_PER_QUARTER * 4, "whole", 0),      # whole  (1920)
    (TICKS_PER_QUARTER * 2 + TICKS_PER_QUARTER + TICKS_PER_QUARTER // 2,
     "half", 2),                              # half double-dotted
    (TICKS_PER_QUARTER * 2 + TICKS_PER_QUARTER,
     "half", 1),                              # half dotted
    (TICKS_PER_QUARTER * 2, "half", 0),       # half   (960)
    (TICKS_PER_QUARTER + TICKS_PER_QUARTER // 2 + TICKS_PER_QUARTER // 4,
     "quarter", 2),                           # quarter double-dotted
    (TICKS_PER_QUARTER + TICKS_PER_QUARTER // 2,
     "quarter", 1),                           # quarter dotted
    (TICKS_PER_QUARTER, "quarter", 0),        # quarter (480)
    (TICKS_PER_QUARTER // 2 + TICKS_PER_QUARTER // 4 + TICKS_PER_QUARTER // 8,
     "eighth", 2),                            # 8th double-dotted
    (TICKS_PER_QUARTER // 2 + TICKS_PER_QUARTER // 4,
     "eighth", 1),                            # 8th dotted
    (TICKS_PER_QUARTER // 2, "eighth", 0),    # 8th    (240)
    (TICKS_PER_QUARTER // 4 + TICKS_PER_QUARTER // 8 + TICKS_PER_QUARTER // 16,
     "16th", 2),                              # 16th double-dotted
    (TICKS_PER_QUARTER // 4 + TICKS_PER_QUARTER // 8,
     "16th", 1),                              # 16th dotted
    (TICKS_PER_QUARTER // 4, "16th", 0),      # 16th   (120)
    (TICKS_PER_QUARTER // 8 + TICKS_PER_QUARTER // 16,
     "32nd", 1),                              # 32nd dotted
    (TICKS_PER_QUARTER // 8, "32nd", 0),      # 32nd   (60)
    (TICKS_PER_QUARTER // 16, "64th", 0),     # 64th   (30)
    (TICKS_PER_QUARTER // 32, "128th", 0),    # 128th  (15)
]

# Pitch class → (step, alter) for MusicXML.  Index = midi % 12.
# Prefers sharps by default — caller can flip to flats via key context.
_SHARP_SPELL: list[tuple[str, int]] = [
    ("C", 0), ("C", 1), ("D", 0), ("D", 1), ("E", 0), ("F", 0),
    ("F", 1), ("G", 0), ("G", 1), ("A", 0), ("A", 1), ("B", 0),
]
_FLAT_SPELL: list[tuple[str, int]] = [
    ("C", 0), ("D", -1), ("D", 0), ("E", -1), ("E", 0), ("F", 0),
    ("G", -1), ("G", 0), ("A", -1), ("A", 0), ("B", -1), ("B", 0),
]

# Keys with flats in their signature
_FLAT_KEYS = frozenset({
    "F major", "Bb major", "Eb major", "Ab major", "Db major", "Gb major", "Cb major",
    "D minor", "G minor", "C minor", "F minor", "Bb minor", "Eb minor", "Ab minor",
    "F dorian", "Bb dorian", "Eb dorian", "Ab dorian",
    "F mixolydian", "Bb mixolydian", "Eb mixolydian",
})

# key name → MusicXML fifths value  (positive = sharps, negative = flats)
_KEY_FIFTHS: dict[str, int] = {
    "C major": 0, "G major": 1, "D major": 2, "A major": 3,
    "E major": 4, "B major": 5, "F# major": 6, "Gb major": -6,
    "Db major": -5, "Ab major": -4, "Eb major": -3, "Bb major": -2,
    "F major": -1,
    "A minor": 0, "E minor": 1, "B minor": 2, "F# minor": 3,
    "C# minor": 4, "G# minor": 5, "D# minor": 6, "Eb minor": -6,
    "Bb minor": -5, "F minor": -4, "C minor": -3, "G minor": -2,
    "D minor": -1,
}

# GM program → MusicXML part-name (just the common ones; fallback = "Piano")
_GM_PART_NAMES: dict[int, str] = {
    0: "Piano", 4: "Electric Piano", 6: "Harpsichord",
    24: "Guitar", 25: "Guitar", 26: "Guitar", 27: "Guitar",
    32: "Bass", 33: "Bass", 34: "Bass", 35: "Bass",
    40: "Violin", 41: "Viola", 42: "Cello", 43: "Contrabass",
    46: "Harp", 47: "Timpani", 48: "Strings",
    56: "Trumpet", 57: "Trombone", 58: "Tuba", 60: "Horn",
    64: "Soprano Sax", 65: "Alto Sax", 66: "Tenor Sax", 67: "Baritone Sax",
    68: "Oboe", 70: "Bassoon", 71: "Clarinet",
    72: "Piccolo", 73: "Flute", 74: "Recorder",
}

# Velocity → MusicXML dynamics text
_VEL_DYNAMICS: list[tuple[int, str]] = [
    (20, "ppp"), (35, "pp"), (50, "p"), (65, "mp"),
    (80, "mf"), (95, "f"), (112, "ff"), (127, "fff"),
]

# Delphi articulation → MusicXML element name(s)
_ART_MAP: dict[str, list[str]] = {
    "stac": ["staccato"], "staccato": ["staccato"],
    "stacc": ["staccato"], "staccatissimo": ["staccato"],
    "ten": ["tenuto"], "tenuto": ["tenuto"],
    "acc": ["accent"], "accent": ["accent"],
    "marc": ["strong-accent"], "marcato": ["strong-accent"],
    "ferm": ["fermata"], "fermata": ["fermata"],
    "port": ["tenuto", "staccato"], "portato": ["tenuto", "staccato"],
    "pizz": ["pizzicato"], "pizzicato": ["pizzicato"],
}

# Delphi ornament → MusicXML ornament element
_ORN_MAP: dict[str, str] = {
    "tr": "trill-mark", "trill": "trill-mark",
    "mord": "mordent", "mordent": "mordent",
    "lmord": "inverted-mordent",
    "turn": "turn", "gruppetto": "turn",
    "trem": "tremolo", "tremolo": "tremolo",
}


def _parse_for_sheet(notation: str):
    """Parse notation into Event objects *without* swing/humanize for clean sheet music."""
    from delphi.notation import parse as _parse
    # Parse with default_velocity so dynamics are preserved in the events
    return _parse(notation, default_velocity=80)


def _tuples_to_events(tuples: list) -> list:
    """Convert (midi, vel, tick, dur) tuples back to Event-like objects for MusicXML."""
    from delphi.notation import Event
    events = []
    for midi, vel, tick, dur in sorted(tuples, key=lambda t: t[2]):
        events.append(Event("note", [midi], vel, tick, dur))
    return events


def _vel_to_dynamic(vel: int) -> str:
    """Map MIDI velocity to a MusicXML dynamics element name."""
    for threshold, name in _VEL_DYNAMICS:
        if vel <= threshold:
            return name
    return "fff"


def _ticks_to_type(ticks: int) -> tuple[str, int]:
    """Find the best MusicXML note type and dot count for a tick duration.

    Returns (type_name, dots).  Falls back to 'quarter' if no exact match.
    """
    for t, name, dots in _TICK_TYPES:
        if ticks == t:
            return name, dots
    # No exact match — find closest
    best_name, best_dots, best_diff = "quarter", 0, abs(ticks - TICKS_PER_QUARTER)
    for t, name, dots in _TICK_TYPES:
        diff = abs(ticks - t)
        if diff < best_diff:
            best_name, best_dots, best_diff = name, dots, diff
    return best_name, best_dots


def _midi_to_pitch(midi: int, use_flat: bool) -> tuple[str, int, int]:
    """Convert MIDI note to (step, alter, octave) for MusicXML."""
    table = _FLAT_SPELL if use_flat else _SHARP_SPELL
    step, alter = table[midi % 12]
    octave = (midi // 12) - 1
    return step, alter, octave


def _sub(parent, tag, text=None, **attrs):
    """Shortcut: append a sub-element, optionally with text and attrs."""
    el = _ET.SubElement(parent, tag, **attrs)
    if text is not None:
        el.text = str(text)
    return el


def _export_musicxml(path: str, events: list, ctx, title: str = "Delphi",
                     program: int = 0) -> None:
    """Generate a single-part MusicXML file from parsed Events."""
    _write_musicxml(path, [(title, program, events)], ctx)
    print(f"Exported MusicXML: {path}")


def export_musicxml_song(song, path: str) -> None:
    """Generate a multi-part MusicXML file from a Song object."""
    from delphi.context import Context
    # Build a temporary context from the song's settings
    ctx = Context()
    ctx.bpm = song.tempo
    ctx.key_name = song.key
    ctx.time_sig_num = song.time_sig_num
    ctx.time_sig_den = song.time_sig_den

    parts: list[tuple[str, int, list]] = []
    for track in song.tracks:
        if not track.notation.strip():
            continue
        # Parse each track's notation cleanly (no swing/humanize)
        trk_events = _parse_for_sheet(track.notation)
        # Apply transpose if the track has it
        if track._transpose_semitones != 0:
            for evt in trk_events:
                evt.midi_notes = [
                    max(0, min(127, m + track._transpose_semitones))
                    for m in evt.midi_notes
                ]
        prog = track.program if isinstance(track.program, int) else 0
        parts.append((track.name, prog, trk_events))

    if not parts:
        print("(no tracks to export as sheet music)")
        return

    _write_musicxml(path, parts, ctx)
    print(f"Exported MusicXML: {path} ({len(parts)} parts)")


def _write_musicxml(path: str, parts: list[tuple[str, int, list]], ctx) -> None:
    """Core MusicXML writer.  *parts* = [(name, program, events), ...]."""

    use_flat = ctx.key_name.strip() in _FLAT_KEYS
    fifths = _KEY_FIFTHS.get(ctx.key_name.strip(), 0)
    mode = "minor" if "minor" in ctx.key_name.lower() else "major"
    ts_num = ctx.time_sig_num
    ts_den = ctx.time_sig_den
    measure_ticks = TICKS_PER_QUARTER * ts_num * (4 // ts_den) if ts_den else TICKS_PER_QUARTER * 4

    root = _ET.Element("score-partwise", version="4.0")

    # ── Identification ────────────────────────────────────
    ident = _sub(root, "identification")
    _sub(ident, "creator", "Delphi", type="software")
    encoding = _sub(ident, "encoding")
    _sub(encoding, "software", "Delphi Music Scripting Language")
    _sub(encoding, "encoding-date", _date.today().isoformat())

    # ── Part list ─────────────────────────────────────────
    part_list = _sub(root, "part-list")
    for i, (name, program, _evts) in enumerate(parts):
        pid = f"P{i + 1}"
        sp = _sub(part_list, "score-part", id=pid)
        display = name or _GM_PART_NAMES.get(program, "Piano")
        _sub(sp, "part-name", display)
        mi = _sub(sp, "midi-instrument", id=f"{pid}-I1")
        _sub(mi, "midi-channel", str(1 + i))
        _sub(mi, "midi-program", str(program + 1))  # MusicXML is 1-based

    # ── Parts ─────────────────────────────────────────────
    for i, (name, program, events) in enumerate(parts):
        pid = f"P{i + 1}"
        part_el = _sub(root, "part", id=pid)

        # Group events into measures by tick
        measures = _events_to_measures(events, measure_ticks, ts_num, ts_den)

        prev_dynamic = None
        for mnum, measure_events in enumerate(measures, 1):
            meas_el = _sub(part_el, "measure", number=str(mnum))

            if mnum == 1:
                # Attributes (key, time, divisions, clef)
                attrs = _sub(meas_el, "attributes")
                _sub(attrs, "divisions", str(_DIVISIONS))
                key_el = _sub(attrs, "key")
                _sub(key_el, "fifths", str(fifths))
                _sub(key_el, "mode", mode)
                time_el = _sub(attrs, "time")
                _sub(time_el, "beats", str(ts_num))
                _sub(time_el, "beat-type", str(ts_den))
                clef_el = _sub(attrs, "clef")
                # Auto-select clef based on first few pitches
                clef_sign, clef_line = _choose_clef(events)
                _sub(clef_el, "sign", clef_sign)
                _sub(clef_el, "line", str(clef_line))

                # Tempo direction
                direction = _sub(meas_el, "direction", placement="above")
                dt = _sub(direction, "direction-type")
                metro = _sub(dt, "metronome")
                _sub(metro, "beat-unit", "quarter")
                _sub(metro, "per-minute", str(int(ctx.bpm)))
                _sub(direction, "sound", tempo=str(int(ctx.bpm)))

            # Emit notes (and rests to fill gaps)
            meas_start = (mnum - 1) * measure_ticks
            cursor = meas_start

            for evt in measure_events:
                # Rest before this event?
                gap = evt.tick - cursor
                if gap > 0:
                    _emit_rests(meas_el, gap, use_flat)
                    cursor += gap

                # Dynamic change?
                dyn = _vel_to_dynamic(evt.velocity)
                if dyn != prev_dynamic and evt.kind != "rest":
                    _emit_dynamic(meas_el, dyn)
                    prev_dynamic = dyn

                if evt.kind == "rest":
                    _emit_rests(meas_el, evt.duration_ticks, use_flat)
                elif evt.kind in ("note", "drum"):
                    _emit_note(meas_el, evt, use_flat)
                elif evt.kind == "chord":
                    _emit_chord(meas_el, evt, use_flat)

                cursor = evt.tick + evt.duration_ticks

            # Fill remaining measure with rests
            remaining = meas_start + measure_ticks - cursor
            if remaining > 0:
                _emit_rests(meas_el, remaining, use_flat)

            # Barline at final measure
            if mnum == len(measures):
                bl = _sub(meas_el, "barline", location="right")
                _sub(bl, "bar-style", "light-heavy")

    # ── Write ─────────────────────────────────────────────
    tree = _ET.ElementTree(root)
    _ET.indent(tree, space="  ")
    with open(path, "wb") as f:
        f.write(b'<?xml version="1.0" encoding="UTF-8"?>\n')
        f.write(b'<!DOCTYPE score-partwise PUBLIC '
                b'"-//Recordare//DTD MusicXML 4.0 Partwise//EN" '
                b'"http://www.musicxml.com/dtds/partwise.dtd">\n')
        tree.write(f, xml_declaration=False, encoding="UTF-8")
    return


def _events_to_measures(events: list, measure_ticks: int,
                        ts_num: int, ts_den: int) -> list[list]:
    """Distribute events across measures.  Splits events that cross barlines."""
    if not events:
        return [[]]

    # Find total span
    max_tick = max(e.tick + e.duration_ticks for e in events)
    num_measures = max(1, -(-max_tick // measure_ticks))  # ceiling division

    from delphi.notation import Event
    measures: list[list] = [[] for _ in range(num_measures)]

    for evt in events:
        if evt.kind == "rest" and not evt.midi_notes:
            # place rest in its measure
            m_idx = min(evt.tick // measure_ticks, num_measures - 1)
            measures[m_idx].append(evt)
            continue

        remaining = evt.duration_ticks
        tick = evt.tick
        first = True
        while remaining > 0 and tick < num_measures * measure_ticks:
            m_idx = tick // measure_ticks
            if m_idx >= num_measures:
                break
            bar_end = (m_idx + 1) * measure_ticks
            space = bar_end - tick
            dur = min(remaining, space)

            split_evt = Event(
                evt.kind, list(evt.midi_notes), evt.velocity,
                tick, dur,
                articulation=evt.articulation if first else None,
                ornament=evt.ornament if first else None,
            )
            # Mark tied notes for cross-barline splits
            if remaining > space:
                split_evt._tie_start = True
            if not first:
                split_evt._tie_stop = True

            measures[m_idx].append(split_evt)
            tick += dur
            remaining -= dur
            first = False

    # Sort each measure by tick
    for m in measures:
        m.sort(key=lambda e: e.tick)

    return measures


def _choose_clef(events: list) -> tuple[str, int]:
    """Pick treble or bass clef based on average pitch."""
    pitches = []
    for e in events:
        if e.kind != "rest":
            pitches.extend(e.midi_notes)
    if not pitches:
        return "G", 2  # treble
    avg = sum(pitches) / len(pitches)
    if avg < 55:       # below G3
        return "F", 4  # bass clef
    return "G", 2      # treble clef


def _emit_note(parent, evt, use_flat: bool) -> None:
    """Emit a single <note> element for a note Event."""
    midi = evt.midi_notes[0] if evt.midi_notes else 60
    step, alter, octave = _midi_to_pitch(midi, use_flat)
    dur_ticks = evt.duration_ticks
    ntype, dots = _ticks_to_type(dur_ticks)
    xml_dur = max(1, round(dur_ticks * _DIVISIONS / TICKS_PER_QUARTER))

    note_el = _sub(parent, "note")
    pitch_el = _sub(note_el, "pitch")
    _sub(pitch_el, "step", step)
    if alter != 0:
        _sub(pitch_el, "alter", str(alter))
    _sub(pitch_el, "octave", str(octave))
    _sub(note_el, "duration", str(xml_dur))

    # Ties
    if getattr(evt, "_tie_start", False):
        _sub(note_el, "tie", type="start")
    if getattr(evt, "_tie_stop", False):
        _sub(note_el, "tie", type="stop")

    _sub(note_el, "type", ntype)
    for _ in range(dots):
        _sub(note_el, "dot")

    # Tie notations
    if getattr(evt, "_tie_start", False) or getattr(evt, "_tie_stop", False):
        notations = _sub(note_el, "notations")
        if getattr(evt, "_tie_stop", False):
            _sub(notations, "tied", type="stop")
        if getattr(evt, "_tie_start", False):
            _sub(notations, "tied", type="start")
    else:
        notations = None

    # Articulations and ornaments
    _emit_notations(note_el, evt, notations)


def _emit_chord(parent, evt, use_flat: bool) -> None:
    """Emit multiple <note> elements for a chord (first note normal, rest with <chord/>)."""
    if not evt.midi_notes:
        return
    dur_ticks = evt.duration_ticks
    ntype, dots = _ticks_to_type(dur_ticks)
    xml_dur = max(1, round(dur_ticks * _DIVISIONS / TICKS_PER_QUARTER))

    for idx, midi in enumerate(sorted(evt.midi_notes)):
        step, alter, octave = _midi_to_pitch(midi, use_flat)
        note_el = _sub(parent, "note")

        if idx > 0:
            _sub(note_el, "chord")

        pitch_el = _sub(note_el, "pitch")
        _sub(pitch_el, "step", step)
        if alter != 0:
            _sub(pitch_el, "alter", str(alter))
        _sub(pitch_el, "octave", str(octave))
        _sub(note_el, "duration", str(xml_dur))

        # Ties
        if getattr(evt, "_tie_start", False):
            _sub(note_el, "tie", type="start")
        if getattr(evt, "_tie_stop", False):
            _sub(note_el, "tie", type="stop")

        _sub(note_el, "type", ntype)
        for _ in range(dots):
            _sub(note_el, "dot")

        if getattr(evt, "_tie_start", False) or getattr(evt, "_tie_stop", False):
            notations = _sub(note_el, "notations")
            if getattr(evt, "_tie_stop", False):
                _sub(notations, "tied", type="stop")
            if getattr(evt, "_tie_start", False):
                _sub(notations, "tied", type="start")

        # Articulations only on first note of chord
        if idx == 0:
            _emit_notations(note_el, evt, None)


def _emit_rests(parent, ticks: int, use_flat: bool) -> None:
    """Emit one or more <note> rest elements to cover *ticks* duration."""
    remaining = ticks
    while remaining > 0:
        # Find largest fitting standard duration
        best_ticks = 0
        best_type = "quarter"
        best_dots = 0
        for t, name, dots in _TICK_TYPES:
            if t <= remaining and t > best_ticks:
                best_ticks = t
                best_type = name
                best_dots = dots
        if best_ticks == 0:
            # Smaller than our smallest unit — just use the smallest
            best_ticks = remaining
            best_type = "128th"
            best_dots = 0

        xml_dur = max(1, round(best_ticks * _DIVISIONS / TICKS_PER_QUARTER))
        note_el = _sub(parent, "note")
        _sub(note_el, "rest")
        _sub(note_el, "duration", str(xml_dur))
        _sub(note_el, "type", best_type)
        for _ in range(best_dots):
            _sub(note_el, "dot")

        remaining -= best_ticks


def _emit_dynamic(parent, dynamic: str) -> None:
    """Emit a MusicXML <direction> + <dynamics> element."""
    direction = _sub(parent, "direction", placement="below")
    dt = _sub(direction, "direction-type")
    dynamics_el = _sub(dt, "dynamics")
    _sub(dynamics_el, dynamic)


def _emit_notations(note_el, evt, notations) -> None:
    """Add articulation + ornament notation elements to a note."""
    art_els = []
    orn_el_name = None

    if evt.articulation and evt.articulation in _ART_MAP:
        art_els = _ART_MAP[evt.articulation]
    if evt.ornament and evt.ornament in _ORN_MAP:
        orn_el_name = _ORN_MAP[evt.ornament]

    if not art_els and not orn_el_name:
        return

    if notations is None:
        notations = _sub(note_el, "notations")

    if art_els:
        # fermata is a top-level notation, not inside <articulations>
        fermata = [a for a in art_els if a == "fermata"]
        regular = [a for a in art_els if a != "fermata"]
        for f in fermata:
            _sub(notations, f)
        if regular:
            artic = _sub(notations, "articulations")
            for a in regular:
                _sub(artic, a)

    if orn_el_name:
        ornaments = _sub(notations, "ornaments")
        _sub(ornaments, orn_el_name)


def sheet(notation: str, path: str = "sheet.musicxml") -> None:
    """Convenience: export notation as MusicXML and try to open it.

    Usage:
        sheet("C4 E4 G4")
        sheet("| Cmaj7 | Am7 | F | G |", "chords.xml")
    """
    export(path, notation)
    _try_open(path)


def _try_open(path: str) -> None:
    """Try to open a file with the system default application."""
    import subprocess
    import platform
    system = platform.system()
    try:
        if system == "Darwin":
            subprocess.Popen(["open", path])
        elif system == "Windows":
            import os
            os.startfile(path)
        else:
            subprocess.Popen(["xdg-open", path],
                             stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    except Exception:
        print(f"(could not auto-open {path} — open it in MuseScore or another score editor)")
