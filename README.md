# 🎵 Delphi

**A music scripting language for composing real music.**

Delphi is a hybrid Rust + Python DSL that lets you write music as code. Describe melodies with a concise notation syntax, build multi-track songs with effects, and hear them instantly — all from your terminal.

```python
from delphi import *

tempo(120)
key("C major")

# Play a melody
play("C4:q E4:q G4:q C5:h")

# Switch to violin and play
instrument("violin")
play("G4:q A4:q B4:q D5:h")

# Play a chord progression
instrument("piano")
play("| Cmaj7 | Am7 | Fmaj7 | G7 |")

# Build a full song
song = Song("My Song", tempo=110, key="G major")
song.track("melody", "G4:q B4:q D5:q G5:h", program="piano", velocity=90)
song.track("bass", "G2:h D2:h", program="acoustic bass", velocity=65)
song.play()
song.export("my_song.mid")
```

## Features

- **Expressive notation** — Notes, chords, rests, dynamics, articulations, ornaments, Euclidean rhythms, ties, tuplets, volta brackets, crescendo/diminuendo, and more
- **50+ scales** — Church modes, pentatonic, blues, bebop, altered, exotic (Hungarian, Persian, Japanese, Egyptian…)
- **Multi-track songs** — Layer instruments with per-track effects (gain, pan, reverb, delay, ADSR)
- **Composition tools** — Patterns, Voices, Sections, Arrangements with rehearsal marks
- **Interactive REPL** — Syntax highlighting, auto-complete, inline playback
- **Delphi Studio** — Terminal notebook IDE for composing multi-track songs (think Jupyter meets a DAW)
- **128 GM instruments** — Switch with `instrument("violin")` and hear it instantly
- **SoundFont synthesis** — Realistic instrument sounds via GeneralUser GS (auto-downloaded)
- **MIDI & WAV export** — Standard MIDI files and rendered audio
- **128 GM instruments** — Piano, strings, brass, woodwinds, drums, synths, and more
- **Rust-powered engine** — Fast audio synthesis and scheduling via PyO3

## Installation

### Requirements

- Python ≥ 3.10
- Linux: ALSA runtime library (`sudo apt install libasound2` or `sudo dnf install alsa-lib`)
- macOS / Windows: No extra dependencies

### Quick install (recommended)

Run the one-liner for your OS — no Rust or build tools needed:

**Linux:**
```bash
curl -sSf https://raw.githubusercontent.com/SharkySeph/delphi/main/deploy/install.sh | bash
```

**macOS:**
```bash
curl -sSf https://raw.githubusercontent.com/SharkySeph/delphi/main/deploy/install-macos.sh | bash
```

**Windows (PowerShell):**
```powershell
irm https://raw.githubusercontent.com/SharkySeph/delphi/main/deploy/install-windows.ps1 | iex
```

The installer will:
1. Find Python 3.10+ on your system
2. Create a virtual environment (`~/.local/share/delphi/venv`)
3. Download and install the pre-built wheel from GitHub Releases (Rust engine included)
4. Add the `delphi` command to your PATH

After installing, restart your terminal and run:
```bash
delphi --version
```

### Manual install (from release wheel)

