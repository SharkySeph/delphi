#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────
# Delphi — Linux Installer
#
# Installs both:
#   • Delphi CLI/REPL  (Python wheel → venv)
#   • Delphi Studio    (native GUI binary or .deb package)
#
# Usage:
#   curl -sSf https://raw.githubusercontent.com/SharkySeph/delphi/main/deploy/install-linux.sh | bash
#
# Options:
#   --deb            Install Delphi Studio via .deb package (requires dpkg)
#   --deb-only       Only install the .deb package (skip Python CLI)
#   --deb-file PATH  Use a local .deb file instead of downloading
# ─────────────────────────────────────────────────────────────
set -euo pipefail

DELPHI_VENV="${DELPHI_VENV:-$HOME/.local/share/delphi/venv}"
DELPHI_VERSION="${DELPHI_VERSION:-}"
GITHUB_REPO="SharkySeph/delphi"
LAUNCHER_DIR="$HOME/.local/bin"
USE_DEB=false
DEB_ONLY=false
DEB_FILE=""

# ── Parse arguments ──────────────────────────────────────────
while [[ $# -gt 0 ]]; do
    case "$1" in
        --deb)      USE_DEB=true; shift ;;
        --deb-only) USE_DEB=true; DEB_ONLY=true; shift ;;
        --deb-file) USE_DEB=true; DEB_FILE="$2"; shift 2 ;;
        *)          shift ;;
    esac
done

# ── Find Python 3.10+ ───────────────────────────────────────
find_python() {
    for py in python3 python; do
        if command -v "$py" &>/dev/null; then
            local ver
            ver="$($py -c 'import sys; print(f"{sys.version_info.major}.{sys.version_info.minor}")')" || continue
            local maj="${ver%%.*}" min="${ver##*.}"
            if (( maj > 3 || (maj == 3 && min >= 10) )); then
                echo "$py"; return 0
            fi
        fi
    done
    echo "Error: Python 3.10+ is required." >&2
    echo "  Ubuntu/Debian: sudo apt install python3 python3-venv" >&2
    echo "  Fedora:        sudo dnf install python3" >&2
    echo "  Arch:          sudo pacman -S python" >&2
    exit 1
}

PYTHON="$(find_python)"
echo "⚙  Using $PYTHON ($($PYTHON --version))"

# ── Check ALSA runtime (needed for audio playback) ──────────
if ! ldconfig -p 2>/dev/null | grep -q libasound.so; then
    echo "⚠  ALSA runtime library not found — audio may not work"
    echo "   Ubuntu/Debian: sudo apt install libasound2"
    echo "   Fedora:        sudo dnf install alsa-lib"
    echo "   Arch:          sudo pacman -S alsa-lib"
fi

# ── Install Python CLI/REPL (skip if --deb-only) ────────────
if [[ "$DEB_ONLY" == false ]]; then

# ── Create venv ──────────────────────────────────────────────
if [[ ! -d "$DELPHI_VENV" ]]; then
    echo "⚙  Creating venv → $DELPHI_VENV"
    mkdir -p "$(dirname "$DELPHI_VENV")"
    $PYTHON -m venv "$DELPHI_VENV"
fi
source "$DELPHI_VENV/bin/activate"

# ── Install Delphi from GitHub Releases ──────────────────────
# Note: "delphi" on PyPI is a different package — always use GitHub Releases
echo "⚙  Installing Delphi..."
if [[ -z "$DELPHI_VERSION" ]]; then
    DELPHI_VERSION="$(curl -sSf "https://api.github.com/repos/${GITHUB_REPO}/releases/latest" \
        | $PYTHON -c 'import sys,json; print(json.load(sys.stdin)["tag_name"].lstrip("v"))' 2>/dev/null)" || {
        echo "Error: Could not find a release. Set DELPHI_VERSION=0.7.0" >&2; exit 1
    }
fi
ARCH="$(uname -m)"
WHEEL_URL="$(curl -sSf "https://api.github.com/repos/${GITHUB_REPO}/releases/tags/v${DELPHI_VERSION}" \
    | $PYTHON -c "
import sys, json
for a in json.load(sys.stdin).get('assets', []):
    if a['name'].endswith('.whl') and '${ARCH}' in a['name']:
        print(a['browser_download_url']); break
" 2>/dev/null)" || true

if [[ -n "$WHEEL_URL" ]]; then
    pip install --quiet "$WHEEL_URL"
    echo "✔  Installed Delphi ${DELPHI_VERSION}"
else
    echo "Error: No wheel found for Linux ${ARCH}." >&2
    echo "  https://github.com/${GITHUB_REPO}/releases" >&2
    exit 1
