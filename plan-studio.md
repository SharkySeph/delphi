# Delphi Studio — Native GUI Application

## Vision
A native desktop GUI for composing multi-track music using the Delphi notation language.
Combines a notebook-style cell editor with a piano roll, mixer, real-time visualizer,
theory explorer, and SoundFont-powered playback — built entirely in Rust with egui.

## Current State: Implemented ✅

### Layout
```
┌─────────────────────────────────────────────────────────────┐
│  Menu │ ▶ Play │ ⏹ Stop │ 🔁 Loop │ ♩=120 │ C major │ 4/4 │  ← Transport bar
├───────────────┬─────────────────────────────┬───────────────┤
│               │                             │               │
│  Track List   │    Center Panel             │  Side Panel   │
│  - Melody     │    (Editor or Piano Roll)   │  (SoundFont,  │
│  - Bass       │                             │   Export,     │
│  - Drums      │    [1] ▶ Track: Melody      │   Script)    │
│               │    # @instrument piano      │               │
│               │    C4:q E4:q G4:q C5:h      │               │
│               │    ─────────────────────     │               │
│               │    [2] ▶ Track: Bass         │               │
│               │    # @instrument bass        │               │
│               │    C2:h G2:h F2:h C2:h      │               │
│               │                             │               │
├───────────────┴─────────────────────────────┴───────────────┤
│  Bottom Panel: Mixer │ Visualizer │ Theory │ Help           │
└─────────────────────────────────────────────────────────────┘
```

### Core Components (all in `crates/delphi-gui/src/`)

#### Cell Model (`studio.rs` → wraps `delphi_core::Project`)
- `Cell`: notation, code, or markdown — with source text and metadata
- `TrackState`: name, program, channel, pan, mute, solo, reverb, delay
- `ProjectSettings`: title, BPM, key, time signature, swing, humanize
- `StudioState`: thin Deref wrapper around `Project`, adds GUI-specific methods
- Pragmas: `# @instrument piano`, `# @channel 1`, `# @velocity 90`

#### Editor (`editor.rs`)
- Cell list with collapsible frames
- Syntax highlighting for Delphi notation
- Token kinds: Note, Chord, Duration, Barline, Rest, Drum, Operator, Pragma, Comment
- Per-cell run button + diagnostic display

#### Transport (`player.rs`)
- Play all cells or single cell
- Loop toggle, BPM override, elapsed time display
- SoundFont resolution → spawns playback thread
- Oscillator fallback if no SoundFont available

#### Export (`export.rs`)
- MIDI or WAV export dialog
- Groups events by channel into tracks
- Uses SoundFont for WAV rendering

#### Mixer (`mixer.rs`)
- Master gain + per-track faders
- Pan, mute, solo, reverb, delay controls per track
- Per-track gain applied during render

#### Piano Roll (`piano_roll.rs`)
- Read-only note visualization
- Zoom X/Y, scroll, snap grid
- Color-coded by track, beat grid lines

#### Theory Explorer (`theory.rs`)
- Interactive chord and scale builder
- Circle of Fifths visualization
- Keyboard display

#### Visualizer (`visualizer.rs`)
- Modes: NowPlaying (active notes), Waveform, Spectrum, Both
- Synthetic waveform generated from active MIDI events

#### SoundFont Manager (`soundfont.rs`)
- Auto-discovery: `~/.delphi/soundfonts/`, system paths, env var
- Prefers "GeneralUser" SoundFont

#### Scripting (`scripting.rs`)
- Rhai expression evaluation
- Built-in functions: `note_to_midi()`, `chord_notes()`, `scale_notes()`
- Studio commands: `set_tempo()`, `add_cell()`, etc.

### File Format (.dstudio)
```json
{
  "title": "My Song",
  "settings": {"tempo": 120, "key": "C major", "time_sig": "4/4"},
  "cells": [
    {
      "type": "notation",
      "source": "C4:q E4:q G4:h",
      "meta": {"label": "Melody", "program": "piano", "velocity": 90, "channel": 0}
    }
  ]
}
```

### Dependencies
- `egui` / `eframe` — immediate-mode GUI framework
- `delphi-core` — notation parser, project model, music types
- `delphi-engine` — SoundFont playback, WAV rendering
- `delphi-midi` — MIDI export

## Future Enhancements

### Editable Piano Roll
- Click/drag to add, move, resize notes
- Snap to grid (beat, half-beat, triplet)
- Selection + copy/paste
- Feeds back into notation cells

### Track-Level Audio Mixing
- Real DSP gain per track (not just CC messages)
- Master bus with gain + limiter

### Undo/Redo
- Cell-level or global undo stack
- History panel

### MIDI Input
- Real-time recording from MIDI keyboard via `midir`
- Quantize + convert to notation

### Arrangement View
- Section blocks (verse, chorus, bridge) on a timeline
- Drag to reorder, repeat via markers
- Full song structure visualization
