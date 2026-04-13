# Composition Guide

For pieces with more structure than a single notation cell — multiple sections, recurring patterns, and complex arrangements — Delphi supports several composition techniques within the notation language and the `.dstudio` project format.

## Table of Contents

- [Multi-Cell Structure](#multi-cell-structure)
- [Repeats and Patterns](#repeats-and-patterns)
- [Structural Markers](#structural-markers)
- [Sections with Markdown](#sections-with-markdown)
- [Arrangement Example](#arrangement-example)

---

## Multi-Cell Structure

In Delphi Studio (or a `.dstudio` file), you compose by creating multiple cells:

1. **Code cell** at the top for global settings (tempo, key, time signature, swing, humanize)
2. **Notation cells** for each instrument part
3. **Markdown cells** for section notes and lyrics

Each notation cell becomes a separate track with its own instrument and MIDI channel.

## Repeats and Patterns

### Note Repeats

The multiply operator repeats a note or pattern:

```
C4:q*4          Play C4 quarter note 4 times
kick*8          8 kick drum hits
[C4 E4]*2       Repeat the subdivision twice
```

### Bar Repeats

Write multiple bars to extend a part:

```
| Dm | Am | Bb | A7 |
| Dm | Am | Bb | A7 |
| Gm | C7 | F | F |
| Bb | A7 | Dm | Dm |
```

### Euclidean Patterns

Distribute hits evenly across steps for naturally grooving rhythms:

```
bd(3,8)         3 kick hits across 8 steps
sd(2,8)         2 snare hits
hh(5,8)         5 hihat hits
```

### Layers

Curly braces play multiple patterns simultaneously:

```
{bd(3,8) sd(2,8) hh(5,8)}
```

## Structural Markers

Use structural repeat markers for traditional form:

| Marking | Meaning |
|---------|---------|
| `DC` | Da Capo — go back to the beginning |
| `DS` | Dal Segno — go back to the `segno` marker |
| `fine` | Fine — end point for D.C./D.S. al Fine |
| `segno` | Segno — bookmark for D.S. |

```
# Verse
C4 D4 E4 F4
segno
G4 A4 B4 C5
fine
# Bridge
D5 E5 F5 G5
DS
```

### Volta Brackets

First and second endings:

```
C4 D4 [1 E4 | [2 F4
```

## Sections with Markdown

Use markdown cells to visually separate sections of your piece:

**Markdown cell:**
```
## Verse 1
```

**Notation cell:**
```
# @instrument piano
# @track Piano
| Dm | Am | Bb | A7 |
```

**Markdown cell:**
```
## Chorus
```

**Notation cell:**
```
# @instrument piano
# @track Piano
| F | C | Dm | Bb |
```

This doesn't affect playback — it's purely organizational. All notation cells play sequentially.

## Arrangement Example

A complete arrangement using Delphi's notation features:

### Setup (code cell)

```
tempo(115)
key("C major")
time_sig(4, 4)
humanize(0.05)
```

With a key set, you can use Roman numeral chords and scale degree notes throughout the piece.

### Melody (notation cell)

```
# @instrument piano
# @track Melody
# @velocity 85

# Intro — Roman numerals resolve to chords in C major
| Imaj7 | vi7 |

# Verse — scale degrees for the melody
^1:q ^2:q ^3:q ^5:q  ^6:q ^5:q ^3:q ^2:q
^1:q ^3:q ^5:q ^1:q  ^7:h ^6:h

# Chorus
G4:q A4:q B4:q C5:q  D5:h B4:h
C5:q B4:q A4:q G4:q  E4:w
```

### Chords (notation cell)

```
# @instrument electric piano
# @track Chords
# @velocity 70

# Verse — letter-name and Roman numeral chords work side by side
| I | vi | IV | V |

# Chorus — modulate to G major mid-cell
# @key G major
| I | IV | V | I |
```

### Bass (notation cell)

```
# @instrument acoustic bass
# @track Bass
# @velocity 70

# Intro
C2:h G2:h  A2:h E2:h

# Verse
C2:h G2:h  A2:h E2:h
F2:h C2:h  G2:h G2:h

# Chorus
C2:h F2:h  G2:h E2:h
F2:h G2:h  C2:w
```

### Drums (notation cell)

```
# @channel 9
# @track Drums
# @velocity 80

# Intro — just hi-hats
hh*8

# Verse
{bd(3,8) sd(2,8) hh(5,8)}
{bd(3,8) sd(2,8) hh(5,8)}

# Chorus — add crash on beat 1
{bd(3,8) sd(2,8) hh(5,8) crash*1}
{bd(3,8) sd(2,8) hh(5,8)}
```

Export via Studio or CLI:

```bash
delphi export arrangement.dstudio --format both --output ./out/
```
