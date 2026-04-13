# Songs & Tracks

Build multi-track compositions in Delphi using `.dstudio` project files. Each track has its own instrument, notation, MIDI channel, and mixer settings.

## Table of Contents

- [Project Structure](#project-structure)
- [Creating Tracks](#creating-tracks)
- [Mixer Settings](#mixer-settings)
- [Export](#export)
- [GM Instruments](#gm-instruments)

---

## Project Structure

A Delphi project (`.dstudio` file) contains:

- **Settings** — Title, tempo, key, time signature, swing, humanize
- **Cells** — Notation, code, or markdown cells
- **Tracks** — Per-instrument mixer settings (gain, pan, reverb, delay, mute/solo)

Each notation cell maps to a track. Code cells set global parameters. Markdown cells are notes.

### Creating a Project

In Delphi Studio, use **File → New** or press **Ctrl+N**.

From the CLI:

```bash
delphi new my_song
```

## Creating Tracks

Each notation cell becomes a track. Use pragmas to configure the instrument and channel:

```
# @track Melody
# @instrument violin
# @velocity 90

G4:q A4:q B4:q D5:h
F#4:q G4:q A4:q B4:h
```

```
# @track Bass
# @instrument acoustic bass
# @velocity 65

C2:h G2:h F2:h C2:h
```

```
# @track Drums
# @channel 9

{bd(3,8) sd(2,8) hh(5,8)}
```

MIDI channels are auto-assigned to avoid conflicts. Channel 9 is reserved for drums.

### Chaining Multiple Cells

Use a code cell at the top to set global parameters:

```
tempo(110)
key("G major")
time_sig(4, 4)
swing(0.2)
```

Then add notation cells for each part. Press **F5** in Delphi Studio or use `delphi play` from the CLI.

## Mixer Settings

In Delphi Studio, the **Tracks** panel provides per-track controls:

| Control | Range | Description |
|---------|-------|-------------|
| **Gain** | 0.0 – 1.5 | Volume level |
| **Pan** | 0.0 – 1.0 | Stereo position (0.0=left, 0.5=center, 1.0=right) |
| **Reverb** | 0.0 – 1.0 | Reverb send amount |
| **Delay** | 0.0 – 1.0 | Delay send amount |
| **Mute** | on/off | Silence the track |
| **Solo** | on/off | Solo the track (mutes all others) |

These settings are persisted in the `.dstudio` file and applied during both playback and export.

### Effects in MIDI Export

Mixer settings are written as MIDI Control Change events:

| Effect | MIDI CC | Range |
|--------|---------|-------|
| Pan | CC #10 | 0–127 (0.5 → 64) |
| Gain | CC #7 | 0–127 (1.0 → 100) |
| Reverb | CC #91 | 0–127 |
| Delay | CC #93 | 0–127 |

## Export

### From Delphi Studio

Use **File → Export** and choose MIDI or WAV format.

### From the CLI

```bash
# Export both MIDI and WAV
delphi export my_song.dstudio --format both --output ./out/

# Export MIDI only
delphi export my_song.dstudio --format midi

# Export WAV with a specific SoundFont
delphi export my_song.dstudio --format wav --sf ~/soundfonts/my_font.sf2
```

## GM Instruments

Delphi supports all 128 General MIDI instruments. Use the string name (case-insensitive) in the `@instrument` or `@program` pragma.

### Instrument List

| Category | Instruments |
|----------|-------------|
| **Piano** | `piano` (0), `bright piano` (1), `electric piano` / `epiano` (4), `harpsichord` (6), `clavinet` (7) |
| **Chromatic Percussion** | `glockenspiel` (9), `vibraphone` (11), `marimba` (12), `xylophone` (13), `tubular bells` (14) |
| **Organ** | `organ` (19), `church organ` (19), `reed organ` (20), `accordion` (21), `harmonica` (22) |
| **Guitar** | `nylon guitar` (24), `acoustic guitar` / `steel guitar` (25), `jazz guitar` (26), `clean guitar` / `electric guitar` (27), `muted guitar` (28), `overdriven guitar` (29), `distortion guitar` (30) |
| **Bass** | `acoustic bass` (32), `electric bass` / `finger bass` (33), `pick bass` (34), `fretless bass` (35), `slap bass` (36), `synth bass` (38) |
| **Strings** | `violin` (40), `viola` (41), `cello` (42), `contrabass` (43), `tremolo strings` (44), `pizzicato strings` (45), `harp` (46), `timpani` (47), `strings` / `string ensemble` (48), `slow strings` (49), `synth strings` (50) |
| **Choir** | `choir` (52), `voice` (53), `synth voice` (54), `orchestra hit` (55) |
| **Brass** | `trumpet` (56), `trombone` (57), `tuba` (58), `muted trumpet` (59), `french horn` / `horn` (60), `brass` (61), `synth brass` (62) |
| **Reeds** | `soprano sax` (64), `alto sax` (65), `tenor sax` (66), `baritone sax` (67), `oboe` (68), `english horn` (69), `bassoon` (70), `clarinet` (71) |
| **Flute** | `piccolo` (72), `flute` (73), `recorder` (74), `pan flute` (75), `whistle` (78), `ocarina` (79) |
| **Synth** | `square lead` (80), `saw lead` / `synth lead` (81), `pad` (88), `warm pad` (89), `polysynth` (90) |
| **World** | `sitar` (104), `banjo` (105), `shamisen` (106), `koto` (107), `kalimba` (108), `bagpipe` (109), `fiddle` (110) |

---

## Full Example

A complete multi-track project:

**Code cell:**
```
tempo(90)
key("D minor")
time_sig(4, 4)
swing(0.1)
humanize(0.05)
```

**Notation cell: Keys**
```
# @instrument electric piano
# @track Keys
# @velocity 70

| Dm7 | Am7 | Bbmaj7 | A7 |
```

**Notation cell: Bass**
```
# @instrument fretless bass
# @track Bass
# @velocity 75

D2:q . D2:8 D2:8  A2:q . A2:8 A2:8
Bb2:q . Bb2:8 Bb2:8  A2:q . A2:8 A2:8
```

**Notation cell: Drums**
```
# @channel 9
# @track Drums
# @velocity 80

{bd(3,8) sd(2,8) hh(5,8) oh(1,8)}
```
