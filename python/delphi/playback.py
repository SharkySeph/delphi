"""
Playback interface — bridges the Python DSL to the Rust audio engine.

Prefers SoundFont playback for all instruments (including drums on channel 9).
Falls back to the built-in oscillator synth if no SoundFont is available,
and to a pure-Python text dump if the Rust engine isn't built.
"""

from delphi.context import get_context
from delphi.notation import parse, events_to_tuples


def play(notation: str, stop_flag=None, channel=None, instrument=None,
         loop: bool = False, visualize: bool = True) -> None:
    """
    Parse and play a notation string.

    Uses SoundFont playback by default. Drum events are automatically
    routed to MIDI channel 9. Falls back to the built-in oscillator
    synth only when no SoundFont is available.

    Args:
        notation: A Delphi notation string.
        stop_flag: Optional StopFlag for cancellation.
        channel: Override MIDI channel (0-15). Drums auto-route to 9.
        instrument: Override instrument name (e.g. "violin", "flute").
        loop: If True, repeat playback until Ctrl+C or stop_flag.stop().
        visualize: If True, show a live scrolling display of notes.
    """
    ctx = get_context()
    events = parse(notation, default_velocity=80)

    # Resolve program number: explicit arg > context
    program = ctx.program
    if instrument:
        from delphi.song import GM_INSTRUMENTS
        key = instrument.lower().strip()
        if key in GM_INSTRUMENTS:
            program = GM_INSTRUMENTS[key]

    while True:
        # Start live visualizer if enabled
        viz_thread = None
        if visualize and events:
            try:
                from delphi.visualizer import visualize as _viz
                viz_thread = _viz(events, ctx.bpm, stop_flag=stop_flag)
            except Exception:
                pass  # visualizer is purely cosmetic

        # Try SoundFont first — it handles all GM instruments + drums properly
        from delphi.soundfont import get_soundfont_path
        sf_path = get_soundfont_path()
        if sf_path:
            try:
                from delphi._engine import play_sf
                sf_tuples = _events_to_sf_tuples(events, program, channel)
                if not sf_tuples:
                    print("(nothing to play)")
                    return
                play_sf(sf_path, sf_tuples, bpm=ctx.bpm, stop_flag=stop_flag)
            except ImportError:
                # Rust engine not built — use fallback
                tuples = events_to_tuples(events)
                if not tuples:
                    print("(nothing to play)")
                    return
                play_notes(tuples, stop_flag=stop_flag)
        else:
            # Fallback: basic oscillator synth (no instrument variety, no drums)
            tuples = events_to_tuples(events)
            if not tuples:
                print("(nothing to play)")
                return
            play_notes(tuples, stop_flag=stop_flag)

        # Wait for visualizer to finish
        if viz_thread is not None:
            viz_thread.join(timeout=1.0)

        if not loop:
            break
        # Check stop_flag between loop iterations
        if stop_flag is not None and hasattr(stop_flag, 'is_stopped') and stop_flag.is_stopped():
            break


def _events_to_sf_tuples(events, program: int, channel=None):
    """Convert parsed events to SoundFont tuples: (midi, vel, tick, dur, channel, program).

    Drum events are always routed to channel 9 with program 0.
    Other events use the provided channel/program.
    """
    sf_tuples = []
    ch = channel if channel is not None else 0
    for evt in events:
        if evt.kind == "rest":
            continue
        if evt.kind == "drum":
            for midi, vel, tick, dur in evt.to_tuples():
                sf_tuples.append((midi, vel, tick, dur, 9, 0))
        else:
            for midi, vel, tick, dur in evt.to_tuples():
                sf_tuples.append((midi, vel, tick, dur, ch, program))
    return sf_tuples


def play_notes(tuples: list[tuple[int, int, int, int]], stop_flag=None) -> None:
    """
    Play raw note tuples: (midi_note, velocity, tick, duration_ticks).
    Uses the Rust engine if available, otherwise a pure-Python fallback.
    """
    ctx = get_context()
    try:
        from delphi._engine import play_events
        play_events(tuples, bpm=ctx.bpm, stop_flag=stop_flag)
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
