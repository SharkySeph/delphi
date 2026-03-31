"""
Mini-notation parser for Delphi.

Parses strings like:
    "C4 E4 G4"              -> individual notes (sequential)
    "| Cmaj7 | Am7 | G7 |"  -> chord progression (one chord per bar)
    "C4:q E4:8 G4:h"        -> notes with explicit durations
    "C4 . E4 ."             -> notes and rests
    "[C4 E4 G4]"            -> subdivision (all in one beat)
    "C4:h~C4:q"             -> tied notes (combined duration)
    "(3 C4 E4 G4)"          -> tuplet (3 items in the space of 2)
    "C4:q*4"                -> repeat shorthand
    "<C4 E4 G4>"            -> slow sequence (one per cycle/repetition)
    "C4!2"                  -> replicate without speedup
    "C4@2"                  -> elongation (hold 2x weight)
    "C4?"                   -> 50% random removal
    "C4?0.3"                -> 30% random removal
    "C4|E4|G4"              -> random choice
    "bd(3,8)"               -> Euclidean rhythm
    "C4.stac"               -> staccato articulation
    "C4.acc"                -> accent articulation
    "C4.tr"                 -> trill ornament
    "C4,E4,G4"              -> simultaneous polyphony

Returns a list of Event dicts suitable for passing to the engine.
"""

import re
import random
from typing import Optional

# Ticks per quarter note (must match Rust Duration::TICKS_PER_QUARTER)
TICKS_PER_QUARTER = 480

# Note name pattern: C, C#, Db, Ebb, F##, etc.
# Captures: (pitch)(accidental)(octave)(duration)(dynamic)(articulation)
NOTE_RE = re.compile(
    r'^([A-Ga-g])(##?|bb?)?(-?\d+)(?::(\w+\.?\.?))?(?:!(\w+))?(?:\.(\w+))?$'
)

# Chord pattern: root + optional accidental + quality + optional octave
CHORD_RE = re.compile(
    r'^([A-Ga-g])(##?|bb?)?(maj7|maj9|maj11|maj13|maj|m7b5|mMaj7|mM7|min7|min9|min11|min6|min|m69|m6|m7|m9|m11|m13|m|dim7|dim|aug7|aug|7sus4|7sus2|sus2|sus4|sus|add9|add11|add2|69|6|7|9|11|13|5|alt|\+|°7|°|ø7|ø)?(-?\d+)?$'
)

# Bass note for slash chords: letter + optional accidental + optional octave
BASS_NOTE_RE = re.compile(
    r'^([A-Ga-g])(##?|bb?)?(-?\d+)?$'
)

# Euclidean rhythm pattern: token(beats,steps) or token(beats,steps,offset)
EUCLIDEAN_RE = re.compile(
    r'^(.+?)\((\d+),(\d+)(?:,(\d+))?\)$'
)

# Crescendo/decrescendo pattern: cresc(p,f,4) or dim(f,p,4)
CRESC_RE = re.compile(
    r'^(cresc|crescendo|dim|diminuendo|decresc|decrescendo)\((\w+),(\w+),(\d+)\)$',
    re.IGNORECASE,
)

# Drum hit names
DRUM_MAP = {
    "kick": 36, "bd": 36,
    "snare": 38, "sd": 38,
    "hihat": 42, "hh": 42,
    "openhat": 46, "oh": 46,
    "closehat": 42, "ch": 42,
    "ride": 51, "rd": 51,
    "crash": 49, "cr": 49,
    "tom1": 50, "tom2": 47, "tom3": 45,
    "clap": 39, "cp": 39,
    "rimshot": 37, "rim": 37,
    "cowbell": 56, "cb": 56,
    "tambourine": 54, "tamb": 54,
    "shaker": 70,
    "clave": 75,
    "woodblock": 76, "wb": 76,
    "triangle": 81, "tri": 81,
    "guiro": 73,
    "cabasa": 69,
    "maracas": 70,
    "pedal": 44,
}

# Duration suffixes
DURATION_MAP = {
    "dw": TICKS_PER_QUARTER * 8,     # double whole / breve
    "breve": TICKS_PER_QUARTER * 8,
    "w": TICKS_PER_QUARTER * 4,
    "whole": TICKS_PER_QUARTER * 4,
    "h": TICKS_PER_QUARTER * 2,
    "half": TICKS_PER_QUARTER * 2,
    "q": TICKS_PER_QUARTER,
    "quarter": TICKS_PER_QUARTER,
    "8": TICKS_PER_QUARTER // 2,
    "eighth": TICKS_PER_QUARTER // 2,
    "8t": TICKS_PER_QUARTER // 3,     # eighth-note triplet
    "qt": (TICKS_PER_QUARTER * 2) // 3,  # quarter-note triplet
    "16t": TICKS_PER_QUARTER // 6,    # sixteenth-note triplet
    "16": TICKS_PER_QUARTER // 4,
    "sixteenth": TICKS_PER_QUARTER // 4,
    "32": TICKS_PER_QUARTER // 8,
    "64": TICKS_PER_QUARTER // 16,
    "128": TICKS_PER_QUARTER // 32,
}

# Add dotted and double-dotted versions (skip triplet durations — dotted triplets are nonsensical)
for k, v in list(DURATION_MAP.items()):
    if len(k) <= 3 and not k.endswith("t"):
        DURATION_MAP[k + "."] = v + v // 2        # dotted
        DURATION_MAP[k + ".."] = v + v // 2 + v // 4  # double-dotted

