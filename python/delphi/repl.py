"""
Interactive REPL for Delphi.
Uses prompt_toolkit for a rich terminal experience with syntax highlighting,
multi-line editing, and intelligent auto-completion.
"""

from prompt_toolkit import PromptSession
from prompt_toolkit.history import FileHistory
from prompt_toolkit.auto_suggest import AutoSuggestFromHistory
from prompt_toolkit.completion import Completer, Completion
from prompt_toolkit.lexers import PygmentsLexer
from prompt_toolkit.formatted_text import HTML
from prompt_toolkit.key_binding import KeyBindings

import os
import traceback

import delphi
from delphi.song import GM_INSTRUMENTS
from delphi.help import get_docs, get_docs_index, get_suggestion


# ── Syntax Highlighting via Pygments ─────────────────────────

try:
    from pygments.lexer import RegexLexer, bygroups
    from pygments.token import (
        Token, Name, String, Number, Keyword, Operator, Comment, Punctuation, Generic
    )

    class DelphiLexer(RegexLexer):
        name = "Delphi"
        tokens = {
            "root": [
                # Comments
                (r'#.*$', Comment.Single),
                # Strings
                (r'"""[\s\S]*?"""', String.Doc),
                (r'"[^"]*"', String),
                (r"'[^']*'", String),
                # Keywords / built-in functions
                (r'\b(play|export|tempo|key|time_sig|note|chord|scale|Song|Track)\b',
                 Keyword),
                (r'\b(Section|Pattern|Voice|section|pattern|voice)\b', Keyword),
                (r'\b(render|add_track|arpeggio|transpose|play_notes)\b', Name.Function),
                (r'\b(help|quit|exit|songs|tracks|instruments|sf)\b', Name.Builtin),
                # Bar pipes
                (r'\|', Operator),
                # Note names with octave: C4, F#5, Bb3
                (r'\b[A-Ga-g][#b]{0,2}\d\b', Generic.Emph),
                # Chord names: Cmaj7, Am7, Dm
                (r'\b[A-Ga-g][#b]{0,2}(?:maj7|maj9|maj|m7b5|mMaj7|m7|m9|m|dim7|dim|aug7|aug|sus2|sus4|add9|7|9|5)\b',
                 Name.Decorator),
                # Duration suffixes
                (r':[whq]\.?\.?\b|:(?:8|16|32|64|128)\.?\.?\b', Number),
                # Dynamics
                (r'!(?:ppp|pp|p|mp|mf|f|ff|fff|sfz|sfp|fp|rfz|fz)\b', Name.Tag),
                # Repeat, elongation, random removal operators
                (r'[*@?]\d*', Operator),
                # Slow sequence brackets
                (r'[<>]', Operator),
                # Articulation/ornament suffixes
                (r'\.[a-z]+', Name.Tag),
                # Drum names
                (r'\b(?:kick|snare|hihat|openhat|closehat|ride|crash|tom[123]|clap|rimshot|cowbell|tambourine|shaker|clave|woodblock|triangle|guiro|cabasa|maracas|pedal)\b',
                 Name.Label),
                # Euclidean pattern e.g. bd(3,8)
                (r'\w+\(\d+,\d+(?:,\d+)?\)', Generic.Strong),
                # Numbers
                (r'\b\d+\.?\d*\b', Number),
                # Brackets
                (r'[\[\](){}]', Punctuation),
                # Assignment and operators
                (r'[=+\-*/,.]', Operator),
                # Identifiers
                (r'\b\w+\b', Name),
            ]
        }
    _LEXER = PygmentsLexer(DelphiLexer)
except ImportError:
    _LEXER = None


# ── Intelligent Completer ────────────────────────────────────

_NOTE_NAMES = [f"{n}{o}" for n in "CDEFGAB" for o in range(0, 9)]
_NOTE_NAMES += [f"{n}#{o}" for n in "CDEFGAB" for o in range(0, 9)]
_NOTE_NAMES += [f"{n}b{o}" for n in "CDEFGAB" for o in range(0, 9)]

