"""
CLI entry point for Delphi.

Usage:
    delphi                  Launch REPL (auto-detects project in cwd)
    delphi studio [target]  Open Delphi Studio notebook TUI
    delphi init <name>      Create a new Delphi project
    delphi open [path]      Open a project in the REPL
    delphi run <file>       Run a Delphi script
    delphi --version        Show version
    delphi --help           Show this help
"""

import os
import platform
import sys
from pathlib import Path

# Name of the project manifest that marks a Delphi project directory
PROJECT_FILE = "delphi.toml"

# ── OS-aware data directories ────────────────────────────────
# Config: ~/.delphi/config.toml  (lightweight, user-editable)
# Data:   platform-standard location for projects & soundfonts
#   Linux:   ~/.local/share/delphi/
#   macOS:   ~/Library/Application Support/Delphi/
#   Windows: %APPDATA%/Delphi/

_CONFIG_DIR = Path.home() / ".delphi"
_CONFIG_FILE = _CONFIG_DIR / "config.toml"


def _get_data_dir() -> Path:
    """Return the OS-standard data directory for Delphi."""
    system = platform.system()
    if system == "Darwin":
        base = Path.home() / "Library" / "Application Support" / "Delphi"
    elif system == "Windows":
        appdata = os.environ.get("APPDATA", str(Path.home() / "AppData" / "Roaming"))
        base = Path(appdata) / "Delphi"
    else:  # Linux / BSD / etc.
        xdg = os.environ.get("XDG_DATA_HOME", str(Path.home() / ".local" / "share"))
        base = Path(xdg) / "delphi"
    return base


def _get_projects_dir() -> Path:
    """Return the projects directory (from config, or OS default)."""
    cfg = _load_config()
    custom = cfg.get("projects_dir", "")
    if custom:
        return Path(os.path.expanduser(custom))
    return _get_data_dir() / "projects"


def main():
    args = sys.argv[1:]

    if not args:
        # No arguments: auto-detect project in cwd, then launch REPL
        project_dir = _find_project_root(os.getcwd())
        from delphi.repl import run_repl
        run_repl(project_dir=project_dir)
        return

    cmd = args[0]

    if cmd == "studio":
        target = args[1] if len(args) > 1 else None
        from delphi.studio import run_studio
        run_studio(target)
    elif cmd == "init":
        name = args[1] if len(args) > 1 else None
        _init_project(name)
    elif cmd == "open":
        name = args[1] if len(args) > 1 else None
        _open_project(name)
    elif cmd == "config":
        _config_command(args[1:])
    elif cmd == "projects":
        _list_projects()
    elif cmd == "run" and len(args) > 1:
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
    delphi                  Launch REPL (auto-detects project in cwd)
    delphi studio [target]  Open Delphi Studio notebook TUI
    delphi init [name]      Create a new Delphi project
    delphi open [path]      Open a project directory in the REPL
    delphi projects         List projects in your projects directory
    delphi config           Show current configuration
    delphi config <k> <v>   Set a config value
    delphi run <file>       Run a .delphi / .py script
    delphi <file>           Run a script (shorthand for 'run')
    delphi --version        Show version
    delphi --help           Show this help

Configuration (~/.delphi/config.toml):
    projects_dir            Override default projects location
    default_tempo           Default tempo for new projects
    default_key             Default key for new projects
    default_soundfont       Path to preferred SoundFont

  Projects are stored in a platform-standard location:
    Linux:     ~/.local/share/delphi/projects/
    macOS:     ~/Library/Application Support/Delphi/projects/
    Windows:   %%APPDATA%%/Delphi/projects/

Project workflow:
    delphi init my-song     Create 'my-song/' with starter files
    cd my-song && delphi    Open the REPL with project loaded
    delphi open my-song/    Same thing, from outside the directory
