#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────
# Delphi — macOS Installer
# Installs Delphi from a pre-built wheel. No Rust required.
#
# Prerequisites: Python 3.10+
# (CoreAudio is built-in on every Mac — no extra audio libs)
#
# Usage:
#   curl -sSf https://raw.githubusercontent.com/SharkySeph/delphi/main/deploy/install-macos.sh | bash
#   # or locally:
#   ./deploy/install-macos.sh
#
# Environment variables:
#   DELPHI_VENV     — Path to venv (default: ~/.local/share/delphi/venv)
#   DELPHI_NO_VENV  — Set to 1 to install into current environment
#   DELPHI_VERSION  — Pin a specific version (default: latest)
# ─────────────────────────────────────────────────────────────
set -euo pipefail

RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[1;33m'
BLUE='\033[0;34m'; BOLD='\033[1m'; NC='\033[0m'

info()    { printf "${BLUE}ℹ${NC}  %s\n" "$*"; }
success() { printf "${GREEN}✔${NC}  %s\n" "$*"; }
warn()    { printf "${YELLOW}⚠${NC}  %s\n" "$*"; }
error()   { printf "${RED}✘${NC}  %s\n" "$*" >&2; }
header()  { printf "\n${BOLD}── %s ──${NC}\n" "$*"; }

if [[ "$(uname -s)" != "Darwin" ]]; then
    error "This script is for macOS. Use install-linux.sh for Linux."
    exit 1
fi

DELPHI_VENV="${DELPHI_VENV:-$HOME/.local/share/delphi/venv}"
DELPHI_NO_VENV="${DELPHI_NO_VENV:-0}"
DELPHI_VERSION="${DELPHI_VERSION:-}"
MIN_PYTHON="3.10"
GITHUB_REPO="SharkySeph/delphi"
ARCH="$(uname -m)"

info "macOS $(sw_vers -productVersion) ($ARCH)"

# ════════════════════════════════════════════════════════════
header "Checking prerequisites"
# ════════════════════════════════════════════════════════════

# 1. CoreAudio (always present on macOS)
if [[ -d "/System/Library/Frameworks/CoreAudio.framework" ]]; then
    success "CoreAudio framework available"
else
    error "CoreAudio not found — unexpected on macOS"
    exit 1
fi

# 2. Python 3.10+
find_python() {
    for py in python3 python; do
        if command -v "$py" &>/dev/null; then
            local ver
            ver="$($py -c "import sys; print(f'{sys.version_info.major}.{sys.version_info.minor}')" 2>/dev/null)" || continue
            local maj="${ver%%.*}" min="${ver##*.}"
            (( maj > 3 || (maj == 3 && min >= 10) )) && { echo "$py:$ver"; return 0; }
        fi
    done
    return 1
}

PYTHON_INFO="$(find_python || true)"
if [[ -n "$PYTHON_INFO" ]]; then
    PYTHON_CMD="${PYTHON_INFO%%:*}"
    PYTHON_VER="${PYTHON_INFO##*:}"
    success "Python $PYTHON_VER ($PYTHON_CMD)"
else
    # Offer to install via Homebrew
    if command -v brew &>/dev/null; then
        warn "Python >= $MIN_PYTHON not found. Installing via Homebrew..."
        brew install python@3.12
        PYTHON_CMD="python3"
        PYTHON_VER="$($PYTHON_CMD -c "import sys; print(f'{sys.version_info.major}.{sys.version_info.minor}')")"
        success "Python $PYTHON_VER installed"
    else
        error "Python >= $MIN_PYTHON is required"
        info "Install options:"
        info "  Homebrew: brew install python@3.12"
        info "  Direct:   https://www.python.org/downloads/"
        exit 1
    fi
fi

# 3. venv + pip
if [[ "$DELPHI_NO_VENV" != "1" ]]; then
    if $PYTHON_CMD -m venv --help &>/dev/null; then
        success "Python venv module available"
    else
        error "Python venv module not found — cannot create virtual environment"
        info "Try: $PYTHON_CMD -m ensurepip && $PYTHON_CMD -m pip install virtualenv"
        exit 1
    fi
fi

if $PYTHON_CMD -m pip --version &>/dev/null; then
    success "pip available"
else
    warn "pip not found — will bootstrap via ensurepip"
fi

# ════════════════════════════════════════════════════════════
header "Setting up Python environment"
# ════════════════════════════════════════════════════════════
if [[ "$DELPHI_NO_VENV" != "1" ]]; then
    if [[ ! -d "$DELPHI_VENV" ]]; then
        info "Creating virtual environment → $DELPHI_VENV"
        mkdir -p "$(dirname "$DELPHI_VENV")"
        $PYTHON_CMD -m venv "$DELPHI_VENV"
    fi
    # shellcheck disable=SC1091
    source "$DELPHI_VENV/bin/activate"
    success "Activated venv: $DELPHI_VENV"
fi

python -m ensurepip --upgrade 2>/dev/null || true
python -m pip install --quiet --upgrade pip

# ════════════════════════════════════════════════════════════
header "Installing Delphi"
# ════════════════════════════════════════════════════════════
VERSION_SPEC=""
[[ -n "$DELPHI_VERSION" ]] && VERSION_SPEC="==${DELPHI_VERSION}"

if pip install --quiet "delphi${VERSION_SPEC}" 2>/dev/null; then
    success "Installed from PyPI"
else
    info "Not on PyPI — trying GitHub Releases..."

    if [[ -z "$DELPHI_VERSION" ]]; then
        DELPHI_VERSION="$(curl -sSf "https://api.github.com/repos/${GITHUB_REPO}/releases/latest" \
            | python -c "import sys,json; print(json.load(sys.stdin)['tag_name'].lstrip('v'))" 2>/dev/null || true)"
        if [[ -z "$DELPHI_VERSION" ]]; then
            error "Could not determine latest release."
            info "Specify a version: DELPHI_VERSION=0.6.0 $0"
            exit 1
        fi
        info "Latest release: v$DELPHI_VERSION"
    fi

    # macOS wheel tags: x86_64 → macosx_*_x86_64, arm64 → macosx_*_arm64
    case "$ARCH" in
        x86_64)  ARCH_PATTERN="x86_64" ;;
        arm64)   ARCH_PATTERN="arm64"  ;;
        *)       error "Unsupported arch: $ARCH"; exit 1 ;;
    esac

    WHEEL_URL="$(curl -sSf "https://api.github.com/repos/${GITHUB_REPO}/releases/tags/v${DELPHI_VERSION}" \
        | python -c "