_CHORD_NAMES = []
for root in ["C", "C#", "Db", "D", "D#", "Eb", "E", "F", "F#",
             "Gb", "G", "G#", "Ab", "A", "A#", "Bb", "B"]:
    for quality in ["", "m", "7", "m7", "maj7", "dim", "aug",
                    "sus4", "sus2", "9", "m9", "add9", "m7b5", "dim7", "aug7",
                    "m6", "6", "69", "m69", "add11", "add2", "maj9", "maj11",
                    "maj13", "m11", "m13", "11", "13", "7sus4", "7sus2", "alt", "5",
                    "mMaj7", "min7", "min9"]:
        _CHORD_NAMES.append(f"{root}{quality}")

_DRUMS = ["kick", "snare", "hihat", "openhat", "closehat", "ride", "crash",
          "tom1", "tom2", "tom3", "clap", "rimshot", "cowbell",
          "tambourine", "shaker", "clave", "woodblock", "triangle",
          "guiro", "cabasa", "maracas", "pedal"]

_ARTICULATIONS = [".stac", ".stacc", ".ten", ".port", ".acc", ".marc", ".ferm",
                  ".ghost", ".leg", ".pizz", ".mute"]

_ORNAMENTS = [".tr", ".trill", ".mord", ".mordent", ".lmord", ".turn",
              ".grace", ".acciaccatura", ".appoggiatura", ".trem", ".tremolo",
              ".gliss", ".glissando", ".arp", ".arpeggio", ".roll"]

_DURATIONS = [":w", ":h", ":q", ":8", ":16", ":32", ":64", ":128",
              ":dw", ":8t", ":qt", ":16t",
              ":w.", ":h.", ":q.", ":8.", ":16.",
              ":w..", ":h..", ":q..", ":8.."]

_DYNAMICS = ["!ppp", "!pp", "!p", "!mp", "!mf", "!f", "!ff", "!fff",
             "!sfz", "!sfp", "!fp", "!rfz", "!fz"]

_FUNCTIONS = [
    'play("', 'export("', 'tempo(', 'key("', 'time_sig(',
    'swing(', 'humanize(', 'instrument("',
    'chord("', 'note("', 'scale("',
    'Song("', 'Track(', 'Section("', 'Pattern("', 'Voice("',
    'Arrangement("', 'PatternLibrary(',
    'build_song_from_sections("',
    'register_pattern("', 'get_pattern("', 'list_patterns()',
    'include("',
    'ensure_soundfont()', 'soundfont_info()',
    'get_context()', 'reset_context()',
]

_COMMANDS = ["help", "quit", "exit", "songs", "tracks", "instruments", "sf", "docs"]

_INSTRUMENTS = sorted(GM_INSTRUMENTS.keys())