fi

fi  # end DEB_ONLY check

# ── Install Delphi Studio (GUI) ──────────────────────────────
if [[ -n "$DEB_FILE" ]]; then
    # Use a local .deb file
    echo "⚙  Installing Delphi Studio from local .deb: $DEB_FILE"
    sudo dpkg -i "$DEB_FILE"
    sudo apt-get install -f -y 2>/dev/null || true  # fix missing deps
    echo "✔  Installed Delphi Studio (deb)"
elif [[ "$USE_DEB" == true ]]; then
    # Download .deb from GitHub Releases
    DEB_URL="$(curl -sSf "https://api.github.com/repos/${GITHUB_REPO}/releases/tags/v${DELPHI_VERSION}" \
        | $PYTHON -c "
import sys, json
for a in json.load(sys.stdin).get('assets', []):
    if a['name'].endswith('.deb'):
        print(a['browser_download_url']); break
" 2>/dev/null)" || true
    if [[ -n "$DEB_URL" ]]; then
        echo "⚙  Installing Delphi Studio (.deb)..."
        TMPDIR="$(mktemp -d)"
        curl -sSfL "$DEB_URL" -o "$TMPDIR/delphi-studio.deb"
        sudo dpkg -i "$TMPDIR/delphi-studio.deb"
        sudo apt-get install -f -y 2>/dev/null || true
        rm -rf "$TMPDIR"
        echo "✔  Installed Delphi Studio (deb)"
    else
        echo "⚠  No .deb found for v${DELPHI_VERSION} — falling back to tarball"
        USE_DEB=false
    fi
fi

if [[ "$USE_DEB" == false && -z "$DEB_FILE" ]]; then
    # Fallback: download standalone binary tarball
    GUI_BIN_URL="$(curl -sSf "https://api.github.com/repos/${GITHUB_REPO}/releases/tags/v${DELPHI_VERSION}" \
        | $PYTHON -c "
import sys, json
for a in json.load(sys.stdin).get('assets', []):
    if 'linux' in a['name'].lower() and a['name'].endswith('.tar.gz'):
        print(a['browser_download_url']); break
" 2>/dev/null)" || true

    if [[ -n "$GUI_BIN_URL" ]]; then
        echo "⚙  Installing Delphi Studio (GUI)..."
        TMPDIR="$(mktemp -d)"
        curl -sSfL "$GUI_BIN_URL" -o "$TMPDIR/delphi-studio.tar.gz"
        tar -xzf "$TMPDIR/delphi-studio.tar.gz" -C "$TMPDIR"
        install -m 755 "$TMPDIR/delphi-studio" "$HOME/.local/bin/delphi-studio"
        rm -rf "$TMPDIR"
        echo "✔  Installed Delphi Studio GUI"
    else
        echo "⚠  No GUI binary found for Linux ${ARCH} — CLI/REPL still available"
    fi
fi

# ── Create launcher on PATH (skip if --deb-only) ────────────
if [[ "$DEB_ONLY" == false ]]; then
mkdir -p "$LAUNCHER_DIR"
cat > "$LAUNCHER_DIR/delphi" << EOF
#!/usr/bin/env bash
exec "$DELPHI_VENV/bin/delphi" "\$@"
EOF
chmod +x "$LAUNCHER_DIR/delphi"

# Add to PATH if needed
if [[ ":$PATH:" != *":$LAUNCHER_DIR:"* ]]; then
    PROFILE="$HOME/.bashrc"
    [[ "$(basename "$SHELL")" == "zsh" ]] && PROFILE="$HOME/.zshrc"
    if ! grep -q "$LAUNCHER_DIR" "$PROFILE" 2>/dev/null; then
        printf '\nexport PATH="%s:$PATH"\n' "$LAUNCHER_DIR" >> "$PROFILE"
        echo "⚙  Added $LAUNCHER_DIR to $PROFILE — restart your shell or: source $PROFILE"
    fi
    export PATH="$LAUNCHER_DIR:$PATH"
fi
fi  # end DEB_ONLY launcher check

# ── Create data dirs ────────────────────────────────────────
mkdir -p "$HOME/.local/share/delphi/projects" "$HOME/.delphi/soundfonts"

# ── Done ─────────────────────────────────────────────────────
echo ""
echo "✔  Delphi installed!"
echo "   Run 'delphi' for the CLI/REPL"
echo "   Run 'delphi-studio' for the GUI"
if [[ "$DEB_ONLY" == false ]]; then
echo "   Venv:       $DELPHI_VENV"
fi
echo "   SoundFonts: ~/.delphi/soundfonts/"
echo "   Projects:   ~/.local/share/delphi/projects/"
