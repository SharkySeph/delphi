"""
Delphi Studio — a terminal notebook IDE for composing multi-track songs.

Think Jupyter meets Strudel meets a DAW — all in the terminal via prompt_toolkit.

Launch:
    delphi studio                    New empty notebook
    delphi studio my-song            Open project as notebook
    delphi studio song.dstudio       Open saved notebook file
"""

from __future__ import annotations

import json
import os
import re
import threading
import time
import traceback
from dataclasses import dataclass, field
from pathlib import Path
from typing import Optional

from prompt_toolkit import Application
from prompt_toolkit.buffer import Buffer
from prompt_toolkit.document import Document
from prompt_toolkit.key_binding import KeyBindings
from prompt_toolkit.layout import (
    HSplit,
    VSplit,
    Window,
    ScrollablePane,
    FormattedTextControl,
    BufferControl,
    Layout,
)
from prompt_toolkit.lexers import PygmentsLexer
from prompt_toolkit.widgets import Frame

import delphi
from delphi.context import get_context
from delphi.song import Song, Track, GM_INSTRUMENTS
from delphi.help import STUDIO_HELP_PANEL, format_error_hint
from delphi.notation import (
    lint as lint_notation,
    format_preview,
    transpose as transpose_notation,
)
from delphi.livecode import (
    build_source_map,
    TickClock,
    format_source_highlighted,
    format_tracks_highlighted,
)

# Reuse the REPL's notation detector, syntax highlighter, and completer
try:
    from delphi.repl import _looks_like_notation
except Exception:
    _looks_like_notation = None

try:
    from delphi.repl import DelphiLexer
    _LEXER = PygmentsLexer(DelphiLexer)
except Exception:
    _LEXER = None

try:
    from delphi.repl import DelphiCompleter
    _COMPLETER = DelphiCompleter()
except Exception:
    _COMPLETER = None

try:
    from delphi.repl import DelphiAutoSuggest
    _AUTO_SUGGEST = DelphiAutoSuggest()
except Exception:
    _AUTO_SUGGEST = None


# ── Cell Model ────────────────────────────────────────────────

@dataclass
class Cell:
    """A single cell in the notebook."""

    id: int
    cell_type: str = "code"          # "code" | "notation" | "markdown"
    source: str = ""
    output: str = ""
    label: str = ""
    instrument: str = ""             # GM instrument name (notation cells)
    channel: int = 0
    velocity: int = 80
    collapsed: bool = False
    _run_state: str = field(default="empty", repr=False)  # empty|ready|ok|error|stale
    _last_run_source: str = field(default="", repr=False)  # source at last run

    def to_dict(self) -> dict:
        d: dict = {"type": self.cell_type, "source": self.source}
        meta: dict = {}
        if self.label:
            meta["label"] = self.label
        if self.instrument:
            meta["program"] = self.instrument
        if self.channel:
            meta["channel"] = self.channel
        if self.velocity != 80:
            meta["velocity"] = self.velocity
        if meta:
            d["meta"] = meta
        return d

    @classmethod
    def from_dict(cls, data: dict, cell_id: int) -> Cell:
        meta = data.get("meta", {})
        return cls(
            id=cell_id,
            cell_type=data.get("type", "code"),
            source=data.get("source", ""),
            label=meta.get("label", ""),
            instrument=meta.get("program", ""),
            channel=meta.get("channel", 0),
            velocity=meta.get("velocity", 80),
        )


# ── Pragma parsing ────────────────────────────────────────────

_PRAGMA_RE = re.compile(
    r'#\s*@(track|program|instrument|velocity|channel)\s+(.+)',
    re.IGNORECASE,
)


def _parse_pragmas(source: str) -> tuple[dict, str]:
    """Extract @pragmas from cell source. Returns (meta_dict, clean_source)."""
    meta: dict = {}
    clean_lines: list[str] = []
    for line in source.splitlines():
        m = _PRAGMA_RE.match(line.strip())
        if m:
            key = m.group(1).lower()
            val = m.group(2).strip()
            if key == "instrument":
                key = "program"
            if key == "velocity":
                try:
                    val = int(val)
                except ValueError:
                    pass
            if key == "channel":
                try:
                    val = int(val)
                except ValueError:
                    pass
            meta[key] = val
        else:
            clean_lines.append(line)
    return meta, "\n".join(clean_lines).strip()


# ── Notebook Model ────────────────────────────────────────────