class DelphiCompleter(Completer):
    """Context-aware completer for the Delphi REPL."""

    def get_completions(self, document, complete_event):
        text = document.text_before_cursor
        word = document.get_word_before_cursor(WORD=True)

        if not word:
            return

        wl = word.lower()

        # After "docs " → complete topic names
        if text.lstrip().startswith("docs "):
            from delphi.help import TOPIC_INDEX
            for topic in TOPIC_INDEX:
                if topic.startswith(wl):
                    yield Completion(topic, start_position=-len(word))
            return

        # Inside a string argument for program= or instrument() → instrument names
        if 'program="' in text or "program='" in text or 'instrument("' in text or "instrument('" in text:
            for inst in _INSTRUMENTS:
                if inst.startswith(wl):
                    yield Completion(inst, start_position=-len(word))
            return

        # After key(" → key names
        if 'key("' in text or "key('" in text:
            for k in ["C major", "C minor", "D major", "D minor", "E major", "E minor",
                       "F major", "F minor", "G major", "G minor", "A major", "A minor",
                       "B major", "B minor", "Bb major", "Eb major", "Ab major",
                       "F# minor", "C# minor", "G# minor",
                       "C blues", "A blues", "E blues", "G blues", "D blues",
                       "C dorian", "D dorian", "E phrygian", "F lydian",
                       "G mixolydian", "A locrian", "C harmonic minor",
                       "C melodic minor", "C whole tone", "C chromatic",
                       "C pentatonic", "A minor pentatonic",
                       "C bebop dominant", "C altered", "C lydian dominant",
                       "C phrygian dominant", "C hungarian minor",
                       "C double harmonic", "C japanese", "C hirajoshi"]:
                if k.lower().startswith(wl):
                    yield Completion(k, start_position=-len(word))
            return

        # Articulation/ornament suffix (if word has a dot after a note/chord)
        if "." in word and not word.startswith("."):
            dot_idx = word.rfind(".")
            prefix = word[:dot_idx]
            partial = word[dot_idx:]  # e.g. ".st"
            for a in _ARTICULATIONS + _ORNAMENTS:
                if a.startswith(partial):
                    yield Completion(prefix + a, start_position=-len(word))
            return

        # Duration suffix (if word has a colon)
        if ":" in word:
            prefix = word.split(":")[0]
            for d in _DURATIONS:
                full = prefix + d
                if full.startswith(word):
                    yield Completion(full, start_position=-len(word))
            return

        # Dynamic suffix (if word has !)
        if "!" in word:
            prefix = word.split("!")[0]
            for d in _DYNAMICS:
                full = prefix + d
                if full.startswith(word):
                    yield Completion(full, start_position=-len(word))
            return

        # Commands / functions
        for f in _COMMANDS + _FUNCTIONS:
            if f.lower().startswith(wl):
                yield Completion(f, start_position=-len(word))

        # Chord names
        for c in _CHORD_NAMES:
            if c.lower().startswith(wl):
                yield Completion(c, start_position=-len(word))

        # Note names
        for n in _NOTE_NAMES:
            if n.lower().startswith(wl):
                yield Completion(n, start_position=-len(word))

        # Drum names
        for d in _DRUMS:
            if d.startswith(wl):
                yield Completion(d, start_position=-len(word))

        # Instrument names (outside of program= too, for general reference)
        if len(wl) >= 3:
            for inst in _INSTRUMENTS:
                if inst.startswith(wl):
                    yield Completion(f'"{inst}"', start_position=-len(word),
                                     display=inst)


# ── Syntax Suggestions ───────────────────────────────────────

from prompt_toolkit.auto_suggest import AutoSuggest, Suggestion


class DelphiAutoSuggest(AutoSuggest):
    """Contextual syntax suggestions with history fallback.

    Shows ghost-text hints for what you could type next based on
    the current input (e.g. a duration after a note, a chord after
    a bar pipe). Falls back to history-based suggestions when no
    contextual suggestion matches.
    """

    def __init__(self):
        self._history = AutoSuggestFromHistory()

    def get_suggestion(self, buffer, document):
        text = document.text_before_cursor
        hint = get_suggestion(text)
        if hint:
            return Suggestion(hint)
        # Fall back to history
        return self._history.get_suggestion(buffer, document)


# ── Key Bindings ─────────────────────────────────────────────

_bindings = KeyBindings()


@_bindings.add("c-p")
def _play_last(event):
    """Ctrl+P: re-play the last played notation."""
    buf = event.app.current_buffer
    # Insert play() wrapper around current text if it looks like notation
    text = buf.text.strip()
    if text and not text.startswith(("play(", "export(", "Song(", "#")):
        buf.text = f'play("{text}")'
        buf.cursor_position = len(buf.text)


# ── Banner and Help ──────────────────────────────────────────

