"""
Live-code playback views for Delphi Studio.

Provides Strudel-style highlighting of source text during playback:
- Single-cell view (F5): cell source fills the screen, current token highlighted
- Consolidated view (F6): all tracks stacked, each highlighting its position
"""

from __future__ import annotations

import re
import threading
import time
from dataclasses import dataclass
from typing import Optional

from delphi.notation import (
    TICKS_PER_QUARTER,
    parse as parse_notation,
)


# ── Source mapping ────────────────────────────────────────────


@dataclass
class SourceSpan:
    """Maps a region of source text to a tick range."""
    char_start: int
    char_end: int
    tick_start: int
    tick_end: int


def build_source_map(source: str) -> list[SourceSpan]:
    """Build a mapping from character positions to tick positions.

    Works for both bar notation (|...|) and sequence notation (space-separated).
    Skips pragma/comment lines.
    """
    # Strip pragma lines, tracking char offsets
    lines = source.split("\n")
    clean_lines: list[tuple[int, str]] = []  # (char_offset, line)
    offset = 0
    for line in lines:
        stripped = line.strip()
        if stripped.startswith("#") or not stripped:
            offset += len(line) + 1
            continue
        clean_lines.append((offset, line))
        offset += len(line) + 1

    if not clean_lines:
        return []

    # Rejoin clean source and track where each line starts in original
    clean_source = "\n".join(line for _, line in clean_lines)

    # Detect bar vs sequence notation
    if "|" in clean_source and not _looks_like_random_choice(clean_source):
        return _map_bars(clean_lines, source)
    else:
        return _map_sequence(clean_lines, source)


def _looks_like_random_choice(notation: str) -> bool:
    stripped = notation.strip()
    if stripped.startswith("|") or stripped.endswith("|"):
        return False
    if " | " in stripped:
        return False
    return True


