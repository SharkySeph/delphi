# Theory Module

Delphi includes a music theory module for working with notes, chords, and scales as first-class objects. These can be inspected, transposed, and played.

## Table of Contents

- [Notes](#notes)
- [Chords](#chords)
- [Scales](#scales)
- [Using Theory in Compositions](#using-theory-in-compositions)

---

## Notes

Create note objects to inspect MIDI values, transpose, and play single pitches.

```python
from delphi import note

n = note("C4")
print(n)          # C4
print(n.midi)     # 60

# Transpose
up = n.transpose(7)    # G4 (up a perfect 5th)
down = n.transpose(-3) # A3 (down a minor 3rd)

# Play it
n.play()
```

### NoteObj API

| Property/Method | Returns | Description |
|----------------|---------|-------------|
| `.midi` | `int` | MIDI note number (0-127) |
| `transpose(semitones)` | `NoteObj` | New note shifted by semitones |
| `play()` | `None` | Play the note through speakers |

## Chords

Create chord objects to inspect their notes, arpeggiate, and play them.

```python
from delphi import chord

c = chord("Cmaj7")
print(c)              # chord('Cmaj7')
print(c.midi_notes)   # [60, 64, 67, 71]

# Get individual notes
for n in c.notes():
    print(n)          # C4, E4, G4, B4

# Create an arpeggio
arp = c.arpeggio(direction="up", duration="eighth")
arp.play()

# Play the chord
c.play()
```

### ChordObj API

| Property/Method | Returns | Description |
|----------------|---------|-------------|
| `.midi_notes` | `list[int]` | MIDI note numbers |
| `notes()` | `list[NoteObj]` | Individual note objects |
| `arpeggio(direction="up", duration="eighth")` | `ArpeggioObj` | Arpeggiated version |
| `play()` | `None` | Play the chord |

### Arpeggio Directions

```python
chord("Am7").arpeggio("up")      # Low to high
chord("Am7").arpeggio("down")    # High to low
```

### Supported Chord Qualities

| Input | Quality |
|-------|---------|
| `C`, `Cmaj` | Major |
| `Cm`, `Cmin` | Minor |
| `Cdim`, `C°` | Diminished |
| `Caug`, `C+` | Augmented |
| `C5` | Power chord |
| `C7` | Dominant 7th |
| `Cmaj7` | Major 7th |
| `Cm7`, `Cmin7` | Minor 7th |
| `Cdim7`, `C°7` | Diminished 7th |
| `Cm7b5`, `Cø7`, `Cø` | Half-diminished 7th |
| `CmMaj7`, `CmM7` | Minor-major 7th |
| `Caug7` | Augmented 7th |
| `Csus2` | Suspended 2nd |
| `Csus4`, `Csus` | Suspended 4th |
| `C6`, `Cm6` | 6th chords |
| `C69`, `Cm69` | 6/9 chords |
| `C9`, `Cmaj9`, `Cm9` | 9th chords |
| `C11`, `Cmaj11`, `Cm11` | 11th chords |
| `C13`, `Cmaj13`, `Cm13` | 13th chords |
| `Cadd9`, `Cadd11`, `Cadd2` | Added-tone chords |
| `C7sus4`, `C7sus2` | 7th suspended |
| `Calt` | Altered dominant |

## Scales

Create scale objects to explore modes, exotic scales, and more. Over 40 scale types are available.

```python
from delphi import scale

s = scale("C", "dorian")
print(s)                # scale('C', 'dorian')
print(s.midi_notes)     # [60, 62, 63, 65, 67, 69, 70]

# Get note objects
for n in s.notes():
    print(n)            # C4, D4, Eb4, F4, G4, A4, Bb4

# Play the scale
s.play()
```

### ScaleObj API

| Property/Method | Returns | Description |
|----------------|---------|-------------|
| `.midi_notes` | `list[int]` | MIDI note numbers |
| `notes()` | `list[NoteObj]` | Individual note objects |
| `play()` | `None` | Play the scale ascending |

### Scale Types

#### Church Modes

| Name | Intervals | Character |
|------|-----------|-----------|
| `major` / `ionian` | 1 2 3 4 5 6 7 | Bright, happy |
| `dorian` | 1 2 ♭3 4 5 6 ♭7 | Minor with bright 6th |
| `phrygian` | 1 ♭2 ♭3 4 5 ♭6 ♭7 | Spanish, dark |
| `lydian` | 1 2 3 ♯4 5 6 7 | Bright, dreamy |
| `mixolydian` | 1 2 3 4 5 6 ♭7 | Bluesy major |
| `minor` / `aeolian` | 1 2 ♭3 4 5 ♭6 ♭7 | Sad, natural minor |
| `locrian` | 1 ♭2 ♭3 4 ♭5 ♭6 ♭7 | Dark, unstable |

#### Minor Variants

| Name | Intervals | Character |
|------|-----------|-----------|
| `natural minor` | 1 2 ♭3 4 5 ♭6 ♭7 | Same as aeolian |
| `harmonic minor` | 1 2 ♭3 4 5 ♭6 7 | Classical minor, augmented 2nd |
| `melodic minor` | 1 2 ♭3 4 5 6 7 | Jazz minor (ascending) |

#### Pentatonic & Blues

| Name | Notes | Character |
|------|-------|-----------|
| `pentatonic` / `major pentatonic` | 1 2 3 5 6 | Universal, pleasant |
| `minor pentatonic` | 1 ♭3 4 5 ♭7 | Rock, blues foundation |
| `blues` | 1 ♭3 4 ♯4 5 ♭7 | Blues with "blue note" |
| `major blues` | 1 2 ♭3 3 5 6 | Major blues |

#### Symmetric Scales

| Name | Notes | Character |
|------|-------|-----------|
| `whole tone` | 1 2 3 ♯4 ♯5 ♯6 | Dreamy, Debussy |
| `chromatic` | All 12 notes | Every semitone |
| `diminished` | W-H-W-H-W-H-W-H | Symmetric, tension |
| `half-whole diminished` | H-W-H-W-H-W-H-W | Dominant diminished |
| `augmented` | m3-H-m3-H-m3-H | Coltrane changes |

#### Bebop Scales

| Name | Character |
|------|-----------|
| `bebop dominant` | Mixolydian + ♮7 passing tone |
| `bebop major` | Major + ♯5 passing tone |
| `bebop minor` | Dorian + ♮3 passing tone |
| `bebop dorian` | Dorian + ♮3 passing tone (variant) |

#### Altered & Exotic

| Name | Character |
|------|-----------|
| `altered` / `super locrian` | All altered tensions (over dom7) |
| `lydian dominant` | Lydian with ♭7 |
| `lydian augmented` | Lydian with ♯5 |
| `phrygian dominant` / `spanish` | Phrygian with ♮3 (flamenco) |
| `double harmonic` | Byzantine / Arabic scale |
| `hungarian minor` | Minor with ♯4, exotic sound |
| `hungarian major` | Major with ♯4 and ♭7 |
| `neapolitan minor` | Minor with ♭2 |
| `neapolitan major` | Major with ♭2 |
| `enigmatic` | Rare, Verdi's enigmatic scale |
| `persian` | Middle Eastern color |
| `arabian` | Arabian maqam feel |
| `japanese` | In-sen scale (5 notes) |
| `hirajoshi` | Japanese koto scale |
| `kumoi` | Japanese 5-note scale |
| `iwato` | Japanese, very dissonant |
| `egyptian` | Ancient Egyptian feel |

#### Jazz / Modal Interchange

| Name | Character |
|------|-----------|
| `dorian b2` | Phrygian with ♮6 |
| `mixolydian b6` | Hindu / Aeolian dominant |
| `locrian #2` | Locrian with ♮2 |

---

## Using Theory in Compositions

Theory objects help you compose programmatically:

```python
from delphi import scale, chord, note, Song, Track

# Generate a melody from a scale
s = scale("D", "dorian")
midi_notes = s.midi_notes

# Build notation from scale degrees
melody = " ".join(f"C{n//12}:8" for n in midi_notes) # (simplified)

# Transpose a chord progression
roots = ["C", "A", "F", "G"]
for root in roots:
    c = chord(f"{root}maj7")
    print(f"{root}maj7: {c.midi_notes}")
```

### In the REPL

Theory functions are available directly in the REPL:

```
🎵 > scale("C", "blues").play()
🎵 > chord("Am7").arpeggio("up").play()
🎵 > note("E4").transpose(5).play()
```