BANNER = """\033[1;36m
    ____       __      __    _
   / __ \\___  / /___  / /_  (_)
  / / / / _ \\/ / __ \\/ __ \\/ /
 / /_/ /  __/ / /_/ / / / / /
/_____/\\___/_/ .___/_/ /_/_/
            /_/
\033[0m
\033[2mMusic scripting language — v{version}
Type expressions to play music. Try: play("C4 E4 G4 C5")
\033[0m
\033[1mQuick start:\033[0m
  C4 E4 G4                   Play notes (auto-detected)
  | C | Am | F | G |         Play chord progression (auto-detected)
  play("C4 E4 G4")           Play notes explicitly
  chord("Am7").play()         Play a chord
  tempo(90)                   Change tempo
  docs                        Quick-reference topics
  help                        Show all commands
  quit                        Exit
""".format(version=delphi.__version__)


HELP_TEXT = """
\033[1mDelphi REPL — Commands & Notation\033[0m

\033[1;33m━━ Playback & Export ━━\033[0m
  C4 E4 G4                         Type notation directly to play it
  | Cmaj7 | Am7 | F | G |          Bar notation is auto-detected too
  play("C4 E4 G4")                 Explicit play()
  chord("Am7").arpeggio("up").play()  Arpeggiate a chord
  export("song.mid", "...")        Export to MIDI

\033[1;33m━━ Multi-Track Songs ━━\033[0m
  s = Song("Title", tempo=120)
  s.track("Piano", "| C | Am |", program="piano")
  s.track("Bass", "C2:h G2:h", program="acoustic bass")
  s.play()
  s.export("song.mid")
  s.render("song.wav")

\033[1;33m━━ Sections & Patterns (large pieces) ━━\033[0m
  verse = Section("Verse")
  verse.add("Piano", "| C | Am | F | G |", program="piano")
  chorus = Section("Chorus")
  chorus.add("Piano", "| F | G | C | Am |", program="piano")
  song = build_song_from_sections("My Song", [(verse, 2), chorus])

\033[1;33m━━ Arrangement (timeline) ━━\033[0m
  arr = Arrangement("My Song", tempo=120)
  arr.section(intro)
  arr.mark("Verse")                    Rehearsal mark
  arr.section(verse, repeat=2)
  arr.mark("Chorus")
  arr.section(chorus)
  arr.build().play()                   Build and play from start
  arr.build(start_from="Chorus").play()  Play from rehearsal mark
  arr.extract("Piano").play()          Extract single voice
  arr.show()                           Print timeline overview

  # Timeline shorthand:
  arr.timeline(intro, "Verse", verse, verse, "Chorus", chorus)

\033[1;33m━━ Pattern Library (reusable motifs) ━━\033[0m
  register_pattern("riff", "C4:8 E4:8 G4:8 C5:8")
  p = get_pattern("riff")              Get as Pattern
  p.repeat(4).transpose(2)             Chain transforms
  list_patterns()                      List all registered

\033[1;33m━━ File Includes (split across files) ━━\033[0m
  ns = include("strings.delphi")       Execute another file
  violin_part = ns["violin"]           Access its variables
  include("patterns.py", globals())    Merge into current scope

\033[1;33m━━ Part Extraction ━━\033[0m
  song.extract("Piano").play()         Solo one track
  song.extract("Bass").export("bass.mid")  Export single part

\033[1;33m━━ Context ━━\033[0m
  tempo(120)                Set tempo (BPM)
  key("D major")            Set key
  time_sig(3, 4)            Set time signature
  instrument("violin")      Set instrument for play() (128 GM instruments)
  swing(0.5)                Set swing feel (0=straight, 0.5=triplet, 1=hard)
  humanize(0.1)             Add timing/velocity randomization

\033[1;33m━━ Theory Objects ━━\033[0m
  note("C4")                Note object (.midi, .transpose(), .play())
  chord("Cmaj7")            Chord object (.notes(), .arpeggio(), .play())
  scale("C", "dorian")      Scale object (.notes(), .play())

\033[1;33m━━ Notes & Chords ━━\033[0m
  Notes:     C4 D#5 Bb3             MIDI notes with octave
  Chords:    Cmaj7 Am7 Dm7b5 G7     Chord symbols in bars
             Cm6 C69 C7sus4 Cadd2   Extended chord types
             Calt Caug7 Cmaj13      Altered / extended
  Bars:      | Cmaj7 | Am7 |        Pipe-delimited bars
  Polyphony: C4,F4  C4,E4,G4       Simultaneous notes (comma)

\033[1;33m━━ Durations ━━\033[0m
  Basic:     C4:w  C4:h  C4:q  C4:8  C4:16  C4:32  C4:64  C4:128
  Breve:     C4:dw                  Double whole note
  Dotted:    C4:q.  C4:h.          Adds 50% duration
  DblDotted: C4:q.. C4:h..         Adds 75% duration
  Triplet:   C4:8t  C4:qt  C4:16t  Triplet subdivisions
  Tuplets:   (3 C4 E4 G4)          General tuplet group
             (5 C4 D4 E4 F4 G4)    Quintuplet group

\033[1;33m━━ Dynamics ━━\033[0m
  Standard:  C4!ppp  C4!pp  C4!p  C4!mp  C4!mf  C4!f  C4!ff  C4!fff
  Special:   C4!sfz  C4!sfp  C4!fp  C4!rfz  C4!fz
  Cresc:     cresc(p,f,4)          Crescendo over 4 notes
  Decresc:   dim(f,p,4)            Decrescendo over 4 notes

\033[1;33m━━ Articulations ━━\033[0m
  C4.stac     Staccato (short)      C4.stacc    Staccatissimo (very short)
  C4.ten      Tenuto (full length)  C4.port     Portato (medium detach)
  C4.acc      Accent (louder)       C4.marc     Marcato (loud + short)
  C4.ferm     Fermata (held)        C4.ghost    Ghost note (very soft)
  C4.leg      Legato (connected)    C4.pizz     Pizzicato (plucked)
  C4.mute     Muted

\033[1;33m━━ Ornaments ━━\033[0m
  C4.tr       Trill                 C4.mord     Mordent (upper)
  C4.lmord    Lower mordent         C4.turn     Turn / Gruppetto
  C4.grace    Grace note            C4.appoggiatura  Appoggiatura
  C4.trem     Tremolo               C4.gliss    Glissando
  C4.arp      Arpeggio              C4.roll     Drum roll

\033[1;33m━━ Advanced Patterns ━━\033[0m
  Subdivide:  [C4 E4 G4]           Three notes in one beat
  Ties:       C4:h~C4:q            Tie notes together
  Repeat:     C4:q*4               Repeat a note
  Euclidean:  bd(3,8)              Euclidean rhythm
  Swing:      swing(0.5)           Triplet swing feel
  Rests:      .  ~  r  rest        Silent beats
  Slow seq:   <C4 E4 G4>           One per cycle
  Elongate:   C4@2                 Hold 2x duration
  Random:     C4?  C4?0.3          Random removal
  Choice:     C4|E4|G4             Random choice
  Drums:      kick snare hihat     GM drum names
  Breath:     breath               Short pause (1/4 beat)
  Caesura:    caesura               Longer pause (1/2 beat)

\033[1;33m━━ Structure & Repeats ━━\033[0m
  D.C.:      ... DC                Da capo (repeat from start)
  D.S.:      ... segno ... DS      Dal segno (repeat from segno)
  Fine:      ... fine ... DC       Repeat to fine then stop
  Volta:     | A | B [1 C | [2 D | 1st/2nd endings

\033[1;33m━━ Track Effects ━━\033[0m  (use on Track objects: t = Track(...); t.gain(0.8))
  .gain(0.8)    Volume (0-2)        .pan(0.3)     Stereo position
  .reverb(0.5)  Reverb amount       .delay(0.3)   Echo effect
  .transpose(2) Shift semitones     .rev()        Reverse pattern
  .octave_up()  Up one octave       .octave_down() Down one octave
  .adsr(0.01, 0.1, 0.8, 0.1)       Envelope shaping

\033[1;33m━━ Utility Commands ━━\033[0m
  instruments                List all GM instrument names
  sf                         Show SoundFont status
  docs                       List all quick-reference topics
  docs <topic>               Show docs for a topic (e.g. docs drums)
  songs                      Show defined Song objects
  help                       This help text
  quit / exit                Leave the REPL

\033[1;33m━━ Keyboard Shortcuts ━━\033[0m
  Ctrl+C    Stop playback
  Ctrl+P    Wrap current text in play()
  Ctrl+D    Exit
  ↑ / ↓     History navigation
  Tab       Auto-complete
"""


