# REPL Guide

Delphi's interactive REPL (Read-Eval-Play Loop) gives you a live environment for writing and hearing music instantly.

## Launching the REPL

```bash
delphi
```

You'll see:

```
🎵 Delphi — Music Scripting Language
   Type 'help' for commands, 'quit' to exit.
   Tip: type notation directly to hear it!

🎵 >
```

## Inline Playback

The REPL detects music notation and plays it automatically — no need to wrap it in `play()`.

```
🎵 > C4 E4 G4 C5
  ♪ Playing...

🎵 > | Am7 | Dm7 | G7 | Cmaj7 |
  ♪ Playing chord progression...

🎵 > bd(3,8) sd(2,8) hh(5,8)
  ♪ Playing...
```

Anything that looks like notation (contains note names, chord names, drum names, or bar lines) is auto-played. You can also use explicit `play()`:

```
🎵 > play("C4:q E4:q G4:q C5:h")
```

## Python Evaluation

The REPL is also a full Python environment. Any input that isn't music notation is evaluated as Python:

```
🎵 > tempo(140)
🎵 > key("D minor")

🎵 > song = Song("Test", tempo=120)
🎵 > song.track("piano", "C4:q E4:q G4:q", program="piano")
🎵 > song.play()

🎵 > scale("C", "blues").play()
🎵 > chord("Am7").arpeggio("up").play()
```

## Auto-Completion

The REPL provides context-aware tab completion for:

| Category | Examples |
|----------|---------|
| **Notes** | `C4`, `F#5`, `Bb3` (all pitches, octaves 0-8) |
| **Chords** | `Cmaj7`, `Am7`, `Dm7b5`, `G7sus4` |
| **Drums** | `kick`, `snare`, `hihat`, `crash`, `ride` |
| **Durations** | `w`, `h`, `q`, `8`, `16`, `32`, `q.`, `8t` |
| **Dynamics** | `ppp`, `pp`, `p`, `mp`, `mf`, `f`, `ff`, `fff` |
| **Articulations** | `.stac`, `.acc`, `.marc`, `.ferm`, `.ghost` (after a note) |
| **Ornaments** | `.tr`, `.mord`, `.turn`, `.grace`, `.trem` (after a note) |
| **Functions** | `play`, `export`, `tempo`, `Song`, `Track`, etc. |
| **Instruments** | `piano`, `violin`, `acoustic bass`, `trumpet`, etc. |
| **Scales** | `dorian`, `phrygian`, `blues`, `whole tone`, etc. |
| **Keys** | `C major`, `A minor`, `F# minor`, etc. |

### Articulation/Ornament Completion

Type a note followed by `.` and press Tab to see available articulations and ornaments:

```
🎵 > C4.  [Tab]
stac  stacc  ten  port  acc  marc  ferm  ghost  leg  pizz  mute
tr  mord  lmord  turn  grace  appoggiatura  trem  gliss  arp  roll
```

## Key Bindings

| Key | Action |
|-----|--------|
| `Tab` | Auto-complete |
| `Ctrl+P` | Re-play the last notation |
| `Ctrl+C` | Stop playback / cancel input |
| `Ctrl+D` | Exit the REPL |
| `↑` / `↓` | Navigate command history |

## Commands

| Command | Description |
|---------|-------------|
| `help` | Show the full help text |
| `docs` | List all quick-reference topics |
| `docs <topic>` | Show docs for a topic (e.g. `docs drums`, `docs layers`) |
| `quit` / `exit` | Exit the REPL |
| `instruments` | List all GM instrument names |
| `sf` | Show SoundFont configuration |

## Syntax Suggestions

As you type, Delphi shows ghost-text hints for what you could type next:

- After a note (`C4`) → suggests a duration (`:q`)
- After a drum name (`kick`) → suggests Euclidean syntax (`(3,8)`)
- After a bar pipe (`|`) → suggests a chord (`Cmaj7 |`)
- After `play(` → suggests a notation string
- After an opening `{` → suggests a drum layer pattern

Press **→** (right arrow) to accept the suggestion, or keep typing to ignore it. History-based suggestions are used as a fallback.

## Syntax Highlighting

The REPL uses Pygments for syntax highlighting (when available). Note names, chord symbols, dynamics, and Python keywords are colored for readability.

## History

Command history is automatically saved between sessions. Use ↑/↓ arrows to recall previous inputs.

## Available Functions

All of these are available in the REPL namespace:

### Playback & Export
```python
play(notation)                    # Parse and play notation
play(notation, channel=2)         # Override MIDI channel
play(notation, instrument="flute")  # Override instrument
play_notes(tuples)                # Play raw MIDI tuples
export(path, notation)            # Export to MIDI or WAV
```

> **SoundFont-first:** `play()` uses SoundFont playback by default for all instruments, including drums (auto-routed to MIDI channel 9). If no SoundFont is installed, it falls back to a basic oscillator synth. Run `ensure_soundfont()` to download the default SoundFont.

### Context
```python
tempo(bpm)                        # Set tempo
key(name)                         # Set key ("C major", "A minor")
time_sig(num, den)                # Set time signature
swing(amount)                     # Set swing (0-1)
humanize(amount)                  # Set humanize (0-1)
get_context()                     # View current settings
reset_context()                   # Reset to defaults
```

### Theory
```python
note(name)                        # Create a note object
chord(name)                       # Create a chord object
scale(root, type)                 # Create a scale object
```

### Song Building
```python
Song(title, tempo, key, time_sig) # Create a song
Track(name, notation, program)    # Create a track
GM_INSTRUMENTS                    # Dict of instrument names
```

### Composition
```python
Section(name)                     # Create a section
Pattern(name, notation)           # Create a pattern
Voice(name, program, velocity)    # Create a voice
Arrangement(title, tempo, key)    # Create an arrangement
PatternLibrary()                  # Create a pattern library
build_song_from_sections(...)     # Build song from section list
register_pattern(name, notation)  # Register a global pattern
get_pattern(name)                 # Retrieve a global pattern
list_patterns()                   # List global patterns
include(path)                     # Import from another file
```

### SoundFont
```python
ensure_soundfont()                # Download SoundFont if needed
soundfont_info()                  # Show current SoundFont config
set_soundfont(path)               # Set a custom SoundFont
```

---

## Example Session

```
🎵 Delphi — Music Scripting Language
   Type 'help' for commands, 'quit' to exit.
   Tip: type notation directly to hear it!

🎵 > tempo(100)
🎵 > key("G major")

🎵 > G4 A4 B4 D5
  ♪ Playing...

🎵 > | G | Em | C | D |
  ♪ Playing chord progression...

🎵 > scale("G", "major pentatonic").play()

🎵 > song = Song("Quick Demo", tempo=100, key="G major")
🎵 > song.track("melody", "G4:q B4:q D5:q G5:h", program="flute")
🎵 > song.track("bass", "G2:h D2:h", program="acoustic bass")
🎵 > song.play()

🎵 > song.export("demo.mid")

🎵 > quit
```