# Articulation map: suffix → (velocity_scale, duration_scale)
# velocity_scale multiplies velocity, duration_scale multiplies duration
ARTICULATION_MAP = {
    "stac": (1.0, 0.5),         # staccato: half duration
    "staccato": (1.0, 0.5),
    "stacc": (1.0, 0.25),       # staccatissimo: quarter duration
    "staccatissimo": (1.0, 0.25),
    "ten": (1.0, 1.1),          # tenuto: slightly held
    "tenuto": (1.0, 1.1),
    "port": (1.0, 0.75),         # portato / mezzo-staccato: between legato and staccato
    "portato": (1.0, 0.75),
    "acc": (1.3, 1.0),          # accent: louder
    "accent": (1.3, 1.0),
    "marc": (1.4, 0.85),        # marcato: loud + slightly short
    "marcato": (1.4, 0.85),
    "ferm": (1.0, 2.0),         # fermata: double duration
    "fermata": (1.0, 2.0),
    "ghost": (0.4, 0.8),        # ghost note: very quiet
    "leg": (1.0, 1.05),         # legato: slightly overlapping
    "legato": (1.0, 1.05),
    "pizz": (1.0, 0.3),         # pizzicato: very short
    "pizzicato": (1.0, 0.3),
    "mute": (0.6, 0.5),         # muted: quieter + shorter
}

# Ornament map: suffix → ornament type
# These generate extra notes around the main note
ORNAMENT_MAP = {
    "tr": "trill",
    "trill": "trill",
    "mord": "mordent",
    "mordent": "mordent",
    "lmord": "lower_mordent",
    "turn": "turn",
    "gruppetto": "turn",
    "grace": "acciaccatura",
    "acciaccatura": "acciaccatura",
    "appoggiatura": "appoggiatura",
    "trem": "tremolo",
    "tremolo": "tremolo",
    "gliss": "glissando",
    "glissando": "glissando",
    "arp": "arpeggio",
    "arpeggio": "arpeggio",
    "roll": "roll",
}

# Expanded dynamics
DYNAMICS_MAP = {
    "ppp": 16, "pp": 33, "p": 49,
    "mp": 64, "mf": 80,
    "f": 96, "ff": 112, "fff": 127,
    "sfz": 120, "sfp": 110, "fp": 100,
    "rfz": 115, "fz": 110,
}