# ── REPL Entry Point ─────────────────────────────────────────

def run_repl(project_dir: str | None = None):
    """Launch the interactive Delphi REPL.

    Args:
        project_dir: Path to a Delphi project directory (with delphi.toml).
                     If provided, the project's settings are loaded and its
                     directory becomes the working directory.
    """
    print(BANNER)

    # Load project if available
    if project_dir:
        _load_project(project_dir)

    # Persistent history across sessions
    history_path = os.path.expanduser("~/.delphi/repl_history")
    os.makedirs(os.path.dirname(history_path), exist_ok=True)

    session_kwargs = dict(
        history=FileHistory(history_path),
        auto_suggest=DelphiAutoSuggest(),
        completer=DelphiCompleter(),
        key_bindings=_bindings,
        multiline=False,
        enable_history_search=True,
    )
    if _LEXER:
        session_kwargs["lexer"] = _LEXER

    session: PromptSession = PromptSession(**session_kwargs)

    # Build namespace with delphi functions + Song/Track tracking
    _songs = {}

    namespace = {
        "play": delphi.play,
        "export": delphi.export,
        "tempo": delphi.tempo,
        "key": delphi.key,
        "time_sig": delphi.time_sig,
        "swing": delphi.swing,
        "humanize": delphi.humanize,
        "instrument": delphi.instrument,
        "note": delphi.note,
        "chord": delphi.chord,
        "scale": delphi.scale,
        "Song": delphi.Song,
        "Track": delphi.Track,
        "get_context": delphi.get_context,
        "reset_context": delphi.reset_context,
        "parse_notation": delphi.parse_notation,
        "ensure_soundfont": delphi.ensure_soundfont,
        "soundfont_info": delphi.soundfont_info,
        "GM_INSTRUMENTS": delphi.GM_INSTRUMENTS,
        "help": lambda: print(HELP_TEXT),
        "_songs": _songs,
    }

    # Try to import Section/Pattern/Voice/Arrangement/etc
    try:
        from delphi.composition import (
            Section, Pattern, Voice, Arrangement, PatternLibrary,
            build_song_from_sections, register_pattern, get_pattern,
            list_patterns, include,
        )
        namespace["Section"] = Section
        namespace["Pattern"] = Pattern
        namespace["Voice"] = Voice
        namespace["Arrangement"] = Arrangement
        namespace["PatternLibrary"] = PatternLibrary
        namespace["build_song_from_sections"] = build_song_from_sections
        namespace["register_pattern"] = register_pattern
        namespace["get_pattern"] = get_pattern
        namespace["list_patterns"] = list_patterns
        namespace["include"] = include
    except ImportError:
        pass

    while True:
        try:
            text = session.prompt(
                HTML('<style fg="ansicyan" bold="true">𝄞 </style>'),
            ).strip()
        except EOFError:
            print("\nBye!")
            break
        except KeyboardInterrupt:
            continue  # Ctrl+C at prompt just clears the line

        if not text:
            continue

        if text in ("quit", "exit", "quit()", "exit()"):
            print("Bye!")
            break

        if text == "help":
            print(HELP_TEXT)
            continue

        # Special commands
        if text == "instruments":
            _show_instruments()
            continue

        if text == "sf":
            delphi.soundfont_info()
            continue

        if text == "docs":
            print(get_docs_index())
            continue

        if text.startswith("docs "):
            topic = text[5:].strip()
            print(get_docs(topic))
            continue

        if text == "songs":
            _show_songs(namespace)
            continue

        if text == "tracks":
            _show_tracks(namespace)
            continue

        # Inline play: detect raw notation and auto-play it
        if _looks_like_notation(text):
            try:
                delphi.play(text)
            except KeyboardInterrupt:
                print("\n\033[33mStopped.\033[0m")
            except Exception as e:
                print(f"\033[31mError:\033[0m {e}")
            continue

        # Execute as Python with our namespace
        try:
            result = eval(text, {"__builtins__": __builtins__}, namespace)
            if result is not None:
                print(repr(result))
                # Track Song objects
                if isinstance(result, delphi.Song):
                    _songs[result.title] = result
        except SyntaxError:
            try:
                exec(text, {"__builtins__": __builtins__}, namespace)
            except KeyboardInterrupt:
                print("\n\033[33mStopped.\033[0m")
            except Exception as e:
                print(f"\033[31mError:\033[0m {e}")
        except KeyboardInterrupt:
            print("\n\033[33mStopped.\033[0m")
        except Exception as e:
            print(f"\033[31mError:\033[0m {e}")


