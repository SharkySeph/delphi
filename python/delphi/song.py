"""
Multi-track song composition.

A Song holds multiple Tracks, each with its own instrument (MIDI program),
channel, and notation. This is the core abstraction for multi-voice music.

Usage:
    song = Song("Canon in D", tempo=120, key="D major")
    song.add_track(Track("Violin I",  program=40, notation="D4 F#4 A4 ..."))
    song.add_track(Track("Cello",     program=42, notation="| D | A | Bm | F#m |"))
    song.export("canon.mid")
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Optional

from delphi.notation import parse, events_to_tuples, TICKS_PER_QUARTER


# General MIDI program numbers for common instruments
GM_INSTRUMENTS = {
    "piano": 0, "acoustic grand piano": 0, "grand piano": 0,
    "bright piano": 1, "electric piano": 4, "epiano": 4,
    "harpsichord": 6, "clavinet": 7,
    "celesta": 8, "glockenspiel": 9, "music box": 10,
    "vibraphone": 11, "marimba": 12, "xylophone": 13,
    "tubular bells": 14, "dulcimer": 15,
    "organ": 19, "church organ": 19, "reed organ": 20, "accordion": 21,
    "harmonica": 22, "bandoneon": 23,
    "nylon guitar": 24, "acoustic guitar": 25, "steel guitar": 25,
    "jazz guitar": 26, "clean guitar": 27, "electric guitar": 27,
    "muted guitar": 28, "overdriven guitar": 29, "distortion guitar": 30,
    "acoustic bass": 32, "electric bass": 33, "finger bass": 33,
    "pick bass": 34, "fretless bass": 35, "slap bass": 36,
    "synth bass": 38,
    "violin": 40, "viola": 41, "cello": 42, "contrabass": 43,
    "tremolo strings": 44, "pizzicato strings": 45,
    "harp": 46, "timpani": 47,
    "strings": 48, "string ensemble": 48,
    "slow strings": 49, "synth strings": 50,
    "choir": 52, "voice": 53, "synth voice": 54,
    "orchestra hit": 55,
    "trumpet": 56, "trombone": 57, "tuba": 58, "muted trumpet": 59,
    "french horn": 60, "horn": 60,
    "brass": 61, "synth brass": 62,
    "soprano sax": 64, "alto sax": 65, "tenor sax": 66, "baritone sax": 67,
    "oboe": 68, "english horn": 69, "bassoon": 70, "clarinet": 71,
    "piccolo": 72, "flute": 73, "recorder": 74, "pan flute": 75,
    "whistle": 78, "ocarina": 79,
    "square lead": 80, "saw lead": 81, "synth lead": 81,
    "pad": 88, "warm pad": 89, "polysynth": 90,
    "sitar": 104, "banjo": 105, "shamisen": 106, "koto": 107,
    "kalimba": 108, "bagpipe": 109, "fiddle": 110,
}

DRUM_CHANNEL = 9  # MIDI channel 10 (0-indexed = 9)


@dataclass
class Track:
    """A single voice/instrument track within a song.

    Supports method chaining for effects and pattern manipulation:
        Track("Piano", "C4 E4 G4", program="piano")
            .gain(0.8).pan(0.3).reverb(0.5)
            .transpose(2).rev()
    """

    name: str
    notation: str = ""
    program: int | str = 0
    channel: Optional[int] = None
    velocity: int = 80

    # Effects (applied during SoundFont rendering or MIDI export)
    _gain: float = field(default=1.0, repr=False)
    _pan: float = field(default=0.5, repr=False)       # 0=left, 0.5=center, 1=right
    _reverb: float = field(default=0.0, repr=False)     # 0-1
    _delay: float = field(default=0.0, repr=False)      # 0-1
    _delay_time: float = field(default=0.25, repr=False) # seconds
    _transpose_semitones: int = field(default=0, repr=False)
    _reversed: bool = field(default=False, repr=False)
    _attack: float = field(default=0.0, repr=False)
    _decay: float = field(default=0.0, repr=False)
    _sustain: float = field(default=1.0, repr=False)
    _release: float = field(default=0.0, repr=False)

    def __post_init__(self):
        # Resolve string instrument names to GM program numbers
        if isinstance(self.program, str):
            key = self.program.lower().strip()
            if key in GM_INSTRUMENTS:
                self.program = GM_INSTRUMENTS[key]
            else:
                raise ValueError(
                    f"Unknown instrument '{self.program}'. "
                    f"Use a GM name like 'piano', 'violin', 'flute' or a number 0-127."
                )

    # ── Effects (method chaining) ────────────────────────

    def gain(self, value: float) -> "Track":
        """Set gain (volume multiplier). 0.0-2.0, default 1.0."""
        self._gain = max(0.0, min(2.0, value))
        return self

    def pan(self, value: float) -> "Track":
        """Set stereo pan. 0=left, 0.5=center, 1=right."""
        self._pan = max(0.0, min(1.0, value))
        return self

    def reverb(self, amount: float) -> "Track":
        """Set reverb level. 0-1."""
        self._reverb = max(0.0, min(1.0, amount))
        return self

    def delay(self, amount: float, time: float = 0.25) -> "Track":
        """Set delay level and time. amount: 0-1, time: seconds."""
        self._delay = max(0.0, min(1.0, amount))
        self._delay_time = time
        return self

    def adsr(self, attack: float = 0.01, decay: float = 0.1,
             sustain: float = 0.8, release: float = 0.1) -> "Track":
        """Set ADSR envelope. Times in seconds, sustain is 0-1 level."""
        self._attack = attack
        self._decay = decay
        self._sustain = sustain
        self._release = release
        return self

    # ── Pattern manipulation (method chaining) ────────────

    def transpose(self, semitones: int) -> "Track":
        """Transpose all notes by N semitones."""
        self._transpose_semitones += semitones
        return self

    def rev(self) -> "Track":
        """Reverse the pattern."""
        self._reversed = not self._reversed
        return self

    def octave_up(self) -> "Track":
        """Shift all notes up one octave."""
        self._transpose_semitones += 12
        return self

    def octave_down(self) -> "Track":
        """Shift all notes down one octave."""
        self._transpose_semitones -= 12
        return self

    # ── Core output ──────────────────────────────────────

    def get_tuples(self) -> list[tuple[int, int, int, int]]:
        """Parse the notation and return (midi, velocity, tick, duration) tuples."""
        if not self.notation.strip():
            return []
        events = parse(self.notation, default_velocity=self.velocity)
        tuples = events_to_tuples(events)

        # Apply gain to velocity
        if self._gain != 1.0:
            tuples = [
                (m, min(127, max(1, int(v * self._gain))), t, d)
                for m, v, t, d in tuples
            ]

        # Apply transpose
        if self._transpose_semitones != 0:
            tuples = [
                (max(0, min(127, m + self._transpose_semitones)), v, t, d)
                for m, v, t, d in tuples
            ]

        # Apply reverse
        if self._reversed and tuples:
            max_tick = max(t + d for _, _, t, d in tuples)
            tuples = [
                (m, v, max_tick - t - d, d)
                for m, v, t, d in tuples
            ]
            tuples.sort(key=lambda x: x[2])

        return tuples


class Song:
    """
    A multi-track musical composition.

    Example:
        song = Song("My Song", tempo=120, key="C major")
        song.add_track(Track("Piano", notation="| C | Am | F | G |", program="piano"))
        song.add_track(Track("Bass",  notation="C2 C2 F2 G2", program="acoustic bass"))
        song.export("my_song.mid")
    """

    def __init__(self, title: str = "Untitled", *, tempo: float = 120,
                 key: str = "C major", time_sig: tuple[int, int] = (4, 4)):
        self.title = title
        self.tempo = tempo
        self.key = key
        self.time_sig_num, self.time_sig_den = time_sig
        self.tracks: list[Track] = []

    def add_track(self, track: Track) -> "Song":
        """Add a track to the song. Returns self for chaining."""
        self.tracks.append(track)
        return self

    def track(self, name: str, notation: str, program: int | str = 0,
              channel: Optional[int] = None, velocity: int = 80) -> "Song":
        """Convenience method to create and add a track inline."""
        self.tracks.append(Track(
            name=name, notation=notation, program=program,
            channel=channel, velocity=velocity,
        ))
        return self

    def play(self, stop_flag=None, visualize: bool = True) -> None:
        """Play all tracks using SoundFont if available, else built-in synth."""
        sf_tuples = self._build_sf_tuples()
        if not sf_tuples:
            print("(nothing to play)")
            return

        # Start live visualizer across all tracks
        viz_thread = None
        if visualize:
            try:
                all_events = []
                for t in self.tracks:
                    if t.notation.strip():
                        all_events.extend(parse(t.notation, default_velocity=t.velocity))
                if all_events:
                    from delphi.visualizer import visualize as _viz
                    viz_thread = _viz(all_events, self.tempo, stop_flag=stop_flag)
            except Exception:
                pass  # visualizer is purely cosmetic

        # Try SoundFont playback first
        from delphi.soundfont import get_soundfont_path
        sf_path = get_soundfont_path()

        if sf_path:
            try:
                from delphi._engine import play_sf
                play_sf(sf_path, sf_tuples, bpm=self.tempo, stop_flag=stop_flag)
                if viz_thread is not None:
                    viz_thread.join(timeout=1.0)
                return
            except ImportError:
                pass  # Rust engine not built, fall through
            except Exception as e:
                print(f"[Delphi] SoundFont playback failed: {e}")
                print("[Delphi] Falling back to built-in synth.")

        # Fallback: mix down to simple tuples and use built-in synth
        from delphi.playback import play_notes
        from delphi.context import get_context
        all_tuples = []
        for t in self.tracks:
            all_tuples.extend(t.get_tuples())
        ctx = get_context()
        old_bpm = ctx.bpm
        ctx.bpm = self.tempo
        try:
            play_notes(all_tuples, stop_flag=stop_flag)
        finally:
            ctx.bpm = old_bpm
            if viz_thread is not None:
                viz_thread.join(timeout=1.0)

    def render(self, path: str) -> None:
        """Render the song to a WAV file using a SoundFont."""
        if not path.endswith(".wav"):
            raise ValueError("render() only supports .wav output")

        sf_tuples = self._build_sf_tuples()
        if not sf_tuples:
            print("(nothing to render)")
            return

        from delphi.soundfont import ensure_soundfont
        sf_path = ensure_soundfont()
        if not sf_path:
            print("[Delphi] Cannot render without a SoundFont.")
            return

        try:
            from delphi._engine import render_wav
            render_wav(sf_path, sf_tuples, path, bpm=self.tempo)
            print(f"Rendered WAV: {path}")
        except ImportError:
            print("[Delphi] Rust engine not built. Run: maturin develop")

    def _build_sf_tuples(self) -> list[tuple[int, int, int, int, int, int]]:
        """Build (midi, vel, tick, dur, channel, program) tuples for all tracks."""
        DRUM_CHANNEL = 9
        result = []
        next_channel = 0

        for track_obj in self.tracks:
            # Assign channel
            if track_obj.channel is not None:
                ch = track_obj.channel
            else:
                ch = next_channel
                if ch == DRUM_CHANNEL:
                    ch += 1
                next_channel = ch + 1
                if next_channel == DRUM_CHANNEL:
                    next_channel += 1
            ch = min(ch, 15)

            program = track_obj.program if isinstance(track_obj.program, int) else 0
            tuples = track_obj.get_tuples()
            for midi, vel, tick, dur in tuples:
                result.append((midi, vel, tick, dur, ch, program))

        return result

    def export(self, path: str) -> None:
        """Export the song to a MIDI, WAV, or MusicXML file."""
        if not self.tracks:
            print("(no tracks to export)")
            return

        if path.endswith((".mid", ".midi")):
            _export_multitrack_midi(self, path)
        elif path.endswith(".wav"):
            self.render(path)
        elif path.endswith((".xml", ".musicxml")):
            from delphi.export import export_musicxml_song
            export_musicxml_song(self, path)
        else:
            raise ValueError(f"Unsupported export format: {path} (use .mid, .wav, .xml, or .musicxml)")

    def __repr__(self):
        track_names = ", ".join(t.name for t in self.tracks)
        return f'Song("{self.title}", {len(self.tracks)} tracks: [{track_names}])'

    def extract(self, track_name: str) -> "Song":
        """Extract a single track into its own Song for practice/debugging."""
        extracted = Song(f"{self.title} — {track_name}",
                         tempo=self.tempo, key=self.key,
                         time_sig=(self.time_sig_num, self.time_sig_den))
        for t in self.tracks:
            if t.name == track_name:
                extracted.add_track(t)
                return extracted
        names = [t.name for t in self.tracks]
        raise ValueError(f"Track '{track_name}' not found. Available: {names}")


def _export_multitrack_midi(song: Song, path: str) -> None:
    """Write a multi-track MIDI Format 1 file."""
    ppq = TICKS_PER_QUARTER  # 480
    num_tracks = len(song.tracks) + 1  # +1 for tempo/meta track

    data = bytearray()

    # --- MIDI Header ---
    data.extend(b"MThd")
    data.extend((6).to_bytes(4, "big"))
    data.extend((1).to_bytes(2, "big"))       # Format 1 (multi-track)
    data.extend(num_tracks.to_bytes(2, "big"))
    data.extend(ppq.to_bytes(2, "big"))

    # --- Track 0: Tempo / Meta ---
    tempo_track = bytearray()

    # Time signature meta event
    # FF 58 04 nn dd cc bb
    tempo_track.extend(b"\x00\xff\x58\x04")
    tempo_track.append(song.time_sig_num)
    # dd = log2(denominator)
    import math
    dd = int(math.log2(song.time_sig_den)) if song.time_sig_den > 0 else 2
    tempo_track.append(dd)
    tempo_track.append(24)  # MIDI clocks per metronome click
    tempo_track.append(8)   # 32nd notes per MIDI quarter note

    # Tempo meta event
    uspqn = int(60_000_000 / song.tempo)
    tempo_track.extend(b"\x00\xff\x51\x03")
    tempo_track.extend(uspqn.to_bytes(3, "big"))

    # Track name
    _write_meta_text(tempo_track, 0x03, song.title)

    # Key signature (informational)
    _write_meta_text(tempo_track, 0x01, f"Key: {song.key}")

    # End of track
    tempo_track.extend(b"\x00\xff\x2f\x00")

    data.extend(b"MTrk")
    data.extend(len(tempo_track).to_bytes(4, "big"))
    data.extend(tempo_track)

    # --- One track per voice ---
    used_channels: set[int] = set()
    next_channel = 0

    for track_obj in song.tracks:
        # Assign MIDI channel
        if track_obj.channel is not None:
            ch = track_obj.channel
        else:
            ch = next_channel
            # Skip drum channel unless explicitly requested
            while ch == DRUM_CHANNEL and track_obj.channel is None:
                ch += 1
            next_channel = ch + 1
            if next_channel == DRUM_CHANNEL:
                next_channel += 1  # auto-skip drum channel

        ch = min(ch, 15)  # clamp to valid range
        used_channels.add(ch)

        tuples = track_obj.get_tuples()
        program = track_obj.program if isinstance(track_obj.program, int) else 0

        track_data = bytearray()

        # Track name meta event
        _write_meta_text(track_data, 0x03, track_obj.name)

        # Program change
        track_data.extend(b"\x00")
        track_data.append(0xC0 | ch)
        track_data.append(program & 0x7F)

        # CC events for track effects
        # Pan (CC#10): 0=left, 64=center, 127=right
        pan_cc = max(0, min(127, int(track_obj._pan * 127)))
        track_data.extend(b"\x00")
        track_data.append(0xB0 | ch)
        track_data.extend([10, pan_cc])

        # Volume / expression from gain (CC#7)
        vol_cc = max(0, min(127, int(track_obj._gain * 100)))
        track_data.extend(b"\x00")
        track_data.append(0xB0 | ch)
        track_data.extend([7, vol_cc])

        # Reverb (CC#91)
        if track_obj._reverb > 0:
            rev_cc = max(0, min(127, int(track_obj._reverb * 127)))
            track_data.extend(b"\x00")
            track_data.append(0xB0 | ch)
            track_data.extend([91, rev_cc])

        # Chorus/delay (CC#93)
        if track_obj._delay > 0:
            delay_cc = max(0, min(127, int(track_obj._delay * 127)))
            track_data.extend(b"\x00")
            track_data.append(0xB0 | ch)
            track_data.extend([93, delay_cc])

        # Sort events by tick
        sorted_events = sorted(tuples, key=lambda t: t[2])

        # Build note-on / note-off messages
        raw: list[tuple[int, int, int, int]] = []
        for midi, vel, tick, dur in sorted_events:
            raw.append((tick, 0x90 | ch, midi & 0x7F, min(vel, 127)))
            raw.append((tick + dur, 0x80 | ch, midi & 0x7F, 0))
        raw.sort(key=lambda r: (r[0], r[1] & 0xF0))  # note-off before note-on at same tick

        current_tick = 0
        for abs_tick, status, d1, d2 in raw:
            delta = max(0, abs_tick - current_tick)
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
    print(f"Exported multi-track MIDI: {path} ({len(song.tracks)} tracks)")


def _write_vlq(buf: bytearray, value: int) -> None:
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


def _write_meta_text(buf: bytearray, meta_type: int, text: str) -> None:
    """Write a MIDI meta text event (track name, text event, etc.)."""
    encoded = text.encode("ascii", errors="replace")
    buf.extend(b"\x00\xff")
    buf.append(meta_type)
    _write_vlq(buf, len(encoded))
    buf.extend(encoded)
