# 🎵 Delphi

**A music composition environment — write music as code, hear it instantly.**

Delphi lets you compose real, multi-track music using a concise notation language. It ships as two pure Rust binaries:

- **Delphi Studio** — A native GUI application with a notebook-style editor, piano roll, mixer, real-time visualizer, and SoundFont-powered playback.
- **`delphi` CLI** — A command-line tool for exporting, playing, and inspecting `.dstudio` project files.

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
- Full notation parser: notes, chords (40+ qualities), Roman numeral chords, scale degree notes, key changes, rests, ties, tuplets, subdivisions, dynamics, articulations, ornaments, Euclidean rhythms, drum patterns, structural markers, and more
- Auto-complete (Ctrl+Space) with 80+ suggestions
- Live diagnostics — warnings for unknown tokens as you type
- Export to MIDI and WAV
- Save/load `.dstudio` project files and `.delphi` scripts
- 6 bundled examples (Hello World, Twinkle Twinkle, 12-Bar Blues, Canon in D, Digital Shrine, Studio Showcase)

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

## `delphi` CLI

A command-line tool for batch operations, scripting, and headless workflows.

```bash
# Export a project to MIDI and WAV
delphi export my_song.dstudio --format both --output ./out/

# Play a project through the audio output
delphi play my_song.dstudio

# Show project info (title, tempo, tracks, cells)
delphi info my_song.dstudio

# Create a new empty project
delphi new my_song
```

| Command | Description |
|---------|-------------|
| `delphi export <file>` | Export to MIDI/WAV/both |
| `delphi play <file>` | Play through audio output |
| `delphi info <file>` | Show project information |
| `delphi new <name>` | Create a new .dstudio file |
| `delphi version` | Show version |
| `delphi help` | Show usage |

## Installation

### From .deb package (Debian/Ubuntu)

Download from [GitHub Releases](https://github.com/SharkySeph/delphi/releases) and install:

```bash
sudo dpkg -i delphi_0.8.0-1_amd64.deb
```

This installs both `delphi` (CLI) and `delphi-studio` (GUI).

### Build from source (any platform)

Requires Rust toolchain ([rustup](https://rustup.rs)) and ALSA headers on Linux (`sudo apt install libasound2-dev`).

```bash
git clone https://github.com/SharkySeph/delphi.git
cd delphi
cargo build --release
# Binaries at target/release/delphi and target/release/delphi-studio
```

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
| I | IV | V7 | I |           # Roman numeral chords (key-relative)
^1:q ^3:q ^5:q ^1:q           # scale degree notes
trill(C4:q)                   # ornaments
e(3,8)                        # Euclidean rhythms
```

Pragmas configure tracks:

```
# @track melody @program violin @velocity 90 @channel 1
# @tempo 120 @key D major @time 3/4
```

See [Notation Reference](docs/notation.md) for the complete syntax.

## Features

- **Delphi Studio GUI** — Native desktop app: notebook editor, piano roll, mixer, visualizer, theory explorer
- **CLI tool** — Export, play, and inspect projects from the command line
- **Expressive notation** — Notes, chords, Roman numeral chords, scale degree notes, key changes, rests, dynamics, articulations, ornaments, Euclidean rhythms, ties, tuplets, crescendo/diminuendo, drum patterns
- **50+ scales** — Church modes, pentatonic, blues, bebop, altered, exotic (Hungarian, Persian, Japanese, Egyptian…)
- **40+ chord qualities** — Major, minor, diminished, augmented, sus, add, extended, altered, slash chords
- **Multi-track songs** — Layer instruments with per-track gain, pan, mute, solo
- **128 GM instruments** — Piano, strings, brass, woodwinds, drums, synths, and more
- **SoundFont synthesis** — Realistic instrument sounds via GeneralUser GS (auto-downloaded)
- **MIDI & WAV export** — Standard MIDI files and rendered audio
- **Rhai scripting** — Automate the Studio with a built-in script engine
- **Rust-powered** — Fast audio synthesis and MIDI scheduling, no runtime dependencies

## Documentation

| Guide | Description |
|-------|-------------|
| [Notation Reference](docs/notation.md) | Complete syntax for notes, chords, rhythms, dynamics |
| [Studio Guide](docs/studio.md) | GUI studio usage |
| [Song & Track Guide](docs/songs-and-tracks.md) | Building multi-track songs with effects |
| [Composition Guide](docs/composition.md) | Patterns, Sections, Arrangements for large pieces |
| [Theory Reference](docs/theory.md) | Scales, chords, notes — music theory types |
| [Examples](examples/) | Complete example scripts |

## Examples

Open the bundled examples from **File → Examples** in Delphi Studio — includes Hello World, Twinkle Twinkle Little Star, 12-Bar Blues, Pachelbel's Canon in D, Digital Shrine, and the Studio Showcase.

Or export from the CLI:

```bash
delphi export examples/solo_piano.dstudio --output ./out/
```

## Project Structure

```
delphi/
├── crates/
│   ├── delphi-core/      # Music theory, notation parser, project model
│   ├── delphi-engine/    # Audio synthesis & SoundFont playback
│   ├── delphi-midi/      # MIDI file export
│   ├── delphi-cli/       # `delphi` CLI binary
│   └── delphi-gui/       # Delphi Studio (egui/eframe)
├── deploy/               # Build & install scripts
├── examples/             # Bundled .delphi and .dstudio examples
└── docs/                 # Documentation
```

## Development

```bash
git clone https://github.com/SharkySeph/delphi.git
cd delphi
cargo build --release
cargo test
```

**Requirements:** Rust toolchain ([rustup](https://rustup.rs)). On Linux: ALSA headers (`sudo apt install libasound2-dev`).

## License

MIT