# ── Project Loading ───────────────────────────────────────────

def _load_project(project_dir: str) -> None:
    """Load a Delphi project: read delphi.toml and apply settings."""
    import os as _os

    toml_path = _os.path.join(project_dir, "delphi.toml")
    if not _os.path.exists(toml_path):
        return

    # Parse the TOML (simple key=value parser, no dependency needed)
    config = _parse_simple_toml(toml_path)

    project_name = config.get("project", {}).get("name", "")
    settings = config.get("settings", {})

    if project_name:
        print(f"  \033[1mProject:\033[0m {project_name}")

    if "tempo" in settings:
        try:
            delphi.tempo(float(settings["tempo"]))
        except (ValueError, TypeError):
            pass
    if "key" in settings:
        delphi.key(str(settings["key"]))
    if "time_sig" in settings:
        ts = str(settings["time_sig"])
        if "/" in ts:
            num, den = ts.split("/", 1)
            try:
                delphi.time_sig(int(num.strip()), int(den.strip()))
            except (ValueError, TypeError):
                pass
    if "soundfont" in settings and str(settings["soundfont"]).strip():
        delphi.set_soundfont(str(settings["soundfont"]).strip())

    # List .delphi files available
    scripts = [f for f in _os.listdir(project_dir)
               if f.endswith((".delphi", ".py")) and not f.startswith(".")]
    if scripts:
        print(f"  \033[1mFiles:\033[0m {', '.join(sorted(scripts))}")

    print()


