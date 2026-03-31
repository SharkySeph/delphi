#Requires -Version 5.1
<#
.SYNOPSIS
    Delphi — Windows Installer
    Installs Delphi from a pre-built wheel. No Rust required.

.DESCRIPTION
    Checks for Python 3.10+, creates a virtual environment, and
    installs the pre-built Delphi wheel (compiled Rust engine
    included). No compiler or build tools needed.

.PARAMETER DelphiVenv
    Path to virtual environment (default: $env:LOCALAPPDATA\delphi\venv)

.PARAMETER NoVenv
    Skip virtual environment — install into current Python environment

.PARAMETER Version
    Install a specific version (default: latest)

.EXAMPLE
    .\deploy\install-windows.ps1
    irm https://raw.githubusercontent.com/SharkySeph/delphi/main/deploy/install-windows.ps1 | iex
#>

[CmdletBinding()]
param(
    [string]$DelphiVenv = "",
    [switch]$NoVenv,
    [string]$Version = ""
)

$ErrorActionPreference = "Stop"

$DelphiPrefix = "$env:LOCALAPPDATA\delphi"
if (-not $DelphiVenv) { $DelphiVenv = Join-Path $DelphiPrefix "venv" }
$MinPython = [version]"3.10"
$GithubRepo = "SharkySeph/delphi"

# ── Helpers ──────────────────────────────────────────────────
function Write-Info    { param([string]$Msg) Write-Host "  i  $Msg" -ForegroundColor Cyan }
function Write-Ok      { param([string]$Msg) Write-Host "  +  $Msg" -ForegroundColor Green }
function Write-Warn    { param([string]$Msg) Write-Host "  !  $Msg" -ForegroundColor Yellow }
function Write-Err     { param([string]$Msg) Write-Host "  x  $Msg" -ForegroundColor Red }
function Write-Header  { param([string]$Msg) Write-Host "`n-- $Msg --" -ForegroundColor White }

function Test-Command { param([string]$Name) $null -ne (Get-Command $Name -ErrorAction SilentlyContinue) }

function Refresh-Path {
    $env:Path = [System.Environment]::GetEnvironmentVariable("Path", "Machine") +
                ";" + [System.Environment]::GetEnvironmentVariable("Path", "User")
}

function Get-PackageManager {
    if (Test-Command "winget") { return "winget" }
    if (Test-Command "scoop")  { return "scoop" }
    if (Test-Command "choco")  { return "choco" }
    return $null
}

# ── Start ────────────────────────────────────────────────────
Write-Host "`nDelphi Installer for Windows" -ForegroundColor White
Write-Host "===========================`n" -ForegroundColor DarkGray
Write-Info "Platform: Windows $([System.Environment]::OSVersion.Version) ($env:PROCESSOR_ARCHITECTURE)"

# ════════════════════════════════════════════════════════════
Write-Header "Checking prerequisites"
# ════════════════════════════════════════════════════════════

# 1. Windows audio — WASAPI is always there
Write-Ok "Windows audio (WASAPI) available"

# 2. Python 3.10+
$PythonCmd = $null
$PythonVer = $null

foreach ($candidate in @("python", "python3", "py")) {
    if (Test-Command $candidate) {
        try {
            if ($candidate -eq "py") {
                $verStr = & py -3 -c "import sys; print(f'{sys.version_info.major}.{sys.version_info.minor}')" 2>$null
            } else {
                $verStr = & $candidate -c "import sys; print(f'{sys.version_info.major}.{sys.version_info.minor}')" 2>$null
            }
            if ($verStr) {
                $ver = [version]$verStr
                if ($ver -ge $MinPython) {
                    $PythonCmd = $candidate
                    $PythonVer = $verStr
                    break
                }
            }
        } catch { continue }
    }
}

if ($PythonCmd) {
    Write-Ok "Python $PythonVer ($PythonCmd)"
} else {
    Write-Warn "Python >= $MinPython not found"

    $pm = Get-PackageManager
    $installed = $false
    if ($pm -eq "winget") {
        Write-Info "Installing Python 3.12 via winget..."
        winget install --id Python.Python.3.12 --accept-source-agreements --accept-package-agreements -e
        $installed = $true
    } elseif ($pm -eq "scoop") {
        Write-Info "Installing Python via scoop..."
        scoop install python
        $installed = $true
    } elseif ($pm -eq "choco") {
        Write-Info "Installing Python 3.12 via chocolatey..."
        choco install python312 -y
        $installed = $true
    }

    if ($installed) {
        Refresh-Path
        $PythonCmd = "python"
        $PythonVer = & python -c "import sys; print(f'{sys.version_info.major}.{sys.version_info.minor}')" 2>$null
        Write-Ok "Python $PythonVer installed"
    } else {
        Write-Err "Python >= $MinPython is required"
        Write-Info "Download from: https://www.python.org/downloads/"
        Write-Info "  IMPORTANT: Check 'Add Python to PATH' during installation"
        exit 1
    }
}

# 3. pip
try {
    & $PythonCmd -m pip --version | Out-Null
    Write-Ok "pip available"
} catch {
    Write-Warn "pip not found - will bootstrap"
}