@dataclass
class Notebook:
    """A collection of cells that form a composition."""

    title: str = "Untitled"
    cells: list[Cell] = field(default_factory=list)
    file_path: str = ""
    settings: dict = field(default_factory=lambda: {
        "tempo": 120,
        "key": "C major",
        "time_sig": "4/4",
    })
    song: Song | None = field(default=None, repr=False)

    _next_id: int = field(default=1, repr=False)
    _stop_flag: object = field(default=None, repr=False)  # StopFlag from Rust engine

    def add_cell(
        self,
        cell_type: str = "code",
        source: str = "",
        label: str = "",
        after: int | None = None,
    ) -> Cell:
        cell = Cell(id=self._next_id, cell_type=cell_type, source=source, label=label)
        self._next_id += 1
        if after is not None:
            idx = next((i for i, c in enumerate(self.cells) if c.id == after), None)
            if idx is not None:
                self.cells.insert(idx + 1, cell)
                return cell
        self.cells.append(cell)
        return cell

    def delete_cell(self, cell_id: int) -> bool:
        for i, c in enumerate(self.cells):
            if c.id == cell_id:
                self.cells.pop(i)
                return True
        return False

    def move_cell(self, cell_id: int, direction: int) -> bool:
        """Move a cell up (-1) or down (+1)."""
        for i, c in enumerate(self.cells):
            if c.id == cell_id:
                new_i = i + direction
                if 0 <= new_i < len(self.cells):
                    self.cells[i], self.cells[new_i] = self.cells[new_i], self.cells[i]
                    return True
                return False
        return False

    # ── Execution ─────────────────────────────────────────────

    def _build_namespace(self) -> dict:
        """Build exec namespace with delphi functions."""
        # Wrap play functions to pass our stop flag for cancellation
        stop = self._stop_flag

        def _play(notation, stop_flag=None, **kwargs):
            kwargs.setdefault("visualize", False)  # TUI conflicts with stdout viz
            return delphi.play(notation, stop_flag=stop or stop_flag, **kwargs)

        def _play_notes(tuples, stop_flag=None):
            return delphi.play_notes(tuples, stop_flag=stop or stop_flag)

        ns = {
            "__builtins__": __builtins__,
            "play": _play,
            "play_notes": _play_notes,
            "export": delphi.export,
            "sheet": delphi.sheet,
            "tempo": delphi.tempo,
            "key": delphi.key,
            "time_sig": delphi.time_sig,
            "swing": delphi.swing,
            "humanize": delphi.humanize,
            "instrument": delphi.instrument,
            "note": delphi.note,
            "chord": delphi.chord,
            "scale": delphi.scale,
            "Song": Song,
            "Track": Track,
            "GM_INSTRUMENTS": GM_INSTRUMENTS,
            "get_context": delphi.get_context,
            "reset_context": delphi.reset_context,
            "parse_notation": delphi.parse_notation,
            "ensure_soundfont": delphi.ensure_soundfont,
            "soundfont_info": delphi.soundfont_info,
            "set_soundfont": delphi.set_soundfont,
            "transpose": transpose_notation,
            "preview": lambda n: format_preview(n),
            "lint": lint_notation,
            "loop": lambda n, **kw: _play(n, loop=True, **kw),
            "cells": self.cells,
        }
        if self.song:
            ns["song"] = self.song
        return ns

    def run_cell(self, cell: Cell, namespace: dict) -> str:
        """Execute a single cell and return output text."""
        source = cell.source.strip()
        if not source:
            cell._run_state = "empty"
            return ""

        if cell.cell_type == "markdown":
            cell._run_state = "ok"
            return ""

        if cell.cell_type == "notation":
            result = self._run_notation_cell(cell, namespace)
        else:
            result = self._run_code_cell(cell, namespace)

        cell._last_run_source = cell.source
        cell._run_state = "error" if result.startswith("✗") else "ok"
        return result

    def _run_notation_cell(self, cell: Cell, namespace: dict) -> str:
        """Parse and play a notation cell."""
        try:
            meta, clean_source = _parse_pragmas(cell.source)
            if not clean_source:
                return ""

            # Apply pragma metadata to cell
            if "track" in meta and not cell.label:
                cell.label = meta["track"]
            if "program" in meta:
                cell.instrument = meta["program"]
            if "velocity" in meta:
                cell.velocity = int(meta["velocity"])
            if "channel" in meta:
                cell.channel = int(meta["channel"])

            events = delphi.parse_notation(clean_source)
            n_notes = len([e for e in events if getattr(e, 'kind', None) == 'note'])
            ctx = get_context()
            ticks_per_bar = 480 * ctx.time_sig_num * (4 / ctx.time_sig_den)
            total_ticks = sum(getattr(e, 'duration_ticks', 0) for e in events)
            bars = total_ticks / ticks_per_bar if ticks_per_bar else 0

            # Build info line
            info = f"♪ {n_notes} notes, {bars:.1f} bars"
            if cell.instrument:
                info += f" [{cell.instrument}]"

            delphi.play(
                clean_source,
                stop_flag=self._stop_flag,
                channel=cell.channel,
                instrument=cell.instrument,
            )

            # Show lint warnings alongside success
            issues = lint_notation(clean_source)
            if issues:
                warn_count = len(issues)
                info += f"  ⚠ {warn_count} issue{'s' if warn_count > 1 else ''}"
                for issue in issues[:2]:
                    hint = f" — {issue['hint']}" if issue.get('hint') else ""
                    info += f"\n  · '{issue['token']}'{hint}"

            return info
        except KeyboardInterrupt:
            return "⏹ Stopped"
        except Exception as e:
            # Provide contextual error hints
            hint = format_error_hint(str(e), clean_source)
            return f"✗ {hint}" if hint != str(e) else f"✗ {e}"

    def _run_code_cell(self, cell: Cell, namespace: dict) -> str:
        """Execute a code cell with REPL-like notation auto-detection.

        Each line is checked: if it looks like music notation, it's played
        via play(). Otherwise, it's executed as Python. This mirrors the
        REPL's behavior so users can mix code and notation freely.
        """
        import io
        import contextlib

        source = cell.source.strip()
        if not source:
            return ""

        # First, try to run the whole cell as Python (fast path for pure code)
        buf = io.StringIO()
        try:
            with contextlib.redirect_stdout(buf):
                try:
                    result = eval(source, namespace)
                    if result is not None:
                        print(repr(result))
                except SyntaxError:
                    exec(source, namespace)
            output = buf.getvalue().strip()
            return output if output else "♪ OK"
        except KeyboardInterrupt:
            return "⏹ Stopped"
        except Exception:
            pass

        # Python failed — try line-by-line with notation auto-detection
        if _looks_like_notation is None:
            return f"✗ {traceback.format_exc().splitlines()[-1]}"

        buf = io.StringIO()
        played = 0
        try:
            code_block: list[str] = []

            def _flush_code():
                """Execute any accumulated Python lines."""
                if not code_block:
                    return
                block = "\n".join(code_block)
                code_block.clear()
                with contextlib.redirect_stdout(buf):
                    try:
                        result = eval(block, namespace)
                        if result is not None:
                            print(repr(result))
                    except SyntaxError:
                        exec(block, namespace)

            for line in source.splitlines():
                stripped = line.strip()
                if not stripped or stripped.startswith("#"):
                    code_block.append(line)
                    continue

                if _looks_like_notation(stripped):
                    _flush_code()
                    delphi.play(stripped, stop_flag=self._stop_flag)
                    played += 1
                else:
                    code_block.append(line)

            _flush_code()

        except KeyboardInterrupt:
            return "⏹ Stopped"
        except Exception:
            # Give a helpful hint about what went wrong
            err = traceback.format_exc().splitlines()[-1]
            hint = ""
            if "SyntaxError" in err:
                hint = "\n  💡 Tip: Switch to a notation cell (Ctrl+T) for pure music notation"
            return f"✗ {err}{hint}"

        output_parts = []
        text = buf.getvalue().strip()
        if text:
            output_parts.append(text)
        if played:
            output_parts.append(f"♪ Played {played} line{'s' if played > 1 else ''}")
        return "\n".join(output_parts) if output_parts else "♪ OK"

    def run_all(self, play: bool = True) -> list[str]:
        """Run all cells top-to-bottom, building a Song from notation cells."""
        s = self.settings
        tempo_val = float(s.get("tempo", 120))
        key_val = str(s.get("key", "C major"))
        ts = str(s.get("time_sig", "4/4"))
        ts_parts = ts.split("/") if "/" in ts else ["4", "4"]

        self.song = Song(
            self.title,
            tempo=tempo_val,
            key=key_val,
            time_sig=(int(ts_parts[0]), int(ts_parts[1])),
        )

        ns = self._build_namespace()
        outputs = []
        track_count = 0

        for cell in self.cells:
            if cell.cell_type == "notation" and cell.source.strip():
                # Parse pragmas and build a Track
                meta, clean_source = _parse_pragmas(cell.source)
                if not clean_source:
                    cell.output = ""
                    outputs.append("")
                    continue

                track_name = meta.get("track", cell.label or f"Track {track_count + 1}")
                program = meta.get("program", cell.instrument or "piano")
                velocity = int(meta.get("velocity", cell.velocity))
                channel = meta.get("channel")
                if channel is not None:
                    channel = int(channel)

                try:
                    track = Track(
                        name=track_name,
                        notation=clean_source,
                        program=program,
                        channel=channel,
                        velocity=velocity,
                    )
                    self.song.add_track(track)
                    track_count += 1

                    events = delphi.parse_notation(clean_source)
                    n_notes = len([e for e in events if getattr(e, 'kind', None) == 'note'])
                    ctx = get_context()
                    ticks_per_bar = 480 * ctx.time_sig_num * (4 / ctx.time_sig_den)
                    total_ticks = sum(getattr(e, 'duration_ticks', 0) for e in events)
                    bars = total_ticks / ticks_per_bar if ticks_per_bar else 0
                    info = f"♪ {n_notes} notes, {bars:.1f} bars [{program}]"
                    cell.output = info
                    outputs.append(info)
                except Exception as e:
                    cell.output = f"✗ {e}"
                    outputs.append(cell.output)
            else:
                out = self.run_cell(cell, ns)
                cell.output = out
                outputs.append(out)

        # Play the assembled song (unless build-only mode for export)
        if play and self.song.tracks:
            try:
                self.song.play(stop_flag=self._stop_flag)
                outputs.append(f"▶ Song: {track_count} tracks")
            except KeyboardInterrupt:
                outputs.append("⏹ Stopped")
            except Exception as e:
                outputs.append(f"✗ Playback: {e}")

        # Make song available in namespace for code cells
        ns["song"] = self.song

        return outputs

    # ── Persistence (.dstudio) ────────────────────────────────

    def save(self, path: str | None = None) -> str:
        path = path or self.file_path
        if not path:
            path = self.title.lower().replace(" ", "-") + ".dstudio"
        self.file_path = path

        data = {
            "version": 1,
            "title": self.title,
            "settings": self.settings,
            "cells": [c.to_dict() for c in self.cells],
        }
        with open(path, "w") as f:
            json.dump(data, f, indent=2)
        return path

    @classmethod
    def load(cls, path: str) -> Notebook:
        with open(path) as f:
            data = json.load(f)

        nb = cls(
            title=data.get("title", "Untitled"),
            file_path=path,
            settings=data.get("settings", {}),
        )
        for i, cell_data in enumerate(data.get("cells", []), start=1):
            cell = Cell.from_dict(cell_data, i)
            nb.cells.append(cell)
            nb._next_id = i + 1

        return nb

    def export_script(self) -> str:
        """Flatten notebook to a .delphi script."""
        lines = [
            '"""',
            f'{self.title} — exported from Delphi Studio',
            '"""',
            'from delphi import *',
            '',
        ]
        s = self.settings
        tempo_val = s.get("tempo", 120)
        key_val = s.get("key", "C major")
        if "tempo" in s:
            lines.append(f'tempo({tempo_val})')
        if "key" in s:
            lines.append(f'key("{key_val}")')
        if "time_sig" in s:
            ts = s["time_sig"]
            if "/" in str(ts):
                num, den = str(ts).split("/")
                lines.append(f'time_sig({num.strip()}, {den.strip()})')
        lines.append('')

        # Check if there are notation cells → generate Song-based script
        has_notation = any(c.cell_type == "notation" and c.source.strip()
                          for c in self.cells)
        if has_notation:
            slug = self.title.replace('"', '\\"')
            lines.append(f'song = Song("{slug}", tempo={tempo_val}, key="{key_val}")')
            lines.append('')

        for cell in self.cells:
            if cell.cell_type == "markdown":
                for line in cell.source.splitlines():
                    lines.append(f'# {line}')
                lines.append('')
            elif cell.cell_type == "notation":
                label = cell.label or f"Cell {cell.id}"
                meta, clean = _parse_pragmas(cell.source)
                if not clean:
                    continue
                program = meta.get('program', cell.instrument or 'piano')
                lines.append(f'# {label}')
                lines.append(f'song.track("{label}", "{clean}", program="{program}")')
                lines.append('')
            elif cell.cell_type == "code":
                # Skip setup-only cells when Song is generated (already in header)
                src = cell.source.strip()
                if has_notation and self._is_setup_only(src):
                    continue
                lines.append(cell.source)
                lines.append('')

        if has_notation:
            lines.append('song.play()')
            lines.append('# song.export("output.mid")')
            lines.append('# song.export("output.musicxml")  # sheet music')
            lines.append('')

        return '\n'.join(lines)

    @staticmethod
    def _is_setup_only(source: str) -> bool:
        """Check if a code cell contains only tempo/key/time_sig calls."""
        setup_re = re.compile(
            r'^(tempo\(.*\)|key\(.*\)|time_sig\(.*\)|swing\(.*\)|humanize\(.*\))$'
        )
        for line in source.splitlines():
            line = line.strip()
            if not line or line.startswith('#'):
                continue
            if not setup_re.match(line):
                return False
        return True


