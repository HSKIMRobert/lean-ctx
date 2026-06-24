<#
install.ps1 - Build lean-ctx locally on Windows and install it into Cargo's bin directory.

Usage:
    .\install.ps1
    .\install.ps1 -BuildOnly
#>

param(
    [switch]$BuildOnly,
    [switch]$Help
)

$ErrorActionPreference = 'Stop'

if ($Help) {
    Write-Host 'Usage: .\install.ps1 [-BuildOnly] [-Help]'
    Write-Host ''
    Write-Host '  (no args)    Build lean-ctx locally and install it into Cargo''s bin directory'
    Write-Host '  -BuildOnly   Build only, do not install'
    Write-Host '  -Help        Show this help message'
    exit 0
}

function Get-CargoBinDir {
    if ($env:CARGO_HOME) {
        return Join-Path $env:CARGO_HOME 'bin'
    }

    return Join-Path $HOME '.cargo\bin'
}

function Stop-RunningLeanCtx {
    $existing = Get-Command lean-ctx -ErrorAction SilentlyContinue
    if ($null -ne $existing) {
        Write-Host 'Stopping running lean-ctx (if any)...'
        try {
            & $existing.Source stop | Out-Null
        }
        catch {
        }
    }
}

$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$rustDir = Join-Path $scriptDir 'rust'

if (-not (Test-Path $rustDir -PathType Container)) {
    throw "Rust project not found at $rustDir"
}

if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    throw 'cargo not found. Install Rust from https://rustup.rs/'
}

$cargoBinDir = Get-CargoBinDir
$builtBinary = Join-Path $rustDir 'target\release\lean-ctx.exe'
$installedBinary = Join-Path $cargoBinDir 'lean-ctx.exe'

Write-Host 'lean-ctx Windows installer'
Write-Host '━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━'
Write-Host 'Mode: build from source'
Write-Host ''
Write-Host 'Building lean-ctx (release)...'

Push-Location $rustDir
try {
    & cargo build --release
}
finally {
    Pop-Location
}

if (-not (Test-Path $builtBinary -PathType Leaf)) {
    throw "Build failed - binary not found at $builtBinary"
}

Write-Host "Built: $builtBinary"

if ($BuildOnly) {
    Write-Host 'Done (build only).'
    exit 0
}

New-Item -ItemType Directory -Path $cargoBinDir -Force | Out-Null

$tempBinary = Join-Path $cargoBinDir ('.lean-ctx.new.' + $PID + '.exe')
Copy-Item -Path $builtBinary -Destination $tempBinary -Force
Stop-RunningLeanCtx
Move-Item -Path $tempBinary -Destination $installedBinary -Force

Write-Host "Installed: $installedBinary"

$pathEntries = @($env:Path -split ';' | Where-Object { $_ })
if ($pathEntries -notcontains $cargoBinDir) {
    Write-Host ''
    Write-Warning "$cargoBinDir is not in your PATH."
    Write-Host 'Add it to your user PATH, then restart your shell.'
}

Write-Host ''
Write-Host 'Done! Verify with: lean-ctx --version'