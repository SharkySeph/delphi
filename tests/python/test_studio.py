"""Tests for Delphi Studio — Cell/Notebook models, pragmas, Song building, export."""
import sys
import os
import json
import tempfile

sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "..", "python"))

from delphi.context import reset_context, tempo, key, time_sig
from delphi.studio import Cell, Notebook, _parse_pragmas


# ── Pragma parsing ────────────────────────────────────────────

def test_parse_pragmas_track():
    meta, clean = _parse_pragmas("# @track melody\nC4:q D4:q")
    assert meta["track"] == "melody"
    assert clean == "C4:q D4:q"


def test_parse_pragmas_program():
    meta, clean = _parse_pragmas("# @program piano\n# @velocity 90\nC4:q")
    assert meta["program"] == "piano"
    assert meta["velocity"] == 90
    assert clean == "C4:q"


def test_parse_pragmas_instrument_alias():
    meta, _ = _parse_pragmas("# @instrument flute\nC4:q")
    assert meta["program"] == "flute"


def test_parse_pragmas_channel():
    meta, _ = _parse_pragmas("# @channel 10\nkick snare")
    assert meta["channel"] == 10


def test_parse_pragmas_empty():
    meta, clean = _parse_pragmas("C4:q E4:q G4:q")
    assert meta == {}
    assert clean == "C4:q E4:q G4:q"


def test_parse_pragmas_multiple():
    meta, clean = _parse_pragmas(
        "# @track bass\n# @program acoustic bass\n# @velocity 70\n# @channel 2\nC2:h G2:h"
    )
    assert meta == {"track": "bass", "program": "acoustic bass", "velocity": 70, "channel": 2}
    assert clean == "C2:h G2:h"


# ── Cell model ────────────────────────────────────────────────

def test_cell_to_dict_basic():
    cell = Cell(id=1, cell_type="notation", source="C4:q E4:q")
    d = cell.to_dict()
    assert d["type"] == "notation"
    assert d["source"] == "C4:q E4:q"
    assert "meta" not in d  # no meta if no label/instrument


def test_cell_to_dict_with_meta():
    cell = Cell(id=1, cell_type="notation", source="C4:q", label="Melody",
                instrument="piano", velocity=100, channel=1)
    d = cell.to_dict()
    assert d["meta"]["label"] == "Melody"
    assert d["meta"]["program"] == "piano"
    assert d["meta"]["velocity"] == 100
    assert d["meta"]["channel"] == 1


def test_cell_from_dict():
    data = {
        "type": "notation",
        "source": "C4:q D4:q",
        "meta": {"label": "Bass", "program": "acoustic bass", "velocity": 70},
    }
    cell = Cell.from_dict(data, cell_id=5)
    assert cell.id == 5
    assert cell.cell_type == "notation"
    assert cell.source == "C4:q D4:q"
    assert cell.label == "Bass"
    assert cell.instrument == "acoustic bass"
    assert cell.velocity == 70


def test_cell_roundtrip():
    original = Cell(id=1, cell_type="code", source='tempo(120)', label="Setup")
    d = original.to_dict()
    restored = Cell.from_dict(d, cell_id=1)
    assert restored.cell_type == original.cell_type
    assert restored.source == original.source
    assert restored.label == original.label


# ── Notebook model ────────────────────────────────────────────

def test_notebook_add_cell():
    nb = Notebook(title="Test")
    c1 = nb.add_cell(cell_type="code", source="tempo(120)", label="Setup")
    c2 = nb.add_cell(cell_type="notation", source="C4:q", label="Melody")
    assert len(nb.cells) == 2
    assert nb.cells[0].id == c1.id
    assert nb.cells[1].id == c2.id


def test_notebook_add_cell_after():
    nb = Notebook(title="Test")
    c1 = nb.add_cell(cell_type="code", source="a")
    c3 = nb.add_cell(cell_type="code", source="c")
    c2 = nb.add_cell(cell_type="code", source="b", after=c1.id)
    assert [c.source for c in nb.cells] == ["a", "b", "c"]


def test_notebook_delete_cell():
    nb = Notebook(title="Test")
    c1 = nb.add_cell(cell_type="code", source="a")
    c2 = nb.add_cell(cell_type="code", source="b")
    assert nb.delete_cell(c1.id) is True
    assert len(nb.cells) == 1
    assert nb.cells[0].id == c2.id


def test_notebook_delete_nonexistent():
    nb = Notebook(title="Test")
    nb.add_cell(cell_type="code", source="a")
    assert nb.delete_cell(999) is False
    assert len(nb.cells) == 1


def test_notebook_move_cell():
    nb = Notebook(title="Test")
    c1 = nb.add_cell(cell_type="code", source="a")
    c2 = nb.add_cell(cell_type="code", source="b")
    c3 = nb.add_cell(cell_type="code", source="c")
    assert nb.move_cell(c2.id, -1) is True
    assert [c.source for c in nb.cells] == ["b", "a", "c"]


def test_notebook_move_cell_boundary():
    nb = Notebook(title="Test")
    c1 = nb.add_cell(cell_type="code", source="a")
    assert nb.move_cell(c1.id, -1) is False  # already at top
    assert nb.move_cell(c1.id, +1) is False  # already at bottom