# ── TUI Application ──────────────────────────────────────────

class StudioApp:
    """The prompt_toolkit Application for Delphi Studio."""

    def __init__(self, notebook: Notebook):
        self.notebook = notebook
        self.focused_cell: int = 0     # index into notebook.cells
        self.message: str = ""         # status line message
        self._buffers: dict[int, Buffer] = {}
        self._confirm_delete: bool = False   # F8 pressed once → confirm
        self._confirm_quit: bool = False     # Ctrl+Q pressed once → confirm
        self._playing: bool = False          # playback in progress
        self._play_thread: threading.Thread | None = None
        self._dirty: bool = False            # unsaved changes

        # Create a StopFlag for cancelling Rust-engine playback
        try:
            from delphi._engine import StopFlag
            self._stop_flag = StopFlag()
        except ImportError:
            self._stop_flag = None
        notebook._stop_flag = self._stop_flag

        self.namespace = notebook._build_namespace()
        self._show_help_panel: bool = False   # F2 toggles reference panel

        # Live code view state
        self._live_mode: Optional[str] = None  # None | "cell" | "consolidated"
        self._live_clock: Optional[TickClock] = None
        self._live_cell_source: str = ""
        self._live_cell_spans: list = []
        self._live_tracks: list[dict] = []
        self._live_timer: Optional[threading.Thread] = None

        self._build_app()

    # ── Buffer management ─────────────────────────────────────

    def _get_buffer(self, cell: Cell) -> Buffer:
        if cell.id not in self._buffers:
            buf_kwargs = dict(
                document=Document(cell.source, 0),
                multiline=True,
                name=f"cell-{cell.id}",
                on_text_changed=lambda buf: self._sync_source(cell, buf),
            )
            if _AUTO_SUGGEST and cell.cell_type in ("code", "notation"):
                buf_kwargs["auto_suggest"] = _AUTO_SUGGEST
            self._buffers[cell.id] = Buffer(**buf_kwargs)
        return self._buffers[cell.id]

    def _sync_source(self, cell: Cell, buf: Buffer) -> None:
        cell.source = buf.text
        self._dirty = True
        # Mark cell stale if it was previously run and source changed
        if cell._run_state in ("ok", "error") and cell.source != cell._last_run_source:
            cell._run_state = "stale"

    # ── Key bindings ──────────────────────────────────────────

    def _make_bindings(self) -> KeyBindings:
        kb = KeyBindings()

        @kb.add("f5")
        def run_cell(event):
            self._confirm_delete = False
            self._confirm_quit = False
            if not self.notebook.cells:
                return
            cell = self.notebook.cells[self.focused_cell]
            if cell.cell_type == "notation" and cell.source.strip():
                self._enter_live_cell(cell)
            else:
                self._run_focused_cell()
                self._refresh_layout()

        @kb.add("f6")
        def run_all(event):
            self._confirm_delete = False
            self._confirm_quit = False
            self._enter_live_consolidated()

        @kb.add("f7")
        @kb.add("c-b")
        def add_cell(event):
            self._confirm_delete = False
            self._confirm_quit = False
            after_id = None
            if self.notebook.cells and 0 <= self.focused_cell < len(self.notebook.cells):
                after_id = self.notebook.cells[self.focused_cell].id
            # Default new cell type matches the current cell (usually what you want next)
            current_type = "code"
            if self.notebook.cells and 0 <= self.focused_cell < len(self.notebook.cells):
                current_type = self.notebook.cells[self.focused_cell].cell_type
            self.notebook.add_cell(cell_type=current_type, after=after_id)
            self.focused_cell = min(self.focused_cell + 1, len(self.notebook.cells) - 1)
            self._dirty = True
            self.message = f"+ Added {current_type} cell (Ctrl+T to change type)"
            self._refresh_layout()

        @kb.add("f8")
        def delete_cell(event):
            if not self.notebook.cells:
                return
            self._confirm_quit = False
            if not self._confirm_delete:
                cell = self.notebook.cells[self.focused_cell]
                self._confirm_delete = True
                self.message = f"Delete cell [{self.focused_cell + 1}]? Press F8 again to confirm"
                self._refresh_layout()
                return
            self._confirm_delete = False
            cell = self.notebook.cells[self.focused_cell]
            self._buffers.pop(cell.id, None)
            self.notebook.delete_cell(cell.id)
            if self.focused_cell >= len(self.notebook.cells):
                self.focused_cell = max(0, len(self.notebook.cells) - 1)
            self._dirty = True
            self.message = "− Deleted cell"
            self._refresh_layout()

        @kb.add("f9")
        def export_menu(event):
            self._confirm_delete = False
            self._export()

        @kb.add("f10")
        @kb.add("c-s")
        def save(event):
            self._confirm_delete = False
            self._confirm_quit = False
            path = self.notebook.save()
            self._dirty = False
            self.message = f"Saved: {path}"
            self._refresh_layout()

        @kb.add("c-up")
        def focus_prev(event):
            self._confirm_delete = False
            self._confirm_quit = False
            if self.focused_cell > 0:
                self.focused_cell -= 1
                self._refresh_layout()

        @kb.add("c-down")
        def focus_next(event):
            self._confirm_delete = False
            self._confirm_quit = False
            if self.focused_cell < len(self.notebook.cells) - 1:
                self.focused_cell += 1
                self._refresh_layout()

        @kb.add("c-s-up")
        def move_up(event):
            if not self.notebook.cells:
                return
            cell = self.notebook.cells[self.focused_cell]
            if self.notebook.move_cell(cell.id, -1):
                self.focused_cell -= 1
                self._dirty = True
                self._refresh_layout()

        @kb.add("c-s-down")
        def move_down(event):
            if not self.notebook.cells:
                return
            cell = self.notebook.cells[self.focused_cell]
            if self.notebook.move_cell(cell.id, +1):
                self.focused_cell += 1
                self._dirty = True
                self._refresh_layout()

        @kb.add("c-t")
        def cycle_type(event):
            if not self.notebook.cells:
                return
            cell = self.notebook.cells[self.focused_cell]
            types = ["code", "notation", "markdown"]
            idx = types.index(cell.cell_type) if cell.cell_type in types else 0
            cell.cell_type = types[(idx + 1) % len(types)]
            self.message = f"Cell → {cell.cell_type}"
            self._refresh_layout()

        @kb.add("c-p")
        def replay_last(event):
            """Replay the last executed cell's output (re-run it)."""
            if not self.notebook.cells:
                return
            # Find the most recently executed cell (has output)
            for i in range(self.focused_cell, -1, -1):
                cell = self.notebook.cells[i]
                if cell.output and not cell.output.startswith("✗"):
                    self._run_cell_async(cell)
                    return
            self.message = "No cell to replay"
            self._refresh_layout()

        @kb.add("c-c")
        def stop_playback(event):
            if self._live_mode:
                self._exit_live_mode()
                return
            if self._playing:
                self._playing = False
                if self._stop_flag is not None:
                    self._stop_flag.stop()
                self.message = "⏹ Stopped"
                self._refresh_layout()

        @kb.add("c-e")
        def toggle_collapse(event):
            if self._live_mode:
                return
            if not self.notebook.cells:
                return
            cell = self.notebook.cells[self.focused_cell]
            cell.collapsed = not cell.collapsed
            self.message = f"Cell [{cell.id}] {'collapsed' if cell.collapsed else 'expanded'}"
            self._refresh_layout()

        @kb.add("c-q")
        def quit_app(event):
            self._confirm_delete = False
            if self._dirty and not self._confirm_quit:
                self._confirm_quit = True
                self.message = "⚠ Unsaved changes. Ctrl+Q again to quit, or Ctrl+S to save first"
                self._refresh_layout()
                return
            event.app.exit()

        @kb.add("f1")
        def show_help(event):
            self.message = (
                "F5:Run  F6:RunAll  F7:Add  F8:Del  F9:Export  F10/Ctrl+S:Save │ "
                "Ctrl+↑↓:Nav  Ctrl+Shift+↑↓:Reorder  Ctrl+T:Type  Ctrl+L:Preview  "
                "Ctrl+Z:Undo  Ctrl+Y:Redo  Ctrl+E:Fold  Ctrl+P:Replay  Ctrl+Q:Quit"
            )
            self._refresh_layout()

        @kb.add("f2")
        def toggle_docs(event):
            self._show_help_panel = not self._show_help_panel
            self._refresh_layout()

        @kb.add("c-l")
        def preview_cell(event):
            """Ctrl+L: preview the focused cell (stats without playing)."""
            if not self.notebook.cells:
                return
            cell = self.notebook.cells[self.focused_cell]
            source = cell.source.strip()
            if not source:
                self.message = "Empty cell — nothing to preview"
                self._refresh_layout()
                return
            if cell.cell_type == "notation":
                _, clean_source = _parse_pragmas(source)
                if clean_source:
                    cell.output = format_preview(clean_source)
                    issues = lint_notation(clean_source)
                    if issues:
                        cell._run_state = "stale"
                    else:
                        cell._run_state = "ready"
                    self.message = f"Preview: cell [{self.focused_cell + 1}]"
                else:
                    self.message = "No notation after pragmas"
            elif cell.cell_type == "code":
                # For code cells, try to detect notation strings
                self.message = "Preview only works for notation cells"
            else:
                self.message = "Preview only works for notation cells"
            self._refresh_layout()

        @kb.add("escape")
        def escape_key(event):
            if self._live_mode:
                self._exit_live_mode()

        return kb

    # ── Live code views ───────────────────────────────────────

    def _enter_live_cell(self, cell: Cell):
        """F5 on a notation cell: full-screen live code view."""
        meta, clean_source = _parse_pragmas(cell.source)
        if not clean_source:
            self.message = "No notation to play"
            self._refresh_layout()
            return

        self._playing = True
        if self._stop_flag is not None:
            self._stop_flag.reset()

        # Build source map from the clean source (post-pragma)
        spans = build_source_map(clean_source)
        ctx = get_context()

        self._live_mode = "cell"
        self._live_cell_source = clean_source
        self._live_cell_spans = spans
        self._live_clock = TickClock(ctx.bpm)

        # Show the live layout immediately
        self.app.layout = self._build_live_layout()
        self.app.invalidate()

        # Start playback + clock in a background thread
        self._live_clock.start()

        def _do_play():
            try:
                instrument = meta.get("program", cell.instrument or "")
                channel = cell.channel
                delphi.play(
                    clean_source,
                    stop_flag=self._stop_flag,
                    channel=channel,
                    instrument=instrument,
                    visualize=False,
                )
            except Exception:
                pass
            finally:
                if self._live_clock:
                    self._live_clock.stop()
                self._playing = False
                # Schedule a final refresh to exit live mode
                try:
                    self.app.invalidate()
                except Exception:
                    pass

        self._play_thread = threading.Thread(target=_do_play, daemon=True)
        self._play_thread.start()

        # Start a refresh timer
        self._start_live_refresh()

    def _enter_live_consolidated(self):
        """F6: consolidated multi-track live view."""
        # Build the song first (runs code cells for setup)
        s = self.notebook.settings
        tempo_val = float(s.get("tempo", 120))
        key_val = str(s.get("key", "C major"))
        ts = str(s.get("time_sig", "4/4"))
        ts_parts = ts.split("/") if "/" in ts else ["4", "4"]

        self.notebook.song = Song(
            self.notebook.title,
            tempo=tempo_val,
            key=key_val,
            time_sig=(int(ts_parts[0]), int(ts_parts[1])),
        )

        ns = self.notebook._build_namespace()
        track_data: list[dict] = []
        track_count = 0

        # Run code cells for side effects, collect notation tracks
        for cell in self.notebook.cells:
            if cell.cell_type == "code" and cell.source.strip():
                try:
                    self.notebook.run_cell(cell, ns)
                except Exception:
                    pass
            elif cell.cell_type == "notation" and cell.source.strip():
                meta, clean_source = _parse_pragmas(cell.source)
                if not clean_source:
                    continue

                track_name = meta.get("track", cell.label or f"Track {track_count + 1}")
                program = meta.get("program", cell.instrument or "piano")
                velocity = int(meta.get("velocity", cell.velocity))
                channel = meta.get("channel")
                if channel is not None:
                    channel = int(channel)

                try:
                    track = Track(
                        name=track_name,
                        notation=clean_source,
                        program=program,
                        channel=channel,
                        velocity=velocity,
                    )
                    self.notebook.song.add_track(track)
                    track_count += 1

                    spans = build_source_map(clean_source)
                    track_data.append({
                        "label": track_name,
                        "clean_source": clean_source,
                        "spans": spans,
                    })
                except Exception:
                    pass

        if not track_data:
            self.message = "No notation tracks to play"
            self._refresh_layout()
            return

        self._playing = True
        if self._stop_flag is not None:
            self._stop_flag.reset()

        ctx = get_context()
        self._live_mode = "consolidated"
        self._live_tracks = track_data
        self._live_clock = TickClock(ctx.bpm)
        self.namespace = self.notebook._build_namespace()

        # Show the live layout
        self.app.layout = self._build_live_layout()
        self.app.invalidate()

        # Start playback
        self._live_clock.start()

        def _do_play():
            try:
                self.notebook.song.play(stop_flag=self._stop_flag, visualize=False)
            except Exception:
                pass
            finally:
                if self._live_clock:
                    self._live_clock.stop()
                self._playing = False
                try:
                    self.app.invalidate()
                except Exception:
                    pass

        self._play_thread = threading.Thread(target=_do_play, daemon=True)
        self._play_thread.start()

        self._start_live_refresh()

    def _start_live_refresh(self):
        """Start a background thread that refreshes the live view ~15 fps."""
        def _refresh_loop():
            while self._live_mode and self._playing:
                time.sleep(0.066)  # ~15 fps
                try:
                    self.app.layout = self._build_live_layout()
                    self.app.invalidate()
                except Exception:
                    break
            # Playback finished — auto-exit live mode after a brief pause
            if self._live_mode:
                time.sleep(0.3)
                self._live_mode = None
                self._live_clock = None
                try:
                    self.app.layout = self._build_layout()
                    self._focus_current_cell()
                    self.app.invalidate()
                except Exception:
                    pass

        self._live_timer = threading.Thread(target=_refresh_loop, daemon=True)
        self._live_timer.start()

    def _exit_live_mode(self):
        """Exit live code view, stop playback."""
        if self._stop_flag is not None:
            self._stop_flag.stop()
        self._playing = False
        if self._live_clock:
            self._live_clock.stop()
        self._live_mode = None
        self._live_clock = None
        self.message = "⏹ Stopped"
        self._refresh_layout()

    def _build_live_layout(self) -> Layout:
        """Build the layout for live code views."""
        current_tick = self._live_clock.current_tick if self._live_clock else 0

        if self._live_mode == "cell":
            formatted = format_source_highlighted(
                self._live_cell_source,
                self._live_cell_spans,
                current_tick,
            )
            content = FormattedTextControl(formatted)
        elif self._live_mode == "consolidated":
            formatted = format_tracks_highlighted(
                self._live_tracks,
                current_tick,
            )
            content = FormattedTextControl(formatted)
        else:
            return self._build_layout()

        # Status line
        ctx = get_context()
        elapsed = 0.0
        if self._live_clock and self._live_clock._start_time:
            elapsed = time.monotonic() - self._live_clock._start_time
        mins, secs = divmod(int(elapsed), 60)

        status_text = (
            f" Delphi Studio | LIVE"
            f"  |  bpm={int(ctx.bpm)}  {ctx.key_name}"
            f"  |  {mins}:{secs:02d}"
        )
        status = Window(
            FormattedTextControl(status_text),
            height=1,
            style="bg:ansicyan fg:ansiblack bold",
            wrap_lines=False,
        )

        toolbar = Window(
            FormattedTextControl(" Ctrl+C: Stop   Esc: Exit"),
            height=1,
            style="reverse",
            wrap_lines=False,
        )

        main_pane = ScrollablePane(Window(content))

        return Layout(HSplit([status, main_pane, toolbar]))

    # ── Cell execution ─────────────────────────────────────────

    def _run_focused_cell(self):
        if not self.notebook.cells:
            return
        cell = self.notebook.cells[self.focused_cell]
        self._run_cell_async(cell)

    def _run_cell_async(self, cell: Cell):
        """Run a cell with a 'Playing…' indicator in a background thread."""
        self._playing = True
        if self._stop_flag is not None:
            self._stop_flag.reset()
        cell.output = "▶ Playing…"
        self.message = f"▶ Cell [{cell.id}] running…"
        self._refresh_layout()

        start = time.monotonic()

        def _do_run():
            try:
                output = self.notebook.run_cell(cell, self.namespace)
                elapsed = time.monotonic() - start
                if self._playing:
                    cell.output = output
                    self.message = f"▶ Cell [{cell.id}] executed ({elapsed:.1f}s)"
                else:
                    cell.output = "⏹ Stopped"
            except KeyboardInterrupt:
                cell.output = "⏹ Stopped"
                self.message = "⏹ Interrupted"
            except Exception as e:
                cell.output = f"✗ {e}"
                self.message = f"✗ Cell [{cell.id}] error"
            finally:
                self._playing = False
                try:
                    self.app.layout = self._build_layout()
                    self._focus_current_cell()
                except Exception:
                    pass
                self.app.invalidate()

        self._play_thread = threading.Thread(target=_do_run, daemon=True)
        self._play_thread.start()

    def _export(self):
        """Export notebook to MIDI, WAV, MusicXML, or .delphi script."""
        if self._playing:
            self.message = "⏳ Already running — wait or Ctrl+C first"
            self._refresh_layout()
            return

        self._playing = True
        self.message = "⏳ Exporting…"
        self._refresh_layout()

        def _do_export():
            try:
                base = self.notebook.file_path or self.notebook.title.lower().replace(" ", "-")
                base = base.replace(".dstudio", "")

                results = []

                # Build Song without playing it
                if not self.notebook.song or not self.notebook.song.tracks:
                    self.notebook.run_all(play=False)

                # Export MIDI + WAV + MusicXML
                if self.notebook.song and self.notebook.song.tracks:
                    mid_path = base + ".mid"
                    try:
                        self.notebook.song.export(mid_path)
                        results.append(f"MIDI → {mid_path}")
                    except Exception as e:
                        results.append(f"MIDI ✗ {e}")

                    # Export WAV (if SoundFont available)
                    wav_path = base + ".wav"
                    try:
                        self.notebook.song.render(wav_path)
                        results.append(f"WAV → {wav_path}")
                    except Exception as e:
                        results.append(f"WAV ✗ {e}")

                    # Export MusicXML (sheet music)
                    xml_path = base + ".musicxml"
                    try:
                        self.notebook.song.export(xml_path)
                        results.append(f"Sheet → {xml_path}")
                    except Exception as e:
                        results.append(f"Sheet ✗ {e}")
                else:
                    results.append("No tracks to export (run F6 first)")

                # Always export script
                script_path = base + ".delphi"
                try:
                    script = self.notebook.export_script()
                    with open(script_path, "w") as f:
                        f.write(script)
                    results.append(f"Script → {script_path}")
                except Exception as e:
                    results.append(f"Script ✗ {e}")

                self.message = "  |  ".join(results)
            except Exception as e:
                self.message = f"✗ Export failed: {e}"
            finally:
                self._playing = False
                try:
                    self.app.layout = self._build_layout()
                    self._focus_current_cell()
                except Exception:
                    pass
                self.app.invalidate()

        self._play_thread = threading.Thread(target=_do_export, daemon=True)
        self._play_thread.start()

    # ── Layout building ───────────────────────────────────────

    def _build_status_bar(self):
        ctx = get_context()
        title = self.notebook.title
        n_cells = len(self.notebook.cells)
        focus = self.focused_cell + 1 if self.notebook.cells else 0
        dirty = " *" if self._dirty else ""

        instrument_name = getattr(ctx, 'program_name', 'piano')
        status_text = (
            f" Delphi Studio | {title}{dirty}"
            f"  |  bpm={int(ctx.bpm)}  {ctx.key_name}  [{instrument_name}]"
            f"  |  Cell {focus}/{n_cells}"
        )
        return Window(
            FormattedTextControl(status_text),
            height=1,
            style="reverse",
            wrap_lines=False,
        )

    def _build_cell_widget(self, cell: Cell, index: int):
        """Build the visual widget for one cell."""
        is_focused = index == self.focused_cell

        # Run-state indicator (like Jupyter's [*] / [1] but simpler)
        state_icon = {
            "empty": "○",   # never run, no content
            "ready": "○",   # has content, never run
            "ok": "✓",      # ran successfully
            "error": "✗",   # ran with error
            "stale": "◐",   # edited since last run
        }.get(cell._run_state, "○")

        # Update "ready" for cells with content that haven't been run
        if cell._run_state == "empty" and cell.source.strip():
            state_icon = "○"

        focus_marker = "▶" if is_focused else " "
        type_badge = {"code": "⌨", "notation": "♪", "markdown": "¶"}.get(cell.cell_type, "?")
        collapse_icon = "▸" if cell.collapsed else "▾"

        # Build a useful label
        label = cell.label or cell.cell_type.title()
        extras = []
        if cell.cell_type == "notation" and cell.instrument:
            extras.append(cell.instrument)
        elif cell.collapsed and cell.cell_type == "code" and cell.source.strip():
            # Show first meaningful line as preview when collapsed
            for line in cell.source.splitlines():
                line = line.strip()
                if line and not line.startswith("#"):
                    if len(line) > 40:
                        line = line[:37] + "…"
                    extras.append(line)
                    break

        extra_text = f"  {extras[0]}" if extras else ""
        header_text = f" {focus_marker} {state_icon} {collapse_icon} [{index + 1}] {type_badge} {label}{extra_text}"

        # Header styling based on focus and run state
        if is_focused:
            header_style = "bold"
        elif cell._run_state == "error":
            header_style = "fg:ansired"
        elif cell._run_state == "stale":
            header_style = "fg:ansiyellow"
        elif cell._run_state == "ok":
            header_style = "fg:ansigreen"
        else:
            header_style = ""

        header = Window(
            FormattedTextControl(header_text),
            height=1,
            style=header_style,
            wrap_lines=False,
        )

        if cell.collapsed:
            parts = [header]
        else:
            buf = self._get_buffer(cell)
            if _COMPLETER and cell.cell_type in ("code", "notation"):
                buf.completer = _COMPLETER
            editor_kwargs = {"buffer": buf}
            if _LEXER and cell.cell_type in ("code", "notation"):
                editor_kwargs["lexer"] = _LEXER
            editor = Window(
                BufferControl(**editor_kwargs),
                height=lambda b=buf: max(3, b.document.line_count + 1),
                style="bg:ansiblack" if is_focused else "",
            )
            parts = [header, editor]

        if cell.output:
            is_error = cell.output.startswith("✗")
            is_stopped = cell.output.startswith("⏹")
            if is_error:
                output_style = "fg:ansired"
            elif is_stopped:
                output_style = "fg:ansiyellow"
            else:
                output_style = "fg:ansigreen"
            # Indent all output lines for consistent alignment
            indented = "\n".join(f"  {line}" for line in cell.output.splitlines())
            output_lines = cell.output.count("\n") + 1
            parts.append(Window(
                FormattedTextControl(indented),
                height=min(output_lines, 5),
                style=output_style,
                wrap_lines=False,
            ))

        border_style = "fg:ansicyan" if is_focused else "fg:#444444"
        parts.append(Window(char="─", height=1, style=border_style))

        return HSplit(parts)

    def _build_toolbar(self):
        # Contextual toolbar — show what matters based on state
        parts = [" F5:Run"]
        parts.append("F6:RunAll")
        parts.append("F7:Add")
        parts.append("F8:Del")
        parts.append("F9:Export")
        parts.append("F2:Docs")
        parts.append("Ctrl+L:Preview")
        parts.append("Ctrl+↑↓:Nav")
        parts.append("Ctrl+T:Type")
        parts.append("F10:Save")
        parts.append("Ctrl+Q:Quit")
        return Window(
            FormattedTextControl("  ".join(parts)),
            height=1,
            style="reverse",
            wrap_lines=False,
        )

    def _build_message_bar(self):
        text = self.message or ""
        return Window(
            FormattedTextControl(f"  {text}"),
            height=1,
            wrap_lines=False,
            style="fg:ansiyellow",
        )

    def _build_layout(self) -> Layout:
        parts = [self._build_status_bar()]

        if self.notebook.cells:
            cell_widgets = [
                self._build_cell_widget(cell, i)
                for i, cell in enumerate(self.notebook.cells)
            ]
            cells_pane = ScrollablePane(HSplit(cell_widgets))
        else:
            cells_pane = Window(
                FormattedTextControl(
                    "\n\n    Empty notebook. Press F7 to add a cell.\n"
                ),
                height=5,
            )

        # Side-by-side layout when help panel is open
        if self._show_help_panel:
            help_panel = Window(
                FormattedTextControl(STUDIO_HELP_PANEL),
                width=61,
                style="fg:ansicyan",
            )
            parts.append(VSplit([cells_pane, help_panel]))
        else:
            parts.append(cells_pane)

        parts.append(self._build_message_bar())
        parts.append(self._build_toolbar())

        return Layout(HSplit(parts))

    def _refresh_layout(self):
        self.app.layout = self._build_layout()
        self._focus_current_cell()

    def _focus_current_cell(self):
        """Move prompt_toolkit focus to the buffer of the focused cell."""
        if self.notebook.cells and 0 <= self.focused_cell < len(self.notebook.cells):
            cell = self.notebook.cells[self.focused_cell]
            if not cell.collapsed:
                buf = self._get_buffer(cell)
                try:
                    self.app.layout.focus(buf)
                except ValueError:
                    pass

    # ── Application entry ────────────────────────────────────

    def _build_app(self):
        self.app = Application(
            layout=self._build_layout(),
            key_bindings=self._make_bindings(),
            full_screen=True,
            mouse_support=True,
        )
        self._focus_current_cell()

    def run(self):
        self.app.run()


