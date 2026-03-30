# Notation Reference

Delphi uses a concise text-based notation for writing music. This document covers every feature of the notation syntax.

## Table of Contents

- [Notes](#notes)
- [Chords](#chords)
- [Rests](#rests)
- [Durations](#durations)
- [Dynamics](#dynamics)
- [Articulations](#articulations)
- [Ornaments](#ornaments)
- [Bar Notation](#bar-notation)
- [Subdivisions](#subdivisions)
- [Ties](#ties)
- [Tuplets](#tuplets)
- [Repeats](#repeats)
- [Euclidean Rhythms](#euclidean-rhythms)
- [Polyphony](#polyphony)
- [Random Elements](#random-elements)
- [Elongation](#elongation)
- [Slow Sequences](#slow-sequences)
- [Crescendo & Diminuendo](#crescendo--diminuendo)
- [Breath & Caesura](#breath--caesura)
- [Volta Brackets](#volta-brackets)
- [Structural Repeats](#structural-repeats)
- [Drums](#drums)

---

## Notes

Notes use scientific pitch notation: a pitch class, optional accidental, and octave number.

```
C4          Middle C (MIDI 60)
F#5         F-sharp in octave 5
Bb3         B-flat in octave 3
Ebb2        E-double-flat in octave 2
D##4        D-double-sharp in octave 4
```

**Pitch classes:** C, D, E, F, G, A, B

**Accidentals:**
| Symbol | Meaning |
|--------|---------|
| `#` | Sharp (+1 semitone) |
| `##` | Double sharp (+2 semitones) |
| `b` | Flat (-1 semitone) |
| `bb` | Double flat (-2 semitones) |

**Octave range:** -1 to 9 (C4 = middle C = MIDI 60)

## Chords

Chords are written as a root note followed by a quality suffix. When played, all notes in the chord sound simultaneously.

```
C           C major triad
Am          A minor
G7          G dominant 7th
Dmaj7       D major 7th
Fm7b5       F half-diminished 7th
Bbsus4      Bb suspended 4th
Eaug        E augmented
```

### Chord Qualities

| Quality | Intervals | Example |
|---------|-----------|---------|
| *(none)* / `maj` | Major triad | `C`, `Cmaj` |
| `m` / `min` | Minor triad | `Am`, `Amin` |
| `dim` / `°` | Diminished triad | `Bdim`, `B°` |
| `aug` / `+` | Augmented triad | `Caug`, `C+` |
| `5` | Power chord (root + 5th) | `C5` |
| `7` | Dominant 7th | `G7` |
| `maj7` | Major 7th | `Cmaj7` |
| `m7` / `min7` | Minor 7th | `Am7` |
| `dim7` / `°7` | Diminished 7th | `Bdim7` |
| `m7b5` / `ø7` / `ø` | Half-diminished 7th | `Bm7b5` |
| `mMaj7` / `mM7` | Minor-major 7th | `CmMaj7` |
| `aug7` | Augmented 7th | `Caug7` |
| `sus2` | Suspended 2nd | `Csus2` |
| `sus4` / `sus` | Suspended 4th | `Csus4` |
| `7sus4` | Dominant 7 sus4 | `G7sus4` |
| `7sus2` | Dominant 7 sus2 | `G7sus2` |
| `6` | Major 6th | `C6` |
| `m6` / `min6` | Minor 6th | `Am6` |
| `69` / `m69` | 6/9 chord | `C69` |
| `9` | Dominant 9th | `C9` |
| `maj9` | Major 9th | `Cmaj9` |
| `m9` / `min9` | Minor 9th | `Am9` |
| `11` | Dominant 11th | `C11` |
| `maj11` | Major 11th | `Cmaj11` |
| `m11` / `min11` | Minor 11th | `Am11` |
| `13` | Dominant 13th | `C13` |
| `maj13` | Major 13th | `Cmaj13` |
| `m13` | Minor 13th | `Am13` |
| `add9` | Add 9 | `Cadd9` |
| `add11` | Add 11 | `Cadd11` |
| `add2` | Add 2 | `Cadd2` |
| `alt` | Altered | `G7alt` |

## Rests

Rests produce silence for one beat (or a specified duration).

```
.           Rest (one beat)
~           Rest (one beat)
r           Rest (one beat)
rest        Rest (one beat)
r:h         Half-note rest
.:w         Whole-note rest
```

## Durations

Durations are specified with a colon suffix after any note or rest.

### Basic Durations

| Suffix | Name | Ticks (at 480 TPQ) |
|--------|------|------|
| `:w` | Whole note | 1920 |
| `:h` | Half note | 960 |
| `:q` | Quarter note | 480 |
| `:8` | Eighth note | 240 |
| `:16` | Sixteenth note | 120 |
| `:32` | Thirty-second note | 60 |
| `:64` | Sixty-fourth note | 30 |
| `:128` | Hundred-twenty-eighth note | 15 |
| `:dw` / `:breve` | Breve (double whole) | 3840 |

### Dotted Durations

A dot adds half the note's value. A double dot adds half + quarter.

| Suffix | Ticks |
|--------|-------|
| `:q.` | 720 (480 + 240) |
| `:h.` | 1440 (960 + 480) |
| `:q..` | 840 (480 + 240 + 120) |
| `:h..` | 1680 (960 + 480 + 240) |

### Triplet Durations

Triplets fit 3 notes in the space of 2.

| Suffix | Ticks |
|--------|-------|
| `:qt` | 320 (quarter triplet) |
| `:8t` | 160 (eighth triplet) |
| `:16t` | 80 (sixteenth triplet) |

### Examples

```
C4:w        Whole note C
C4:h        Half note C
C4:q        Quarter note C
C4:8        Eighth note C
C4:q.       Dotted quarter C
C4:8t       Eighth triplet C
```

## Dynamics

Dynamics set the velocity (loudness) of a note using an exclamation mark suffix.

```
C4!p        Piano (soft)
C4!f        Forte (loud)
C4!mf       Mezzo-forte
C4!fff      Fortissimo possible
```

### Dynamic Levels

| Marking | Velocity | Description |
|---------|----------|-------------|
| `ppp` | 16 | Pianississimo |
| `pp` | 33 | Pianissimo |
| `p` | 49 | Piano |
| `mp` | 64 | Mezzo-piano |
| `mf` | 80 | Mezzo-forte (default) |
| `f` | 96 | Forte |
| `ff` | 112 | Fortissimo |
| `fff` | 127 | Fortississimo |
| `sfz` | 120 | Sforzando |
| `sfp` | 110 | Sforzando-piano |
| `fp` | 100 | Forte-piano |
| `rfz` | 115 | Rinforzando |
| `fz` | 110 | Forzando |

You can combine dynamics with durations:

```
C4:h!f      Half note, forte
G4:q!pp     Quarter note, pianissimo
```

## Articulations

Articulations modify how a note is played. Apply them with a dot suffix.

```
C4.stac     Staccato (short)
C4.acc      Accent (louder)
C4.marc     Marcato (loud + clipped)
C4.ferm     Fermata (held)
```

### Articulation Reference

| Suffix | Full Name | Effect |
|--------|-----------|--------|
| `.stac` / `.staccato` | Staccato | Half duration |
| `.stacc` / `.staccatissimo` | Staccatissimo | Quarter duration |
| `.ten` / `.tenuto` | Tenuto | Slightly held (1.1× duration) |
| `.port` / `.portato` | Portato | Mezzo-staccato (0.75× duration) |
| `.acc` / `.accent` | Accent | 1.3× velocity |
| `.marc` / `.marcato` | Marcato | 1.4× velocity, 0.85× duration |
| `.ferm` / `.fermata` | Fermata | 2× duration |
| `.ghost` | Ghost note | 0.4× velocity, 0.8× duration |
| `.leg` / `.legato` | Legato | 1.05× duration |
| `.pizz` / `.pizzicato` | Pizzicato | 0.3× duration |
| `.mute` | Muted | 0.6× velocity, 0.5× duration |

Articulations can be combined with durations and dynamics:

```
C4:q.stac       Staccato quarter C
E4:8.acc!f      Accented eighth, forte
```

## Ornaments

Ornaments add melodic embellishments that expand into multiple notes during playback.

```
C4.tr       Trill (alternates with note above)
D4.mord     Mordent (quick alternation)
E4.turn     Turn (4-note figure)
F4.grace    Grace note (acciaccatura)
G4.trem     Tremolo (rapid repetition)
```

### Ornament Reference

| Suffix | Full Name | Description |
|--------|-----------|-------------|
| `.tr` / `.trill` | Trill | Rapid alternation with note above |
| `.mord` / `.mordent` | Mordent | Quick upper-note alternation |
| `.lmord` | Lower mordent | Quick lower-note alternation |
| `.turn` / `.gruppetto` | Turn | 4-note figure: above, main, below, main |
| `.grace` / `.acciaccatura` | Acciaccatura | Quick grace note before main note |
| `.appoggiatura` | Appoggiatura | Longer grace note (steals time) |
| `.trem` / `.tremolo` | Tremolo | Rapid repetition of the note |
| `.gliss` / `.glissando` | Glissando | Slide between notes |
| `.arp` / `.arpeggio` | Arpeggio | Break chord into sequential notes |
| `.roll` | Roll | Repeated notes (drum roll effect) |

## Bar Notation

Bar notation lets you write chord progressions using pipe characters. Each bar gets one chord, automatically split across the time signature.

```
| C | Am | F | G |           Simple I-vi-IV-V
| Cmaj7 | Am7 | Dm7 | G7 |  Jazz ii-V-I turnaround
| A7 | D7 | A7 | E7 |       Blues changes
```

The time signature determines how beats are distributed within each bar. By default, 4/4 time gives each chord 4 beats.

## Subdivisions

Square brackets divide one beat equally among the notes inside.

```
[C4 E4 G4]          Three notes in one beat (triplet feel)
[C4 E4 G4 C5]       Four notes in one beat (sixteenths)
C4 [E4 G4] C5       Mixed: quarter, two eighths, quarter
```

## Ties

Ties connect two notes of the same pitch, adding their durations together.

```
C4:h~C4:q       Hold C4 for half + quarter = dotted half
G4:w~G4:h       Hold G4 for whole + half = 6 beats
```

## Tuplets

Tuplets group notes to fit N notes in the space of a different number.

```
(3 C4 E4 G4)        Triplet: 3 notes in the space of 2
(5 C4 D4 E4 F4 G4)  Quintuplet: 5 notes in the space of 4
```

## Repeats

The multiply operator repeats a note or pattern a given number of times.

```
C4:q*4          Play C4 quarter note 4 times
kick*8          8 kick drum hits
[C4 E4]*2       Repeat the subdivision twice
```

## Euclidean Rhythms

Euclidean rhythms distribute N hits across M steps as evenly as possible. This is incredibly useful for drum patterns.

```
bd(3,8)         3 kick hits across 8 steps: x..x..x.
sd(2,8)         2 snare hits: x...x...
hh(5,8)         5 hihat hits: x.xx.xx.
bd(3,8,2)       Same pattern, rotated by 2 steps
```

The syntax is `instrument(beats, steps)` or `instrument(beats, steps, offset)`.

## Polyphony

Comma-separated notes play simultaneously (vertically stacked).

```
C4,E4,G4        C major triad (simultaneous)
D3,A3           Power chord
```

## Random Elements

### Random Removal

The `?` operator randomly removes a note with a given probability.

```
C4?             50% chance of being replaced by rest
C4?0.3          30% chance of rest
hihat?0.2       Hihat with 20% dropouts (more human feel)
```

### Random Choice

The `|` operator (outside bar notation) randomly selects one of several options.

```
C4|E4|G4        Randomly pick one note
kick|snare      Random drum hit
```

## Elongation

The `@` operator multiplies a note's duration by a weight factor.

```
C4@2            Hold C4 for 2× the default duration
G4@0.5          Play G4 at half the default duration
```

## Slow Sequences

Angle brackets create a sequence that advances one note per cycle. Useful when combined with repeating patterns.

```
<C4 E4 G4>      Plays C4 first time, E4 second time, G4 third time
```

## Crescendo & Diminuendo

Gradually increase or decrease volume over a number of beats.

```
cresc(p,f,4)    Crescendo from piano to forte over 4 notes
dim(f,p,4)      Diminuendo from forte to piano over 4 notes
cresc(pp,ff,8)  Crescendo from pp to ff over 8 notes
```

Aliases: `crescendo`, `diminuendo`, `decresc`, `decrescendo`

The velocity is linearly interpolated across the specified number of note events following the marking.

## Breath & Caesura

Insert brief pauses between notes.

```
C4 breath D4    Quarter-beat pause between C and D
C4 caesura D4   Half-beat pause between C and D
```

## Volta Brackets

Volta brackets create first and second endings for repeated sections.

```
C4 D4 [1 E4 | [2 F4
```

This plays:
1. First time: C4 D4 E4
2. Second time: C4 D4 F4

## Structural Repeats

Navigate the structure of a piece with traditional repeat markings.

```
C4 D4 E4 DC            Da Capo — repeat from the beginning
C4 segno D4 E4 DS      Dal Segno — repeat from the segno marker
C4 D4 fine E4 DC       Da Capo al Fine — repeat from start, stop at fine
```

| Marking | Meaning |
|---------|---------|
| `DC` | Da Capo — go back to the beginning |
| `DS` | Dal Segno — go back to the `segno` marker |
| `fine` | Fine — end point for D.C./D.S. al Fine |
| `segno` | Segno — bookmark for D.S. |

## Drums

Drum hits use named tokens instead of pitched notes. Drums are automatically assigned to MIDI channel 10.

```
kick snare hihat hihat       Basic rock beat
bd sd hh oh                  Same with abbreviations
kick [hihat hihat] snare     Kick, two eighths hihat, snare
```

### Drum Map

| Name | Abbreviation | MIDI Note | Description |
|------|-------------|-----------|-------------|
| `kick` | `bd` | 36 | Bass drum |
| `snare` | `sd` | 38 | Snare drum |
| `hihat` | `hh` | 42 | Closed hi-hat |
| `openhat` | `oh` | 46 | Open hi-hat |
| `closehat` | `ch` | 42 | Closed hi-hat |
| `ride` | `rd` | 51 | Ride cymbal |
| `crash` | `cr` | 49 | Crash cymbal |
| `pedal` | — | 44 | Hi-hat pedal |
| `tom1` | — | 50 | High tom |
| `tom2` | — | 47 | Mid tom |
| `tom3` | — | 45 | Low tom |
| `clap` | `cp` | 39 | Hand clap |
| `rimshot` | `rim` | 37 | Side stick |
| `cowbell` | `cb` | 56 | Cowbell |
| `tambourine` | `tamb` | 54 | Tambourine |
| `shaker` | — | 70 | Shaker |
| `clave` | — | 75 | Claves |
| `woodblock` | `wb` | 76 | Wood block |
| `triangle` | `tri` | 81 | Triangle |
| `guiro` | — | 73 | Guiro |
| `cabasa` | — | 69 | Cabasa |
| `maracas` | — | 70 | Maracas |

---

## Combining Features

All notation features can be combined freely:

```python
# Melody with dynamics, articulations, and durations
play("C4:q!mf E4:8.stac G4:q.acc!f C5:h.ferm")

# Drum pattern with Euclidean rhythms
play("bd(3,8) sd(2,8) hh(5,8)")

# Crescendo into a accented chord
play("C4:q D4:q cresc(p,f,4) E4:q F4:q G4:q A4.acc:q")

# Volta with dynamics
play("C4!mf D4 [1 E4!f | [2 G4!ff")
```