""")


# ── Project scaffolding ──────────────────────────────────────


_STARTER_SCRIPT = '''#!/usr/bin/env python3
"""
{title} — a Delphi project
"""
from delphi import *

ensure_soundfont()

tempo(120)
key("C major")
time_sig(4, 4)

# ── Write your music below ──────────────────────────────────

# Quick test: play a melody
play("C4:q E4:q G4:q C5:h")

# Build a multi-track song
# song = Song("{title}", tempo=120, key="C major")
# song.track("melody", "C4:q E4:q G4:q C5:h", program="piano")
# song.track("bass", "C2:h G2:h", program="acoustic bass")
# song.play()
# song.export("{slug}.mid")
'''

_STARTER_TOML = '''# Delphi project configuration
[project]
name = "{title}"
version = "0.1.0"

[settings]
tempo = {tempo}
key = "{key}"
time_sig = "4/4"
soundfont = "{soundfont}"   # Leave blank for default (GeneralUser GS)
'''


def _init_project(name: str | None) -> None:
    """Create a new Delphi project directory with starter files."""
    if name is None:
        name = input("Project name: ").strip()
        if not name:
            print("Aborted.")
            return

    slug = name.lower().replace(" ", "-").replace("_", "-")

    # Always create in the OS-standard projects directory
    parent = _get_projects_dir()
    parent.mkdir(parents=True, exist_ok=True)

    project_dir = str(parent / slug)
    cfg = _load_config()

    if os.path.exists(project_dir):
        print(f"\033[31mError:\033[0m Directory '{slug}/' already exists.")
        return

    os.makedirs(project_dir, exist_ok=True)

    # Create delphi.toml
    toml_content = _STARTER_TOML.format(
        title=name,
        tempo=cfg.get("default_tempo", 120),
        key=cfg.get("default_key", "C major"),
        soundfont=cfg.get("default_soundfont", ""),
    )
    toml_path = os.path.join(project_dir, PROJECT_FILE)
    with open(toml_path, "w") as f:
        f.write(toml_content)

    # Create main script
    main_path = os.path.join(project_dir, "main.delphi")
    with open(main_path, "w") as f:
        f.write(_STARTER_SCRIPT.format(title=name, slug=slug))

    # Create patterns/ directory for reusable motifs
    patterns_dir = os.path.join(project_dir, "patterns")
    os.makedirs(patterns_dir, exist_ok=True)

    # Create exports/ directory for MIDI/WAV output
    exports_dir = os.path.join(project_dir, "exports")
    os.makedirs(exports_dir, exist_ok=True)

    print(f"""
