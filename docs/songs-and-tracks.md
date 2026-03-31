# Songs & Tracks

Build multi-track compositions with the `Song` and `Track` classes. Each track has its own instrument, notation, and effects chain.

## Table of Contents

- [Creating a Song](#creating-a-song)
- [Adding Tracks](#adding-tracks)
- [Track Effects](#track-effects)
- [Playback & Export](#playback--export)
- [Extracting Parts](#extracting-parts)
- [GM Instruments](#gm-instruments)

---

## Creating a Song

```python
from delphi import Song, Track

song = Song("My Song",
            tempo=120,          # BPM (default: 120)
            key="C major",      # Key signature (default: "C major")
            time_sig=(4, 4))    # Time signature (default: 4/4)
```

## Adding Tracks

There are two ways to add tracks:

### Shorthand (chain-friendly)

```python
song.track("melody",
           "C4:q E4:q G4:q C5:h",
           program="piano",       # Instrument name or MIDI program number
           velocity=90)           # Default velocity (0-127)

song.track("bass",
           "C2:h G2:h",
           program="acoustic bass",
           channel=None,          # Auto-assigned (drums get channel 9)
           velocity=65)
```

### Explicit Track objects

```python
melody = Track("Melody",
               notation="C4:q E4:q G4:q C5:h",
               program="piano",
               velocity=90)

song.add_track(melody)
```

Both `Song.track()` and `Song.add_track()` return the Song, so you can chain calls:

```python
song = (Song("Jazz Tune", tempo=140)
    .track("piano", "| Cmaj7 | Am7 | Dm7 | G7 |", program="piano")
    .track("bass", "C2:h A2:h D2:h G2:h", program="acoustic bass")
    .track("drums", "bd(3,8) sd(2,8) hh(5,8)", program=0, channel=9))
```

## Track Effects

Tracks support a chainable effects API. All effect methods return the Track for chaining.

### Volume & Panning

```python
track = Track("Lead", "C4:q E4:q G4:q", program="piano")

track.gain(0.8)       # Volume: 0.0 (silent) to 2.0 (double), default 1.0
track.pan(0.3)        # Stereo: 0.0 (left) to 1.0 (right), 0.5 = center
```

### Reverb & Delay

```python
track.reverb(0.4)               # Reverb amount: 0.0 to 1.0
track.delay(0.3, time=0.25)     # Delay amount + time in seconds
```

### ADSR Envelope

Shape the amplitude of each note with attack, decay, sustain level, and release time.

```python
track.adsr(
    attack=0.01,    # Attack time in seconds
    decay=0.1,      # Decay time in seconds
    sustain=0.8,    # Sustain level (0-1)
    release=0.3     # Release time in seconds
)
```

### Pitch Manipulation

```python
track.transpose(7)      # Shift all notes up 7 semitones (a 5th)
track.transpose(-12)    # Shift down one octave
track.octave_up()       # Shorthand for transpose(12)
track.octave_down()     # Shorthand for transpose(-12)
track.rev()             # Reverse the entire pattern
```

### Chaining Effects

All effect methods return the Track, so they can be chained:

```python
melody = (Track("Lead", "C4:q E4:q G4:h", program="piano")
    .gain(0.9)
    .pan(0.6)
    .reverb(0.3)
    .delay(0.2, time=0.125))

bass = (Track("Bass", "C2:h G2:h", program="acoustic bass")
    .gain(1.1)
    .pan(0.4)
    .octave_down())
```

### Effects in MIDI Export

When exporting to MIDI, the following effects are written as MIDI Control Change events:

| Effect | MIDI CC | Range |
|--------|---------|-------|
| `pan()` | CC #10 | 0-127 (0.5 → 64) |
| `gain()` | CC #7 | 0-127 (1.0 → 100) |
| `reverb()` | CC #91 | 0-127 |
| `delay()` | CC #93 | 0-127 |

## Playback & Export

Delphi uses **SoundFont playback by default** for all instruments and drums. The built-in oscillator synth is only used as a fallback when no SoundFont is available. Run `ensure_soundfont()` to download the default SoundFont.

Drum tracks are automatically routed to MIDI channel 9 — no manual channel assignment needed.

```python
# Play through speakers (SoundFont is default)
song.play()

# Render to WAV file
song.render("output.wav")

# Export MIDI file (Format 1, multi-track)
song.export("output.mid")
```

## Extracting Parts

Pull out a single track as its own Song:

```python
# Extract just the melody track
melody_song = song.extract("Melody")
melody_song.play()
melody_song.export("melody_only.mid")
```

## GM Instruments

Delphi supports all 128 General MIDI instruments. Use the string name (case-insensitive) or MIDI program number.

```python
Track("Lead", notation, program="piano")           # By name
Track("Lead", notation, program=0)                  # By number
Track("Drums", notation, program=0, channel=9)      # Drum channel
```

### Instrument List

| Category | Instruments |
|----------|-------------|
| **Piano** | `piano` (0), `bright piano` (1), `electric piano` / `epiano` (4), `harpsichord` (6), `clavinet` (7) |
| **Chromatic Percussion** | `glockenspiel` (9), `vibraphone` (11), `marimba` (12), `xylophone` (13), `tubular bells` (14) |
| **Organ** | `organ` (19), `church organ` (19), `reed organ` (20), `accordion` (21), `harmonica` (22) |
| **Guitar** | `nylon guitar` (24), `acoustic guitar` / `steel guitar` (25), `jazz guitar` (26), `clean guitar` / `electric guitar` (27), `muted guitar` (28), `overdriven guitar` (29), `distortion guitar` (30) |
| **Bass** | `acoustic bass` (32), `electric bass` / `finger bass` (33), `pick bass` (34), `fretless bass` (35), `slap bass` (36), `synth bass` (38) |
| **Strings** | `violin` (40), `viola` (41), `cello` (42), `contrabass` (43), `tremolo strings` (44), `pizzicato strings` (45), `harp` (46), `timpani` (47), `strings` / `string ensemble` (48), `slow strings` (49), `synth strings` (50) |
| **Choir** | `choir` (52), `voice` (53), `synth voice` (54), `orchestra hit` (55) |
| **Brass** | `trumpet` (56), `trombone` (57), `tuba` (58), `muted trumpet` (59), `french horn` / `horn` (60), `brass` (61), `synth brass` (62) |
| **Reeds** | `soprano sax` (64), `alto sax` (65), `tenor sax` (66), `baritone sax` (67), `oboe` (68), `english horn` (69), `bassoon` (70), `clarinet` (71) |
| **Flute** | `piccolo` (72), `flute` (73), `recorder` (74), `pan flute` (75), `whistle` (78), `ocarina` (79) |
| **Synth** | `square lead` (80), `saw lead` / `synth lead` (81), `pad` (88), `warm pad` (89), `polysynth` (90) |
| **World** | `sitar` (104), `banjo` (105), `shamisen` (106), `koto` (107), `kalimba` (108), `bagpipe` (109), `fiddle` (110) |

### In the REPL

Type `instruments` to see the full list:

```
🎵 > instruments
```

---

## Full Example

```python
from delphi import Song, Track, ensure_soundfont

ensure_soundfont()

song = Song("Chill Beat",
            tempo=90,
            key="D minor",
            time_sig=(4, 4))

# Rhodes piano — main chords
song.add_track(
    Track("Keys", """
        | Dm7 | Am7 | Bbmaj7 | A7 |
    """, program="electric piano", velocity=70)
    .pan(0.4)
    .reverb(0.5)
)

# Bass line
song.add_track(
    Track("Bass", """
        D2:q . D2:8 D2:8  A2:q . A2:8 A2:8
        Bb2:q . Bb2:8 Bb2:8  A2:q . A2:8 A2:8
    """, program="fretless bass", velocity=75)
    .gain(1.1)
    .pan(0.5)
)

# Drums — Euclidean rhythms
song.add_track(
    Track("Drums", """
        bd(3,8) sd(2,8) hh(5,8) oh(1,8)
    """, channel=9, velocity=80)
)

song.play()
song.export("chill_beat.mid")
```
