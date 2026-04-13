# Plan: Delphi — Music Composition Toolkit

Delphi is a **pure Rust** music composition toolkit with a custom notation language, filling the gap between pattern-based live-coding tools (Strudel/Tidal) and full DAWs. It ships as two native binaries — a **CLI** for scripting/export workflows and a **GUI** (Delphi Studio) for interactive composition.

## Architecture

```
┌──────────────────────────────────────┐
│     delphi-gui  (Delphi Studio)      │
│  - egui notebook editor              │
│  - Piano roll, mixer, visualizer     │
│  - Theory explorer, Rhai scripting   │
├──────────────────────────────────────┤
│     delphi-cli  (delphi command)     │
│  - export (MIDI/WAV), play, info     │
├──────────────────────────────────────┤
│     Shared Rust Libraries            │
│  - delphi-core:  music types, theory,│
│    notation parser, project model    │
│  - delphi-engine: SoundFont synth,   │
│    audio output, oscillator fallback │
│  - delphi-midi: MIDI file export     │
└──────────────────────────────────────┘
```

## Notation Language

```
# Single notes with durations
C4:q E4:q G4:h

# Chord progressions (bar notation)
| Cmaj7 | Am7 | Fmaj7 | G7 |

# Subdivisions, rests, repeats
[C4 E4 G4] . [F4 A4 C5] .
kick snare hihat*2 snare

# Articulation and ornaments
C4:q:stac E4:q:ten G4:h:tr

# Pragmas (per-cell metadata)
# @instrument piano
# @channel 1
# @velocity 90
```

## CLI Usage

```bash
delphi export song.dstudio --format both --sf ~/soundfonts/GM.sf2
delphi play song.dstudio
delphi info song.dstudio
delphi new my-song
```

## Completed Phases

### Phase 1: Core Foundation ✅
- `delphi-core` — Note, Interval, Chord, Scale, Duration, Dynamics, Tempo, TimeSignature
- `delphi-midi` — MIDI file export (SMF Format 1, 480 PPQ) via `midly`
- `delphi-engine` — oscillator synth (sine/saw/square/triangle) via `cpal`, audio scheduler
- Notation parser — notes, chords, durations, rests, subdivisions

### Phase 2: Musical Structure ✅
- SoundFont playback via `rustysynth` — GM patch mapping, multi-channel rendering
- Multi-track project model — cells (notation/code/markdown), tracks with instrument/channel
- Bar notation — `| Cmaj7 | Am7 |` auto-distributed across time signature
- Extended notation — drums, tuplets, ties, dotted notes, repeats, polyphony
- SoundFont auto-discovery — `~/.delphi/soundfonts/`, `/usr/share/sounds/sf2/`

### Phase 3: Composition Toolkit ✅
- WAV export — offline SoundFont rendering
- Dynamics/articulation — staccato, tenuto, accent, marcato, pizzicato
- Ornaments — trills, mordents, turns, grace notes
- Per-channel effects — pan, reverb, delay, volume (CC messages to SoundFont)
- Swing and humanize parameters
- Euclidean rhythms, velocity ramps

### Phase 4: Native GUI (Delphi Studio) ✅
- egui/eframe desktop application
- Notebook-style multi-cell editor with syntax highlighting
- Piano roll (read-only note visualization)
- Mixer panel — per-track gain, pan, mute, solo, reverb, delay
- Real-time audio visualizer (waveform, spectrum, active notes)
- Interactive theory explorer (chord/scale builder, circle of fifths)
- Rhai scripting integration
- MIDI and WAV export dialogs
- SoundFont manager with auto-discovery

### Phase 5: Pure Rust CLI ✅
- `delphi` CLI binary — export, play, info, new commands
- Replaced Python layer entirely — zero external runtime dependencies
- Single `.deb` package installs both `delphi` and `delphi-studio`

## Future Directions
- MIDI import (`midir`)
- Live-coding mode with hot-reload
- Editable piano roll (click-to-add notes)
- MusicXML export for notation software
- VS Code extension for `.delphi` syntax
- Web playground (WASM build of delphi-core)
- Package manager for instruments/samples

## Notation Mini-Language Reference

