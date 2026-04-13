# Delphi Studio

Delphi Studio is a native desktop application for composing multi-track music. It combines a notebook-style editor with a piano roll, mixer, real-time visualizer, theory explorer, and SoundFont-powered playback.

## Launching

```bash
delphi-studio                       # From the command line
```

Or find **Delphi Studio** in your desktop application menu.

Open files from the command line:

```bash
delphi-studio my-song.dstudio       # Open an existing project
```

A fresh project starts with a single empty notation cell.

---

## Cell Types

Studio has three cell types.

### Notation Cells `♪`

Pure music notation — each notation cell becomes a separate **Track** when you play.

Use **pragmas** at the top of a notation cell to set track metadata:

```
# @instrument flute
# @velocity 90
# @channel 2
# @track Melody

C4:q E4:q G4:q C5:h
```

| Pragma | Effect |
|--------|--------|
| `@instrument <name>` | Set the GM instrument |
| `@program <name>` | Alias for `@instrument` |
| `@track <label>` | Name the track |
| `@velocity <0-127>` | Default velocity for the cell |
| `@channel <0-15>` | MIDI channel |
| `@key <key>` | Set or change the key (enables Roman numerals & scale degrees) |

### Code Cells `⌨`

Directives that configure the project. Supported directives:

```
tempo(120)
key("D minor")
time_sig(4, 4)
swing(0.3)
humanize(0.1)
```

Setting a key enables Roman numeral chords (`I`, `IV`, `V7`) and scale degree notes (`^1`, `^5`) in notation cells. You can also change the key mid-cell with `# @key G major`.

### Markdown Cells `¶`

Freeform text for notes, section headers, or lyrics.

---

## Key Bindings

### Execution

| Key | Action |
|-----|--------|
| **F5** | Play all cells |
| **F6** | Play current cell |
| **Esc** | Stop playback |

### Navigation & Editing

| Key | Action |
|-----|--------|
| **Ctrl+Space** | Auto-complete |
| **Ctrl+S** | Save project |
| **Ctrl+O** | Open file |
| **Ctrl+N** | New project |

---

## Built-in Panels

### Piano Roll

Visual note display synced to your notation. Notes are drawn as colored bars showing pitch, timing, and duration.

### Mixer

Per-track controls:
- **Gain** — Volume (0.0 to 1.5)
- **Pan** — Stereo position (0.0 left, 0.5 center, 1.0 right)
- **Reverb** — Reverb send (0.0 to 1.0)
- **Delay** — Delay send (0.0 to 1.0)
- **Mute / Solo** — Quick track isolation

### Visualizer

Real-time playback animation with note bars and token highlighting.

### Theory Explorer

Interactive reference for scales, chords, and intervals. Explore any scale type or chord quality.

### SoundFont Manager

Load and switch SoundFont banks. Auto-downloads GeneralUser GS on first launch.

### Script Engine

Rhai scripting for automation tasks within the Studio.

---

## The Multi-Track Workflow

### 1. Set Up

Create a code cell at the top for global settings:

```
tempo(110)
key("D minor")
time_sig(4, 4)
```

### 2. Write Parts

Add a notation cell for each instrument. Use pragmas to label and assign instruments:

**Cell: Piano**
```
# @instrument piano
# @track Piano

| Dm | Am/E | Bb | A7 |
| Dm | Am/E | Bb | A7/C# |
```

**Cell: Bass**
```
# @instrument acoustic bass
# @track Bass
# @velocity 100

D2:h A2:h Bb2:h A2:h
D2:h A2:h Bb2:h A2:h
```

**Cell: Strings**
```
# @instrument strings
# @track Strings
# @velocity 70

D4,F4,A4:w D4,F4,A4:w
D4,F4,A4:w C#4,E4,A4:w
```

### 3. Play

Press **F5** to play all cells together — each notation cell becomes a track with its own instrument and MIDI channel.

### 4. Export

Use **File → Export** to export:
- `.mid` — MIDI file with each track on its own channel
- `.wav` — Rendered audio through the active SoundFont

Or use the CLI:

```bash
delphi export my-song.dstudio --format both --output ./out/
```

---

## File Format

Studio projects are saved as `.dstudio` files — JSON with this structure:

```json
{
  "settings": {
    "title": "My Song",
    "bpm": 120.0,
    "key_name": "C major",
    "time_sig_num": 4,
    "time_sig_den": 4,
    "swing": 0.0,
    "humanize": 0.0,
    "soundfont_path": null
  },
  "cells": [
    {
      "cell_type": "notation",
      "source": "C4:q E4:q G4:q C5:h",
      "output": "",
      "label": "Melody",
      "instrument": "piano",
      "channel": 0,
      "velocity": 80,
      "collapsed": false
    }
  ],
  "tracks": [
    {
      "name": "Melody",
      "instrument": "piano",
      "program": 0,
      "channel": 0,
      "gain": 1.0,
      "pan": 0.5,
      "muted": false,
      "solo": false,
      "reverb": 0.0,
      "delay": 0.0
    }
  ]
}
```

Delphi Studio also reads the legacy Python `.dstudio` format and plain `.delphi` notation files.

---

## Auto-Completion

Press **Ctrl+Space** to trigger auto-complete suggestions:

- Note names (`C4`, `F#5`, `Bb3`)
- Chord symbols (`Cmaj7`, `Am7`, `Dm`)
- Drum hits (`kick`, `snare`, `hihat`)
- Duration suffixes (`:q`, `:h`, `:8`)
- Dynamic markings (`!f`, `!pp`, `!mf`)
- GM instrument names
- Articulations (`.stac`, `.acc`, `.ferm`)
- Ornaments (`.tr`, `.mord`, `.grace`)
