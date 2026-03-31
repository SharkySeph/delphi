"""
Delphi Jupyter/IPython extension.

Brings Delphi's music scripting into Jupyter notebooks with inline audio
playback and magic commands.

Load in a notebook:
    %load_ext delphi.notebook

Then use:
    %%delphi                   — Play notation in a cell
    %delphi_tempo 120          — Set tempo
    %delphi_key C major        — Set key
    %delphi_time_sig 4 4       — Set time signature
    %delphi_instruments        — List GM instruments

Or use the Python API directly in code cells:
    from delphi import *
    song = Song("My Song", tempo=120)
    song.track("piano", "C4:q E4:q G4:h", program="piano")
    song.to_audio()            — Returns IPython Audio widget
"""

import os
import tempfile

# ── Audio helpers ─────────────────────────────────────────────


def _render_notation_to_audio(notation: str, **kwargs):
    """Parse notation, render to WAV via SoundFont, return IPython Audio."""
    from IPython.display import Audio

    from delphi.context import get_context
    from delphi.notation import parse, events_to_tuples
    from delphi.soundfont import ensure_soundfont

    events = parse(notation, default_velocity=kwargs.get("velocity", 80))
    tuples = events_to_tuples(events)
    if not tuples:
        print("(nothing to play)")
        return None

    sf_path = ensure_soundfont()
    if not sf_path:
        print("[Delphi] No SoundFont available. Run: ensure_soundfont()")
        return None

    ctx = get_context()
    bpm = kwargs.get("tempo", ctx.bpm)

    # Build sf_tuples: (midi, vel, tick, dur, channel, program)
    program = kwargs.get("program", 0)
    channel = kwargs.get("channel", 0)
    sf_tuples = [(m, v, t, d, channel, program) for m, v, t, d in tuples]

    # Render to a temporary WAV
    tmp = tempfile.NamedTemporaryFile(suffix=".wav", delete=False)
    tmp_path = tmp.name
    tmp.close()

    try:
        from delphi._engine import render_wav
        render_wav(sf_path, sf_tuples, tmp_path, bpm=bpm)

        with open(tmp_path, "rb") as f:
            wav_data = f.read()

        return Audio(data=wav_data, rate=44100, autoplay=kwargs.get("autoplay", True))
    except ImportError:
        print("[Delphi] Rust engine not built. Run: maturin develop")
        return None
    finally:
        try:
            os.unlink(tmp_path)
        except OSError:
            pass


def _render_song_to_audio(song, **kwargs):
    """Render a Song object to an IPython Audio widget."""
    from IPython.display import Audio

    from delphi.soundfont import ensure_soundfont

    sf_path = ensure_soundfont()
    if not sf_path:
        print("[Delphi] No SoundFont available.")
        return None

    sf_tuples = song._build_sf_tuples()
    if not sf_tuples:
        print("(nothing to render)")
        return None

    tmp = tempfile.NamedTemporaryFile(suffix=".wav", delete=False)
    tmp_path = tmp.name
    tmp.close()

    try:
        from delphi._engine import render_wav
        render_wav(sf_path, sf_tuples, tmp_path, bpm=song.tempo)

        with open(tmp_path, "rb") as f:
            wav_data = f.read()

        return Audio(data=wav_data, rate=44100, autoplay=kwargs.get("autoplay", True))
    except ImportError:
        print("[Delphi] Rust engine not built. Run: maturin develop")
        return None
    finally:
        try:
            os.unlink(tmp_path)
        except OSError:
            pass


# ── Song.to_audio() monkey-patch ──────────────────────────────

def _song_to_audio(self, autoplay=True):
    """Render this Song to an IPython Audio widget for inline playback."""
    return _render_song_to_audio(self, autoplay=autoplay)


# ── IPython Magics ────────────────────────────────────────────

