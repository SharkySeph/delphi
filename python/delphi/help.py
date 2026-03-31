"""
Built-in documentation and syntax suggestions for the Delphi tooling.

Provides:
  - Topic-based quick reference (used by REPL `docs` command and Studio F2 panel)
  - Contextual syntax suggestions (ghost-text hints based on what the user is typing)
"""

# ── Quick-Reference Topics ────────────────────────────────────
#
# Each topic is a short, terminal-friendly reference card.
# Shown via `docs <topic>` in the REPL and the F2 panel in Studio.

TOPICS = {
    "notes": """\
\033[1mNotes\033[0m

  C4  D#5  Bb3  E4  F##2  Gbb1       Pitch + accidental + octave
  Octave range: -1 to 9              C4 = middle C (MIDI 60)

  Accidentals: # (sharp)  b (flat)  ## (double sharp)  bb (double flat)
""",

    "chords": """\
\033[1mChords\033[0m

  C  Am  G7  Dmaj7  Fm7b5  Bbsus4    Root + quality suffix
  Am4  G75  Cmaj73                    Chord with octave voicing
  Am/E  C/G  Cmaj7/B                  Slash chord (bass note)
  Am/E3  Am4/E3                       Slash chord with octave

  Qualities: m  7  m7  maj7  dim  aug  sus4  sus2  9  m9  add9
             m7b5  dim7  aug7  6  m6  69  11  13  alt  5  mMaj7
""",

    "durations": """\
\033[1mDurations\033[0m

  C4:w   whole       C4:h   half        C4:q   quarter
  C4:8   eighth      C4:16  sixteenth   C4:32  thirty-second
  C4:dw  breve (double whole)

  Dotted:    C4:q.   (1.5×)     C4:q..  (1.75×)
  Triplet:   C4:8t   C4:qt     C4:16t
  Tuplets:   (3 C4 E4 G4)      (5 C4 D4 E4 F4 G4)
""",

    "dynamics": """\
\033[1mDynamics\033[0m

  C4!ppp  C4!pp  C4!p  C4!mp  C4!mf  C4!f  C4!ff  C4!fff
  C4!sfz  C4!sfp  C4!fp  C4!rfz  C4!fz

  Gradual:
    cresc(p,f,4)     Crescendo from p to f over 4 notes
    dim(f,p,4)       Diminuendo from f to p over 4 notes
""",

    "articulations": """\
\033[1mArticulations & Ornaments\033[0m

  Articulations (append to note):
    .stac   staccato       .stacc  staccatissimo    .ten    tenuto
    .port   portato        .acc    accent           .marc   marcato
    .ferm   fermata        .ghost  ghost note       .leg    legato
    .pizz   pizzicato      .mute   muted

  Ornaments:
    .tr     trill          .mord   mordent          .lmord  lower mordent
    .turn   turn           .grace  grace note       .trem   tremolo
    .gliss  glissando      .arp    arpeggio         .roll   drum roll

  Example: C4:q.stac!f   (quarter, staccato, forte)
""",

    "drums": """\
\033[1mDrums\033[0m

  Drum names (auto-route to MIDI channel 9):
    kick/bd   snare/sd   hihat/hh   openhat/oh   ride/rd   crash/cr
    tom1  tom2  tom3  clap/cp  rimshot/rim  cowbell/cb  pedal

  Euclidean rhythms: instrument(beats, steps[, offset])
    bd(3,8)             3 kicks across 8 steps
    sd(2,8)             2 snares across 8 steps
    hh(5,8)             5 hats across 8 steps

  Layers: {pattern1 pattern2 ...}  — play simultaneously
    {bd(3,8) sd(2,8) hh(5,8)}     Full drum groove in one bar
""",

    "layers": """\
\033[1mLayers\033[0m

  Curly braces play multiple patterns at the same time:

    {bd(3,8) sd(2,8) hh(5,8)}     Kick, snare, hihat all at once

  Without braces, patterns are sequential.
  Each {} block advances by the longest inner pattern.

  Multi-bar:
    {bd(3,8) sd(2,8) hh(5,8)}     Bar 1
    {bd(3,8) sd(2,8) hh(5,8)}     Bar 2
    {bd(3,8) sd(2,8) crash*1}     Bar 3 (with crash)
""",

    "bars": """\
\033[1mBar Notation\033[0m

  Use pipes | for bar lines — chords are auto-voiced:

    | C | Am | F | G |              Simple progression
    | Cmaj7 | Am7 | Dm7 | G7 |     Jazz changes
    | Am/E | Dm | G7 | C |         With slash chords
""",

    "rests": """\
\033[1mRests\033[0m

  .         Quarter rest (one beat of silence)
  ~         Rest (same as .)
  r         Rest
  rest      Rest
  breath    Short pause (1/4 beat between notes)
  caesura   Longer pause (1/2 beat between notes)
""",

    "repeats": """\
\033[1mRepeats & Structure\033[0m

  C4:q*4              Repeat note 4 times
  [C4 E4]*2           Repeat group twice
  DC                  Da Capo (from start)
  DS                  Dal Segno (from segno marker)
  segno ... DS        Place segno, jump back with DS
  fine ... DC         Play to fine, then stop
  [1 E4 | [2 F4      Volta brackets (1st/2nd endings)
""",

    "patterns": """\
\033[1mAdvanced Patterns\033[0m

  Subdivision:  [C4 E4 G4]           Three notes in one beat
  Ties:         C4:h~C4:q            Tie notes together
  Elongation:   C4@2                 Hold 2× duration
  Random:       C4?  C4?0.3          Random removal (50% / 30%)
  Choice:       C4|E4|G4             Pick one randomly
  Slow seq:     <C4 E4 G4>           One per cycle
  Polyphony:    C4,E4,G4             Simultaneous notes
""",

    "playback": """\
\033[1mPlayback\033[0m

  play("C4 E4 G4")                   Play notation string
  play("...", instrument="flute")     Override instrument
  play("...", channel=2)              Override MIDI channel
  play_notes([(60,80,0,480), ...])    Play raw MIDI tuples

  SoundFont is the default playback engine for all instruments.
  Drums auto-route to MIDI channel 9.
  Falls back to oscillator synth only if no SoundFont is available.

  ensure_soundfont()    Download default SoundFont if needed
  soundfont_info()      Show loaded SoundFont details
  set_soundfont(path)   Use a custom SoundFont
""",

    "songs": """\
\033[1mSongs & Tracks\033[0m

  s = Song("My Song", tempo=120, key="C major")
  s.track("Piano", "| C | Am | F | G |", program="piano")
  s.track("Bass", "C2:h G2:h", program="acoustic bass")
  s.track("Drums", "{bd(3,8) sd(2,8) hh(5,8)}", program=0, channel=9)
  s.play()
  s.export("song.mid")
  s.render("song.wav")

  Track effects:
    .gain(0.8)  .pan(0.3)  .reverb(0.5)  .delay(0.3)
    .transpose(7)  .octave_up()  .rev()
    .adsr(0.01, 0.1, 0.8, 0.1)
""",

    "studio": """\
\033[1mStudio Quick Reference\033[0m

  F5  Run cell       F6  Run all        F7  Add cell      F8  Delete cell
  F9  Export         F10 Save           F1  Keybindings   F2  Docs
  Ctrl+↑↓  Navigate     Ctrl+Shift+↑↓  Reorder cells
  Ctrl+T   Cycle type   Ctrl+E  Collapse    Ctrl+C  Stop playback
  Ctrl+P   Replay       Ctrl+S  Save        Ctrl+Q  Quit

  Pragmas (notation cells):
    # @instrument flute     # @track Melody
    # @channel 2            # @velocity 90
""",

    "context": """\
\033[1mContext Functions\033[0m

  tempo(120)             Set BPM
  key("D minor")         Set key signature
  time_sig(3, 4)         Set time signature
  instrument("violin")   Set default instrument
  swing(0.5)             Swing feel (0=straight, 1=hard)
  humanize(0.1)          Timing/velocity variation
  get_context()          View current settings
  reset_context()        Reset to defaults
""",

    "theory": """\
\033[1mTheory Objects\033[0m

  note("C4")                Note — .midi, .freq, .transpose(n), .play()
  chord("Cmaj7")            Chord — .notes(), .arpeggio("up"), .play()
  scale("C", "dorian")      Scale — .notes(), .play()

  Scale types: major, minor, dorian, phrygian, lydian, mixolydian,
    locrian, blues, pentatonic, minor pentatonic, whole tone,
    chromatic, harmonic minor, melodic minor, bebop dominant,
    altered, lydian dominant, hungarian minor, hirajoshi, japanese
""",
}

