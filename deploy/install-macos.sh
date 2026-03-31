#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────
# Delphi — macOS Installer
#
#   1. Find or create a Python venv
#   2. pip install the pre-built wheel (Rust binary included)
#   3. Create a launcher on PATH
#
# CoreAudio is built-in on every Mac — no extra audio libs.
#
# Usage:
#   curl -sSf https://raw.githubusercontent.com/SharkySeph/delphi/main/deploy/install-macos.sh | bash
# ─────────────────────────────────────────────────────────────
set -euo pipefail

DELPHI_VENV="${DELPHI_VENV:-$HOME/.local/share/delphi/venv}"
DELPHI_VERSION="${DELPHI_VERSION:-}"
GITHUB_REPO="SharkySeph/delphi"
LAUNCHER_DIR="$HOME/.local/bin"

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
    echo "  Install via: brew install python@3.12" >&2
    echo "  Or download: https://www.python.org/downloads/" >&2
    exit 1
}

PYTHON="$(find_python)"
echo "⚙  Using $PYTHON ($($PYTHON --version))"

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
        echo "Error: Could not find a release. Set DELPHI_VERSION=0.6.0" >&2; exit 1
    }
fi
ARCH="$(uname -m)"
[[ "$ARCH" == "arm64" ]] && WHL_ARCH="arm64" || WHL_ARCH="x86_64"
WHEEL_URL="$(curl -sSf "https://api.github.com/repos/${GITHUB_REPO}/releases/tags/v${DELPHI_VERSION}" \
    | $PYTHON -c "
import sys, json
for a in json.load(sys.stdin).get('assets', []):
    if a['name'].endswith('.whl') and 'macosx' in a['name'] and '${WHL_ARCH}' in a['name']:
        print(a['browser_download_url']); break
" 2>/dev/null)" || true

if [[ -n "$WHEEL_URL" ]]; then
    pip install --quiet "$WHEEL_URL"
    echo "✔  Installed Delphi ${DELPHI_VERSION}"
else
    echo "Error: No wheel found for macOS ${ARCH}." >&2
    echo "  https://github.com/${GITHUB_REPO}/releases" >&2
    exit 1
fi

# ── Create launcher on PATH ─────────────────────────────────
mkdir -p "$LAUNCHER_DIR"
cat > "$LAUNCHER_DIR/delphi" << EOF
#!/usr/bin/env bash
exec "$DELPHI_VENV/bin/delphi" "\$@"
EOF
chmod +x "$LAUNCHER_DIR/delphi"

if [[ ":$PATH:" != *":$LAUNCHER_DIR:"* ]]; then
    PROFILE="$HOME/.zshrc"  # macOS default shell is zsh
    [[ "$(basename "$SHELL")" == "bash" ]] && PROFILE="$HOME/.bash_profile"
    if ! grep -q "$LAUNCHER_DIR" "$PROFILE" 2>/dev/null; then
        printf '\nexport PATH="%s:$PATH"\n' "$LAUNCHER_DIR" >> "$PROFILE"
        echo "⚙  Added $LAUNCHER_DIR to $PROFILE — restart your shell or: source $PROFILE"
    fi
    export PATH="$LAUNCHER_DIR:$PATH"
fi

# ── Create data dirs ────────────────────────────────────────
mkdir -p "$HOME/.local/share/delphi/projects" "$HOME/.delphi/soundfonts"

# ── Done ─────────────────────────────────────────────────────
echo ""
echo "✔  Delphi installed! Run 'delphi' to start."
echo "   Venv:       $DELPHI_VENV"
echo "   SoundFonts: ~/.delphi/soundfonts/"
echo "   Projects:   ~/.local/share/delphi/projects/"
