"""
CLI entry point for Delphi.

Usage:
    delphi              Launch the interactive REPL
    delphi run file.py  Run a Delphi script
    delphi export ...   Export from CLI
"""

import sys


def main():
    args = sys.argv[1:]

    if not args:
        # Launch REPL
        from delphi.repl import run_repl
        run_repl()
        return

    cmd = args[0]

    if cmd == "run" and len(args) > 1:
        _run_script(args[1])
    elif cmd in ("--help", "-h"):
        _print_help()
    elif cmd in ("--version", "-V"):
        import delphi
        print(f"Delphi {delphi.__version__}")
    else:
        # Treat argument as a script file
        _run_script(cmd)


def _run_script(path: str):
    """Execute a Delphi script file."""
    import delphi
    from delphi.composition import (
        Section, Pattern, Voice, Arrangement, PatternLibrary,
        build_song_from_sections, register_pattern, get_pattern,
        list_patterns, include,
    )
    namespace = {
        "__builtins__": __builtins__,
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
        "get_context": delphi.get_context,
        "reset_context": delphi.reset_context,
        "parse_notation": delphi.parse_notation,
        "ensure_soundfont": delphi.ensure_soundfont,
        "soundfont_info": delphi.soundfont_info,
        "set_soundfont": delphi.set_soundfont,
    }
    with open(path) as f:
        code = f.read()
    exec(compile(code, path, "exec"), namespace)


def _print_help():
    print("""
Delphi — Music scripting language

Usage:
    delphi                Launch interactive REPL
    delphi <file>         Run a .delphi / .py script
    delphi run <file>     Run a script
    delphi --version      Show version
    delphi --help         Show this help
""")


if __name__ == "__main__":
    main()
