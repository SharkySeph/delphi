"""
Delphi — A music scripting language for composing real music.

Usage:
    from delphi import *

    tempo(120)
    key("C major")
    play("C4 E4 G4")
    play("| Cmaj7 | Am7 | Fmaj7 | G7 |")
"""

from delphi.context import (
    tempo,
    key,
    time_sig,
    swing,
    humanize,
    instrument,
    get_context,
    reset_context,
)
from delphi.notation import parse as parse_notation
from delphi.theory import chord, scale, note
from delphi.playback import play, play_notes
from delphi.export import export
from delphi.song import Song, Track, GM_INSTRUMENTS
from delphi.composition import (
    Section,
    Pattern,
    Voice,
    Arrangement,
    PatternLibrary,
    build_song_from_sections,
    register_pattern,
    get_pattern,
    list_patterns,
    include,
)
from delphi.soundfont import (
    set_soundfont,
    get_soundfont_path,
    ensure_soundfont,
    soundfont_info,
)


def run_studio(target=None):
    """Launch Delphi Studio TUI (lazy import to avoid loading prompt_toolkit eagerly)."""
    from delphi.studio import run_studio as _run
    return _run(target)

__version__ = "0.2.0"

__all__ = [
    "tempo",
    "key",
    "time_sig",
    "swing",
    "humanize",
    "instrument",
    "get_context",
    "reset_context",
    "parse_notation",
    "chord",
    "scale",
    "note",
    "play",
    "play_notes",
    "export",
    "Song",
    "Track",
    "GM_INSTRUMENTS",
    "Section",
    "Pattern",
    "Voice",
    "Arrangement",
    "PatternLibrary",
    "build_song_from_sections",
    "register_pattern",
    "get_pattern",
    "list_patterns",
    "include",
    "set_soundfont",
    "get_soundfont_path",
    "ensure_soundfont",
    "soundfont_info",
    "run_studio",
]