def load_ipython_extension(ipython):
    """Called by %load_ext delphi.notebook."""
    from IPython.core.magic import Magics, cell_magic, line_magic, magics_class

    # Patch Song with .to_audio()
    from delphi.song import Song
    Song.to_audio = _song_to_audio

    @magics_class
    class DelphiMagics(Magics):
        """IPython magic commands for Delphi music scripting."""

        @cell_magic
        def delphi(self, line, cell):
            """Play music notation written in a cell.

            Usage:
                %%delphi
                C4:q E4:q G4:q C5:h

                %%delphi --program piano --tempo 100
                | Cmaj7 | Am7 | Fmaj7 | G7 |
            """
            # Parse optional flags from the magic line
            kwargs = _parse_magic_args(line)
            return _render_notation_to_audio(cell.strip(), **kwargs)

        @line_magic
        def delphi_play(self, line):
            """Play a short notation snippet inline.

            Usage:
                %delphi_play C4 E4 G4 C5
            """
            if not line.strip():
                print("Usage: %delphi_play C4 E4 G4 C5")
                return
            return _render_notation_to_audio(line.strip())

        @line_magic
        def delphi_tempo(self, line):
            """Set the global tempo.

            Usage:
                %delphi_tempo 140
            """
            import delphi
            try:
                bpm = float(line.strip())
                delphi.tempo(bpm)
                print(f"Tempo: {bpm} BPM")
            except ValueError:
                print("Usage: %delphi_tempo 120")

        @line_magic
        def delphi_key(self, line):
            """Set the global key.

            Usage:
                %delphi_key D minor
            """
            import delphi
            key_name = line.strip()
            if key_name:
                delphi.key(key_name)
                print(f"Key: {key_name}")
            else:
                print("Usage: %delphi_key C major")

        @line_magic
        def delphi_time_sig(self, line):
            """Set the time signature.

            Usage:
                %delphi_time_sig 3 4
            """
            import delphi
            parts = line.strip().split()
            if len(parts) == 2:
                try:
                    delphi.time_sig(int(parts[0]), int(parts[1]))
                    print(f"Time signature: {parts[0]}/{parts[1]}")
                except ValueError:
                    print("Usage: %delphi_time_sig 4 4")
            else:
                print("Usage: %delphi_time_sig 4 4")

        @line_magic
        def delphi_instruments(self, line):
            """List available GM instruments."""
            from delphi.song import GM_INSTRUMENTS
            names = sorted(GM_INSTRUMENTS.keys())
            col_width = 24
            cols = 3
            print(f"\nGeneral MIDI Instruments ({len(names)}):\n")
            for i in range(0, len(names), cols):
                row = names[i:i + cols]
                print("  " + "".join(n.ljust(col_width) for n in row))
            print()

        @line_magic
        def delphi_context(self, line):
            """Show current context settings."""
            from delphi.context import get_context
            ctx = get_context()
            print(f"Tempo: {ctx.bpm} BPM")
            print(f"Key: {ctx.key_name}")
            print(f"Time: {ctx.time_sig_num}/{ctx.time_sig_den}")
            if ctx.swing > 0:
                print(f"Swing: {ctx.swing}")
            if ctx.humanize > 0:
                print(f"Humanize: {ctx.humanize}")

    ipython.register_magics(DelphiMagics)

    # Import all delphi functions into the notebook namespace
    import delphi
    ipython.push({
        "play": delphi.play,
        "play_notes": delphi.play_notes,
        "export": delphi.export,
        "tempo": delphi.tempo,
        "key": delphi.key,
        "time_sig": delphi.time_sig,
        "swing": delphi.swing,
        "humanize": delphi.humanize,
        "note": delphi.note,
        "chord": delphi.chord,
        "scale": delphi.scale,
        "Song": delphi.Song,
        "Track": delphi.Track,
        "GM_INSTRUMENTS": delphi.GM_INSTRUMENTS,
        "ensure_soundfont": delphi.ensure_soundfont,
        "soundfont_info": delphi.soundfont_info,
        "set_soundfont": delphi.set_soundfont,
        "parse_notation": delphi.parse_notation,
        "get_context": delphi.get_context,
        "reset_context": delphi.reset_context,
    })

    # Import composition tools
    try:
        from delphi.composition import (
            Section, Pattern, Voice, Arrangement, PatternLibrary,
            build_song_from_sections, register_pattern, get_pattern,
            list_patterns, include,
        )
        ipython.push({
            "Section": Section,
            "Pattern": Pattern,
            "Voice": Voice,
            "Arrangement": Arrangement,
            "PatternLibrary": PatternLibrary,
            "build_song_from_sections": build_song_from_sections,
            "register_pattern": register_pattern,
            "get_pattern": get_pattern,
            "list_patterns": list_patterns,
            "include": include,
        })
    except ImportError:
        pass

    # Convenience function for inline audio
    ipython.push({
        "delphi_audio": _render_notation_to_audio,
    })

    print("🎵 Delphi loaded! Use %%delphi cells or the Python API.")
    print("   Magics: %delphi_play, %delphi_tempo, %delphi_key, %delphi_context")
    print("   Tip: song.to_audio() returns an inline audio player.\n")


def _parse_magic_args(line: str) -> dict:
    """Parse %%delphi magic line arguments like --tempo 120 --program piano."""
    kwargs = {}
    parts = line.split()
    i = 0
    while i < len(parts):
        if parts[i] == "--tempo" and i + 1 < len(parts):
            try:
                kwargs["tempo"] = float(parts[i + 1])
            except ValueError:
                pass
            i += 2
        elif parts[i] == "--program" and i + 1 < len(parts):
            # Try as int, else keep as string for GM lookup
            try:
                kwargs["program"] = int(parts[i + 1])
            except ValueError:
                from delphi.song import GM_INSTRUMENTS
                kwargs["program"] = GM_INSTRUMENTS.get(parts[i + 1].lower(), 0)
            i += 2
        elif parts[i] == "--velocity" and i + 1 < len(parts):
            try:
                kwargs["velocity"] = int(parts[i + 1])
            except ValueError:
                pass
            i += 2
        elif parts[i] == "--no-autoplay":
            kwargs["autoplay"] = False
            i += 1
        else:
            i += 1
    return kwargs
