# 🎵 Delphi

**A music scripting language for composing real music.**

Delphi is a hybrid Rust + Python DSL that lets you write music as code. Describe melodies with a concise notation syntax, build multi-track songs with effects, and hear them instantly — all from your terminal.

```python
from delphi import *

tempo(120)
key("C major")

# Play a melody
play("C4:q E4:q G4:q C5:h")

# Play a chord progression
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
- **SoundFont synthesis** — Realistic instrument sounds via GeneralUser GS (auto-downloaded)
- **MIDI & WAV export** — Standard MIDI files and rendered audio
- **128 GM instruments** — Piano, strings, brass, woodwinds, drums, synths, and more
- **Rust-powered engine** — Fast audio synthesis and scheduling via PyO3

## Installation

### Requirements

- Python ≥ 3.10
- Rust toolchain (for building the native engine)
- Linux: ALSA development libraries (`sudo apt install libasound2-dev`)

### From source

```bash
git clone https://github.com/SharkySeph/delphi.git
cd delphi
./delphi --version
```

That's it. The `./delphi` launcher script automatically:
1. Creates a Python virtual environment (`.venv/`) if one doesn't exist
2. Installs `maturin` if missing
3. Builds the Rust engine when source files have changed
4. Installs `prompt_toolkit` for the REPL
5. Runs the command

No manual `source .venv/bin/activate` or `maturin develop` needed.

> **Tip:** Add the repo directory to your `PATH` to run `delphi` from anywhere,
> or create a symlink: `ln -s /path/to/delphi/delphi ~/.local/bin/delphi`

### Manual setup (alternative)

If you prefer to manage the environment yourself:

```bash
git clone https://github.com/SharkySeph/delphi.git
cd delphi
python -m venv .venv
source .venv/bin/activate
pip install maturin
maturin develop
delphi --version
```

## Quick Start

### Launch the REPL

```bash
./delphi
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
./delphi init my-song
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
./delphi main.delphi
```

Or open the project in the REPL (loads settings from `delphi.toml`):

```bash
./delphi open .
```

### Run a script

```bash
./delphi examples/twinkle.delphi
```

### CLI commands

| Command | Description |
|---------|-------------|
| `./delphi` | Launch REPL (auto-detects project in cwd) |
| `./delphi init <name>` | Create a new Delphi project |
| `./delphi open [path]` | Open a project directory in the REPL |
| `./delphi run <file>` | Run a `.delphi` script |
| `./delphi <file>` | Run a script (shorthand) |
| `./delphi --version` | Show version |
| `./delphi --help` | Show help |

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
./delphi hello.delphi
```

## Documentation

| Guide | Description |
|-------|-------------|
| [Notation Reference](docs/notation.md) | Complete syntax for notes, chords, rhythms, dynamics, and more |
| [Song & Track API](docs/songs-and-tracks.md) | Building multi-track songs with effects |
| [Composition Guide](docs/composition.md) | Patterns, Sections, Arrangements for large pieces |
| [Theory Module](docs/theory.md) | Scales, chords, notes — music theory as code |
| [REPL Guide](docs/repl.md) | Interactive environment features and keybindings |
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

Install notebook support: `pip install delphi[notebook]`

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
│   ├── notebook.py       # Jupyter/IPython extension
│   └── cli.py            # Command-line interface
├── crates/               # Rust engine
│   ├── delphi-core/      # Music theory types
│   ├── delphi-midi/      # MIDI file export
│   ├── delphi-engine/    # Audio synthesis & scheduling
│   └── delphi-py/        # PyO3 Python bindings
├── delphi                # ← Launcher script (start here)
├── examples/             # Example scripts
│   ├── hello.delphi
│   ├── twinkle.delphi
│   ├── blues.delphi
│   └── canon.delphi      # Pachelbel's Canon in D (4 tracks)
└── tests/
    └── python/           # Python test suite
```

## Building & Testing

The `./delphi` launcher handles building automatically. For manual builds:

```bash
source .venv/bin/activate
maturin develop

# Run Python tests
python -m pytest tests/python/ -v

# Run Rust tests
cargo test
```

## License

MIT
