# Plan: Delphi — Music Scripting Language

Delphi is a hybrid **Rust + Python** music scripting language that fills the gap between pattern-based live-coding tools (Strudel/Tidal) and full DAWs. Rust handles the audio engine, synthesis, and export; Python provides the scripting DSL and REPL. The syntax uses Python as the host language with **string-based music notation** parsed by a custom engine — giving musicians readable notation and programmers familiar control flow.

## Architecture

```
┌──────────────────────────────────────┐
│     Python Layer (DSL + REPL)        │
│  - Music notation parser             │
│  - Python control flow / scripting   │
│  - REPL (prompt_toolkit)             │
│  - Voice/section/arrangement API     │
├──────────────────────────────────────┤
│     PyO3 Bindings (delphi-py crate)  │
├──────────────────────────────────────┤
│     Rust Core Engine                 │
│  - delphi-core: music types, theory  │
│  - delphi-engine: scheduler, synth,  │
│    SoundFont player, audio output    │
│  - delphi-midi: MIDI I/O + export    │
│  - delphi-export: WAV, MusicXML      │
└──────────────────────────────────────┘
```

## Syntax Vision

```python
from delphi import *

tempo(120)
key("C major")
time_sig(4, 4)

piano = voice("piano")
bass  = voice("electric_bass")
drums = voice("drums")

# Sections use string-based notation
verse = section("verse")
verse[piano] = "| Cmaj7 | Am7 | Fmaj7 | G7 |"
verse[bass]  = "| C2 . . E2 | A2 . . C3 | F2 . . A2 | G2 . . B2 |"
verse[drums] = "| kick snare kick snare |"

chorus = section("chorus")
chorus[piano] = "| F | G | Am | C |"

# Arrangement with Python control flow
song = arrange(verse * 2, chorus, verse, chorus * 2)

song.play()
song.export("my_song.mid")
```

REPL interactive mode:
```
>>> tempo(120)
>>> play("| Cmaj7 | Am7 | Fmaj7 | G7 |")
>>> chord("Cmaj7").arpeggio("up").play()
```

## Phases

### Phase 1: Core Foundation (MVP)
**Goal:** Play notes/chords from a Python REPL, export to MIDI.

1. Initialize Rust workspace with `delphi-core` crate — `Note`, `Pitch`, `Octave`, `Duration`, `Chord`, `Scale`, `Tempo`, `TimeSignature`
2. `delphi-midi` crate — MIDI file export using `midly`
3. `delphi-engine` crate — basic synth (sine/saw/square via `cpal` + `dasp`), audio scheduler for timed event playback
4. `delphi-py` crate — PyO3 bindings exposing core types + `play()` + `export_midi()` + `tempo()` + `key()`
5. Python DSL — `notation.py` parser (parse `"C4 E4 G4"` and `"| Cmaj7 | Am7 |"` into events), `context.py` for global state, clean `__init__.py` API
6. Terminal REPL — `prompt_toolkit`-based with syntax highlighting and autocompletion for note/chord names
7. Build tooling — `pyproject.toml` with maturin, `delphi` CLI command

**Verification:** REPL launches → `play("C4 E4 G4")` produces sound → `play("| Cmaj7 | Am7 |")` plays progression → MIDI export opens in a DAW

### Phase 2: Musical Structure
**Goal:** Multi-voice sections, song arrangement, SoundFont playback. *Depends on Phase 1.*

1. SoundFont loader in `delphi-engine` (via `oxisynth` or `rustysynth`) — map voice names to GM patches
2. `Voice` type — instrument assignment, channel routing
3. `Section` type — `section[voice] = "notation"` via `__setitem__`, duration in bars
4. `Arrangement` type — `arrange()`, repeat via `section * N`, flatten to event timeline
5. Multi-voice `Mixer` — concurrent playback, per-voice gain/pan
6. Extend notation parser — drum names, rests (`.`/`~`), tuplets, ties, dotted notes
7. Default SoundFont — auto-download FluidR3_GM.sf2 to `~/.delphi/soundfonts/` on first run

**Verification:** Multi-voice plays simultaneously → `arrange(verse * 2, chorus)` plays full structure → MIDI export has separate tracks per voice

### Phase 3: Full Composition Toolkit
**Goal:** Audio export, dynamics, effects. *Depends on Phase 2.*

1. WAV export (`hound` crate), MP3 export
2. MusicXML export for notation software
3. Dynamics/articulation — `p`, `mf`, `f`, `ff`, staccato, legato
4. Effects chain — reverb, delay, filter in Rust audio graph
5. Mid-song tempo/key changes — `section("bridge", tempo=80, key="Ab major")`
6. Real-time MIDI I/O via `midir`
7. Advanced notation — grace notes, ornaments, complex time signatures

**Verification:** `.export("song.wav")` produces playable audio → `.export("song.xml")` opens in MuseScore → dynamics audibly affect playback

### Phase 4: Polish & Ecosystem (Future)
VS Code extension, web playground (WASM), package manager for instruments/samples, MIDI import, live-coding mode, documentation site.

