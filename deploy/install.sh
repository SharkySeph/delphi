#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────
# Delphi — Universal Installer
# Auto-detects the OS and runs the appropriate installer.
#
# Usage:
#   curl -sSf https://raw.githubusercontent.com/SharkySeph/delphi/main/deploy/install.sh | bash
# ─────────────────────────────────────────────────────────────
set -euo pipefail

REPO_URL="https://raw.githubusercontent.com/SharkySeph/delphi/main/deploy"

case "$(uname -s)" in
    Linux)
        echo "Detected Linux — running Linux installer..."
        # If running from the repo, use local script
        SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
        if [[ -f "$SCRIPT_DIR/install-linux.sh" ]]; then
            exec bash "$SCRIPT_DIR/install-linux.sh" "$@"
        else
            curl -sSf "$REPO_URL/install-linux.sh" | bash -s -- "$@"
        fi
        ;;
    Darwin)
        echo "Detected macOS — running macOS installer..."
        SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
        if [[ -f "$SCRIPT_DIR/install-macos.sh" ]]; then
            exec bash "$SCRIPT_DIR/install-macos.sh" "$@"
        else
            curl -sSf "$REPO_URL/install-macos.sh" | bash -s -- "$@"
        fi
        ;;
    MINGW*|MSYS*|CYGWIN*)
        echo "Detected Windows (Git Bash/MSYS) — please use PowerShell instead:"
        echo ""
        echo "  irm $REPO_URL/install-windows.ps1 | iex"
        echo ""
        echo "Or download and run: deploy/install-windows.ps1"
        exit 1
        ;;
    *)
        echo "Unsupported OS: $(uname -s)"
        echo "Delphi supports Linux, macOS, and Windows."
        exit 1
        ;;
esac