# topic aliases
TOPICS["note"] = TOPICS["notes"]
TOPICS["chord"] = TOPICS["chords"]
TOPICS["duration"] = TOPICS["durations"]
TOPICS["dynamic"] = TOPICS["dynamics"]
TOPICS["articulation"] = TOPICS["articulations"]
TOPICS["ornament"] = TOPICS["articulations"]
TOPICS["ornaments"] = TOPICS["articulations"]
TOPICS["drum"] = TOPICS["drums"]
TOPICS["layer"] = TOPICS["layers"]
TOPICS["bar"] = TOPICS["bars"]
TOPICS["rest"] = TOPICS["rests"]
TOPICS["repeat"] = TOPICS["repeats"]
TOPICS["structure"] = TOPICS["repeats"]
TOPICS["pattern"] = TOPICS["patterns"]
TOPICS["play"] = TOPICS["playback"]
TOPICS["soundfont"] = TOPICS["playback"]
TOPICS["sf"] = TOPICS["playback"]
TOPICS["song"] = TOPICS["songs"]
TOPICS["track"] = TOPICS["songs"]
TOPICS["ctx"] = TOPICS["context"]
TOPICS["scale"] = TOPICS["theory"]

# Canonical topic names (no aliases) for the index listing
TOPIC_INDEX = [
    "notes", "chords", "durations", "dynamics", "articulations",
    "drums", "layers", "bars", "rests", "repeats", "patterns",
    "playback", "songs", "studio", "context", "theory",
]