# ── Public entry points ───────────────────────────────────────

def _load_or_create_notebook(target: str | None) -> Notebook:
    """Resolve the target argument into a Notebook."""
    if target is None:
        # New empty notebook with a setup cell
        nb = Notebook(title="Untitled")
        nb.add_cell(
            cell_type="code",
            source='tempo(120)\nkey("C major")\ntime_sig(4, 4)',
            label="Setup",
        )
        return nb

    path = Path(target)

    # Open existing .dstudio file
    if path.suffix == ".dstudio" and path.exists():
        return Notebook.load(str(path))

    # Open a project directory (look for .dstudio or delphi.toml)
    if path.is_dir():
        # Check for .dstudio files in the directory
        dstudio_files = list(path.glob("*.dstudio"))
        if dstudio_files:
            return Notebook.load(str(dstudio_files[0]))

        # Check for delphi.toml → create notebook from project
        toml_path = path / "delphi.toml"
        if toml_path.exists():
            return _notebook_from_project(path)

        # Empty notebook named after directory
        nb = Notebook(title=path.name)
        nb.file_path = str(path / f"{path.name}.dstudio")
        nb.add_cell(
            cell_type="code",
            source='tempo(120)\nkey("C major")\ntime_sig(4, 4)',
            label="Setup",
        )
        return nb

    # Try as a project name in the projects directory
    from delphi.cli import _get_projects_dir
    projects_dir = _get_projects_dir()
    candidate = projects_dir / target
    if candidate.is_dir():
        return _load_or_create_notebook(str(candidate))

    # New notebook with given name
    slug = target.lower().replace(" ", "-")
    nb = Notebook(title=target)
    nb.file_path = f"{slug}.dstudio"
    nb.add_cell(
        cell_type="code",
        source='tempo(120)\nkey("C major")\ntime_sig(4, 4)',
        label="Setup",
    )
    return nb