def _parse_simple_toml(path: str) -> dict:
    """Minimal TOML parser for delphi.toml (handles sections and key=value)."""
    config: dict = {}
    current_section = config

    with open(path) as f:
        for line in f:
            line = line.strip()
            if not line or line.startswith("#"):
                continue
            if line.startswith("[") and line.endswith("]"):
                section_name = line[1:-1].strip()
                config[section_name] = {}
                current_section = config[section_name]
            elif "=" in line:
                key, _, value = line.partition("=")
                key = key.strip()
                value = value.strip()
                # Parse quoted strings (find matching close quote, ignore rest)
                if value.startswith('"'):
                    inner = value[1:]
                    end = inner.find('"')
                    value = inner[:end] if end >= 0 else inner
                elif value.startswith("'"):
                    inner = value[1:]
                    end = inner.find("'")
                    value = inner[:end] if end >= 0 else inner
                # Try numeric conversion
                else:
                    # Strip inline comments for unquoted values
                    if "#" in value:
                        value = value[:value.index("#")].strip()
                    try:
                        value = int(value)
                    except ValueError:
                        try:
                            value = float(value)
                        except ValueError:
                            pass
                current_section[key] = value

    return config


# ── Notation Detection ────────────────────────────────────────

import re as _re

# All known articulation / ornament dot-suffixes (from notation.py)
_DOT_SUFFIXES = (
    "stac", "staccato", "stacc", "staccatissimo",
    "ten", "tenuto", "port", "portato",
    "acc", "accent", "marc", "marcato",
    "ferm", "fermata", "ghost", "leg", "legato",
    "pizz", "pizzicato", "mute",
    "tr", "trill", "mord", "mordent", "lmord",
    "turn", "gruppetto", "grace", "acciaccatura", "appoggiatura",
    "trem", "tremolo", "gliss", "glissando", "arp", "arpeggio", "roll",
)
_DOT_SUFFIX_RE = r'(?:\.(?:' + '|'.join(_DOT_SUFFIXES) + r'))?'

