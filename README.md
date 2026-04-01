# 🎵 Delphi

**A music composition environment — write music as code, hear it instantly.**

Delphi lets you compose real, multi-track music using a concise notation language. The primary interface is **Delphi Studio**, a native GUI application with a notebook-style editor, piano roll, mixer, real-time visualizer, and SoundFont-powered playback. A Python CLI/REPL is also available for scripting and automation.

## Delphi Studio

The recommended way to use Delphi. A native desktop application built in Rust with egui.

**Notebook editor** — Write notation, code, and markdown in cells. Each cell can target a different instrument and MIDI channel.

```
# @track melody @program violin @velocity 90
G4:q A4:q B4:q D5:h
F#4:q G4:q A4:q B4:h

# @track bass @program acoustic bass @channel 2
C2:h G2:h  F2:h C2:h
```

**Built-in panels:**
- **Piano Roll** — Visual note display synced to your notation
- **Mixer** — Per-track gain, pan, mute, and solo
- **Visualizer** — Real-time playback animation with note bars
- **Theory Explorer** — Scales, chords, and intervals reference
- **SoundFont Manager** — Load and switch SoundFont banks
- **Script Engine** — Rhai scripting for automation

**Key features:**
- 128 GM instruments with SoundFont synthesis (auto-downloads GeneralUser GS)
- Full notation parser: notes, chords (40+ qualities), rests, ties, tuplets, subdivisions, dynamics, articulations, ornaments, Euclidean rhythms, drum patterns, structural markers, and more
- Auto-complete (Ctrl+Space) with 80+ suggestions
- Live diagnostics — warnings for unknown tokens as you type
- Export to MIDI and WAV
- Save/load `.dstudio` project files and `.delphi` scripts
- 5 bundled examples (Hello World, Twinkle Twinkle, 12-Bar Blues, Canon in D, Studio Showcase)

### Keyboard shortcuts

| Key | Action |
|-----|--------|
| F5 | Play all cells |
| F6 | Play current cell |
| Esc | Stop playback |
| Ctrl+Space | Auto-complete |
| Ctrl+S | Save project |
| Ctrl+O | Open file |
| Ctrl+N | New project |

## Installation

### Delphi Studio (recommended)

**Linux (.deb):**

