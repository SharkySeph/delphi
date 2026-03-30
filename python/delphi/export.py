"""
Export interface — write notation to MIDI (and later WAV, MusicXML).
"""

from delphi.context import get_context
from delphi.notation import parse, events_to_tuples


def export(path: str, notation: str = "", events: list | None = None,
           track_name: str = "Delphi", program: int = 0) -> None:
    """
    Export notation or events to a file.

    Supports:
        export("out.mid", "| Cmaj7 | Am7 | Fmaj7 | G7 |")
        export("out.mid", events=my_tuples)

    The format is inferred from the file extension.
    """
    ctx = get_context()

    # Get tuples from notation or events
    if events is not None:
        tuples = events
    elif notation:
        parsed = parse(notation, default_velocity=80)
        tuples = events_to_tuples(parsed)
    else:
        raise ValueError("Provide either notation string or events list")

    if not tuples:
        print("(nothing to export)")
        return

    if path.endswith(".mid") or path.endswith(".midi"):
        _export_midi(tuples, path, ctx, track_name, program)
    else:
        raise ValueError(f"Unsupported export format: {path}")


def _export_midi(tuples, path, ctx, track_name, program):
    """Export to MIDI file."""
    try:
        from delphi._engine import export_midi
        export_midi(
            tuples,
            path,
            bpm=ctx.bpm,
            time_sig=(ctx.time_sig_num, ctx.time_sig_den),
            track_name=track_name,
            program=program,
        )
        print(f"Exported MIDI: {path}")
    except ImportError:
        _fallback_midi(tuples, path, ctx)


def _fallback_midi(tuples, path, ctx):
    """Pure-Python MIDI export fallback (minimal, no Rust needed)."""
    print(f"[Delphi] Rust engine not built. Writing minimal MIDI to {path}")
    # Write a bare-minimum MIDI file using pure Python
    ppq = 480
    data = bytearray()

    # Header
    data.extend(b"MThd")
    data.extend((6).to_bytes(4, "big"))
    data.extend((0).to_bytes(2, "big"))  # format 0
    data.extend((1).to_bytes(2, "big"))  # 1 track
    data.extend(ppq.to_bytes(2, "big"))

    # Track
    track_data = bytearray()

    # Tempo meta event
    uspqn = int(60_000_000 / ctx.bpm)
    track_data.extend(b"\x00\xff\x51\x03")
    track_data.extend(uspqn.to_bytes(3, "big"))

    # Sort events
    sorted_events = sorted(tuples, key=lambda t: t[2])

    # Build note-on / note-off
    raw = []
    for midi, vel, tick, dur in sorted_events:
        raw.append((tick, 0x90, midi, vel))
        raw.append((tick + dur, 0x80, midi, 0))
    raw.sort(key=lambda r: (r[0], r[1] & 0xF0))

    current_tick = 0
    for abs_tick, status, d1, d2 in raw:
        delta = abs_tick - current_tick
        _write_vlq(track_data, delta)
        track_data.extend([status, d1, d2])
        current_tick = abs_tick

    # End of track
    track_data.extend(b"\x00\xff\x2f\x00")

    data.extend(b"MTrk")
    data.extend(len(track_data).to_bytes(4, "big"))
    data.extend(track_data)

    with open(path, "wb") as f:
        f.write(data)
    print(f"Exported MIDI (fallback): {path}")


def _write_vlq(buf: bytearray, value: int):
    """Write a MIDI variable-length quantity."""
    if value == 0:
        buf.append(0)
        return
    bytes_list = []
    while value > 0:
        bytes_list.append(value & 0x7F)
        value >>= 7
    bytes_list.reverse()
    for i, b in enumerate(bytes_list):
        if i < len(bytes_list) - 1:
            buf.append(b | 0x80)
        else:
            buf.append(b)
