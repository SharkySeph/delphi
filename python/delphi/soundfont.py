"""
SoundFont manager — discovers, downloads, and caches .sf2 files.

On first use, Delphi auto-downloads the free GeneralUser GS SoundFont (~30 MB)
to ~/.delphi/soundfonts/. Users can also set a custom path.
"""

from __future__ import annotations

import os
import sys
from pathlib import Path
from typing import Optional

# Default SoundFont: GeneralUser GS (free for any use, ~30 MB)
# https://schristiancollins.com/generaluser.php
_DEFAULT_SF_NAME = "GeneralUser-GS.sf2"
_DEFAULT_SF_URL = (
    "https://github.com/mrbumpy409/GeneralUser-GS/raw/main/GeneralUser-GS.sf2"
)

_DELPHI_DIR = Path.home() / ".delphi"
_SOUNDFONTS_DIR = _DELPHI_DIR / "soundfonts"

# Module-level override
_custom_sf_path: Optional[str] = None


def set_soundfont(path: str) -> None:
    """Set a custom SoundFont path. Use this if you have your own .sf2 file."""
    global _custom_sf_path
    if not os.path.isfile(path):
        raise FileNotFoundError(f"SoundFont not found: {path}")
    _custom_sf_path = path
    print(f"SoundFont set to: {path}")


def get_soundfont_path() -> Optional[str]:
    """
    Get the path to the active SoundFont.

    Resolution order:
    1. User-set path via set_soundfont()
    2. DELPHI_SOUNDFONT environment variable
    3. ~/.delphi/soundfonts/GeneralUser_GS.sf2 (auto-downloaded)
    4. None (fall back to built-in oscillator synth)
    """
    # 1. Explicit override
    if _custom_sf_path and os.path.isfile(_custom_sf_path):
        return _custom_sf_path

    # 2. Environment variable
    env_path = os.environ.get("DELPHI_SOUNDFONT")
    if env_path and os.path.isfile(env_path):
        return env_path

    # 3. Default location
    default_path = _SOUNDFONTS_DIR / _DEFAULT_SF_NAME
    if default_path.is_file():
        return str(default_path)

    return None


def ensure_soundfont() -> Optional[str]:
    """
    Ensure a SoundFont is available, downloading the default if needed.
    Returns the path, or None if download fails/is declined.
    """
    path = get_soundfont_path()
    if path:
        return path

    # Need to download
    return _download_default_soundfont()


def _download_default_soundfont() -> Optional[str]:
    """Download GeneralUser GS to ~/.delphi/soundfonts/."""
    dest = _SOUNDFONTS_DIR / _DEFAULT_SF_NAME

    print(f"\n[Delphi] No SoundFont found. Downloading GeneralUser GS (~30 MB)...")
    print(f"  Destination: {dest}")
    print(f"  License: Free for any use (by S. Christian Collins)")
    print()

    try:
        _SOUNDFONTS_DIR.mkdir(parents=True, exist_ok=True)
    except OSError as e:
        print(f"[Delphi] Cannot create directory {_SOUNDFONTS_DIR}: {e}")
        return None

    # Try primary URL
    result = _download_file(_DEFAULT_SF_URL, dest)
    if result:
        print(f"[Delphi] SoundFont ready: {dest}")
        return str(dest)

    print("[Delphi] Could not download SoundFont. Using built-in synth.")
    print("[Delphi] You can manually download a GM SoundFont and set it with:")
    print("           set_soundfont('/path/to/your.sf2')")
    return None


def _download_file(url: str, dest: Path) -> bool:
    """Download a URL to a file path. Returns True on success."""
    try:
        import urllib.request
        import shutil

        print(f"  Downloading from: {url}")

        # Create a temporary file first, then rename (atomic)
        tmp = dest.with_suffix(".tmp")

        req = urllib.request.Request(url, headers={"User-Agent": "Delphi/0.1"})
        with urllib.request.urlopen(req, timeout=60) as response:
            total = response.headers.get("Content-Length")
            if total:
                total = int(total)

            downloaded = 0
            with open(tmp, "wb") as f:
                while True:
                    chunk = response.read(65536)
                    if not chunk:
                        break
                    f.write(chunk)
                    downloaded += len(chunk)
                    if total:
                        pct = downloaded * 100 // total
                        mb = downloaded / (1024 * 1024)
                        total_mb = total / (1024 * 1024)
                        sys.stdout.write(
                            f"\r  Progress: {mb:.1f}/{total_mb:.1f} MB ({pct}%)"
                        )
                        sys.stdout.flush()

            print()  # newline after progress

        # Validate: should be at least 1 MB
        if tmp.stat().st_size < 1_000_000:
            print(f"  Downloaded file too small, removing.")
            tmp.unlink(missing_ok=True)
            return False

        # Move into place
        shutil.move(str(tmp), str(dest))
        return True

    except Exception as e:
        print(f"  Download failed: {e}")
        # Clean up temp file
        tmp = dest.with_suffix(".tmp")
        if tmp.exists():
            tmp.unlink(missing_ok=True)
        return False


def soundfont_info() -> None:
    """Print information about the current SoundFont configuration."""
    path = get_soundfont_path()
    if path:
        size_mb = os.path.getsize(path) / (1024 * 1024)
        print(f"SoundFont: {path}")
        print(f"Size: {size_mb:.1f} MB")
    else:
        print("No SoundFont configured.")
        print(f"Default location: {_SOUNDFONTS_DIR / _DEFAULT_SF_NAME}")
        print("Run ensure_soundfont() to download, or set_soundfont('/path/to/file.sf2')")
