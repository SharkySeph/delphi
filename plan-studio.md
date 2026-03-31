# Delphi Studio TUI — Implementation Plan

## Vision
A bespoke terminal notebook IDE for composing full multi-track songs.
Think Jupyter meets Strudel meets a DAW — all in the terminal via prompt_toolkit.

## Layout

```
┌─────────────────────────────────────────────────────┐
│  🎵 Delphi Studio — my-song          ♩=120  C major │  ← Status bar
├─────────────────────────────────────────────────────┤
│ [1] ▶ Setup                                  [Run]  │
│   tempo(120)                                        │
│   key("C major")                                    │
│   ───────────────────────────────────────────────── │
│   ♪ Context set: 120 BPM, C major                   │  ← Output
├─────────────────────────────────────────────────────┤
│ [2] ▶ Track: Melody (piano)                  [Run]  │
│   C4:q E4:q G4:q C5:h                              │
│   F4:q A4:q C5:q F5:h                              │
│   ───────────────────────────────────────────────── │
│   ♪ 8 notes, 4 bars                                 │
├─────────────────────────────────────────────────────┤
│ [3] ▶ Track: Bass (acoustic bass)            [Run]  │
│   C2:h G2:h  F2:h C2:h                             │
│   ───────────────────────────────────────────────── │
│   ♪ 4 notes, 2 bars                                 │
├─────────────────────────────────────────────────────┤
│ [4] ▶ Mixdown                                [Run]  │
│   song = Song("my-song", tempo=120)                 │
│   song.track("melody", cells[2], program="piano")   │
│   song.track("bass", cells[3], program="acoustic…") │
│   song.play()                                       │
├─────────────────────────────────────────────────────┤
│ F1:Help  F5:Run Cell  F6:Run All  F7:Add Cell       │
│ F8:Delete  F9:Export  F10:Save  Ctrl-Q:Quit         │
└─────────────────────────────────────────────────────┘
```

## Core Components

### 1. Cell Model
```python
class Cell:
    id: int
    cell_type: "code" | "notation" | "markdown"
    source: str          # editable content
    output: str          # last run result
    label: str           # "Track: Piano", "Setup", etc.
    instrument: str      # GM instrument name (for notation cells)
    channel: int
    collapsed: bool
```

### 2. Notebook Model
```python
class Notebook:
    title: str
    cells: list[Cell]
    song: Song           # accumulated Song object
    file_path: str       # .dstudio file
    
    def run_cell(index)
    def run_all()        # builds Song from all cells top-to-bottom
    def add_cell(after)
    def delete_cell(index)
    def move_cell(from, to)
    def save() / load(path)
    def export_midi() / export_wav() / export_script()
```

### 3. Cell Types & Execution

- **code cell**: Executes as Python in the Delphi namespace
- **notation cell**: Pure notation → parsed, played via SoundFont.
  Assigned to a track/instrument via cell metadata pragma:
  ```
  # @track melody @program piano @velocity 90
  C4:q E4:q G4:q C5:h
  ```
- **markdown cell**: Notes, headings, documentation

### 4. Syntax Highlighting
Reuse `DelphiLexer` from repl.py inside `prompt_toolkit` `BufferControl`.
Notes = cyan, chords = green, dynamics = yellow, bar lines = dim.

### 5. Key Bindings

| Key | Action |
|-----|--------|
| F5 / Ctrl+Enter | Run current cell |
| F6 | Run all cells |
| F7 / Ctrl+B | Insert cell below |
| F8 | Delete cell (confirm) |
| F9 | Export menu (MIDI/WAV/script) |
| F10 / Ctrl+S | Save notebook |
| Ctrl+Up/Down | Navigate between cells |
| Ctrl+Shift+Up/Down | Reorder cells |
| Tab | Autocomplete |
| Ctrl+P | Replay last output |
| Ctrl+Q | Quit |

### 6. Multi-Track Workflow
Each notation cell has metadata: instrument, velocity, channel.
"Run All" builds a Song from all notation cells (each → a track).
A final "mixdown" code cell can customize effects, add structure.

### 7. File Format (.dstudio)
```json
{
  "version": 1,
  "title": "My Song",
  "settings": {"tempo": 120, "key": "C major", "time_sig": "4/4"},
  "cells": [
    {
      "type": "notation",
      "source": "C4:q E4:q G4:h",
      "meta": {"track": "melody", "program": "piano", "velocity": 90}
    },
    {"type": "code", "source": "song.play()"}
  ]
}
```

### 8. Status Bar
Always visible: project name, tempo, key, time sig, cell #/total, last action.

### 9. Output Pane (per cell)
- Notation: note count, bar count, duration, parse errors
- Code: return value or print output
- Playback: "Playing…" with elapsed time, Ctrl+C to stop

## Implementation Phases

### Phase 1: Core shell (~400 lines)
- Cell + Notebook models
- Save/load .dstudio files
- prompt_toolkit Application with HSplit layout
- Cell navigation (focus up/down)
- Basic multiline text editing within cells
- Status bar + toolbar

### Phase 2: Execution (~200 lines)
- Run cell (code → exec, notation → parse + play)
- Run all (build Song from notation cells)
- Output display below each cell
- Error display with red highlighting

### Phase 3: Editor features (~200 lines)
- Syntax highlighting via DelphiLexer
- Autocomplete (instruments, scales, functions)
- Cell metadata pragmas (@track, @program)
- Add / delete / reorder cells
- Cell type switching (code ↔ notation ↔ markdown)

### Phase 4: Polish (~200 lines)
- Export: MIDI, WAV, flatten to .delphi script
- Cell collapse/expand
- Undo per cell
- Project integration: `delphi studio my-song` reads delphi.toml

## CLI Integration
```bash
delphi studio                    # New empty notebook
delphi studio my-song            # Open project as notebook
delphi studio song.dstudio       # Open saved notebook file
```

## Dependencies
- `prompt_toolkit` (already required) — Application, HSplit, VSplit, Buffer, etc.
- `pygments` (already used) — DelphiLexer for highlighting
- No new dependencies needed.

## What This Enables
A skilled user can:
1. `delphi studio` → opens a blank notebook
2. Cell 1: set tempo, key, time signature
3. Cell 2: write melody notation (highlighted, with autocomplete)
4. Cell 3: write bass line
5. Cell 4: write drum pattern
6. F5 on any cell → hear that part solo
7. F6 → hear the full mix (all cells → Song → play)
8. Tweak velocities, effects, structure
9. F9 → export MIDI + WAV
10. All without leaving the terminal
