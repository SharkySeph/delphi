"""Live playback visualizer -- scrolling display of notes as they sound."""

import sys
import threading
import time
from delphi.notation import TICKS_PER_QUARTER, DRUM_MAP

# Reverse drum map: MIDI number -> shortest name
_DRUM_NAMES: dict[int, str] = {}
for _name, _midi in DRUM_MAP.items():
    if _midi not in _DRUM_NAMES or len(_name) < len(_DRUM_NAMES[_midi]):
        _DRUM_NAMES[_midi] = _name

_NOTE_NAMES = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"]

# ANSI colors
_CYAN = "\033[36m"
_YELLOW = "\033[33m"
_MAGENTA = "\033[35m"
_DIM = "\033[2m"
_BOLD = "\033[1m"
_RESET = "\033[0m"


def _midi_to_label(midi: int) -> str:
    """Convert MIDI number to note name like C4."""
    octave = (midi // 12) - 1
    return f"{_NOTE_NAMES[midi % 12]}{octave}"


def _format_event(kind: str, midi_notes: list[int], velocity: int) -> str:
    """Format a single event for display."""
    if kind == "drum":
        names = [_DRUM_NAMES.get(m, f"drum{m}") for m in midi_notes]
        return f"{_MAGENTA}{' '.join(names)}{_RESET}"
    elif kind == "chord" or len(midi_notes) > 1:
        labels = [_midi_to_label(m) for m in sorted(midi_notes)]
        return f"{_YELLOW}{'+'.join(labels)}{_RESET}"
    elif kind == "rest" or not midi_notes:
        return f"{_DIM}.{_RESET}"
    else:
        return f"{_CYAN}{_midi_to_label(midi_notes[0])}{_RESET}"


def _build_timeline(events, bpm: float):
    """Build a sorted list of (real_time_seconds, display_string) from events.

    Groups events that start at the same tick into a single display moment.
    """
    if not events:
        return []

    # Seconds per tick
    spt = 60.0 / (bpm * TICKS_PER_QUARTER)

    # Group by tick
    by_tick: dict[int, list] = {}
    for evt in events:
        if evt.kind == "rest":
            continue
        t = evt.tick
        if t not in by_tick:
            by_tick[t] = []
        by_tick[t].append(evt)

    timeline = []
    for tick in sorted(by_tick):
        real_time = tick * spt
        parts = []
        for evt in by_tick[tick]:
            parts.append(_format_event(evt.kind, evt.midi_notes, evt.velocity))
        display = "  ".join(parts)
        timeline.append((real_time, display))

    return timeline


def visualize(events, bpm: float, stop_flag=None) -> threading.Thread:
    """Start a background thread that prints events as they play.

    Returns the thread so the caller can join() it if needed.
    The display thread stops when stop_flag is set or all events are shown.
    """
    timeline = _build_timeline(events, bpm)
    if not timeline:
        return threading.Thread()  # no-op

    def _display():
        start = time.monotonic()
        idx = 0
        # Use a limited-width scrolling line
        try:
            cols = min(100, max(40, _get_term_width() - 2))
        except Exception:
            cols = 80

        buf: list[str] = []
        buf_plain_len = 0

        while idx < len(timeline):
            if stop_flag is not None and hasattr(stop_flag, 'is_stopped') and stop_flag.is_stopped():
                break

            target_time, display = timeline[idx]
            now = time.monotonic() - start
            wait = target_time - now
            if wait > 0:
                # Sleep in small increments to check stop_flag
                end = time.monotonic() + wait
                while time.monotonic() < end:
                    if stop_flag is not None and hasattr(stop_flag, 'is_stopped') and stop_flag.is_stopped():
                        return
                    time.sleep(min(0.05, end - time.monotonic()))

            # Calculate plain text length of display (strip ANSI)
            plain = _strip_ansi(display)
            token_len = len(plain)

            # If adding this token would overflow, start a new line
            if buf_plain_len + token_len + 1 > cols and buf:
                sys.stdout.write(f"\r\033[K{_DIM}♪{_RESET} {'  '.join(buf)}\n")
                sys.stdout.flush()
                buf = []
                buf_plain_len = 0

            buf.append(display)
            buf_plain_len += token_len + 2  # +2 for separator

            # Write current line (overwrite)
            line = f"{_DIM}♪{_RESET} {'  '.join(buf)}"
            sys.stdout.write(f"\r\033[K{line}")
            sys.stdout.flush()

            idx += 1

        # Final newline
        if buf:
            sys.stdout.write("\n")
            sys.stdout.flush()

    t = threading.Thread(target=_display, daemon=True)
    t.start()
    return t


def _strip_ansi(s: str) -> str:
    """Remove ANSI escape codes from a string."""
    import re
    return re.sub(r'\033\[[0-9;]*m', '', s)


def _get_term_width() -> int:
    """Get terminal width, fallback to 80."""
    try:
        import shutil
        return shutil.get_terminal_size().columns
    except Exception:
        return 80