| Element | Syntax | Example |
|---------|--------|---------|
| Notes | name + accidental + octave | `C4`, `C#4`, `Db5` |
| Durations | `:` suffix | `C4:q` (quarter), `C4:h` (half), `C4:8` (eighth) |
| Chords | name notation | `Cmaj7`, `Am7`, `Dm7b5`, `G7#9` |
| Bars | pipe-delimited | `\| Cmaj7 \| Am7 \| G7 \|` |
| Rests | `.` | `C4 . E4 .` |
| Subdivisions | brackets | `[C4 E4 G4]` (equal subdivision within one beat) |
| Tuplets | parentheses | `(3 C4 E4 G4)` (triplet) |
| Ties | `~` | `C4:h~C4:q` |
| Repeats | `*N` | `C4:q*4` |
| Polyphony | `,` | `C4,E4,G4` (simultaneous) |
| Dynamics | `!` suffix | `C4!ff`, `Am7!pp` |
| Articulation | `:` suffix | `:stac`, `:ten`, `:acc`, `:marc`, `:pizz` |
| Ornaments | `:` suffix | `:tr` (trill), `:mord` (mordent), `:turn` |
| Drums | named hits | `kick`, `snare`, `hihat`, `ride`, `crash` |
| Pragmas | `# @key value` | `# @instrument piano`, `# @channel 9` |
| Probability | `?` suffix | `C4?` (50% chance), `C4?75` (75%) |

## Project Structure

```
delphi/
├── Cargo.toml                          # Rust workspace root
├── crates/
│   ├── delphi-core/                    # Music primitives + notation parser + project model
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── note.rs                 # Note, Pitch, Octave, Accidental
│   │       ├── interval.rs             # Interval types
│   │       ├── chord.rs                # Chord (name→notes resolution)
│   │       ├── scale.rs                # Scale/Key (major, minor, modes)
│   │       ├── duration.rs             # Duration, TimeSignature, Tempo
│   │       ├── dynamics.rs             # Velocity, articulation
│   │       ├── event.rs                # NoteEvent (universal event type)
│   │       ├── gm.rs                   # General MIDI instrument/drum mapping
│   │       ├── notation.rs             # Full notation parser (~1100 lines)
│   │       └── project.rs              # Project model (Cell, Track, Settings, load/save)
│   ├── delphi-engine/                  # Audio engine
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── scheduler.rs            # Event scheduler (tick-based)
│   │       ├── synth.rs                # Built-in oscillator synths
│   │       ├── soundfont.rs            # SoundFont playback + WAV rendering
│   │       └── output.rs              # CPAL audio output stream
│   ├── delphi-midi/                    # MIDI subsystem
│   │   └── src/
│   │       ├── lib.rs
│   │       └── export.rs               # Write Standard MIDI Files
│   ├── delphi-cli/                     # CLI binary
│   │   └── src/
│   │       └── main.rs                 # export, play, info, new commands
│   └── delphi-gui/                     # GUI binary (Delphi Studio)
│       ├── assets/                     # Desktop file, icons
│       └── src/
│           ├── main.rs                 # Entry point
│           ├── app.rs                  # Root app state + layout
│           ├── studio.rs               # Thin wrapper around Project (Deref)
│           ├── editor.rs               # Cell editor + syntax highlighting
│           ├── player.rs               # Transport (play/stop/loop)
│           ├── export.rs               # Export dialog (MIDI/WAV)
│           ├── mixer.rs                # Per-track faders, pan, effects
│           ├── piano_roll.rs           # Note visualization
│           ├── theory.rs               # Chord/scale builder, circle of fifths
│           ├── visualizer.rs           # Waveform/spectrum display
│           ├── scripting.rs            # Rhai integration
│           ├── soundfont.rs            # SoundFont discovery + manager
│           └── theme.rs                # Color palette
├── deploy/                             # Build + install scripts
├── docs/                               # User documentation
├── examples/                           # .delphi and .dstudio examples
└── dist/                               # Built packages (.deb, .tar.gz)
```

## Key Rust Dependencies
- `cpal` — cross-platform audio output
- `rustysynth` — SoundFont synthesis
- `midly` — MIDI file reading/writing
- `egui` / `eframe` — immediate-mode GUI
- `serde` / `serde_json` — project serialization
- `ctrlc` — graceful CLI interrupt handling
- `rand` — humanize/probability features

## Key Decisions
- **Pure Rust**: No runtime dependencies — ships as static binaries
- **String-based notation**: Custom mini-language parsed in Rust, not a general-purpose language
- **Two binaries**: `delphi` (CLI for scripting/CI/export) and `delphi-studio` (GUI for composition)
- **SoundFont for realism**: GM instrument rendering via rustysynth; oscillator synth as fallback
- **Composition-first**: Designed for writing and exporting music, not live performance
- **SoundFont not bundled**: Auto-discovered from `~/.delphi/soundfonts/` and system paths