## Notation Mini-Language Spec (Draft)

| Element | Syntax | Example |
|---------|--------|---------|
| Notes | name + accidental + octave | `C4`, `C#4`, `Db5` |
| Durations | `:` suffix | `C4:q` (quarter), `C4:h` (half), `C4:8` (eighth) |
| Chords | name notation | `Cmaj7`, `Am7`, `Dm7b5`, `G7#9` |
| Bars | pipe-delimited | `\| Cmaj7 \| Am7 \| G7 \|` |
| Rests | `.` or `~` or `r` | `C4 . E4 .` |
| Subdivisions | brackets | `[C4 E4 G4]` (equal subdivision) |
| Dynamics | `!` suffix | `C4!ff`, `Am7!pp` |
| Drums | named hits | `kick`, `snare`, `hihat`, `ride`, `crash` |

## Project Structure

```
delphi/
├── Cargo.toml                          # Rust workspace root
├── pyproject.toml                      # Python package config (maturin build)
├── crates/
│   ├── delphi-core/                    # Music primitives
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── note.rs                 # Note, Pitch, Octave, Accidental
│   │       ├── interval.rs             # Interval types
│   │       ├── chord.rs                # Chord (name→notes resolution)
│   │       ├── scale.rs                # Scale/Key (major, minor, modes)
│   │       ├── duration.rs             # Duration, TimeSignature, Tempo
│   │       └── dynamics.rs             # Velocity, articulation
│   ├── delphi-engine/                  # Audio engine
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── scheduler.rs            # Event scheduler (tick-based)
│   │       ├── synth.rs                # Built-in oscillator synths
│   │       ├── soundfont.rs            # SF2 loading and playback
│   │       ├── mixer.rs                # Voice mixing, gain, pan
│   │       ├── sampler.rs              # Sample playback (drums, etc.)
│   │       └── output.rs              # CPAL audio output stream
│   ├── delphi-midi/                    # MIDI subsystem
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── export.rs               # Write Standard MIDI Files
│   │       ├── import.rs               # Read MIDI files (future)
│   │       └── realtime.rs             # MIDI I/O via midir (future)
│   └── delphi-py/                      # PyO3 Python bindings
│       └── src/
│           ├── lib.rs                  # #[pymodule] entry point
│           ├── types.rs                # Python wrappers for core types
│           ├── engine.rs               # Python API for playback engine
│           └── midi.rs                 # Python API for MIDI export
├── python/
│   └── delphi/
│       ├── __init__.py                 # Public API re-exports
│       ├── notation.py                 # Mini-notation parser
│       ├── theory.py                   # High-level music theory helpers
│       ├── voice.py                    # Voice/instrument abstraction
│       ├── section.py                  # Section definition
│       ├── arrangement.py              # arrange(), song structure, repeat
│       ├── repl.py                     # Interactive REPL (prompt_toolkit)
│       ├── context.py                  # Global state (tempo, key, time_sig)
│       └── cli.py                      # CLI entry point (delphi command)
├── resources/
│   ├── soundfonts/                     # Default GM SoundFont
│   └── samples/                        # Built-in drum samples
├── examples/
│   ├── hello.delphi                    # First example
│   ├── blues.delphi                    # 12-bar blues
│   └── pop_song.delphi                # Full song arrangement
└── tests/
    ├── rust/                           # Rust unit tests
    └── python/                         # Python integration tests
```

## Key Rust Crates
- `cpal` — cross-platform audio output
- `dasp` — DSP primitives (sample conversion, ring buffers)
- `midly` — MIDI file reading/writing
- `midir` — real-time MIDI I/O
- `oxisynth` or `rustysynth` — SoundFont synthesis
- `pyo3` — Python bindings
- `hound` — WAV export

## Key Python Dependencies
- `maturin` — build system for Rust→Python
- `prompt_toolkit` — terminal REPL framework
- `pytest` — testing

## Key Decisions
- **Rust + Python hybrid**: Rust for audio performance, Python for UX and rapid DSL iteration (PyO3/maturin bridge)
- **String-based notation, not custom grammar**: keeps the language as valid Python — no compiler needed
- **MVP = notes + chords + REPL + MIDI export**: SoundFont, sections, arrangement are Phase 2
- **SoundFont for realism**: musicians expect recognizable instruments; built-in synths are a fallback
- **Composition-first, not live-coding**: live coding is a future add-on
- **SoundFont not bundled** (~140MB): auto-downloaded to `~/.delphi/soundfonts/` on first run

## Open Considerations
1. **Notation parser in Python vs Rust** — Start in Python (simpler iteration), move to Rust via `nom` only if parsing becomes a bottleneck
2. **SoundFont choice** — FluidR3_GM (MIT-licensed, ~140MB) is the standard. Alternatively, a smaller GM SoundFont for faster first-run experience
3. **Drum sample licensing** — Synthesize basic drums or source from permissive-licensed packs to avoid legal issues
