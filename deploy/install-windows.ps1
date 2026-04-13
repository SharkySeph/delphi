#Requires -Version 5.1
<#
.SYNOPSIS
    Delphi - Windows Installer. Downloads and installs native binaries.
.EXAMPLE
    .\deploy\install-windows.ps1
    irm https://raw.githubusercontent.com/SharkySeph/delphi/main/deploy/install-windows.ps1 | iex
#>
$ErrorActionPreference = "Stop"

$Version      = $env:DELPHI_VERSION
$GithubRepo   = "SharkySeph/delphi"
$InstallDir   = Join-Path $env:LOCALAPPDATA "delphi\bin"

# ── Resolve latest version ───────────────────────────────────
if (-not $Version) {
    try {
        $rel = Invoke-RestMethod "https://api.github.com/repos/$GithubRepo/releases/latest"
        $Version = $rel.tag_name.TrimStart("v")
    } catch {
        Write-Host "Error: Could not find a release. Set DELPHI_VERSION=0.8.0" -ForegroundColor Red; exit 1
    }
}
Write-Host "[*] Installing Delphi v$Version"

# ── Download Windows zip ─────────────────────────────────────
$zipUrl = $null
try {
    $rel = Invoke-RestMethod "https://api.github.com/repos/$GithubRepo/releases/tags/v$Version"
    foreach ($asset in $rel.assets) {
        if ($asset.name -match 'windows' -and $asset.name -match '\.zip$') {
            $zipUrl = $asset.browser_download_url; break
        }
    }
} catch {}

if (-not $zipUrl) {
    Write-Host "Error: No Windows package found for v$Version." -ForegroundColor Red
    Write-Host "  https://github.com/$GithubRepo/releases"
    exit 1
}

Write-Host "[*] Downloading..."
$tmpZip = Join-Path $env:TEMP "delphi.zip"
Invoke-WebRequest -Uri $zipUrl -OutFile $tmpZip
New-Item -Path $InstallDir -ItemType Directory -Force | Out-Null
Expand-Archive -Path $tmpZip -DestinationPath $InstallDir -Force
Remove-Item $tmpZip -Force
Write-Host "[+] Installed delphi and delphi-studio to $InstallDir" -ForegroundColor Green

# ── Add to PATH ──────────────────────────────────────────────
$userPath = [System.Environment]::GetEnvironmentVariable("Path", "User")
if ($userPath -notlike "*$InstallDir*") {
    [System.Environment]::SetEnvironmentVariable("Path", "$InstallDir;$userPath", "User")
    $env:Path = "$InstallDir;$env:Path"
    Write-Host "[*] Added $InstallDir to PATH — restart your terminal to use 'delphi'"
}

# ── Create data dirs ────────────────────────────────────────
$dataDir = Join-Path $env:LOCALAPPDATA "delphi"
New-Item -Path "$dataDir\projects" -ItemType Directory -Force | Out-Null
$sfDir = Join-Path $env:USERPROFILE ".delphi\soundfonts"
New-Item -Path $sfDir -ItemType Directory -Force | Out-Null

# ── Done ─────────────────────────────────────────────────────
Write-Host ""
Write-Host "[+] Delphi installed!" -ForegroundColor Green
Write-Host "    delphi          — CLI (export, play, info, new)"
Write-Host "    delphi-studio   — GUI (editor, piano roll, mixer)"
Write-Host "    SoundFonts: $sfDir"
Write-Host "    Projects:   $dataDir\projects"