# ── Save / Load ───────────────────────────────────────────────

def test_notebook_save_load_roundtrip():
    nb = Notebook(title="My Song")
    nb.settings = {"tempo": 140, "key": "D major", "time_sig": "3/4"}
    nb.add_cell(cell_type="code", source='tempo(140)\nkey("D major")', label="Setup")
    nb.add_cell(cell_type="notation", source="# @track melody\nD4:q F#4:q A4:q", label="Melody")

    with tempfile.NamedTemporaryFile(suffix=".dstudio", delete=False) as f:
        path = f.name

    try:
        nb.save(path)
        assert os.path.exists(path)

        # Verify JSON structure
        with open(path) as f:
            data = json.load(f)
        assert data["version"] == 1
        assert data["title"] == "My Song"
        assert data["settings"]["tempo"] == 140
        assert len(data["cells"]) == 2

        # Load and verify
        nb2 = Notebook.load(path)
        assert nb2.title == "My Song"
        assert len(nb2.cells) == 2
        assert nb2.cells[0].label == "Setup"
        assert nb2.cells[1].cell_type == "notation"
        assert nb2.settings["key"] == "D major"
    finally:
        os.unlink(path)


# ── run_all / Song building ──────────────────────────────────

def test_run_all_builds_song():
    reset_context()
    nb = Notebook(title="Test Song")
    nb.add_cell(cell_type="code", source='tempo(120)\nkey("C major")', label="Setup")
    nb.add_cell(
        cell_type="notation",
        source="# @track melody\n# @program piano\nC4:q E4:q G4:q C5:h",
        label="Melody",
    )
    nb.add_cell(
        cell_type="notation",
        source="# @track bass\n# @program acoustic bass\nC2:h G2:h",
        label="Bass",
    )

    outputs = nb.run_all()

    assert nb.song is not None
    assert len(nb.song.tracks) == 2
    assert nb.song.tracks[0].name == "melody"
    assert nb.song.tracks[0].program == 0   # "piano" resolves to GM program 0
    assert nb.song.tracks[1].name == "bass"
    # "acoustic bass" resolves to a GM program number
    assert isinstance(nb.song.tracks[1].program, int)


def test_run_all_skips_empty_notation():
    reset_context()
    nb = Notebook(title="Test")
    nb.add_cell(cell_type="code", source='tempo(120)', label="Setup")
    nb.add_cell(cell_type="notation", source="# @track melody\n", label="Empty")

    outputs = nb.run_all()

    assert nb.song is not None
    assert len(nb.song.tracks) == 0


def test_run_all_code_cell_output():
    reset_context()
    nb = Notebook(title="Test")
    nb.add_cell(cell_type="code", source='tempo(120)', label="Setup")
    nb.add_cell(cell_type="code", source='1 + 1', label="Math")

    ns = nb._build_namespace()
    output = nb.run_cell(nb.cells[1], ns)
    assert "2" in output


# ── export_script ─────────────────────────────────────────────

def test_export_script_basic():
    reset_context()
    nb = Notebook(title="Test Song")
    nb.settings = {"tempo": 120, "key": "C major", "time_sig": "4/4"}
    nb.add_cell(cell_type="code", source='tempo(120)\nkey("C major")', label="Setup")
    nb.add_cell(
        cell_type="notation",
        source="# @track melody\n# @program piano\nC4:q E4:q G4:h",
        label="Melody",
    )

    script = nb.export_script()
    assert "tempo(120)" in script
    assert 'key("C major")' in script
    assert 'Song("Test Song"' in script
    assert 'song.track("Melody"' in script
    assert 'program="piano"' in script
    assert "song.play()" in script


def test_export_script_skips_setup_cell():
    reset_context()
    nb = Notebook(title="Test")
    nb.settings = {"tempo": 120, "key": "C major"}
    nb.add_cell(cell_type="code", source='tempo(120)\nkey("C major")', label="Setup")
    nb.add_cell(cell_type="notation", source="C4:q", label="Melody")

    script = nb.export_script()
    # The setup-only code cell should NOT appear twice (once in header, not again)
    lines = script.split("\n")
    tempo_lines = [l for l in lines if l.strip() == "tempo(120)"]
    assert len(tempo_lines) == 1


def test_export_script_includes_markdown_as_comments():
    nb = Notebook(title="Test")
    nb.settings = {"tempo": 120}
    nb.add_cell(cell_type="markdown", source="This is a note", label="Notes")
    nb.add_cell(cell_type="notation", source="C4:q", label="Melody")

    script = nb.export_script()
    assert "# This is a note" in script


# ── _is_setup_only ────────────────────────────────────────────

def test_is_setup_only_true():
    assert Notebook._is_setup_only('tempo(120)\nkey("C major")\ntime_sig(4, 4)') is True


def test_is_setup_only_with_comments():
    assert Notebook._is_setup_only('# settings\ntempo(120)') is True


def test_is_setup_only_false():
    assert Notebook._is_setup_only('tempo(120)\nprint("hello")') is False


def test_is_setup_only_empty():
    assert Notebook._is_setup_only('') is True
    assert Notebook._is_setup_only('# just a comment') is True