Download from [GitHub Releases](https://github.com/SharkySeph/delphi/releases) and install:

```bash
sudo dpkg -i delphi-studio_0.7.0-1_amd64.deb
```

Then launch:

```bash
delphi-studio
```

Or find **Delphi Studio** in your application menu.

**Build from source (any platform):**

Requires Rust toolchain ([rustup](https://rustup.rs)) and ALSA headers on Linux (`sudo apt install libasound2-dev`).

```bash
git clone https://github.com/SharkySeph/delphi.git
cd delphi
cargo build --release -p delphi-gui
# Binary at target/release/delphi-studio
```

### Python CLI & REPL (alternative)

For terminal-based workflows, scripting, and Jupyter integration.

**Requirements:** Python ≥ 3.10 and ALSA on Linux (`sudo apt install libasound2`).

**Linux / macOS:**
```bash
curl -sSf https://raw.githubusercontent.com/SharkySeph/delphi/main/deploy/install.sh | bash
```

**Windows (PowerShell):**
```powershell
irm https://raw.githubusercontent.com/SharkySeph/delphi/main/deploy/install-windows.ps1 | iex
```

The installer creates a virtual environment, downloads the pre-built wheel, and adds `delphi` to your PATH.

## Notation Language

Delphi uses the same notation syntax across the GUI and CLI. Write notes, chords, and rhythms in a single line:

```
C4:q E4:q G4:q C5:h          # melody with durations
| Cmaj7 | Am7 | Fmaj7 | G7 | # chord progression in bar notation
kick snare kick snare          # drum patterns
[C4 E4 G4]                    # subdivisions
(3 C4 D4 E4)                  # triplets
C4:q~ C4:q                    # tied notes
C4:q:ff                       # dynamics (pp, p, mp, mf, f, ff, fff)
C4:q.accent                   # articulations
trill(C4:q)                   # ornaments
e(3,8)                        # Euclidean rhythms
```

Pragmas configure tracks:

```
# @track melody @program violin @velocity 90 @channel 1
# @tempo 120 @key "D major" @time 3/4
```

See [Notation Reference](docs/notation.md) for the complete syntax.

## Python API

```python
from delphi import *

tempo(120)
key("C major")

# Play a melody
play("C4:q E4:q G4:q C5:h")

# Switch instruments
instrument("violin")
play("G4:q A4:q B4:q D5:h")

# Play a chord progression
instrument("piano")
play("| Cmaj7 | Am7 | Fmaj7 | G7 |")

# Build a multi-track song
song = Song("My Song", tempo=110, key="G major")
song.track("melody", "G4:q B4:q D5:q G5:h", program="piano", velocity=90)
song.track("bass", "G2:h D2:h", program="acoustic bass", velocity=65)
song.play()
song.export("my_song.mid")
```

### CLI commands

| Command | Description |
|---------|-------------|
| `delphi` | Launch REPL |
| `delphi studio` | Open terminal Studio (TUI) |
| `delphi run <file>` | Run a `.delphi` script |
| `delphi examples` | List bundled examples |
| `delphi init <name>` | Scaffold a new project |
| `delphi --version` | Show version |

### Jupyter Notebooks

```python
%load_ext delphi.notebook
```

```python
%%delphi --program piano --tempo 120
| Cmaj7 | Am7 | Fmaj7 | G7 |
```

## Features

- **Delphi Studio GUI** — Native desktop app: notebook editor, piano roll, mixer, visualizer, theory explorer
- **Expressive notation** — Notes, chords, rests, dynamics, articulations, ornaments, Euclidean rhythms, ties, tuplets, crescendo/diminuendo, drum patterns
- **50+ scales** — Church modes, pentatonic, blues, bebop, altered, exotic (Hungarian, Persian, Japanese, Egyptian…)
- **40+ chord qualities** — Major, minor, diminished, augmented, sus, add, extended, altered, slash chords
- **Multi-track songs** — Layer instruments with per-track gain, pan, mute, solo
- **128 GM instruments** — Piano, strings, brass, woodwinds, drums, synths, and more
- **SoundFont synthesis** — Realistic instrument sounds via GeneralUser GS (auto-downloaded)
- **MIDI & WAV export** — Standard MIDI files and rendered audio
- **Composition tools** — Patterns, Voices, Sections, Arrangements with rehearsal marks
- **Python REPL** — Interactive terminal with syntax highlighting and auto-complete
- **Terminal Studio** — TUI notebook IDE (Jupyter meets a DAW)
- **Jupyter integration** — `%%delphi` magic cells with inline audio
- **Rust-powered engine** — Fast audio synthesis and MIDI scheduling

## Documentation

| Guide | Description |
|-------|-------------|
| [Notation Reference](docs/notation.md) | Complete syntax for notes, chords, rhythms, dynamics |
| [Studio Guide](docs/studio.md) | GUI and TUI studio usage |
| [Song & Track API](docs/songs-and-tracks.md) | Building multi-track songs with effects |
| [Composition Guide](docs/composition.md) | Patterns, Sections, Arrangements for large pieces |
| [Theory Module](docs/theory.md) | Scales, chords, notes — music theory as code |
| [REPL Guide](docs/repl.md) | Interactive terminal environment |
| [Jupyter Notebooks](docs/notebook.md) | Using Delphi in Jupyter with inline audio |
| [Examples](examples/) | Complete example scripts |

## Examples

### In Delphi Studio

Open the bundled examples from **File → Examples** — includes Hello World, Twinkle Twinkle Little Star, 12-Bar Blues, Pachelbel's Canon in D, and the Studio Showcase.

### Python scripts

**Twinkle Twinkle (2 tracks):**

```python
from delphi import Song, Track, ensure_soundfont
ensure_soundfont()

song = Song("Twinkle Twinkle", tempo=100, key="C major")
song.add_track(Track("Melody", """
    C4:q C4:q G4:q G4:q  A4:q A4:q G4:h
    F4:q F4:q E4:q E4:q  D4:q D4:q C4:h
""", program="piano", velocity=90))
song.add_track(Track("Bass", """
    C2:h G2:h  F2:h C2:h
""", program="acoustic bass", velocity=60))
song.play()
```

**12-Bar Blues:**

```python
from delphi import *
tempo(110)
key("A blues")
play("""
| A7 | A7 | A7 | A7 |
| D7 | D7 | A7 | A7 |
| E7 | D7 | A7 | E7 |
""")
```

## Project Structure

```
delphi/
├── crates/                   # Rust workspace
│   ├── delphi-gui/           # Delphi Studio (egui/eframe)
│   ├── delphi-core/          # Music theory types
│   ├── delphi-engine/        # Audio synthesis & SoundFont playback
│   ├── delphi-midi/          # MIDI file export
│   └── delphi-py/            # PyO3 Python bindings
├── python/delphi/            # Python DSL & CLI
│   ├── notation.py           # Notation parser
│   ├── song.py               # Song & Track classes
│   ├── composition.py        # Patterns, Sections, Arrangements
│   ├── theory.py             # Scales, chords, intervals
│   ├── repl.py               # Interactive REPL
│   ├── studio.py             # Terminal Studio (TUI)
│   └── cli.py                # CLI entry point
├── deploy/                   # Install scripts (Linux, macOS, Windows)
├── examples/                 # Bundled .delphi and .dstudio examples
└── tests/                    # Python and Rust test suites
```

## Development

Build everything from source:

```bash
git clone https://github.com/SharkySeph/delphi.git
cd delphi

# Build the Studio GUI
cargo build --release -p delphi-gui

# Build the Python wheel
python -m venv .venv && source .venv/bin/activate
pip install maturin
maturin develop

# Run tests
cargo test
python -m pytest tests/python/ -v
```

**Requirements:** Rust toolchain ([rustup](https://rustup.rs)), Python ≥ 3.10, Linux ALSA headers (`sudo apt install libasound2-dev`).

## License

MIT