# ════════════════════════════════════════════════════════════
Write-Header "Setting up Python environment"
# ════════════════════════════════════════════════════════════
if (-not $NoVenv) {
    if (-not (Test-Path $DelphiVenv)) {
        Write-Info "Creating virtual environment -> $DelphiVenv"
        New-Item -ItemType Directory -Path (Split-Path $DelphiVenv -Parent) -Force | Out-Null
        if ($PythonCmd -eq "py") {
            & py -3 -m venv $DelphiVenv
        } else {
            & $PythonCmd -m venv $DelphiVenv
        }
    }

    $activateScript = Join-Path $DelphiVenv "Scripts\Activate.ps1"
    if (Test-Path $activateScript) {
        & $activateScript
        Write-Ok "Activated venv: $DelphiVenv"
    } else {
        Write-Err "Venv activation script not found at $activateScript"
        exit 1
    }
}

python -m ensurepip --upgrade 2>$null
python -m pip install --quiet --upgrade pip

# ════════════════════════════════════════════════════════════
Write-Header "Installing Delphi"
# ════════════════════════════════════════════════════════════
$versionSpec = ""
if ($Version) { $versionSpec = "==$Version" }

# Try PyPI first
$pypiOk = $false
try {
    pip install --quiet "delphi$versionSpec" 2>$null
    $pypiOk = $true
    Write-Ok "Installed from PyPI"
} catch { }

if (-not $pypiOk) {
    Write-Info "Not on PyPI - trying GitHub Releases..."

    if (-not $Version) {
        try {
            $release = Invoke-RestMethod -Uri "https://api.github.com/repos/$GithubRepo/releases/latest" -UseBasicParsing
            $Version = $release.tag_name -replace '^v', ''
            Write-Info "Latest release: v$Version"
        } catch {
            Write-Err "Could not determine latest release."
            Write-Info "Specify a version: .\install-windows.ps1 -Version 0.6.0"
            exit 1
        }
    }

    # Find a matching Windows wheel
    try {
        $release = Invoke-RestMethod -Uri "https://api.github.com/repos/$GithubRepo/releases/tags/v$Version" -UseBasicParsing
        $wheelAsset = $release.assets | Where-Object { $_.name -like "*.whl" -and $_.name -like "*win*" } | Select-Object -First 1

        if ($wheelAsset) {
            Write-Info "Downloading wheel..."
            pip install --quiet $wheelAsset.browser_download_url
            Write-Ok "Installed from GitHub Releases"
        } else {
            Write-Err "No pre-built wheel for Windows / Python $PythonVer"
            Write-Info "Open an issue: https://github.com/$GithubRepo/issues"
            exit 1
        }
    } catch {
        Write-Err "Failed to fetch release: $_"
        exit 1
    }
}

# ── Data directories ─────────────────────────────────────────
New-Item -ItemType Directory -Path (Join-Path $DelphiPrefix "projects") -Force | Out-Null
New-Item -ItemType Directory -Path (Join-Path $env:USERPROFILE ".delphi\soundfonts") -Force | Out-Null

# ════════════════════════════════════════════════════════════
Write-Header "Installing launcher to PATH"
# ════════════════════════════════════════════════════════════
$launcherDir = Join-Path $DelphiPrefix "bin"
New-Item -ItemType Directory -Path $launcherDir -Force | Out-Null

# Create a .cmd launcher (works from cmd.exe and PowerShell)
$launcherCmd = Join-Path $launcherDir "delphi.cmd"
$venvDelphi = Join-Path $DelphiVenv "Scripts\delphi.exe"
@"
@echo off
REM Delphi launcher - installed by deploy/install-windows.ps1
"$venvDelphi" %*
"@ | Set-Content -Path $launcherCmd -Encoding ASCII
Write-Ok "Installed $launcherCmd"

# Add launcher dir to user PATH permanently
$userPath = [System.Environment]::GetEnvironmentVariable("Path", "User")
if ($userPath -notlike "*$launcherDir*") {
    [System.Environment]::SetEnvironmentVariable("Path", "$userPath;$launcherDir", "User")
    $env:Path = "$env:Path;$launcherDir"
    Write-Ok "Added $launcherDir to user PATH"
} else {
    Write-Ok "$launcherDir already on PATH"
}

# ════════════════════════════════════════════════════════════
Write-Header "Verifying installation"
# ════════════════════════════════════════════════════════════
try {
    $verOutput = & $launcherCmd --version 2>$null
    Write-Ok "$verOutput"
} catch {
    Write-Warn "Launcher created but --version check failed. Try: delphi --help"
}

# ── Done ─────────────────────────────────────────────────────
Write-Header "Installation complete!"
Write-Host ""
Write-Info "Command:    $launcherCmd"
if (-not $NoVenv) { Write-Info "Venv:       $DelphiVenv" }
Write-Info "SoundFonts: $env:USERPROFILE\.delphi\soundfonts\"
Write-Info "Projects:   $DelphiPrefix\projects\"
Write-Host ""
Write-Ok "Run 'delphi' to start composing!"