def get_docs_index() -> str:
    """Return a formatted list of available doc topics."""
    lines = ["\033[1mDelphi Quick Reference\033[0m", ""]
    lines.append("  Type \033[1mdocs <topic>\033[0m to view. Available topics:")
    lines.append("")
    col_width = 18
    cols = 3
    for i in range(0, len(TOPIC_INDEX), cols):
        row = TOPIC_INDEX[i:i + cols]
        lines.append("    " + "".join(t.ljust(col_width) for t in row))
    lines.append("")
    lines.append("  Examples:  docs drums  •  docs layers  •  docs playback")
    return "\n".join(lines)


def get_docs(topic: str) -> str:
    """Return help text for a given topic, or the index if not found."""
    key = topic.strip().lower()
    if key in TOPICS:
        return TOPICS[key]
    return get_docs_index() + f"\n\n  Unknown topic: '{topic}'. Try one of the topics above."


# ── Contextual Syntax Suggestions ─────────────────────────────
#
# These provide proactive ghost-text hints (not completions).
# They show what you *could* type next based on context.

import re

_NOTE_RE = re.compile(r'[A-Ga-g][#b]{0,2}\d$')
_CHORD_RE = re.compile(r'[A-Ga-g][#b]{0,2}(?:m|maj|dim|aug|7|sus|add|alt|°|ø)\S*$')
_BARE_ROOT_RE = re.compile(r'^[A-Ga-g][#b]{0,2}$')
_DRUM_RE = re.compile(r'(?:kick|snare|hihat|bd|sd|hh|oh|crash|ride|tom\d|clap|rim)$')
_BAR_RE = re.compile(r'\|\s*$')
_BRACE_RE = re.compile(r'\{\s*$')
_PLAY_RE = re.compile(r'play\(\s*$')
_EMPTY_RE = re.compile(r'^\s*$')
_EUCLIDEAN_RE = re.compile(r'\w+\(\d+,\d+\)$')


def get_suggestion(text: str) -> str:
    """Return a contextual ghost-text suggestion for the given input, or ''."""
    text = text.rstrip()
    if not text:
        return ""

    # After a note → suggest duration
    if _NOTE_RE.search(text):
        return ":q"

    # After a chord quality → suggest bar context
    if _CHORD_RE.search(text):
        return " | "

    # After a drum name → suggest Euclidean rhythm
    if _DRUM_RE.search(text):
        return "(3,8)"

    # After a bar pipe → suggest chord
    if _BAR_RE.search(text):
        return " Cmaj7 |"

    # After opening brace → suggest drum layer
    if _BRACE_RE.search(text):
        return "bd(3,8) sd(2,8) hh(5,8)}"

    # After a Euclidean pattern → suggest layer syntax
    if _EUCLIDEAN_RE.search(text):
        return " sd(2,8)"

    # After play( → suggest notation string
    if _PLAY_RE.search(text):
        return '"C4 E4 G4")'

    return ""


# ── Studio Help Panel Content ─────────────────────────────────
#
# A compact, column-friendly version for the F2 floating reference.

STUDIO_HELP_PANEL = """\
╔═══ Delphi Quick Reference ════════════════════════════════╗
║                                                           ║
║  NOTES       C4  D#5  Bb3  (pitch + accidental + octave) ║
║  CHORDS      Am  G7  Cmaj7  Dm7b5  Am/E  (slash chords) ║
║  DURATIONS   :w :h :q :8 :16  (dotted: :q.  triplet: :8t)║
║  DYNAMICS    !p !mf !f !ff  cresc(p,f,4)  dim(f,p,4)    ║
║  ARTIC.      .stac .acc .marc .ferm .ghost .tr .mord     ║
║  RESTS       .  ~  r  rest  breath  caesura              ║
║  BARS        | C | Am | F | G |                          ║
║  DRUMS       kick/bd  snare/sd  hihat/hh  crash/cr       ║
║  EUCLIDEAN   bd(3,8)  sd(2,8)  hh(5,8)                  ║
║  LAYERS      {bd(3,8) sd(2,8) hh(5,8)}                  ║
║  PATTERNS    [C4 E4 G4]  C4*4  C4@2  C4?  <C4 E4 G4>   ║
║  REPEATS     *4  DC  DS  segno  fine  [1 ... | [2 ...    ║
║  POLYPHONY   C4,E4,G4  (comma = simultaneous)            ║
║                                                           ║
║  PRAGMAS     # @instrument flute   # @channel 2          ║
║              # @track Melody       # @velocity 90        ║
║                                                           ║
║  PLAYBACK    SoundFont is default — drums → channel 9    ║
║              play("...", instrument="flute", channel=2)   ║
║                                                           ║
║  Press F2 to close                                        ║
╚═══════════════════════════════════════════════════════════╝\
"""
