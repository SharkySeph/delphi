"""
Global musical context: tempo, key, time signature.
These are module-level singletons — set once, used by play/export.
"""

from dataclasses import dataclass, field


@dataclass
class Context:
    bpm: float = 120.0
    key_name: str = "C major"
    time_sig_num: int = 4
    time_sig_den: int = 4
    swing: float = 0.0          # 0.0 = straight, 0.5 = triplet swing, 1.0 = hard swing
    humanize: float = 0.0       # 0.0-1.0 timing/velocity randomization
    program: int = 0            # GM instrument program number (0 = piano)
    program_name: str = "piano" # Human-readable instrument name


_ctx = Context()


def tempo(bpm: float) -> None:
    """Set the global tempo in BPM."""
    _ctx.bpm = float(bpm)


def key(name: str) -> None:
    """Set the global key, e.g. 'C major', 'F# minor', 'Bb dorian'."""
    _ctx.key_name = name


def time_sig(numerator: int, denominator: int) -> None:
    """Set the global time signature, e.g. time_sig(4, 4)."""
    _ctx.time_sig_num = numerator
    _ctx.time_sig_den = denominator


def swing(amount: float = 0.5) -> None:
    """Set swing feel. 0=straight, 0.5=triplet swing, 1.0=hard swing.

    Swing delays every other eighth note. At 0.5 (default), the offbeat
    lands on the last triplet eighth — the classic jazz shuffle feel.
    """
    _ctx.swing = max(0.0, min(1.0, float(amount)))


def humanize(amount: float = 0.1) -> None:
    """Add timing/velocity randomization. 0=robotic, 1.0=very loose."""
    _ctx.humanize = max(0.0, min(1.0, float(amount)))


def instrument(name: str) -> None:
    """Set the default instrument for play().

    Accepts any GM instrument name, e.g.:
        instrument("violin")
        instrument("flute")
        instrument("acoustic guitar nylon")

    Use 'instruments' in the REPL to see all 128 names.
    """
    from delphi.song import GM_INSTRUMENTS
    key = name.lower().strip()
    if key not in GM_INSTRUMENTS:
        raise ValueError(
            f"Unknown instrument '{name}'. "
            f"Use a GM name like 'piano', 'violin', 'flute' or type 'instruments' in the REPL."
        )
    _ctx.program = GM_INSTRUMENTS[key]
    _ctx.program_name = key


def get_context() -> Context:
    """Return the current global context."""
    return _ctx


def reset_context() -> None:
    """Reset all context to defaults."""
    global _ctx
    _ctx = Context()
