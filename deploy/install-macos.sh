#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────
# Delphi — macOS Installer
#
# Downloads and installs both native binaries:
#   • delphi        (CLI: export, play, info, new)
#   • delphi-studio (native GUI)
#
# CoreAudio is built-in on every Mac — no extra audio libs.
#
# Usage:
#   curl -sSf https://raw.githubusercontent.com/SharkySeph/delphi/main/deploy/install-macos.sh | bash
# ─────────────────────────────────────────────────────────────
set -euo pipefail

DELPHI_VERSION="${DELPHI_VERSION:-}"
GITHUB_REPO="SharkySeph/delphi"
INSTALL_DIR="$HOME/.local/bin"

# ── Resolve latest version ──────────────────────────────────
if [[ -z "$DELPHI_VERSION" ]]; then
    DELPHI_VERSION="$(curl -sSf "https://api.github.com/repos/${GITHUB_REPO}/releases/latest" \
        | grep '"tag_name"' | sed 's/.*"v\(.*\)".*/\1/')" || {
        echo "Error: Could not find a release. Set DELPHI_VERSION=0.8.0" >&2; exit 1
    }
fi
ARCH="$(uname -m)"
[[ "$ARCH" == "arm64" ]] && TAR_ARCH="arm64" || TAR_ARCH="x86_64"
echo "⚙  Installing Delphi v${DELPHI_VERSION} for macOS ${ARCH}"

# ── Download tarball ─────────────────────────────────────────
TAR_URL="$(curl -sSf "https://api.github.com/repos/${GITHUB_REPO}/releases/tags/v${DELPHI_VERSION}" \
    | grep '"browser_download_url".*macos.*\.tar\.gz"' | head -1 | sed 's/.*"\(https[^"]*\)".*/\1/')" || true

if [[ -z "$TAR_URL" ]]; then
    echo "Error: No macOS tarball found for v${DELPHI_VERSION}." >&2
    echo "  https://github.com/${GITHUB_REPO}/releases" >&2
    exit 1
fi

echo "⚙  Downloading..."
TMPDIR="$(mktemp -d)"
curl -sSfL "$TAR_URL" -o "$TMPDIR/delphi.tar.gz"
tar -xzf "$TMPDIR/delphi.tar.gz" -C "$TMPDIR"

mkdir -p "$INSTALL_DIR"
find "$TMPDIR" -name "delphi" -type f -perm +111 -exec install -m 755 {} "$INSTALL_DIR/delphi" \;
find "$TMPDIR" -name "delphi-studio" -type f -perm +111 -exec install -m 755 {} "$INSTALL_DIR/delphi-studio" \;
rm -rf "$TMPDIR"
echo "✔  Installed delphi and delphi-studio to $INSTALL_DIR"

# ── Add to PATH ─────────────────────────────────────────────
if [[ ":$PATH:" != *":$INSTALL_DIR:"* ]]; then
    PROFILE="$HOME/.zshrc"  # macOS default shell is zsh
    [[ "$(basename "$SHELL")" == "bash" ]] && PROFILE="$HOME/.bash_profile"
    if ! grep -q "$INSTALL_DIR" "$PROFILE" 2>/dev/null; then
        printf '\nexport PATH="%s:$PATH"\n' "$INSTALL_DIR" >> "$PROFILE"
        echo "⚙  Added $INSTALL_DIR to $PROFILE — restart your shell or: source $PROFILE"
    fi
    export PATH="$INSTALL_DIR:$PATH"
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
