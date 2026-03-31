#Requires -Version 5.1
<#
.SYNOPSIS
    Delphi - Windows Installer. Creates a venv, installs the pre-built wheel, adds to PATH.
.EXAMPLE
    .\deploy\install-windows.ps1
    irm https://raw.githubusercontent.com/SharkySeph/delphi/main/deploy/install-windows.ps1 | iex
#>
$ErrorActionPreference = "Stop"

$DelphiVenv   = if ($env:DELPHI_VENV) { $env:DELPHI_VENV } else { Join-Path $env:LOCALAPPDATA "delphi\venv" }
$Version      = $env:DELPHI_VERSION
$GithubRepo   = "SharkySeph/delphi"
$LauncherDir  = Join-Path $env:LOCALAPPDATA "delphi\bin"

# ── Find Python 3.10+ ───────────────────────────────────────
$PythonCmd = $null
foreach ($py in @("python3", "python", "py")) {
    try {
        $ver = & $py -c "import sys; print(f'{sys.version_info.major}.{sys.version_info.minor}')" 2>$null
        if ($ver) {
            $parts = $ver.Split(".")
            if ([int]$parts[0] -gt 3 -or ([int]$parts[0] -eq 3 -and [int]$parts[1] -ge 10)) {
                $PythonCmd = $py; break
            }
        }
    } catch {}
}
if (-not $PythonCmd) {
    Write-Host "Error: Python 3.10+ is required." -ForegroundColor Red
    Write-Host "  Download: https://www.python.org/downloads/"
    Write-Host "  Or: winget install Python.Python.3.12"
    exit 1
}
Write-Host "[*] Using $PythonCmd ($(& $PythonCmd --version))"

# ── Create venv ──────────────────────────────────────────────
if (-not (Test-Path $DelphiVenv)) {
    Write-Host "[*] Creating venv -> $DelphiVenv"
    & $PythonCmd -m venv $DelphiVenv
}
$pip = Join-Path $DelphiVenv "Scripts\pip.exe"
$delphiBin = Join-Path $DelphiVenv "Scripts\delphi.exe"

# ── Install Delphi from GitHub Releases ──────────────────────
# Note: "delphi" on PyPI is a different package — always use GitHub Releases
Write-Host "[*] Installing Delphi..."
if (-not $Version) {
    try {
        $rel = Invoke-RestMethod "https://api.github.com/repos/$GithubRepo/releases/latest"
        $Version = $rel.tag_name.TrimStart("v")
    } catch {
        Write-Host "Error: Could not find a release. Set DELPHI_VERSION=0.6.0" -ForegroundColor Red; exit 1
    }
}
$wheelUrl = $null
try {
    $rel = Invoke-RestMethod "https://api.github.com/repos/$GithubRepo/releases/tags/v$Version"
    foreach ($asset in $rel.assets) {
        if ($asset.name -match '\.whl$' -and $asset.name -match 'win') {
            $wheelUrl = $asset.browser_download_url; break
        }
    }
} catch {}

if ($wheelUrl) {
    & $pip install --quiet $wheelUrl
    Write-Host "[+] Installed Delphi $Version" -ForegroundColor Green
} else {
    Write-Host "Error: No wheel found for Windows." -ForegroundColor Red
    Write-Host "  https://github.com/$GithubRepo/releases"
    exit 1
}

# ── Create launcher on PATH ─────────────────────────────────
New-Item -Path $LauncherDir -ItemType Directory -Force | Out-Null
$launcher = Join-Path $LauncherDir "delphi.cmd"
Set-Content $launcher "@echo off`r`n`"$delphiBin`" %*"

$userPath = [System.Environment]::GetEnvironmentVariable("Path", "User")
if ($userPath -notlike "*$LauncherDir*") {
    [System.Environment]::SetEnvironmentVariable("Path", "$LauncherDir;$userPath", "User")
    $env:Path = "$LauncherDir;$env:Path"
    Write-Host "[*] Added $LauncherDir to PATH — restart your terminal to use 'delphi'"
}

# ── Create data dirs ────────────────────────────────────────
$dataDir = Join-Path $env:LOCALAPPDATA "delphi"
New-Item -Path "$dataDir\projects" -ItemType Directory -Force | Out-Null
$sfDir = Join-Path $env:USERPROFILE ".delphi\soundfonts"
New-Item -Path $sfDir -ItemType Directory -Force | Out-Null

# ── Done ─────────────────────────────────────────────────────
Write-Host ""
Write-Host "[+] Delphi installed! Run 'delphi' to start." -ForegroundColor Green
Write-Host "    Venv:       $DelphiVenv"
Write-Host "    SoundFonts: $sfDir"
Write-Host "    Projects:   $dataDir\projects"