import sys, json
data = json.load(sys.stdin)
for asset in data.get('assets', []):
    n = asset['name']
    if n.endswith('.whl') and 'macosx' in n and '${ARCH_PATTERN}' in n:
        print(asset['browser_download_url']); break
" 2>/dev/null || true)"

    if [[ -n "$WHEEL_URL" ]]; then
        info "Downloading wheel for macOS ${ARCH}..."
        pip install --quiet "$WHEEL_URL"
        success "Installed from GitHub Releases"
    else
        error "No pre-built wheel for macOS ${ARCH} / Python ${PYTHON_VER}"
        info "Open an issue: https://github.com/${GITHUB_REPO}/issues"
        exit 1
    fi
fi

# ── Data directories ─────────────────────────────────────────
mkdir -p "$HOME/.local/share/delphi/projects"
mkdir -p "$HOME/.delphi/soundfonts"

# ════════════════════════════════════════════════════════════
header "Installing launcher to PATH"
# ════════════════════════════════════════════════════════════
LAUNCHER_DIR="$HOME/.local/bin"
mkdir -p "$LAUNCHER_DIR"

LAUNCHER="$LAUNCHER_DIR/delphi"
cat > "$LAUNCHER" << LAUNCHER_EOF
#!/usr/bin/env bash
# Delphi launcher — installed by deploy/install-macos.sh
set -euo pipefail
exec "${DELPHI_VENV}/bin/delphi" "\$@"
LAUNCHER_EOF
chmod +x "$LAUNCHER"
success "Installed $LAUNCHER"

if [[ ":$PATH:" == *":$LAUNCHER_DIR:"* ]]; then
    success "$LAUNCHER_DIR is already on PATH"
else
    SHELL_NAME="$(basename "$SHELL")"
    case "$SHELL_NAME" in
        zsh)  PROFILE="$HOME/.zshrc" ;;
        bash) PROFILE="$HOME/.bash_profile" ;;
        *)    PROFILE="$HOME/.profile" ;;
    esac

    if [[ -f "$PROFILE" ]] && ! grep -q "$LAUNCHER_DIR" "$PROFILE" 2>/dev/null; then
        printf '\n# Added by Delphi installer\nexport PATH="%s:$PATH"\n' "$LAUNCHER_DIR" >> "$PROFILE"
        export PATH="$LAUNCHER_DIR:$PATH"
        success "Added $LAUNCHER_DIR to $PROFILE"
        info "Run: source $PROFILE  (or open a new terminal)"
    elif ! grep -q "$LAUNCHER_DIR" "$PROFILE" 2>/dev/null; then
        warn "$LAUNCHER_DIR is not on your PATH"
        info "Add this to your shell profile:"
        printf "   export PATH=\"%s:\$PATH\"\n" "$LAUNCHER_DIR"
    fi
fi

# ════════════════════════════════════════════════════════════
header "Verifying installation"
# ════════════════════════════════════════════════════════════
if "$LAUNCHER" --version &>/dev/null; then
    VER="$("$LAUNCHER" --version 2>/dev/null || echo "?")"
    success "$VER"
else
    warn "Launcher created but --version check failed. Try: delphi --help"
fi

# ── Done ─────────────────────────────────────────────────────
header "Installation complete!"
printf "\n"
info "Command:    $LAUNCHER"
[[ "$DELPHI_NO_VENV" != "1" ]] && info "Venv:       $DELPHI_VENV"
info "SoundFonts: ~/.delphi/soundfonts/"
info "Projects:   ~/.local/share/delphi/projects/"
printf "\n"
success "Run 'delphi' to start composing!"