class Event:
    """A parsed musical event."""
    __slots__ = ("kind", "midi_notes", "velocity", "tick", "duration_ticks",
                 "articulation", "ornament")

    def __init__(self, kind: str, midi_notes: list[int], velocity: int,
                 tick: int, duration_ticks: int,
                 articulation: Optional[str] = None,
                 ornament: Optional[str] = None):
        self.kind = kind  # "note", "chord", "drum", "rest"
        self.midi_notes = midi_notes
        self.velocity = velocity
        self.tick = tick
        self.duration_ticks = duration_ticks
        self.articulation = articulation
        self.ornament = ornament

    def __repr__(self):
        parts = (f"Event({self.kind}, midi={self.midi_notes}, vel={self.velocity}, "
                 f"tick={self.tick}, dur={self.duration_ticks}")
        if self.articulation:
            parts += f", art={self.articulation}"
        if self.ornament:
            parts += f", orn={self.ornament}"
        return parts + ")"

    def to_tuples(self) -> list[tuple[int, int, int, int]]:
        """Convert to list of (midi_note, velocity, tick, duration_ticks) tuples.
        Expands ornaments into multiple note events."""
        if self.ornament:
            return self._expand_ornament()
        return [
            (n, self.velocity, self.tick, self.duration_ticks)
            for n in self.midi_notes
        ]

    def _expand_ornament(self) -> list[tuple[int, int, int, int]]:
        """Expand ornaments into concrete note tuples."""
        if not self.midi_notes:
            return []
        note = self.midi_notes[0]
        vel = self.velocity
        tick = self.tick
        dur = self.duration_ticks
        ornament_dur = max(dur // 8, TICKS_PER_QUARTER // 8)

        if self.ornament == "trill":
            # Alternate between note and note+2 (whole step)
            tuples = []
            t = tick
            upper = note + 2
            toggle = False
            while t < tick + dur:
                n = upper if toggle else note
                d = min(ornament_dur, tick + dur - t)
                tuples.append((n, vel, t, d))
                t += ornament_dur
                toggle = not toggle
            return tuples

        elif self.ornament == "mordent":
            # Main → upper → main
            third = dur // 3
            return [
                (note, vel, tick, third),
                (note + 2, vel, tick + third, third),
                (note, vel, tick + 2 * third, dur - 2 * third),
            ]

        elif self.ornament == "lower_mordent":
            third = dur // 3
            return [
                (note, vel, tick, third),
                (note - 1, vel, tick + third, third),
                (note, vel, tick + 2 * third, dur - 2 * third),
            ]

        elif self.ornament == "turn":
            # upper → note → lower → note
            quarter = dur // 4
            return [
                (note + 2, vel, tick, quarter),
                (note, vel, tick + quarter, quarter),
                (note - 1, vel, tick + 2 * quarter, quarter),
                (note, vel, tick + 3 * quarter, dur - 3 * quarter),
            ]

        elif self.ornament == "acciaccatura":
            # Quick grace note before main note
            grace_dur = min(ornament_dur, dur // 4)
            return [
                (note - 1, vel, tick, grace_dur),
                (note, vel, tick + grace_dur, dur - grace_dur),
            ]

        elif self.ornament == "appoggiatura":
            # Grace note takes half the main note's duration
            half = dur // 2
            return [
                (note + 2, vel, tick, half),
                (note, vel, tick + half, dur - half),
            ]

        elif self.ornament == "tremolo":
            # Rapid repeated notes
            tuples = []
            t = tick
            while t < tick + dur:
                d = min(ornament_dur, tick + dur - t)
                tuples.append((note, vel, t, d))
                t += ornament_dur
            return tuples

        elif self.ornament == "glissando":
            # Slide from note to note+7 (fifth) over duration
            steps = 7
            step_dur = dur // steps
            return [
                (note + i, vel, tick + i * step_dur,
                 step_dur if i < steps - 1 else dur - i * step_dur)
                for i in range(steps)
            ]

        elif self.ornament == "arpeggio":
            # Roll chord notes if multiple, else arpeggiate up triad
            notes = self.midi_notes if len(self.midi_notes) > 1 else [note, note + 4, note + 7]
            step_dur = dur // len(notes)
            return [
                (n, vel, tick + i * step_dur,
                 step_dur if i < len(notes) - 1 else dur - i * step_dur)
                for i, n in enumerate(notes)
            ]

        elif self.ornament == "roll":
            # Drum roll: rapid hits
            tuples = []
            t = tick
            while t < tick + dur:
                d = min(ornament_dur, tick + dur - t)
                tuples.append((note, vel, t, d))
                t += ornament_dur
            return tuples

        # Fallback: just the note
        return [(n, vel, tick, dur) for n in self.midi_notes]


# ── Euclidean rhythm generator ────────────────────────────────

def euclidean(beats: int, steps: int, offset: int = 0) -> list[bool]:
    """Generate a Euclidean rhythm pattern.
    Returns a list of bools: True = hit, False = rest.
    E.g. euclidean(3, 8) -> [True, False, False, True, False, False, True, False]
    """
    if steps <= 0:
        return []
    if beats >= steps:
        return [True] * steps

    pattern = []
    bucket = 0
    for i in range(steps):
        bucket += beats
        if bucket >= steps:
            bucket -= steps
            pattern.append(True)
        else:
            pattern.append(False)

    # Apply offset (rotation)
    if offset:
        offset = offset % steps
        pattern = pattern[offset:] + pattern[:offset]

    return pattern


# ── Main parse entry point ────────────────────────────────────

def parse(notation: str, default_duration: Optional[int] = None,
          default_velocity: int = 80) -> list[Event]:
    """
    Parse a notation string into a list of Events.

    Supports:
    - Individual notes: "C4 E4 G4"
    - Bar notation: "| Cmaj7 | Am7 | G7 |"
    - Rests: "." or "~" or "r"
    - Drum hits: "kick snare hihat"
    - Duration suffixes: "C4:q", "E4:8", "C4:64", "C4:q.."
    - Dynamic suffixes: "C4!ff", "C4!sfz"
    - Subdivisions: "[C4 E4 G4]"
    - Ties: "C4:h~C4:q"
    - Tuplets: "(3 C4 E4 G4)"
    - Repeats: "C4:q*4"
    - Slow sequences: "<C4 E4 G4>" (one per cycle)
    - Replication: "C4!2" (repeat without speedup, if numeric)
    - Elongation: "C4@2" (hold 2x weight)
    - Random removal: "C4?" (50%) or "C4?0.3" (30%)
    - Random choice: "C4|E4|G4"
    - Euclidean rhythms: "bd(3,8)" or "bd(3,8,2)"
    - Polyphony: "C4,E4,G4" (simultaneous)
    - Articulations: "C4.stac", "C4.acc", "C4.marc", "C4.ferm"
    - Ornaments: "C4.tr", "C4.mord", "C4.turn", "C4.grace", "C4.trem"
    - Triplet durations: "C4:8t", "C4:qt", "C4:16t"
    - Breve/double-whole: "C4:dw"
    - Portato: "C4.port"
    - Structural: "DC" (da capo), "DS" (dal segno), "segno", "coda", "fine"
    - Volta: "[1" ... "[2" (1st/2nd endings)
    - Breath/caesura: "breath" or "caesura" (inserts a small pause)
    - Crescendo/decrescendo: "cresc(p,f,4)" or "dim(f,p,4)" over N beats
    - Swing/humanize: applied from global context (swing(), humanize())
    """
    notation = notation.strip()
    if not notation:
        return []

    # Handle structural repeats (D.C., D.S., segno, coda, fine)
    events = _handle_structure(notation, default_duration, default_velocity)

    # Apply swing from global context
    from delphi.context import get_context
    ctx = get_context()
    if ctx.swing > 0:
        events = _apply_swing(events, ctx.swing)
    if ctx.humanize > 0:
        events = _apply_humanize(events, ctx.humanize)

    return events


def _handle_structure(notation: str, default_duration: Optional[int],
                      default_velocity: int) -> list[Event]:
    """Handle structural markers: D.C., D.S., segno, coda, fine, volta."""
    # Check for D.C. (Da Capo) — repeat from beginning
    upper = notation.upper().strip()

    # Simple D.C. al fine: play, then repeat from start to 'fine'
    if " DC" in upper or " D.C." in upper or notation.strip().endswith("DC"):
        parts = re.split(r'\bDC\b|\bD\.C\.\b', notation, flags=re.IGNORECASE)
        main = parts[0].strip()
        events = _parse_inner(main, default_duration, default_velocity)
        # Find 'fine' marker if present
        fine_text = _find_until_marker(main, "fine")
        if fine_text:
            events += _parse_inner(fine_text, default_duration, default_velocity,
                                   tick_offset=_events_end_tick(events))
        else:
            # Repeat entire piece
            offset = _events_end_tick(events)
            repeat_events = _parse_inner(main, default_duration, default_velocity,
                                         tick_offset=offset)
            events += repeat_events
        return events

    # D.S. (Dal Segno) — repeat from segno marker
    if " DS" in upper or " D.S." in upper or notation.strip().endswith("DS"):
        parts = re.split(r'\bDS\b|\bD\.S\.\b', notation, flags=re.IGNORECASE)
        main = parts[0].strip()
        events = _parse_inner(main, default_duration, default_velocity)
        # Find segno marker and replay from there
        segno_text = _find_from_marker(main, "segno")
        if segno_text:
            offset = _events_end_tick(events)
            events += _parse_inner(segno_text, default_duration, default_velocity,
                                   tick_offset=offset)
        return events

    # Volta brackets: | ... [1 ... | [2 ... |
    # Play through once with 1st ending, then repeat with 2nd ending
    if "[1" in notation and "[2" in notation:
        return _parse_volta(notation, default_duration, default_velocity)

    # No structural markers — standard parse
    return _parse_inner(notation, default_duration, default_velocity)


def _find_until_marker(notation: str, marker: str) -> Optional[str]:
    """Return notation text from beginning up to (but not including) a marker."""
    idx = notation.lower().find(marker.lower())
    if idx >= 0:
        return notation[:idx].strip()
    return None


def _find_from_marker(notation: str, marker: str) -> Optional[str]:
    """Return notation text from after a marker to the end."""
    idx = notation.lower().find(marker.lower())
    if idx >= 0:
        after = notation[idx + len(marker):].strip()
        return after if after else None
    return None


def _events_end_tick(events: list[Event]) -> int:
    """Get the tick position at the end of the last event."""
    if not events:
        return 0
    return max(e.tick + e.duration_ticks for e in events)


def _parse_volta(notation: str, default_duration: Optional[int],
                 default_velocity: int) -> list[Event]:
    """Parse notation with volta brackets (1st/2nd endings).

    Syntax: main_body [1 first_ending | [2 second_ending |

    The main body is played twice:
      - First time through with the [1 ending
      - Second time through with the [2 ending
    """
    # Split on volta markers
    parts = re.split(r'\[1\b', notation, maxsplit=1)
    main_body = parts[0].strip()
    rest = parts[1] if len(parts) > 1 else ""

    volta_parts = re.split(r'\[2\b', rest, maxsplit=1)
    first_ending = volta_parts[0].strip()
    second_ending = volta_parts[1].strip() if len(volta_parts) > 1 else ""

    # Clean up trailing/leading pipes
    for s in (main_body, first_ending, second_ending):
        s = s.strip().strip("|").strip()

    events = []

    # Pass 1: main body + first ending
    pass1 = f"{main_body} {first_ending}".strip()
    if pass1:
        events = _parse_inner(pass1, default_duration, default_velocity)

    # Pass 2: main body + second ending
    pass2 = f"{main_body} {second_ending}".strip()
    if pass2:
        offset = _events_end_tick(events)
        events += _parse_inner(pass2, default_duration, default_velocity,
                               tick_offset=offset)

    return events


def _parse_inner(notation: str, default_duration: Optional[int],
                 default_velocity: int, tick_offset: int = 0) -> list[Event]:
    """Core parse dispatcher — bars vs sequence."""
    # Filter out structural marker tokens
    notation = re.sub(r'\b(segno|coda|fine)\b', '', notation, flags=re.IGNORECASE).strip()
    if not notation:
        return []

    if "|" in notation and not _looks_like_random_choice(notation):
        events = _parse_bars(notation, default_velocity)
    else:
        events = _parse_sequence(notation, default_duration, default_velocity)

    if tick_offset:
        for e in events:
            e.tick += tick_offset

    return events


def _apply_swing(events: list[Event], amount: float) -> list[Event]:
    """Apply swing feel by delaying every other eighth note.

    amount: 0.0=straight, 0.5=triplet swing, 1.0=hard swing.
    The offbeat eighth notes (those on the "and" of each beat) are
    delayed proportionally.
    """
    eighth = TICKS_PER_QUARTER // 2
    max_shift = TICKS_PER_QUARTER // 3  # max swing delay

    for evt in events:
        # Check if this event is on an offbeat eighth-note position
        beat_pos = evt.tick % TICKS_PER_QUARTER
        if beat_pos == eighth:
            # This is an offbeat eighth — delay it
            shift = int(max_shift * amount)
            evt.tick += shift
            evt.duration_ticks = max(1, evt.duration_ticks - shift)
    return events


def _apply_humanize(events: list[Event], amount: float) -> list[Event]:
    """Add slight randomization to timing and velocity for a human feel."""
    max_tick_shift = int(TICKS_PER_QUARTER * 0.04 * amount)  # up to ~4% of a beat
    max_vel_shift = int(12 * amount)  # up to ±12 velocity

    for evt in events:
        if evt.kind == "rest":
            continue
        # Timing jitter
        if max_tick_shift > 0:
            shift = random.randint(-max_tick_shift, max_tick_shift)
            evt.tick = max(0, evt.tick + shift)
        # Velocity jitter
        if max_vel_shift > 0:
            vshift = random.randint(-max_vel_shift, max_vel_shift)
            evt.velocity = max(1, min(127, evt.velocity + vshift))
    return events


def _looks_like_random_choice(notation: str) -> bool:
    """Distinguish bar notation '| C | Am |' from random choice 'C4|E4|G4'."""
    stripped = notation.strip()
    # Bar notation starts/ends with | or has spaces around |
    if stripped.startswith("|") or stripped.endswith("|"):
        return False
    # If there's " | " with spaces, it's bars
    if " | " in stripped:
        return False
    # Otherwise if | appears without spaces it's random choice
    return True


def _parse_bars(notation: str, default_velocity: int) -> list[Event]:
    """Parse bar-delimited notation like '| Cmaj7 | Am7 | G7 |'."""
    bars = [b.strip() for b in notation.split("|") if b.strip()]

    from delphi.context import get_context
    ctx = get_context()
    measure_ticks = TICKS_PER_QUARTER * ctx.time_sig_num * (4 // ctx.time_sig_den)
    events = []
    current_tick = 0

    for bar_str in bars:
        tokens = bar_str.split()
        if not tokens:
            current_tick += measure_ticks
            continue

        ticks_per_token = measure_ticks // len(tokens)

        for token in tokens:
            evt = _parse_token(token, current_tick, ticks_per_token, default_velocity,
                               prefer_chords=True)
            if evt is not None:
                events.append(evt)
            current_tick += ticks_per_token

    return events


def _parse_sequence(notation: str, default_duration: Optional[int],
                    default_velocity: int) -> list[Event]:
    """Parse a space-separated sequence of notes/chords with extended syntax."""
    if default_duration is None:
        default_duration = TICKS_PER_QUARTER

    tokens = _tokenize(notation)
    events = []
    current_tick = 0
    _pending_ramps = []

    i = 0
    while i < len(tokens):
        token = tokens[i]

        # ── Slow sequence: <C4 E4 G4> ─────────────────
        # Returns only the first element; the cycle index is handled at a
        # higher level. For now, parse all and return the first.
        if token.startswith("<") and token.endswith(">"):
            inner = token[1:-1].strip()
            inner_tokens = inner.split()
            if inner_tokens:
                # Pick first by default (cycle-aware playback selects by index)
                chosen = inner_tokens[0]
                evt = _parse_token(chosen, current_tick, default_duration, default_velocity)
                dur = evt.duration_ticks if evt else default_duration
                if evt is not None:
                    events.append(evt)
                current_tick += dur
            i += 1
            continue

        # ── Tuplet groups: (3 C4 E4 G4) ───────────────
        if token.startswith("(") and len(token) > 1 and token[1:].isdigit():
            n = int(token[1:])
            group_tokens = []
            i += 1
            while i < len(tokens) and len(group_tokens) < n:
                t = tokens[i]
                if t.endswith(")"):
                    group_tokens.append(t[:-1] if len(t) > 1 else t)
                    i += 1
                    break
                group_tokens.append(t)
                i += 1
            total_ticks = default_duration * (n - 1) if n > 1 else default_duration
            tuplet_dur = total_ticks // max(len(group_tokens), 1)
            for gt in group_tokens:
                if gt and gt != ")":
                    evt = _parse_token(gt, current_tick, tuplet_dur, default_velocity)
                    if evt is not None:
                        events.append(evt)
                    current_tick += tuplet_dur
            continue

        # ── Subdivisions: [C4 E4 G4] ──────────────────
        if token.startswith("[") and token.endswith("]"):
            inner = token[1:-1].strip()
            sub_tokens = inner.split()
            sub_dur = default_duration // max(len(sub_tokens), 1)
            for st in sub_tokens:
                evt = _parse_token(st, current_tick, sub_dur, default_velocity)
                if evt is not None:
                    events.append(evt)
                current_tick += sub_dur
            i += 1
            continue

        # ── Layer group: {bd(3,8) sd(2,8) hh(5,8)} ───
        # All patterns inside braces start at the same tick.
        # current_tick advances by the longest sub-pattern.
        if token.startswith("{") and token.endswith("}"):
            inner = token[1:-1].strip()
            layer_tokens = _tokenize(inner)
            layer_start = current_tick
            max_end = current_tick
            for lt in layer_tokens:
                # Parse each sub-token starting at the same tick
                sub_tick = layer_start
                lem = EUCLIDEAN_RE.match(lt)
                if lem:
                    base_tok = lem.group(1)
                    lbeats = int(lem.group(2))
                    lsteps = int(lem.group(3))
                    loffset = int(lem.group(4)) if lem.group(4) else 0
                    pat = euclidean(lbeats, lsteps, loffset)
                    step_d = default_duration
                    for hit in pat:
                        if hit:
                            evt = _parse_token(base_tok, sub_tick, step_d, default_velocity)
                            if evt is not None:
                                events.append(evt)
                        sub_tick += step_d
                else:
                    evt = _parse_token(lt, sub_tick, default_duration, default_velocity)
                    if evt is not None:
                        events.append(evt)
                    sub_tick += evt.duration_ticks if evt else default_duration
                max_end = max(max_end, sub_tick)
            current_tick = max_end
            i += 1
            continue

        # ── Euclidean rhythm: bd(3,8) or bd(3,8,2) ────
        em = EUCLIDEAN_RE.match(token)
        if em:
            base_token = em.group(1)
            beats = int(em.group(2))
            steps = int(em.group(3))
            offset = int(em.group(4)) if em.group(4) else 0
            pattern = euclidean(beats, steps, offset)
            step_dur = default_duration
            for hit in pattern:
                if hit:
                    evt = _parse_token(base_token, current_tick, step_dur, default_velocity)
                    if evt is not None:
                        events.append(evt)
                else:
                    events.append(Event("rest", [], 0, current_tick, step_dur))
                current_tick += step_dur
            i += 1
            continue

        # ── Random choice: C4|E4|G4 ───────────────────
        if "|" in token:
            choices = [c.strip() for c in token.split("|") if c.strip()]
            if choices:
                chosen = random.choice(choices)
                evt = _parse_token(chosen, current_tick, default_duration, default_velocity)
                dur = evt.duration_ticks if evt else default_duration
                if evt is not None:
                    events.append(evt)
                current_tick += dur
            i += 1
            continue

        # ── Polyphony/simultaneous: C4,E4,G4 ──────────
        if "," in token and not token.startswith(("(", "<")) and not CRESC_RE.match(token):
            parts = [p.strip() for p in token.split(",") if p.strip()]
            max_dur = default_duration
            for part in parts:
                evt = _parse_token(part, current_tick, default_duration, default_velocity)
                if evt is not None:
                    events.append(evt)
                    max_dur = max(max_dur, evt.duration_ticks)
            current_tick += max_dur
            i += 1
            continue

        # ── Random removal: C4? or C4?0.3 ─────────────
        if "?" in token and token != "?":
            qparts = token.split("?")
            base_token = qparts[0]
            try:
                prob = float(qparts[1]) if qparts[1] else 0.5
            except ValueError:
                prob = 0.5
            if random.random() < prob:
                # Remove — emit a rest instead
                events.append(Event("rest", [], 0, current_tick, default_duration))
            else:
                evt = _parse_token(base_token, current_tick, default_duration, default_velocity)
                if evt is not None:
                    events.append(evt)
            current_tick += default_duration
            i += 1
            continue

        # ── Elongation: C4@2 ──────────────────────────
        if "@" in token:
            parts = token.split("@")
            base_token = parts[0]
            try:
                weight = float(parts[1])
            except (ValueError, IndexError):
                weight = 1.0
            elongated_dur = int(default_duration * weight)
            evt = _parse_token(base_token, current_tick, elongated_dur, default_velocity)
            if evt is not None:
                evt.duration_ticks = elongated_dur
                events.append(evt)
            current_tick += elongated_dur
            i += 1
            continue

        # ── Repeat/multiply: C4:q*4 ───────────────────
        if "*" in token:
            parts = token.rsplit("*", 1)
            base_token = parts[0]
            try:
                repeat_count = int(parts[1])
            except ValueError:
                repeat_count = 1
            for _ in range(repeat_count):
                evt = _parse_token(base_token, current_tick, default_duration, default_velocity)
                dur = evt.duration_ticks if evt else default_duration
                if evt is not None:
                    events.append(evt)
                current_tick += dur
            i += 1
            continue

        # ── Ties: C4:h~C4:q ───────────────────────────
        if "~" in token and not token.startswith(("~", ".")):
            tie_parts = token.split("~")
            first_evt = _parse_token(tie_parts[0], current_tick, default_duration, default_velocity)
            if first_evt is not None:
                total_dur = first_evt.duration_ticks
                for tpart in tie_parts[1:]:
                    sub_evt = _parse_token(tpart, 0, default_duration, default_velocity)
                    if sub_evt is not None:
                        total_dur += sub_evt.duration_ticks
                first_evt.duration_ticks = total_dur
                events.append(first_evt)
                current_tick += total_dur
            i += 1
            continue

        # ── Breath / caesura: insert a small rest ──────
        if token.lower() in ("breath", "caesura"):
            pause = TICKS_PER_QUARTER // 4 if token.lower() == "breath" else TICKS_PER_QUARTER // 2
            events.append(Event("rest", [], 0, current_tick, pause))
            current_tick += pause
            i += 1
            continue

        # ── Crescendo / decrescendo over N beats ───────
        cm = CRESC_RE.match(token)
        if cm:
            direction = cm.group(1).lower()
            start_dyn = DYNAMICS_MAP.get(cm.group(2).lower(), default_velocity)
            end_dyn = DYNAMICS_MAP.get(cm.group(3).lower(), default_velocity)
            beat_count = int(cm.group(4))
            # Apply as a velocity ramp to the next N note events
            events.append(Event("rest", [], 0, current_tick, 0,
                                articulation=None, ornament=None))
            # Store ramp info to apply after parsing
            _pending_ramps.append({
                "start_idx": len(events),
                "beats": beat_count,
                "start_vel": start_dyn,
                "end_vel": end_dyn,
            })
            i += 1
            continue

        # ── Regular token ─────────────────────────────
        evt = _parse_token(token, current_tick, default_duration, default_velocity)
        dur = evt.duration_ticks if evt else default_duration
        if evt is not None:
            events.append(evt)
        current_tick += dur
        i += 1

    # ── Apply pending velocity ramps (crescendo/decrescendo) ──
    for ramp in _pending_ramps:
        start_idx = ramp["start_idx"]
        beats = ramp["beats"]
        start_vel = ramp["start_vel"]
        end_vel = ramp["end_vel"]
        # Find the next N note events after the marker
        note_events = [e for e in events[start_idx:] if e.kind != "rest"]
        count = min(beats, len(note_events))
        for j, evt in enumerate(note_events[:count]):
            if count > 1:
                frac = j / (count - 1)
            else:
                frac = 1.0
            evt.velocity = max(1, min(127, int(start_vel + (end_vel - start_vel) * frac)))

    return events


def _tokenize(notation: str) -> list[str]:
    """Tokenize notation, keeping bracket/angle/brace groups and parens together."""
    tokens = []
    i = 0
    chars = notation.strip()

    while i < len(chars):
        if chars[i] == "{":
            # Find matching } (layer group)
            depth = 1
            j = i + 1
            while j < len(chars) and depth > 0:
                if chars[j] == "{":
                    depth += 1
                elif chars[j] == "}":
                    depth -= 1
                j += 1
            tokens.append(chars[i:j])
            i = j
        elif chars[i] == "[":
            # Find matching ]
            depth = 1
            j = i + 1
            while j < len(chars) and depth > 0:
                if chars[j] == "[":
                    depth += 1
                elif chars[j] == "]":
                    depth -= 1
                j += 1
            tokens.append(chars[i:j])
            i = j
        elif chars[i] == "<":
            # Find matching > (slow sequence)
            depth = 1
            j = i + 1
            while j < len(chars) and depth > 0:
                if chars[j] == "<":
                    depth += 1
                elif chars[j] == ">":
                    depth -= 1
                j += 1
            tokens.append(chars[i:j])
            i = j
        elif chars[i] == "(":
            # Check if it's Euclidean: previous token + (digits,digits)
            # Or tuplet: (digit followed by space/notes)
            j = i + 1
            while j < len(chars) and chars[j].isdigit():
                j += 1
            if j < len(chars) and chars[j] == ",":
                # Euclidean rhythm — attach to previous token
                k = j + 1
                while k < len(chars) and chars[k] != ")":
                    k += 1
                if k < len(chars):
                    k += 1  # include )
                eucl_part = chars[i:k]
                if tokens:
                    tokens[-1] = tokens[-1] + eucl_part
                else:
                    tokens.append(eucl_part)
                i = k
            else:
                # Tuplet start: emit "(" + digit as one token
                tokens.append(chars[i:j])
                i = j
        elif chars[i] == ")":
            # Tuplet end
            if tokens:
                tokens[-1] = tokens[-1] + ")"
            i += 1
        elif chars[i].isspace():
            i += 1
        else:
            j = i
            while j < len(chars) and not chars[j].isspace() and chars[j] not in "[<":
                if chars[j] == "(":
                    # Include through matching ) for function-like tokens (cresc, dim)
                    k = j + 1
                    while k < len(chars) and chars[k] != ")":
                        k += 1
                    if k < len(chars):
                        k += 1  # include )
                    j = k
                    break
                if chars[j] == ")":
                    j += 1
                    break
                j += 1
            tokens.append(chars[i:j])
            i = j

    return tokens


def _parse_token(token: str, tick: int, default_duration: int,
                 default_velocity: int,
                 prefer_chords: bool = False) -> Optional[Event]:
    """Parse a single token into an Event."""
    token = token.strip()
    if not token:
        return None

    # Rest
    if token in (".", "~", "r", "rest", "_"):
        return Event("rest", [], 0, tick, default_duration)

    # Check for articulation/ornament suffix: note.stac, note.tr
    articulation = None
    ornament = None
    artic_token = token
    if "." in token:
        # Split on last dot that could be an articulation
        dot_idx = token.rfind(".")
        suffix = token[dot_idx + 1:]
        if suffix in ARTICULATION_MAP:
            articulation = suffix
            artic_token = token[:dot_idx]
        elif suffix in ORNAMENT_MAP:
            ornament = ORNAMENT_MAP[suffix]
            artic_token = token[:dot_idx]

    # Drum hit
    lower = artic_token.lower().split("!")[0]
    if lower in DRUM_MAP:
        vel = default_velocity
        if "!" in artic_token:
            vel = _parse_dynamic(artic_token.split("!")[1]) or default_velocity
        if articulation:
            vs, ds = ARTICULATION_MAP[articulation]
            vel = min(127, int(vel * vs))
            dur = int(default_duration * ds)
        else:
            dur = default_duration
        return Event("drum", [DRUM_MAP[lower]], vel, tick, dur,
                     articulation=articulation, ornament=ornament)

    # In bar notation context, try chord before note
    if prefer_chords:
        chord_evt = _try_parse_chord(artic_token, tick, default_duration, default_velocity)
        if chord_evt is not None:
            if articulation:
                _apply_articulation(chord_evt, articulation)
            if ornament:
                chord_evt.ornament = ornament
            return chord_evt

    # Try as a note
    m = NOTE_RE.match(artic_token)
    if m:
        pitch = m.group(1).upper()
        acc = m.group(2) or ""
        octave = int(m.group(3))
        dur_suffix = m.group(4)
        dyn_suffix = m.group(5)
        note_artic = m.group(6)

        # Articulation from regex match overrides dot-split
        if note_artic:
            if note_artic in ARTICULATION_MAP:
                articulation = note_artic
            elif note_artic in ORNAMENT_MAP:
                ornament = ORNAMENT_MAP[note_artic]

        midi = _note_to_midi(pitch, acc, octave)
        dur = DURATION_MAP.get(dur_suffix, default_duration) if dur_suffix else default_duration
        vel = _parse_dynamic(dyn_suffix) if dyn_suffix else default_velocity

        if articulation:
            vs, ds = ARTICULATION_MAP[articulation]
            vel = min(127, int(vel * vs))
            dur = int(dur * ds)

        return Event("note", [midi], vel, tick, dur,
                     articulation=articulation, ornament=ornament)

    # Try as a chord (fallback for non-bar context)
    chord_evt = _try_parse_chord(artic_token, tick, default_duration, default_velocity)
    if chord_evt is not None:
        if articulation:
            _apply_articulation(chord_evt, articulation)
        if ornament:
            chord_evt.ornament = ornament
        return chord_evt

    return None


def _apply_articulation(evt: Event, articulation: str) -> None:
    """Apply articulation modifiers to an event in-place."""
    if articulation in ARTICULATION_MAP:
        vs, ds = ARTICULATION_MAP[articulation]
        evt.velocity = min(127, int(evt.velocity * vs))
        evt.duration_ticks = int(evt.duration_ticks * ds)
        evt.articulation = articulation


def _try_parse_chord(token: str, tick: int, default_duration: int,
                     default_velocity: int) -> Optional[Event]:
    """Try to parse a token as a chord, returning Event or None."""
    # Strip duration/dynamic suffixes before matching
    chord_str = token.split("!")[0].split(":")[0]

    # Check for slash chord (bass note): Am/E, C/G, Am4/E3
    bass_midi = None
    if "/" in chord_str:
        chord_str, bass_str = chord_str.split("/", 1)

    m = CHORD_RE.match(chord_str)
    if not m:
        return None

    root = m.group(1).upper()
    acc = m.group(2) or ""
    quality = m.group(3) or ""
    octave = int(m.group(4)) if m.group(4) else 4  # default octave 4

    root_midi = _note_to_midi(root, acc, octave)
    intervals = _chord_intervals(quality)
    midi_notes = [(root_midi + iv) for iv in intervals]

    # Parse bass note for slash chords
    if "/" in token.split("!")[0].split(":")[0]:
        bm = BASS_NOTE_RE.match(bass_str)
        if bm:
            bass_pitch = bm.group(1).upper()
            bass_acc = bm.group(2) or ""
            bass_oct = int(bm.group(3)) if bm.group(3) else octave - 1
            bass_midi = _note_to_midi(bass_pitch, bass_acc, bass_oct)
            midi_notes.insert(0, bass_midi)

    vel = default_velocity
    if "!" in token:
        vel = _parse_dynamic(token.split("!")[1]) or default_velocity

    dur = default_duration
    if ":" in token:
        dur_part = token.split(":")[1].split("!")[0]
        dur = DURATION_MAP.get(dur_part, default_duration)

    return Event("chord", midi_notes, vel, tick, dur)


def _note_to_midi(pitch: str, accidental: str, octave: int) -> int:
    """Convert note name + accidental + octave to MIDI number."""
    pitch_map = {"C": 0, "D": 2, "E": 4, "F": 5, "G": 7, "A": 9, "B": 11}
    base = pitch_map.get(pitch.upper(), 0)

    acc_offset = 0
    if accidental == "#":
        acc_offset = 1
    elif accidental == "##":
        acc_offset = 2
    elif accidental == "b":
        acc_offset = -1
    elif accidental == "bb":
        acc_offset = -2

    return (octave + 1) * 12 + base + acc_offset


def _chord_intervals(quality: str) -> list[int]:
    """Return semitone intervals from root for a chord quality string."""
    mapping = {
        "": [0, 4, 7],            # major
        "maj": [0, 4, 7],
        "m": [0, 3, 7],           # minor
        "min": [0, 3, 7],
        "dim": [0, 3, 6],
        "aug": [0, 4, 8],
        "+": [0, 4, 8],
        "maj7": [0, 4, 7, 11],
        "maj9": [0, 4, 7, 11, 14],
        "maj11": [0, 4, 7, 11, 14, 17],
        "maj13": [0, 4, 7, 11, 14, 21],
        "m7": [0, 3, 7, 10],
        "min7": [0, 3, 7, 10],
        "m9": [0, 3, 7, 10, 14],
        "min9": [0, 3, 7, 10, 14],
        "m11": [0, 3, 7, 10, 14, 17],
        "min11": [0, 3, 7, 10, 14, 17],
        "m13": [0, 3, 7, 10, 14, 21],
        "7": [0, 4, 7, 10],       # dominant 7
        "9": [0, 4, 7, 10, 14],   # dominant 9
        "11": [0, 4, 7, 10, 14, 17],  # dominant 11
        "13": [0, 4, 7, 10, 14, 21],  # dominant 13
        "6": [0, 4, 7, 9],        # major 6th
        "dim7": [0, 3, 6, 9],
        "m7b5": [0, 3, 6, 10],    # half-diminished
        "mMaj7": [0, 3, 7, 11],
        "mM7": [0, 3, 7, 11],
        "aug7": [0, 4, 8, 10],
        "sus2": [0, 2, 7],
        "sus4": [0, 5, 7],
        "sus": [0, 5, 7],
        "add9": [0, 4, 7, 14],
        "add11": [0, 4, 7, 17],
        "add2": [0, 2, 4, 7],     # add2
        "m6": [0, 3, 7, 9],       # minor 6th
        "min6": [0, 3, 7, 9],
        "69": [0, 4, 7, 9, 14],    # 6/9 chord
        "m69": [0, 3, 7, 9, 14],  # minor 6/9
        "7sus4": [0, 5, 7, 10],   # dominant 7 sus4
        "7sus2": [0, 2, 7, 10],   # dominant 7 sus2
        "alt": [0, 4, 8, 10, 13], # altered dominant (b5 b9)
        "5": [0, 7],              # power chord
    }
    return mapping.get(quality, [0, 4, 7])


def _parse_dynamic(s: str) -> Optional[int]:
    """Parse a dynamic marking to a MIDI velocity."""
    return DYNAMICS_MAP.get(s.lower())


def events_to_tuples(events: list[Event]) -> list[tuple[int, int, int, int]]:
    """Flatten events to (midi_note, velocity, tick, duration_ticks) tuples."""
    tuples = []
    for evt in events:
        if evt.kind != "rest":
            tuples.extend(evt.to_tuples())
    return tuples
