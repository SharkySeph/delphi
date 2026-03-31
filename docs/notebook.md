# Jupyter Notebook Support

Delphi includes an IPython extension that brings music scripting into Jupyter notebooks with inline audio playback.

## Installation

```bash
pip install delphi[notebook]
```

Or if you already have Delphi installed:

```bash
pip install ipython jupyter
```

## Getting Started

In a Jupyter notebook, load the extension:

```python
%load_ext delphi.notebook
```

This does three things:
1. Registers `%%delphi` cell magic and `%delphi_*` line magics
2. Imports all Delphi functions into the notebook namespace
3. Adds a `.to_audio()` method to `Song` objects

## Cell Magic: `%%delphi`

Write notation in a cell and hear it play inline:

```python
%%delphi
C4:q E4:q G4:q C5:h
```

### Options

Pass flags on the magic line:

```python
%%delphi --tempo 140 --program piano
| Cmaj7 | Am7 | Fmaj7 | G7 |
```

| Flag | Description |
|------|-------------|
| `--tempo <bpm>` | Override the global tempo |
| `--program <name>` | GM instrument name or number |
| `--velocity <0-127>` | Default note velocity |
| `--no-autoplay` | Return audio widget without auto-playing |

## Line Magics

### `%delphi_play`

Play a short snippet inline:

```python
%delphi_play C4 E4 G4 C5
```

### `%delphi_tempo`

Set the global tempo:

```python
%delphi_tempo 120
```

### `%delphi_key`

Set the global key:

```python
%delphi_key D minor
```

### `%delphi_time_sig`

Set the time signature:

```python
%delphi_time_sig 3 4
```

### `%delphi_instruments`

List all 128 GM instruments:

```python
%delphi_instruments
```

### `%delphi_context`

Show current context (tempo, key, time signature, swing, humanize):

```python
%delphi_context
```

## Python API in Notebooks

Since the extension imports everything, you can use the full Delphi API directly:

```python
tempo(120)
key("C major")

# Play inline
play("C4:q E4:q G4:h")
```

### Songs with Inline Audio

Use `.to_audio()` on any Song to get an embedded audio player:

```python
song = Song("Blues", tempo=110, key="A blues")
song.track("piano", """
    | A7 | A7 | A7 | A7 |
    | D7 | D7 | A7 | A7 |
    | E7 | D7 | A7 | E7 |
""", program="piano")

song.to_audio()  # Returns IPython Audio widget
```

### Composition Tools

```python
verse = Section("verse")
verse.add("melody", "C4:q D4:q E4:q F4:q G4:h E4:h", program="piano")
verse.add("bass", "C2:h G2:h F2:h C2:h", program="acoustic bass")

chorus = Section("chorus")
chorus.add("melody", "G4:q A4:q G4:q E4:q C4:w", program="piano")
chorus.add("bass", "C2:h F2:h G2:h C2:h", program="acoustic bass")

arr = Arrangement("My Song", tempo=120, key="C major")
arr.section(verse, repeat=2)
arr.section(chorus)

song = arr.build()
song.to_audio()
```

### Audio Helper Function

`delphi_audio()` renders notation directly to an Audio widget:

```python
widget = delphi_audio("C4:q E4:q G4:h", tempo=90, program=0)
widget  # Display in cell output
```

## Tips

- **Autoplay**: By default, `%%delphi` cells auto-play. Use `--no-autoplay` to return a silent widget.
- **Export alongside**: You can still use `song.export("file.mid")` for MIDI files and `song.render("file.wav")` for WAV files.
- **SoundFont**: The first playback will auto-download GeneralUser GS (~30 MB). You can set a custom SoundFont with `set_soundfont("/path/to/sf2")`.
- **Scales & theory**: Use `scale("C dorian")`, `chord("Cmaj7")`, `note("C4")` for theory exploration.