If you prefer to manage it yourself, download the wheel for your platform from
[GitHub Releases](https://github.com/SharkySeph/delphi/releases) and install it:

```bash
python3 -m venv ~/.local/share/delphi/venv
source ~/.local/share/delphi/venv/bin/activate
pip install /path/to/delphi-0.6.0-cp310-abi3-manylinux_2_28_x86_64.whl
delphi --version
```

## Quick Start

### Launch the REPL

```bash
delphi
```

Type notation directly and hear it play:

```
🎵 > C4 E4 G4 C5
  ♪ Playing...

🎵 > | Am7 | Dm7 | G7 | Cmaj7 |
  ♪ Playing chord progression...
```

### Create a project

```bash
delphi init my-song
cd my-song
```

This scaffolds a project directory:

```
my-song/
├── delphi.toml       # Project config (tempo, key, soundfont)
├── main.delphi       # Starter script
├── patterns/         # Reusable patterns
└── exports/          # Rendered MIDI/WAV output
```

Edit `main.delphi` and run it:

```bash
delphi main.delphi
```

Or open the project in the REPL (loads settings from `delphi.toml`):

```bash
delphi open .
```

### Run a script

```bash
delphi run twinkle              # Run a bundled example
delphi run my-script.delphi     # Run a local file
```

### CLI commands

| Command | Description |
|---------|-------------|
| `delphi` | Launch REPL (auto-detects project in cwd) |
| `delphi studio` | Open Delphi Studio (terminal notebook IDE) |
| `delphi studio <name>` | Open a project or `.dstudio` file in Studio |
| `delphi examples` | List bundled example scripts |
| `delphi examples <name>` | Run a bundled example |
| `delphi examples --copy <name>` | Copy an example to current directory |
| `delphi init <name>` | Create a new Delphi project |
| `delphi open [path]` | Open a project directory in the REPL |
| `delphi run <file>` | Run a `.delphi` script (checks bundled examples first) |
| `delphi <file>` | Run a script (shorthand) |
| `delphi --version` | Show version |
| `delphi --help` | Show help |

### Switching instruments

By default, `play()` uses piano. Switch to any of the 128 GM instruments:

```
𝄞 instrument("violin")
𝄞 C4:q E4:q G4:q C5:h
  ♪ Playing...

𝄞 instrument("flute")
𝄞 G5:8 A5:8 B5:8 D6:q
  ♪ Playing...

𝄞 instrument("piano")     # switch back
𝄞 instruments              # list all 128 GM instruments
```

This works in scripts too:

```python
from delphi import *

instrument("trumpet")
play("C4:q E4:q G4:q C5:h")

instrument("acoustic guitar nylon")
play("| Am | Em | F | G |")
```

### Your first script

Create `hello.delphi`:

```python
from delphi import *

tempo(120)
key("C major")

play("C4 E4 G4 C5")
play("| C | Am | F | G |")
export("hello.mid", "| C | Am | F | G |")
```

Run it:

```bash
delphi hello.delphi
```

### Delphi Studio

A terminal notebook IDE for composing multi-track songs — think Jupyter meets a DAW.

```bash
delphi studio                    # New empty notebook
delphi studio my-song            # Open project as notebook
delphi studio showcase           # Open bundled showcase example
delphi studio song.dstudio       # Open saved notebook file
```

```
┌─────────────────────────────────────────────────────┐
│  🎵 Delphi Studio — my-song          ♩=120  C major │
├─────────────────────────────────────────────────────┤
│ ▶ ▾ [1] ⌨ Setup                                     │
│   tempo(120)                                        │
│   key("C major")                                    │
│   ♪ OK                                               │
│ ──────────────────────────────────────────────────── │
│   ▾ [2] ♪ Melody (piano)                             │
│   # @track melody @program piano                    │
│   C4:q E4:q G4:q C5:h                              │
│   ♪ 4 notes, 2.0 bars [piano]                       │
│ ──────────────────────────────────────────────────── │
│   ▾ [3] ♪ Bass (acoustic bass)                       │
│   # @track bass @program acoustic bass              │
│   C2:h G2:h  F2:h C2:h                             │
│   ♪ 4 notes, 2.0 bars [acoustic bass]               │
├─────────────────────────────────────────────────────┤
│ F1:Help  F5:Run  F6:RunAll  F7:Add  F8:Del          │
│ F9:Export  F10:Save  Ctrl+P:Replay  Ctrl+Q:Quit     │
└─────────────────────────────────────────────────────┘
```

| Key | Action |
|-----|--------|
| F5 | Run current cell |
| F6 | Run all cells (builds Song, plays multi-track) |
| F7 / Ctrl+B | Insert cell below |
| F8 | Delete cell (with confirmation) |
| F9 | Export MIDI + WAV + .delphi script |
| F10 / Ctrl+S | Save .dstudio notebook |
| Ctrl+↑/↓ | Navigate between cells |
| Ctrl+Shift+↑/↓ | Reorder cells |
| Ctrl+T | Cycle cell type (code → notation → markdown) |
| Ctrl+E | Collapse/expand cell |
| Ctrl+P | Replay last cell |
| Ctrl+C | Stop playback |
| Ctrl+Q | Quit |

Notation cells use `# @pragma` comments to assign instruments:

```
# @track melody @program violin @velocity 90
G4:q A4:q B4:q D5:h
F#4:q G4:q A4:q B4:h
```

## Documentation

| Guide | Description |
|-------|-------------|
| [Notation Reference](docs/notation.md) | Complete syntax for notes, chords, rhythms, dynamics, and more |
| [Song & Track API](docs/songs-and-tracks.md) | Building multi-track songs with effects |
| [Composition Guide](docs/composition.md) | Patterns, Sections, Arrangements for large pieces |
| [Theory Module](docs/theory.md) | Scales, chords, notes — music theory as code |
| [REPL Guide](docs/repl.md) | Interactive environment features and keybindings |
| [Studio Guide](docs/studio.md) | Terminal notebook IDE for multi-track composition |
| [Jupyter Notebooks](docs/notebook.md) | Using Delphi in Jupyter with inline audio |
| [Examples](examples/) | Complete example scripts |

## Examples

### Twinkle Twinkle Little Star (2 tracks)

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

### 12-Bar Blues

```python
from delphi import *

tempo(110)
key("A blues")

blues = """
| A7 | A7 | A7 | A7 |
| D7 | D7 | A7 | A7 |
| E7 | D7 | A7 | E7 |
"""

play(blues)
export("blues.mid", blues)
```

### Composition with Sections

```python
from delphi import *

verse = Section("verse")
verse.add("melody", "C4:q D4:q E4:q F4:q  G4:h E4:h", program="piano")
verse.add("bass", "C2:h G2:h  F2:h C2:h", program="acoustic bass")

chorus = Section("chorus")
chorus.add("melody", "G4:q A4:q G4:q E4:q  C4:w", program="piano")
chorus.add("bass", "C2:h F2:h  G2:h C2:h", program="acoustic bass")

arr = Arrangement("My Song", tempo=120, key="C major")
arr.section(verse, repeat=2)
arr.mark("chorus")
arr.section(chorus)
arr.section(verse)
arr.section(chorus, repeat=2)

song = arr.build()
song.play()
song.export("my_song.mid")
```

### Jupyter Notebook

```python
%load_ext delphi.notebook
```

```python
%%delphi
C4:q E4:q G4:q C5:h
```

```python
%%delphi --program piano --tempo 120
| Cmaj7 | Am7 | Fmaj7 | G7 |
```

```python
song = Song("My Song", tempo=120)
song.track("piano", "C4:q E4:q G4:h", program="piano")
song.to_audio()   # inline audio player
```

Install notebook support:

```bash
source ~/.local/share/delphi/venv/bin/activate
pip install ipython jupyter
```

## Project Structure

```
delphi/
├── python/delphi/        # Python DSL & API
│   ├── __init__.py       # Public exports
│   ├── notation.py       # Music notation parser
│   ├── song.py           # Song & Track classes
│   ├── composition.py    # Patterns, Sections, Arrangements
│   ├── theory.py         # Scales, chords, notes
│   ├── context.py        # Global tempo/key/time signature
│   ├── playback.py       # Audio playback
│   ├── export.py         # MIDI/WAV export
│   ├── soundfont.py      # SoundFont management
│   ├── repl.py           # Interactive REPL
│   ├── studio.py         # Delphi Studio TUI notebook IDE
│   ├── notebook.py       # Jupyter/IPython extension
│   └── cli.py            # Command-line interface
├── crates/               # Rust engine
│   ├── delphi-core/      # Music theory types
│   ├── delphi-midi/      # MIDI file export
│   ├── delphi-engine/    # Audio synthesis & scheduling
│   └── delphi-py/        # PyO3 Python bindings
├── delphi                # ← Dev launcher script (builds from source)
├── deploy/               # End-user install scripts
│   ├── install.sh        # Universal OS-detecting dispatcher
│   ├── install-linux.sh  # Linux installer
│   ├── install-macos.sh  # macOS installer
│   └── install-windows.ps1  # Windows installer
├── examples/             # Example scripts
│   ├── hello.delphi
│   ├── twinkle.delphi
│   ├── blues.delphi
│   └── canon.delphi      # Pachelbel's Canon in D (4 tracks)
└── tests/
    └── python/           # Python test suite
```

## Development

If you want to contribute or build from source, you'll need:

- Python ≥ 3.10
- Rust toolchain (install via [rustup](https://rustup.rs))
- Linux: ALSA development headers (`sudo apt install libasound2-dev`)

The `./delphi` launcher handles everything automatically:

```bash
git clone https://github.com/SharkySeph/delphi.git
cd delphi
./delphi --version
```

It will create a `.venv/`, install `maturin`, build the Rust engine, and run the CLI. No manual setup needed.

For manual builds:

```bash
python -m venv .venv
source .venv/bin/activate
pip install maturin
maturin develop

# Run Python tests
python -m pytest tests/python/ -v

# Run Rust tests
cargo test
```

## License

MIT
