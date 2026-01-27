# Gralph uninstaller for Windows
# Run: .\uninstall.ps1 [-InstallDir "C:\path\to\gralph"]

param(
    [string]$InstallDir = "$env:LOCALAPPDATA\gralph\bin"
)

$ErrorActionPreference = "Stop"

function Write-Info { param($Message) Write-Host "[INFO] $Message" -ForegroundColor Green }
function Write-Warn { param($Message) Write-Host "[WARN] $Message" -ForegroundColor Yellow }
function Write-Err { param($Message) Write-Host "[ERROR] $Message" -ForegroundColor Red; exit 1 }

function Remove-FromPath {
    param([string]$Dir)
    
    $currentPath = [Environment]::GetEnvironmentVariable("Path", "User")
    if ($currentPath -like "*$Dir*") {
        $newPath = ($currentPath -split ';' | Where-Object { $_ -ne $Dir }) -join ';'
        [Environment]::SetEnvironmentVariable("Path", $newPath, "User")
        Write-Info "Removed $Dir from PATH"
    }
}

function Main {
    Write-Host ""
    Write-Info "Gralph Uninstaller for Windows"
    Write-Host ""

    $BinaryPath = Join-Path $InstallDir "gralph.exe"

    # Try to find the binary
    if (-not (Test-Path $BinaryPath)) {
        # Check if it's in PATH
        $InPath = Get-Command gralph -ErrorAction SilentlyContinue
        if ($InPath) {
            $BinaryPath = $InPath.Source
            $InstallDir = Split-Path $BinaryPath -Parent
        } else {
            Write-Err "gralph not found in $InstallDir or PATH"
        }
    }

    Write-Info "Found gralph at: $BinaryPath"

    # Remove binary
    Remove-Item -Path $BinaryPath -Force
    Write-Info "Removed gralph.exe"

    # Remove from PATH
    Remove-FromPath -Dir $InstallDir

    # Remove install directory if empty
    if ((Test-Path $InstallDir) -and ((Get-ChildItem $InstallDir | Measure-Object).Count -eq 0)) {
        Remove-Item -Path $InstallDir -Force
        Write-Info "Removed empty install directory"
    }

    # Check for config directory
    $ConfigDir = Join-Path $env:APPDATA "gralph"
    if (Test-Path $ConfigDir) {
        Write-Warn "Config directory exists at $ConfigDir"
        $response = Read-Host "Remove config directory? [y/N]"
        if ($response -match '^[Yy]$') {
            Remove-Item -Path $ConfigDir -Recurse -Force
            Write-Info "Removed config directory"
        } else {
            Write-Info "Config directory preserved"
        }
    }

    Write-Host ""
    Write-Info "Successfully uninstalled gralph"
}

Main
