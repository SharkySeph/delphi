#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────
# Delphi — Linux Installer
#
# Installs both:
#   • delphi        (CLI: export, play, info, new)
#   • delphi-studio (native GUI)
#
# Usage:
#   curl -sSf https://raw.githubusercontent.com/SharkySeph/delphi/main/deploy/install-linux.sh | bash
#
# Options:
#   --deb            Install via .deb package (requires dpkg)
#   --deb-file PATH  Use a local .deb file instead of downloading
# ─────────────────────────────────────────────────────────────
set -euo pipefail

DELPHI_VERSION="${DELPHI_VERSION:-}"
GITHUB_REPO="SharkySeph/delphi"
INSTALL_DIR="$HOME/.local/bin"
USE_DEB=false
DEB_FILE=""

# ── Parse arguments ──────────────────────────────────────────
while [[ $# -gt 0 ]]; do
    case "$1" in
        --deb)      USE_DEB=true; shift ;;
        --deb-file) USE_DEB=true; DEB_FILE="$2"; shift 2 ;;
        *)          shift ;;
    esac
done

# ── Check ALSA runtime (needed for audio playback) ──────────
if ! ldconfig -p 2>/dev/null | grep -q libasound.so; then
    echo "⚠  ALSA runtime library not found — audio may not work"
    echo "   Ubuntu/Debian: sudo apt install libasound2"
    echo "   Fedora:        sudo dnf install alsa-lib"
    echo "   Arch:          sudo pacman -S alsa-lib"
fi

# ── Resolve latest version ──────────────────────────────────
if [[ -z "$DELPHI_VERSION" ]]; then
    DELPHI_VERSION="$(curl -sSf "https://api.github.com/repos/${GITHUB_REPO}/releases/latest" \
        | grep '"tag_name"' | sed 's/.*"v\(.*\)".*/\1/')" || {
        echo "Error: Could not find a release. Set DELPHI_VERSION=0.8.0" >&2; exit 1
    }
fi
ARCH="$(uname -m)"
echo "⚙  Installing Delphi v${DELPHI_VERSION} for ${ARCH}"

# ── Install via .deb ─────────────────────────────────────────
if [[ -n "$DEB_FILE" ]]; then
    echo "⚙  Installing from local .deb: $DEB_FILE"
    sudo dpkg -i "$DEB_FILE"
    sudo apt-get install -f -y 2>/dev/null || true
    echo "✔  Installed Delphi (deb)"
elif [[ "$USE_DEB" == true ]]; then
    DEB_URL="$(curl -sSf "https://api.github.com/repos/${GITHUB_REPO}/releases/tags/v${DELPHI_VERSION}" \
        | grep '"browser_download_url".*\.deb"' | head -1 | sed 's/.*"\(https[^"]*\)".*/\1/')" || true
    if [[ -n "$DEB_URL" ]]; then
        echo "⚙  Downloading .deb..."
        TMPDIR="$(mktemp -d)"
        curl -sSfL "$DEB_URL" -o "$TMPDIR/delphi.deb"
        sudo dpkg -i "$TMPDIR/delphi.deb"
        sudo apt-get install -f -y 2>/dev/null || true
        rm -rf "$TMPDIR"
        echo "✔  Installed Delphi (deb)"
    else
        echo "⚠  No .deb found for v${DELPHI_VERSION} — falling back to tarball"
        USE_DEB=false
    fi
fi

# ── Install via tarball (fallback) ───────────────────────────
if [[ "$USE_DEB" == false && -z "$DEB_FILE" ]]; then
    TAR_URL="$(curl -sSf "https://api.github.com/repos/${GITHUB_REPO}/releases/tags/v${DELPHI_VERSION}" \
        | grep '"browser_download_url".*linux.*\.tar\.gz"' | head -1 | sed 's/.*"\(https[^"]*\)".*/\1/')" || true

    if [[ -n "$TAR_URL" ]]; then
        echo "⚙  Downloading tarball..."
        TMPDIR="$(mktemp -d)"
        curl -sSfL "$TAR_URL" -o "$TMPDIR/delphi.tar.gz"
        tar -xzf "$TMPDIR/delphi.tar.gz" -C "$TMPDIR"
        mkdir -p "$INSTALL_DIR"
        find "$TMPDIR" -name "delphi" -type f -executable -exec install -m 755 {} "$INSTALL_DIR/delphi" \;
        find "$TMPDIR" -name "delphi-studio" -type f -executable -exec install -m 755 {} "$INSTALL_DIR/delphi-studio" \;
        rm -rf "$TMPDIR"
        echo "✔  Installed delphi and delphi-studio to $INSTALL_DIR"
    else
        echo "Error: No tarball found for Linux ${ARCH}." >&2
        echo "  https://github.com/${GITHUB_REPO}/releases" >&2
        exit 1
    fi

    # Add to PATH if needed
    if [[ ":$PATH:" != *":$INSTALL_DIR:"* ]]; then
        PROFILE="$HOME/.bashrc"
        [[ "$(basename "$SHELL")" == "zsh" ]] && PROFILE="$HOME/.zshrc"
        if ! grep -q "$INSTALL_DIR" "$PROFILE" 2>/dev/null; then
            printf '\nexport PATH="%s:$PATH"\n' "$INSTALL_DIR" >> "$PROFILE"
            echo "⚙  Added $INSTALL_DIR to $PROFILE — restart your shell or: source $PROFILE"
        fi
        export PATH="$INSTALL_DIR:$PATH"
    fi
fi

# ── Create data dirs ────────────────────────────────────────
mkdir -p "$HOME/.local/share/delphi/projects" "$HOME/.delphi/soundfonts"

# ── Done ─────────────────────────────────────────────────────
echo ""
echo "✔  Delphi installed!"
echo "   delphi          — CLI (export, play, info, new)"
echo "   delphi-studio   — GUI (editor, piano roll, mixer)"
echo "   SoundFonts: ~/.delphi/soundfonts/"
echo "   Projects:   ~/.local/share/delphi/projects/"