def _notebook_from_project(project_dir: Path) -> Notebook:
    """Create a notebook pre-populated from a Delphi project's delphi.toml."""
    from delphi.repl import _parse_simple_toml

    toml_path = project_dir / "delphi.toml"
    config = _parse_simple_toml(str(toml_path))

    title = config.get("project", {}).get("name", project_dir.name)
    settings_raw = config.get("settings", {})

    nb = Notebook(
        title=title,
        file_path=str(project_dir / f"{project_dir.name}.dstudio"),
        settings={
            "tempo": settings_raw.get("tempo", 120),
            "key": settings_raw.get("key", "C major"),
            "time_sig": settings_raw.get("time_sig", "4/4"),
        },
    )

    # Setup cell from project settings
    tempo_val = settings_raw.get("tempo", 120)
    key_val = settings_raw.get("key", "C major")
    ts = settings_raw.get("time_sig", "4/4")
    ts_parts = str(ts).split("/") if "/" in str(ts) else ["4", "4"]

    nb.add_cell(
        cell_type="code",
        source=(
            f"tempo({tempo_val})\n"
            f'key("{key_val}")\n'
            f"time_sig({ts_parts[0].strip()}, {ts_parts[1].strip()})"
        ),
        label="Setup",
    )

    # Add empty notation cells for melody and bass
    nb.add_cell(cell_type="notation", source="", label="Melody")
    nb.add_cell(cell_type="notation", source="", label="Bass")

    return nb


def run_studio(target: str | None = None):
    """Launch Delphi Studio.

    Args:
        target: A .dstudio file path, project directory, project name, or None.
    """
    notebook = _load_or_create_notebook(target)

    # Apply notebook settings to global context
    s = notebook.settings
    if "tempo" in s:
        delphi.tempo(float(s["tempo"]))
    if "key" in s:
        delphi.key(str(s["key"]))
    if "time_sig" in s:
        ts = str(s["time_sig"])
        if "/" in ts:
            num, den = ts.split("/")
            delphi.time_sig(int(num.strip()), int(den.strip()))

    app = StudioApp(notebook)
    app.run()
