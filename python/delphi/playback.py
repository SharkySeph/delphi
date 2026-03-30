"""
Playback interface — bridges the Python DSL to the Rust audio engine.
Falls back to a pure-Python beep if the Rust engine isn't built yet.
"""

from delphi.context import get_context
from delphi.notation import parse, events_to_tuples


def play(notation: str) -> None:
    """
    Parse and play a notation string.

    Examples:
        play("C4 E4 G4")
        play("| Cmaj7 | Am7 | Fmaj7 | G7 |")
    """
    ctx = get_context()
    events = parse(notation, default_velocity=80)
    tuples = events_to_tuples(events)
    if not tuples:
        print("(nothing to play)")
        return
    play_notes(tuples)


def play_notes(tuples: list[tuple[int, int, int, int]]) -> None:
    """
    Play raw note tuples: (midi_note, velocity, tick, duration_ticks).
    Uses the Rust engine if available, otherwise a pure-Python fallback.
    """
    ctx = get_context()
    try:
        from delphi._engine import play_events
        play_events(tuples, bpm=ctx.bpm)
    except ImportError:
        _fallback_play(tuples, ctx.bpm)


def _fallback_play(tuples: list[tuple[int, int, int, int]], bpm: float) -> None:
    """Pure-Python fallback using simpleaudio or just printing."""
    print(f"[Delphi] Rust engine not built. Showing events (tempo={bpm} BPM):")
    for midi, vel, tick, dur in tuples:
        beat = tick / 480
        dur_beats = dur / 480
        print(f"  MIDI {midi:3d}  vel={vel:3d}  beat={beat:.2f}  dur={dur_beats:.2f}")
    print("[Delphi] To hear audio, build with: maturin develop")
