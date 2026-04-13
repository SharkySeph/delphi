# Theory Reference

Delphi includes a music theory engine built into the Rust core. Notes, chords, and scales are first-class types used by the notation parser, the Theory Explorer panel in Delphi Studio, and the CLI.

## Table of Contents

- [Notes](#notes)
- [Chords](#chords)
- [Scales](#scales)

---

## Notes

Notes use scientific pitch notation: pitch class, optional accidental, and octave number.

```
C4          Middle C (MIDI 60)
F#5         F-sharp in octave 5
Bb3         B-flat in octave 3
```

**Pitch classes:** C, D, E, F, G, A, B

**Accidentals:** `#` (sharp), `##` (double sharp), `b` (flat), `bb` (double flat)

**Octave range:** -1 to 9 (C4 = middle C = MIDI 60)

## Chords

Chords are written as a root note followed by a quality suffix. All notes sound simultaneously.

```
C           C major triad
Am          A minor
G7          G dominant 7th
Dmaj7       D major 7th
```

### Chord Qualities

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

### Slash Chords

```
Am/E        A minor with E bass (first inversion)
C/G         C major with G bass (second inversion)
Cmaj7/B     C major 7th over B bass
Am4/E3      A minor at octave 4 with E3 bass
```

## Scales

Over 50 scale types are built in. Use them in the Theory Explorer panel or reference them when composing.

### Church Modes

| Name | Intervals | Character |
|------|-----------|-----------|
| `major` / `ionian` | 1 2 3 4 5 6 7 | Bright, happy |
| `dorian` | 1 2 ♭3 4 5 6 ♭7 | Minor with bright 6th |
| `phrygian` | 1 ♭2 ♭3 4 5 ♭6 ♭7 | Spanish, dark |
| `lydian` | 1 2 3 ♯4 5 6 7 | Bright, dreamy |
| `mixolydian` | 1 2 3 4 5 6 ♭7 | Bluesy major |
| `minor` / `aeolian` | 1 2 ♭3 4 5 ♭6 ♭7 | Sad, natural minor |
| `locrian` | 1 ♭2 ♭3 4 ♭5 ♭6 ♭7 | Dark, unstable |

### Minor Variants

| Name | Intervals | Character |
|------|-----------|-----------|
| `natural minor` | 1 2 ♭3 4 5 ♭6 ♭7 | Same as aeolian |
| `harmonic minor` | 1 2 ♭3 4 5 ♭6 7 | Classical minor |
| `melodic minor` | 1 2 ♭3 4 5 6 7 | Jazz minor (ascending) |

### Pentatonic & Blues

| Name | Notes | Character |
|------|-------|-----------|
| `pentatonic` / `major pentatonic` | 1 2 3 5 6 | Universal, pleasant |
| `minor pentatonic` | 1 ♭3 4 5 ♭7 | Rock, blues foundation |
| `blues` | 1 ♭3 4 ♯4 5 ♭7 | Blues with "blue note" |
| `major blues` | 1 2 ♭3 3 5 6 | Major blues |

### Symmetric Scales

| Name | Notes | Character |
|------|-------|-----------|
| `whole tone` | 1 2 3 ♯4 ♯5 ♯6 | Dreamy, Debussy |
| `chromatic` | All 12 notes | Every semitone |
| `diminished` | W-H-W-H-W-H-W-H | Symmetric, tension |
| `half-whole diminished` | H-W-H-W-H-W-H-W | Dominant diminished |
| `augmented` | m3-H-m3-H-m3-H | Coltrane changes |

### Bebop Scales

| Name | Character |
|------|-----------|
| `bebop dominant` | Mixolydian + ♮7 passing tone |
| `bebop major` | Major + ♯5 passing tone |
| `bebop minor` | Dorian + ♮3 passing tone |
| `bebop dorian` | Dorian + ♮3 passing tone (variant) |

### Altered & Exotic

| Name | Character |
|------|-----------|
| `altered` / `super locrian` | All altered tensions (over dom7) |
| `lydian dominant` | Lydian with ♭7 |
| `lydian augmented` | Lydian with ♯5 |
| `phrygian dominant` / `spanish` | Phrygian with ♮3 (flamenco) |
| `double harmonic` | Byzantine / Arabic scale |
| `hungarian minor` | Minor with ♯4 |
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

### Jazz / Modal Interchange

| Name | Character |
|------|-----------|
| `dorian b2` | Phrygian with ♮6 |
| `mixolydian b6` | Hindu / Aeolian dominant |
| `locrian #2` | Locrian with ♮2 |

## Key-Relative Notation

When a key is set for the project, Delphi's theory engine resolves Roman numeral chords and scale degree notes to concrete pitches using the key's scale.

### Roman Numeral Resolution

Each Roman numeral maps to a scale degree root, then the chord quality is applied:

| Key | I | ii | iii | IV | V | vi | vii° |
|-----|---|----|-----|----|---|-----|------|
| C major | C | Dm | Em | F | G | Am | B° |
| G major | G | Am | Bm | C | D | Em | F#° |
| D minor | Dm | E° | F | Gm | Am | Bb | C |

Upper-case numerals produce **major** chords; lower-case produce **minor**. Append any quality suffix (e.g. `V7`, `IVsus4`, `viidim7`) — the suffix overrides the default quality.

### Scale Degree Notes

The `^` prefix resolves a degree number (1–7) to the corresponding note in the scale:

| Key | ^1 | ^2 | ^3 | ^4 | ^5 | ^6 | ^7 |
|-----|----|----|----|----|----|----|-----|
| C major | C4 | D4 | E4 | F4 | G4 | A4 | B4 |
| G major | G4 | A4 | B4 | C4 | D4 | E4 | F#4 |

See the [Notation Reference](notation.md#roman-numeral-chords) for full syntax details.
