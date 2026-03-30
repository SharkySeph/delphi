# Composition Guide

For pieces with more structure than a single notation string — multiple sections, recurring patterns, and complex arrangements — Delphi provides a composition API.

## Table of Contents

- [Patterns](#patterns)
- [Voices](#voices)
- [Sections](#sections)
- [Building Songs from Sections](#building-songs-from-sections)
- [Arrangements](#arrangements)
- [Pattern Library](#pattern-library)
- [File Includes](#file-includes)

---

## Patterns

A `Pattern` is a named, reusable chunk of notation. Patterns can be transposed, reversed, and repeated.

```python
from delphi import Pattern

riff = Pattern("main_riff", "C4:8 E4:8 G4:8 C5:8")
riff.repeat(4)              # Play 4 times when used
riff.transpose(7)           # Shift up a perfect 5th

# Get parsed events
events = riff.get_events()

# Get total duration in ticks
dur = riff.duration_ticks()
```

### Pattern Methods

| Method | Returns | Description |
|--------|---------|-------------|
| `repeat(n)` | `Pattern` | Set repeat count |
| `transpose(semitones)` | `Pattern` | Shift all notes |
| `rev()` | `Pattern` | Reverse the pattern |
| `get_events(default_velocity=80)` | `list[Event]` | Parse and return events |
| `duration_ticks()` | `int` | Total duration including repeats |

## Voices

A `Voice` represents a single instrument part across the piece. It holds an ordered list of Patterns that play sequentially.

```python
from delphi import Voice

melody = Voice("melody", program="piano", velocity=90)
melody.add("C4:q D4:q E4:q F4:q")        # Add notation string
melody.add(Pattern("bridge", "G4:h E4:h")) # Add Pattern object
melody.add("C4:w")                          # More notation

# Get all note tuples (midi, velocity, tick, duration)
tuples = melody.get_tuples()
```

### Voice Constructor

```python
Voice(name: str,
      program: int | str = 0,      # GM instrument name or number
      velocity: int = 80,          # Default velocity
      channel: int | None = None)  # MIDI channel (auto-assigned if None)
```

## Sections

A `Section` is a named segment of a composition (verse, chorus, bridge, etc.) that contains multiple Voices playing simultaneously.

```python
from delphi import Section, Voice

# Method 1: Quick add (creates Voice internally)
verse = Section("verse")
verse.add("melody", "C4:q D4:q E4:q F4:q  G4:h E4:h", program="piano", velocity=90)
verse.add("bass", "C2:h G2:h  F2:h C2:h", program="acoustic bass", velocity=65)

# Method 2: Add existing Voice objects
chorus = Section("chorus")
lead = Voice("melody", program="piano", velocity=95)
lead.add("G4:q A4:q G4:q E4:q  C4:w")
chorus.add_voice(lead)

# Set repeat count
verse.repeat(2)  # Play the verse twice

# Get total duration
ticks = verse.duration_ticks()
```

### Section Methods

| Method | Returns | Description |
|--------|---------|-------------|
| `add(name, notation, program=0, velocity=80)` | `Section` | Create and add a Voice |
| `add_voice(voice)` | `Section` | Add an existing Voice |
| `repeat(n)` | `Section` | Set repeat count |
| `duration_ticks()` | `int` | Duration of one pass |

## Building Songs from Sections

The simplest way to turn Sections into a playable Song:

```python
from delphi import build_song_from_sections, Section

intro = Section("intro")
intro.add("keys", "| Cmaj7 | Fmaj7 |", program="electric piano")

verse = Section("verse")
verse.add("melody", "C4:q D4:q E4:q F4:q  G4:h E4:h", program="piano")
verse.add("bass", "C2:h G2:h  F2:h C2:h", program="acoustic bass")

chorus = Section("chorus")
chorus.add("melody", "G4:q A4:q G4:q E4:q  C4:w", program="piano")
chorus.add("bass", "C2:h F2:h  G2:h C2:h", program="acoustic bass")

song = build_song_from_sections(
    "My Song",
    sections=[
        intro,
        (verse, 2),      # Tuple = (section, repeat_count)
        chorus,
        verse,
        (chorus, 2),
    ],
    tempo=120,
    key="C major"
)

song.play()
```

## Arrangements

For more control over song structure, use the `Arrangement` class. It supports rehearsal marks, start-from playback, and part extraction.

```python
from delphi import Arrangement, Section

verse = Section("verse")
verse.add("melody", "C4:q D4:q E4:q F4:q", program="piano")
verse.add("bass", "C2:h G2:h", program="acoustic bass")

chorus = Section("chorus")
chorus.add("melody", "G4:q A4:q G4:h", program="piano")
chorus.add("bass", "C2:h F2:h", program="acoustic bass")

bridge = Section("bridge")
bridge.add("melody", "A4:q B4:q C5:h", program="piano")

# Build the arrangement
arr = Arrangement("Pop Song", tempo=120, key="C major")
arr.section(verse, repeat=2)
arr.mark("chorus")
arr.section(chorus)
arr.section(verse)
arr.mark("bridge")
arr.section(bridge)
arr.section(chorus, repeat=2)

# View the timeline
arr.show()
#   Pop Song (120 BPM, C major)
#
#    1. verse ×2  [melody, bass]
#   ── chorus ──
#    2. chorus  [melody, bass]
#    3. verse  [melody, bass]
#   ── bridge ──
#    4. bridge  [melody, bass]
#    5. chorus ×2  [melody, bass]

# Build to a playable Song
song = arr.build()
song.play()

# Start from a rehearsal mark
song = arr.build(start_from="bridge")

# Extract a single voice
melody_song = arr.extract("melody")
melody_song.play()
```

### Arrangement with `timeline()`

Add multiple items at once with `timeline()`:

```python
arr = Arrangement("My Song", tempo=110)
arr.timeline(
    verse,
    verse,
    "chorus",       # String = rehearsal mark
    chorus,
    verse,
    "bridge",
    bridge,
    chorus,
    chorus,
)
```

### Arrangement Methods

| Method | Returns | Description |
|--------|---------|-------------|
| `section(sec, repeat=1)` | `Arrangement` | Add a section |
| `mark(name)` | `Arrangement` | Insert a rehearsal mark |
| `timeline(*items)` | `Arrangement` | Add sections and marks in sequence |
| `build(start_from=None)` | `Song` | Build a playable Song |
| `extract(voice_name)` | `Song` | Extract a single voice as a Song |
| `marks()` | `list[str]` | List all rehearsal marks |
| `show()` | `None` | Print the timeline |

## Pattern Library

Store and retrieve reusable patterns by name with the global Pattern Library.

```python
from delphi import register_pattern, get_pattern, list_patterns

# Register patterns
register_pattern("verse_riff", "C4:8 E4:8 G4:8 C5:8  B4:8 G4:8 E4:8 C4:8")
register_pattern("chorus_riff", "G4:q A4:q G4:q E4:q")
register_pattern("bass_line", "C2:h G2:h  F2:h G2:h")

# List all registered patterns
print(list_patterns())
# ['verse_riff', 'chorus_riff', 'bass_line']

# Retrieve and use a pattern
riff = get_pattern("verse_riff")

# Use in a Voice
melody = Voice("melody", program="piano")
melody.add(get_pattern("verse_riff"))
melody.add(get_pattern("chorus_riff"))
```

### Local Pattern Libraries

Create isolated libraries for different projects:

```python
from delphi import PatternLibrary

lib = PatternLibrary()
lib.register("motif_a", "C4 E4 G4")
lib.register("motif_b", "D4 F4 A4")

print("motif_a" in lib)  # True
print(lib.names())        # ['motif_a', 'motif_b']

pattern = lib.get("motif_a")
lib.remove("motif_b")
```

## File Includes

Split large compositions across multiple files with `include()`.

### `themes.py`

```python
from delphi import Pattern

main_theme = Pattern("main", "C4:q E4:q G4:q C5:h")
bass_theme = Pattern("bass", "C2:h G2:h F2:h C2:h")
```

### `song.py`

```python
from delphi import include, Section, Arrangement

# Import from another file (path is relative to the calling file)
themes = include("themes.py")

verse = Section("verse")
v = Voice("melody", program="piano")
v.add(themes["main_theme"])
verse.add_voice(v)

arr = Arrangement("My Song", tempo=120)
arr.section(verse, repeat=4)
song = arr.build()
song.play()
```

`include()` executes the target file and returns its namespace as a dictionary. Paths are resolved relative to the calling file's directory.

You can also merge into the current namespace:

```python
ns = {}
include("themes.py", namespace=ns)
# ns now contains main_theme and bass_theme
```

---

## Full Example: Song with Everything

```python
from delphi import *

ensure_soundfont()

# Register reusable patterns
register_pattern("intro_keys", "| Cmaj7 | Am7 | Fmaj7 | G7 |")
register_pattern("verse_melody", """
    C4:q D4:q E4:q G4:q  A4:q G4:q E4:q D4:q
    C4:q E4:q G4:q C5:q  B4:h A4:h
""")
register_pattern("chorus_melody", """
    G4:q A4:q B4:q C5:q  D5:h B4:h
    C5:q B4:q A4:q G4:q  E4:w
""")
register_pattern("bass_verse", "C2:h G2:h  A2:h E2:h  F2:h C2:h  G2:h G2:h")
register_pattern("bass_chorus", "C2:h F2:h  G2:h E2:h  F2:h G2:h  C2:w")

# Build sections
intro = Section("intro")
intro.add("keys", get_pattern("intro_keys").notation, program="electric piano")

verse = Section("verse")
verse.add("melody", get_pattern("verse_melody").notation, program="piano", velocity=85)
verse.add("bass", get_pattern("bass_verse").notation, program="acoustic bass", velocity=70)

chorus = Section("chorus")
chorus.add("melody", get_pattern("chorus_melody").notation, program="piano", velocity=95)
chorus.add("bass", get_pattern("bass_chorus").notation, program="acoustic bass", velocity=75)

# Arrange the song
arr = Arrangement("Patterns Demo", tempo=115, key="C major")
arr.timeline(
    intro,
    "verse 1",
    verse,
    verse,
    "chorus",
    chorus,
    "verse 2",
    verse,
    "chorus (out)",
    chorus,
    chorus,
)

arr.show()

song = arr.build()
song.play()
song.export("patterns_demo.mid")
```