# Duration suffixes: :w :h :q :e :s etc.
_DUR_RE = r'(?::[whqes](?:\.{0,2})?)?'

_NOTATION_RE = _re.compile(
    r'^[\s|]*'  # optional leading whitespace/pipes
    r'(?:'
    r'[A-Ga-g][#b]{0,2}\d' + _DOT_SUFFIX_RE + _DUR_RE +  # note with octave: C4, F#5.stac:q
    r'|[A-Ga-g][#b]{0,2}(?:maj|min|m|dim|aug|sus|add|alt|7|9|11|13|6|5|°|ø|\+)\S*'  # chord with quality: Cmaj7, Am
    r'|[A-Ga-g][#b]{0,2}(?=[\s|]|$)'  # bare note as chord shorthand: C, F, Bb (in bar context)
    r'|(?:kick|snare|hihat|bd|sd|hh|oh|ch|rd|cr|tom|clap|rim|cp)' + _DOT_SUFFIX_RE + _DUR_RE +
    r'|\|'      # bar lines
    r'|[.~r]'   # rests
    r'|cresc\([^)]*\)'   # crescendo
    r'|dim\([^)]*\)'     # diminuendo
    r'|breath|caesura'    # breaths
    r'|\s+'
    r')+'
    r'[\s|]*$'
)


def _looks_like_notation(text: str) -> bool:
    """Check if text looks like raw musical notation (not Python code)."""
    # Must not look like Python assignments or definitions
    if any(kw in text for kw in ('=', 'import ', 'def ', 'class ', 'from ')):
        return False
    # Allow parens only if they look like notation functions (cresc, dim, euclidean)
    if '(' in text or ')' in text:
        # Only pass through if the entire text matches notation regex
        pass
    # Must not be a known command
    if text in ('help', 'quit', 'exit', 'instruments', 'sf', 'songs', 'tracks'):
        return False
    return bool(_NOTATION_RE.match(text))


# ── Utility Displays ─────────────────────────────────────────

def _show_instruments():
    """Display available GM instruments in columns."""
    names = sorted(GM_INSTRUMENTS.keys())
    print(f"\n\033[1mGeneral MIDI Instruments ({len(names)}):\033[0m\n")
    col_width = 24
    cols = 3
    for i in range(0, len(names), cols):
        row = names[i:i + cols]
        print("  " + "".join(n.ljust(col_width) for n in row))
    print()


def _show_songs(namespace):
    """Show Song objects in the REPL namespace."""
    songs = [v for v in namespace.values() if isinstance(v, delphi.Song)]
    if not songs:
        print("  (no Song objects defined yet)")
        return
    print()
    for s in songs:
        print(f"  {s}")
    print()


def _show_tracks(namespace):
    """Show tracks from all Song objects."""
    songs = [v for v in namespace.values() if isinstance(v, delphi.Song)]
    if not songs:
        print("  (no Song objects with tracks)")
        return
    for s in songs:
        print(f"\n  \033[1m{s.title}\033[0m ({s.tempo} BPM, {s.key})")
        for i, t in enumerate(s.tracks):
            prog = t.program if isinstance(t.program, str) else f"GM {t.program}"
            print(f"    {i + 1}. {t.name} [{prog}] vel={t.velocity}")
    print()
