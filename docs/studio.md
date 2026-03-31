# Delphi Studio

Delphi Studio is a terminal notebook IDE for composing multi-track songs. It combines the interactivity of the REPL with the structure of a multi-cell notebook — each cell can hold notation, Python code, or markdown notes.

## Launching

```bash
delphi studio                       # New empty notebook
delphi studio my-song               # Open or create "my-song" project
delphi studio song.dstudio          # Open an existing .dstudio file
```

When you pass a name, Studio checks (in order):
1. A `.dstudio` file at that exact path
2. A project directory containing `delphi.toml`
3. A project under `~/.local/share/delphi/projects/`
4. Otherwise, creates a new empty notebook with that title

A fresh notebook starts with a single **Setup** code cell pre-filled with `tempo()`, `key()`, and `time_sig()`.

---

## Cell Types

Studio has three cell types. Press **Ctrl+T** to cycle between them on the current cell.

### Code Cells `⌨`

Run arbitrary Python. All REPL functions are available — `play()`, `tempo()`, `key()`, `instrument()`, `Song`, `Track`, and more.

Code cells also auto-detect notation: if a line isn't valid Python but looks like music notation, it's played automatically. This means you can mix freely:

```
instrument("violin")
tempo(100)
C4 E4 G4 C5
```

### Notation Cells `♪`

Pure music notation — no Python needed. Each notation cell becomes a separate **Track** when you Run All (F6).

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

### Markdown Cells `¶`

Freeform text for notes, section headers, or lyrics. These don't execute — they're exported as comments in `.delphi` scripts.

---

## Key Bindings

### Execution

| Key | Action |
|-----|--------|
| **F5** | Run the focused cell |
| **F6** | Run All — build Song from all cells and play |
| **Ctrl+P** | Replay last successfully executed cell |
| **Ctrl+C** | Stop playback |

### Navigation

| Key | Action |
|-----|--------|
| **Ctrl+↑** | Focus previous cell |
| **Ctrl+↓** | Focus next cell |
| **Ctrl+Shift+↑** | Move cell up |
| **Ctrl+Shift+↓** | Move cell down |

### Editing

| Key | Action |
|-----|--------|
| **F7** / **Ctrl+B** | Insert new cell below (inherits current cell type) |
| **F8** | Delete cell (press twice to confirm) |
| **Ctrl+T** | Cycle cell type: code → notation → markdown |
| **Ctrl+E** | Collapse / expand cell |
| **Tab** | Auto-complete |

### File Operations

| Key | Action |
|-----|--------|
| **F9** | Export (MIDI + WAV + .delphi script) |
| **F10** / **Ctrl+S** | Save notebook |
| **F1** | Show all keybindings |
| **Ctrl+Q** | Quit (press twice if unsaved changes) |

---

## Cell Run States

Each cell tracks its execution state, shown as an icon in the cell header:

| Icon | State | Meaning |
|------|-------|---------|
| `○` | Empty | Cell hasn't been run yet |
| `✓` | OK | Last run succeeded (green) |
| `✗` | Error | Last run failed (red) |
| `◐` | Stale | Source was edited since last run (yellow) |

---

## Status Bar

The status bar at the top shows live engine state:

```
 my-song ●  ♩120  C major  🎹 Piano  [3/8]
```

| Element | Meaning |
|---------|---------|
| Title | Notebook name |
| `●` | Unsaved changes (hidden when clean) |
| `♩120` | Current tempo |
| `C major` | Current key |
| `🎹 Piano` | Active instrument |
| `[3/8]` | Focused cell / total cells |

---

## The Multi-Track Workflow

Studio's real power is building multi-track songs from notation cells.

### 1. Set Up

Create a code cell at the top for global settings:

```python
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

### 3. Run All (F6)

Press **F6** to build a `Song` from all notation cells — each becomes a `Track`. Code cells execute in order (setting tempo, key, etc.) and the song plays with all parts layered together.

### 4. Export (F9)

Press **F9** to export:
- `.mid` — MIDI file with each track on its own channel
- `.wav` — Rendered audio through the active SoundFont
- `.delphi` — A standalone Python script that recreates the song

---

## File Format

Studio notebooks are saved as `.dstudio` files — JSON with this structure:

```json
{
  "version": 1,
  "title": "my-song",
  "settings": {
    "tempo": 120,
    "key": "C major",
    "time_sig": "4/4"
  },
  "cells": [
    {
      "id": 1,
      "cell_type": "code",
      "source": "tempo(120)\nkey(\"C major\")",
      "output": "",
      "meta": {}
    }
  ]
}
```

Metadata from pragmas (`@instrument`, `@track`, etc.) is stored in each cell's `meta` field.

---

## Auto-Completion

Studio shares the REPL's auto-complete system. Press **Tab** to complete:

- Note names (`C4`, `F#5`, `Bb3`)
- Chord symbols (`Cmaj7`, `Am7`, `Dm`)
- Drum hits (`kick`, `snare`, `hihat`)
- Duration suffixes (`:q`, `:h`, `:8`)
- Dynamic markings (`!f`, `!pp`, `!mf`)
- Function names (`play`, `tempo`, `instrument`)
- GM instrument names
- Articulations (`.stac`, `.acc`, `.ferm`)
- Ornaments (`.tr`, `.mord`, `.grace`)

---

## Available Functions

All functions from the REPL are available in code cells:

### Playback & Export
- `play(notation)` — parse and play notation
- `play_notes(events)` — play raw note events
- `export(notation, path)` — export to MIDI file

### Context
- `tempo(bpm)` — set tempo
- `key(name)` — set key signature
- `time_sig(num, den)` — set time signature
- `swing(amount)` — set swing feel (0.0–1.0)
- `humanize(amount)` — add timing variation (0.0–1.0)
- `instrument(name)` — set GM instrument
- `get_context()` — inspect current settings
- `reset_context()` — reset to defaults

### Theory
- `note(name)` — create a Note object
- `chord(name)` — create a Chord object
- `scale(root, quality)` — create a Scale object

### Song Building
- `Song(title)` — create a multi-track Song
- `Track(instrument, notation)` — create a Track

### SoundFont
- `ensure_soundfont()` — download default SoundFont if needed
- `soundfont_info()` — show loaded SoundFont details
- `set_soundfont(path)` — use a custom SoundFont

---

## Example Session

A typical Studio workflow for a short piece:

1. Launch: `delphi studio ballad`

2. **Cell 1** (code — Setup):
   ```python
   tempo(72)
   key("G major")
   ```

3. **Cell 2** (notation — Piano):
   ```
   # @instrument piano
   # @track Right Hand

   | G | Em | C | D |
   | G | Em | C/E | D/F# |
   ```

4. **Cell 3** (notation — Bass):
   ```
   # @instrument acoustic bass
   # @track Bass

   G2:h B2:h E2:h G2:h C2:h E2:h D2:h F#2:h
   ```

5. **Cell 4** (markdown):
   ```
   Bridge section — try modulating to Bb major
   ```

6. Press **F6** to hear all parts together
7. Press **F9** to export MIDI + WAV
8. Press **Ctrl+S** to save