\033[1;32m✓ Created project: {name}\033[0m

  {project_dir}/
  ├── delphi.toml      Project configuration
  ├── main.delphi      Main script (start here)
  ├── patterns/        Reusable patterns & motifs
  └── exports/         MIDI and WAV output

  Get started:
    delphi open {slug}     Open the REPL with this project
    delphi projects        List all your projects
""")


def _open_project(name: str | None) -> None:
    """Open a Delphi project directory in the REPL.

    Resolution order:
    1. If name is None or '.', use cwd
    2. If name is an existing directory path, use it directly
    3. Look for name inside the projects directory
    4. Look for name in cwd
    """
    if name is None or name == ".":
        path = os.getcwd()
    elif os.path.isdir(name):
        path = os.path.abspath(name)
    else:
        # Try projects_dir first, then cwd
        projects_dir = _get_projects_dir()
        found = None

        candidate = str(projects_dir / name)
        if os.path.isdir(candidate):
            found = candidate

        if not found:
            candidate = os.path.join(os.getcwd(), name)
            if os.path.isdir(candidate):
                found = candidate

        if not found:
            print(f"\033[31mError:\033[0m Project '{name}' not found in:")
            print(f"  - {projects_dir}")
            print(f"  - {os.getcwd()}")
            print(f"\n  Use 'delphi projects' to list available projects.")
            return

        path = os.path.abspath(found)

    if not os.path.isdir(path):
        print(f"\033[31mError:\033[0m '{path}' is not a directory.")
        return

    toml_path = os.path.join(path, PROJECT_FILE)
    if not os.path.exists(toml_path):
        print(f"\033[33mWarning:\033[0m No {PROJECT_FILE} found in '{path}'.")
        print("  Launching REPL anyway (use 'delphi init' to create a project).\n")

    os.chdir(path)
    from delphi.repl import run_repl
    run_repl(project_dir=path)


def _find_project_root(start: str) -> str | None:
    """Walk up from start directory looking for delphi.toml."""
    current = os.path.abspath(start)
    while True:
        if os.path.exists(os.path.join(current, PROJECT_FILE)):
            return current
        parent = os.path.dirname(current)
        if parent == current:
            return None
        current = parent


# ── Global config (~/.delphi/config.toml) ─────────────────────

_DEFAULT_CONFIG = """\
# Delphi global configuration
# Located at: ~/.delphi/config.toml

[paths]
# Where 'delphi init' creates new projects (default: current directory)
# projects_dir = "~/Music/delphi-projects"

[defaults]
# Default settings applied to new projects created with 'delphi init'
# default_tempo = 120
# default_key = "C major"
# default_soundfont = ""
"""


def _load_config() -> dict:
    """Load ~/.delphi/config.toml and return a flat key→value dict."""
    if not _CONFIG_FILE.exists():
        return {}

    config: dict = {}
    current_section = ""

    with open(_CONFIG_FILE) as f:
        for line in f:
            line = line.strip()
            if not line or line.startswith("#"):
                continue
            if line.startswith("[") and line.endswith("]"):
                current_section = line[1:-1].strip()
                continue
            if "=" in line:
                key, _, value = line.partition("=")
                key = key.strip()
                value = value.strip()
                # Parse quoted strings
                if value.startswith('"'):
                    inner = value[1:]
                    end = inner.find('"')
                    value = inner[:end] if end >= 0 else inner
                elif value.startswith("'"):
                    inner = value[1:]
                    end = inner.find("'")
                    value = inner[:end] if end >= 0 else inner
                else:
                    # Strip inline comments for unquoted values
                    if "#" in value:
                        value = value[:value.index("#")].strip()
                    try:
                        value = int(value)
                    except ValueError:
                        try:
                            value = float(value)
                        except ValueError:
                            pass
                config[key] = value

    return config


def _save_config(config: dict) -> None:
    """Write config dict to ~/.delphi/config.toml."""
    _CONFIG_DIR.mkdir(parents=True, exist_ok=True)

    lines = [
        "# Delphi global configuration",
        f"# Located at: {_CONFIG_FILE}",
        "",
    ]

    # Group keys by known sections
    path_keys = {"projects_dir"}
    default_keys = {"default_tempo", "default_key", "default_soundfont"}

    paths = {k: v for k, v in config.items() if k in path_keys}
    defaults = {k: v for k, v in config.items() if k in default_keys}
    other = {k: v for k, v in config.items() if k not in path_keys and k not in default_keys}

    if paths:
        lines.append("[paths]")
        for k, v in paths.items():
            lines.append(f'{k} = "{v}"')
        lines.append("")

    if defaults:
        lines.append("[defaults]")
        for k, v in defaults.items():
            if isinstance(v, str):
                lines.append(f'{k} = "{v}"')
            else:
                lines.append(f'{k} = {v}')
        lines.append("")

    if other:
        lines.append("[other]")
        for k, v in other.items():
            if isinstance(v, str):
                lines.append(f'{k} = "{v}"')
            else:
                lines.append(f'{k} = {v}')
        lines.append("")

    with open(_CONFIG_FILE, "w") as f:
        f.write("\n".join(lines) + "\n")


def _config_command(args: list[str]) -> None:
    """Handle 'delphi config [key] [value]'."""
    cfg = _load_config()

    if not args:
        # Show current config
        if not _CONFIG_FILE.exists():
            print(f"  No custom config yet.")
            print(f"  Config location: {_CONFIG_FILE}")
            print(f"  Projects directory: {_get_projects_dir()}")
            print(f"\n  Override with: delphi config projects_dir /my/custom/path")
            return
        print(f"\033[1mConfig:\033[0m {_CONFIG_FILE}\n")
        print(f"  projects_dir = {_get_projects_dir()}")
        for k, v in cfg.items():
            if k != "projects_dir":
                print(f"  {k} = {v}")
        print()
        return

    if len(args) == 1:
        # Show single key
        key = args[0]
        if key in cfg:
            print(f"  {key} = {cfg[key]}")
        else:
            print(f"  {key} is not set")
        return

    # Set a value
    key = args[0]
    value = " ".join(args[1:])
    # Expand ~ for path keys
    if key == "projects_dir":
        expanded = os.path.expanduser(value)
        os.makedirs(expanded, exist_ok=True)
        print(f"  Projects directory: {expanded}")
    cfg[key] = value
    _save_config(cfg)
    print(f"  \033[32m✓\033[0m {key} = {value}")


def _list_projects() -> None:
    """List all projects in the projects directory."""
    projects_dir = _get_projects_dir()

    if not projects_dir.is_dir():
        print(f"  No projects yet. Create one with: delphi init my-song")
        print(f"  Projects directory: {projects_dir}")
        return

    projects = []
    for entry in sorted(os.listdir(projects_dir)):
        full = projects_dir / entry
        if full.is_dir() and (full / PROJECT_FILE).exists():
            projects.append(entry)

    if not projects:
        print(f"  No projects found in {projects_dir}")
        print(f"  Create one with: delphi init my-song")
        return

    print(f"\n\033[1mProjects\033[0m ({projects_dir}):\n")
    for p in projects:
        print(f"  {p}")
    print(f"\n  Open one with: delphi open {projects[0]}\n")


if __name__ == "__main__":
    main()
