#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────
# Delphi — Linux Installer
# Installs Delphi from a pre-built wheel. No Rust required.
#
# Prerequisites: Python 3.10+, ALSA runtime library
#
# Usage:
#   curl -sSf https://raw.githubusercontent.com/SharkySeph/delphi/main/deploy/install-linux.sh | bash
#   # or locally:
#   ./deploy/install-linux.sh
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

DELPHI_VENV="${DELPHI_VENV:-$HOME/.local/share/delphi/venv}"
DELPHI_NO_VENV="${DELPHI_NO_VENV:-0}"
DELPHI_VERSION="${DELPHI_VERSION:-}"
MIN_PYTHON="3.10"
GITHUB_REPO="SharkySeph/delphi"

# ── Detect distro ────────────────────────────────────────────
detect_distro() {
    if [[ -f /etc/os-release ]]; then
        . /etc/os-release; echo "${ID:-unknown}"
    else echo "unknown"; fi
}
DISTRO="$(detect_distro)"
info "Detected: ${BOLD}${DISTRO}${NC} ($(uname -m))"

# ── Package installer ────────────────────────────────────────
pkg_install() {
    local packages=("$@")
    case "$DISTRO" in
        ubuntu|debian|pop|linuxmint|elementary|zorin)
            sudo apt-get update -qq && sudo apt-get install -y -qq "${packages[@]}" ;;
        fedora)      sudo dnf install -y "${packages[@]}" ;;
        centos|rhel|rocky|almalinux) sudo yum install -y "${packages[@]}" ;;
        arch|manjaro|endeavouros)    sudo pacman -S --noconfirm --needed "${packages[@]}" ;;
        opensuse*|suse*)             sudo zypper install -y "${packages[@]}" ;;
        alpine)                      sudo apk add "${packages[@]}" ;;
        *) error "Unsupported distro — please install manually: ${packages[*]}"; return 1 ;;
    esac
}

map_alsa_runtime() {
    case "$DISTRO" in
        ubuntu|debian|pop|linuxmint|elementary|zorin) echo "libasound2" ;;
        fedora|centos|rhel|rocky|almalinux)           echo "alsa-lib" ;;
        arch|manjaro|endeavouros)                      echo "alsa-lib" ;;
        opensuse*|suse*)                               echo "alsa" ;;
        alpine)                                        echo "alsa-lib" ;;
        *) echo "alsa-lib" ;;
    esac
}

map_python_venv() {
    case "$DISTRO" in
        ubuntu|debian|pop|linuxmint|elementary|zorin) echo "python3-venv" ;;
        *) echo "" ;;  # most distros bundle venv with python
    esac
}

# ════════════════════════════════════════════════════════════
header "Checking prerequisites"
# ════════════════════════════════════════════════════════════
MISSING=()

# 1. ALSA runtime (the native extension links to libasound)
check_alsa() {
    ldconfig -p 2>/dev/null | grep -q libasound.so && return 0
    for d in /usr/lib /usr/lib64 /usr/lib/x86_64-linux-gnu /usr/lib/aarch64-linux-gnu; do
        [[ -f "$d/libasound.so.2" ]] && return 0
    done
    return 1
}
if check_alsa; then
    success "ALSA runtime library found"
else
    warn "ALSA runtime library not found (needed for audio)"
    MISSING+=("$(map_alsa_runtime)")
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
    error "Python >= $MIN_PYTHON is required"
    info "Install it with your package manager, e.g.:"
    info "  Ubuntu/Debian: sudo apt install python3"
    info "  Fedora:        sudo dnf install python3"
    info "  Arch:          sudo pacman -S python"
    exit 1
fi

# 3. venv module (if using venv)
if [[ "$DELPHI_NO_VENV" != "1" ]]; then
    if $PYTHON_CMD -m venv --help &>/dev/null; then
        success "Python venv module available"
    else
        warn "Python venv module not found"
        mapped="$(map_python_venv)"
        [[ -n "$mapped" ]] && MISSING+=("$mapped")
    fi
fi

# 4. pip
if $PYTHON_CMD -m pip --version &>/dev/null; then
    success "pip available"
else
    warn "pip not found — will bootstrap via ensurepip"
fi

# ── Install missing packages ────────────────────────────────
if [[ ${#MISSING[@]} -gt 0 ]]; then
    header "Installing system packages"
    info "Packages: ${MISSING[*]}"
    pkg_install "${MISSING[@]}"
    success "Done"
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

# Try PyPI first
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

    ARCH="$(uname -m)"
    WHEEL_URL="$(curl -sSf "https://api.github.com/repos/${GITHUB_REPO}/releases/tags/v${DELPHI_VERSION}" \
        | python -c "
import sys, json
data = json.load(sys.stdin)
for asset in data.get('assets', []):
    n = asset['name']
    if n.endswith('.whl') and '${ARCH}' in n:
        print(asset['browser_download_url']); break
" 2>/dev/null || true)"

    if [[ -n "$WHEEL_URL" ]]; then
        info "Downloading wheel for ${ARCH}..."
        pip install --quiet "$WHEEL_URL"
        success "Installed from GitHub Releases"
    else
        error "No pre-built wheel for Linux ${ARCH} / Python ${PYTHON_VER}"
        info "Open an issue: https://github.com/${GITHUB_REPO}/issues"
        exit 1
    fi
fi

# ── Create data directories ─────────────────────────────────
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
# Delphi launcher — installed by deploy/install-linux.sh
set -euo pipefail
exec "${DELPHI_VENV}/bin/delphi" "\$@"
LAUNCHER_EOF
chmod +x "$LAUNCHER"
success "Installed $LAUNCHER"

# Verify it's reachable
if [[ ":$PATH:" == *":$LAUNCHER_DIR:"* ]]; then
    success "$LAUNCHER_DIR is already on PATH"
else
    # Try to add it to the shell profile
    SHELL_NAME="$(basename "$SHELL")"
    case "$SHELL_NAME" in
        zsh)  PROFILE="$HOME/.zshrc" ;;
        bash) PROFILE="$HOME/.bashrc" ;;
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