def _map_bars(clean_lines: list[tuple[int, str]], full_source: str) -> list[SourceSpan]:
    """Map bar notation: each |...| segment gets a tick range."""
    from delphi.context import get_context
    ctx = get_context()
    measure_ticks = TICKS_PER_QUARTER * ctx.time_sig_num * (4 // ctx.time_sig_den)

    spans: list[SourceSpan] = []
    current_tick = 0

    for line_offset, line in clean_lines:
        # Find all bar segments in this line by locating | delimiters
        pipe_positions = [i for i, ch in enumerate(line) if ch == "|"]

        if len(pipe_positions) < 2:
            # No bars on this line — treat as sequence tokens
            spans.extend(_map_tokens_in_line(line, line_offset, current_tick))
            # Advance tick by approximate duration
            if spans:
                current_tick = spans[-1].tick_end
            continue

        # Each pair of pipes is a bar
        for j in range(len(pipe_positions) - 1):
            bar_start_in_line = pipe_positions[j] + 1
            bar_end_in_line = pipe_positions[j + 1]
            bar_text = line[bar_start_in_line:bar_end_in_line].strip()

            if not bar_text:
                current_tick += measure_ticks
                continue

            # Character span includes the content between pipes
            char_start = line_offset + bar_start_in_line
            char_end = line_offset + bar_end_in_line

            spans.append(SourceSpan(
                char_start=char_start,
                char_end=char_end,
                tick_start=current_tick,
                tick_end=current_tick + measure_ticks,
            ))
            current_tick += measure_ticks

    return spans


def _map_tokens_in_line(
    line: str, line_offset: int, start_tick: int
) -> list[SourceSpan]:
    """Map individual tokens in a line to tick ranges."""
    from delphi.notation import parse as _parse
    tokens = _find_token_spans(line)
    if not tokens:
        return []

    # Parse the line to get events with tick info
    events = _parse(line.strip(), default_velocity=80)

    spans: list[SourceSpan] = []
    current_tick = start_tick

    for i, (tok_start, tok_end, tok_text) in enumerate(tokens):
        if tok_text.startswith("#"):
            continue
        # Match to the next event's duration
        if i < len(events):
            evt = events[i]
            dur = evt.duration_ticks
        else:
            dur = TICKS_PER_QUARTER

        spans.append(SourceSpan(
            char_start=line_offset + tok_start,
            char_end=line_offset + tok_end,
            tick_start=current_tick,
            tick_end=current_tick + dur,
        ))
        current_tick += dur

    return spans


def _map_sequence(
    clean_lines: list[tuple[int, str]], full_source: str
) -> list[SourceSpan]:
    """Map sequence notation: each token gets a tick range."""
    # Parse all clean text to get events with ticks
    clean_text = "\n".join(line for _, line in clean_lines)
    events = parse_notation(clean_text, default_velocity=80)

    spans: list[SourceSpan] = []
    event_idx = 0

    for line_offset, line in clean_lines:
        tokens = _find_token_spans(line)
        for tok_start, tok_end, tok_text in tokens:
            stripped = tok_text.strip()
            if not stripped or stripped.startswith("#"):
                continue

            if event_idx < len(events):
                evt = events[event_idx]
                tick_start = evt.tick
                tick_end = evt.tick + evt.duration_ticks
                event_idx += 1
            else:
                # Ran out of events — estimate
                prev_end = spans[-1].tick_end if spans else 0
                tick_start = prev_end
                tick_end = prev_end + TICKS_PER_QUARTER

            spans.append(SourceSpan(
                char_start=line_offset + tok_start,
                char_end=line_offset + tok_end,
                tick_start=tick_start,
                tick_end=tick_end,
            ))

    return spans


def _find_token_spans(line: str) -> list[tuple[int, int, str]]:
    """Find (start, end, text) for each whitespace-delimited token in a line."""
    results = []
    for m in re.finditer(r'\S+', line):
        results.append((m.start(), m.end(), m.group()))
    return results


# ── Tick tracking ─────────────────────────────────────────────


class TickClock:
    """Thread-safe playback clock that maps wall time to ticks."""

    def __init__(self, bpm: float):
        self.bpm = bpm
        self._ticks_per_sec = (bpm * TICKS_PER_QUARTER) / 60.0
        self._start_time: Optional[float] = None
        self._stopped = False

    def start(self):
        self._start_time = time.monotonic()
        self._stopped = False

    def stop(self):
        self._stopped = True

    @property
    def current_tick(self) -> int:
        if self._start_time is None or self._stopped:
            return 0
        elapsed = time.monotonic() - self._start_time
        return int(elapsed * self._ticks_per_sec)

    @property
    def is_running(self) -> bool:
        return self._start_time is not None and not self._stopped


# ── Formatted text builders ──────────────────────────────────


def format_source_highlighted(
    source: str,
    spans: list[SourceSpan],
    current_tick: int,
) -> list[tuple[str, str]]:
    """Build prompt_toolkit formatted text with the active span highlighted.

    Returns a list of (style, text) tuples.
    """
    # Find the active span(s) — all spans whose tick range contains current_tick
    active_ranges: list[tuple[int, int]] = []
    for sp in spans:
        if sp.tick_start <= current_tick < sp.tick_end:
            active_ranges.append((sp.char_start, sp.char_end))

    if not active_ranges:
        # Nothing active — return dimmed source
        return [("fg:ansiwhite", source)]

    # Merge overlapping ranges
    active_ranges.sort()
    merged: list[tuple[int, int]] = [active_ranges[0]]
    for start, end in active_ranges[1:]:
        if start <= merged[-1][1]:
            merged[-1] = (merged[-1][0], max(merged[-1][1], end))
        else:
            merged.append((start, end))

    # Build formatted text segments
    result: list[tuple[str, str]] = []
    pos = 0
    for hl_start, hl_end in merged:
        if pos < hl_start:
            result.append(("fg:#888888", source[pos:hl_start]))
        result.append(("bold fg:ansiwhite bg:ansicyan", source[hl_start:hl_end]))
        pos = hl_end

    if pos < len(source):
        result.append(("fg:#888888", source[pos:]))

    return result


def format_tracks_highlighted(
    tracks: list[dict],
    current_tick: int,
) -> list[tuple[str, str]]:
    """Build formatted text for the consolidated multi-track view.

    Each track dict has: label, source, spans, clean_source.
    """
    result: list[tuple[str, str]] = []

    for i, track in enumerate(tracks):
        if i > 0:
            result.append(("", "\n"))
        # Track header
        label = track.get("label", f"Track {i + 1}")
        result.append(("bold fg:ansicyan", f"── {label} "))
        result.append(("fg:#444444", "─" * max(0, 50 - len(label))))
        result.append(("", "\n"))

        # Highlighted source
        source = track["clean_source"]
        spans = track["spans"]
        result.extend(format_source_highlighted(source, spans, current_tick))
        result.append(("", "\n"))

    return result
