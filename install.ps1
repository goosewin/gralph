# Gralph installer for Windows
# Run: irm https://raw.githubusercontent.com/goosewin/gralph/main/install.ps1 | iex
# Or:  .\install.ps1 [-Version "0.1.0"] [-InstallDir "C:\Program Files\gralph"]

param(
    [string]$Version = "latest",
    [string]$InstallDir = "$env:LOCALAPPDATA\gralph\bin"
)

$ErrorActionPreference = "Stop"
$Repo = "goosewin/gralph"

function Write-Info { param($Message) Write-Host "[INFO] $Message" -ForegroundColor Green }
function Write-Warn { param($Message) Write-Host "[WARN] $Message" -ForegroundColor Yellow }
function Write-Err { param($Message) Write-Host "[ERROR] $Message" -ForegroundColor Red; exit 1 }

function Get-LatestVersion {
    $response = Invoke-RestMethod -Uri "https://api.github.com/repos/$Repo/releases/latest"
    return $response.tag_name -replace '^v', ''
}

function Add-ToPath {
    param([string]$Dir)
    
    $currentPath = [Environment]::GetEnvironmentVariable("Path", "User")
    if ($currentPath -notlike "*$Dir*") {
        [Environment]::SetEnvironmentVariable("Path", "$currentPath;$Dir", "User")
        $env:Path = "$env:Path;$Dir"
        Write-Info "Added $Dir to PATH"
    }
}

function Main {
    Write-Host ""
    Write-Info "Gralph Installer for Windows"
    Write-Host ""

    # Get version
    if ($Version -eq "latest") {
        Write-Info "Fetching latest version..."
        $Version = Get-LatestVersion
    }
    Write-Info "Installing version: $Version"

    # Platform is always windows-x86_64 for now
    $Platform = "windows-x86_64"
    $Url = "https://github.com/$Repo/releases/download/v$Version/gralph-$Version-$Platform.zip"

    # Create temp directory
    $TempDir = Join-Path $env:TEMP "gralph-install-$(Get-Random)"
    New-Item -ItemType Directory -Force -Path $TempDir | Out-Null

    try {
        # Download
        $ZipPath = Join-Path $TempDir "gralph.zip"
        Write-Info "Downloading from $Url..."
        try {
            Invoke-WebRequest -Uri $Url -OutFile $ZipPath -UseBasicParsing
        } catch {
            Write-Err "Download failed. Check if version $Version exists for platform $Platform"
        }

        # Extract
        Write-Info "Extracting..."
        Expand-Archive -Path $ZipPath -DestinationPath $TempDir -Force

        # Find binary
        $BinaryPath = Join-Path $TempDir "gralph-$Version" "gralph.exe"
        if (-not (Test-Path $BinaryPath)) {
            Write-Err "Binary not found in archive"
        }

        # Create install directory
        if (-not (Test-Path $InstallDir)) {
            New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
        }

        # Install
        Write-Info "Installing to $InstallDir..."
        Copy-Item -Path $BinaryPath -Destination (Join-Path $InstallDir "gralph.exe") -Force

        # Add to PATH
        Add-ToPath -Dir $InstallDir

        # Verify
        Write-Host ""
        $InstalledPath = Join-Path $InstallDir "gralph.exe"
        if (Test-Path $InstalledPath) {
            Write-Info "Successfully installed gralph $Version"
            Write-Info "Run 'gralph --help' to get started"
            Write-Host ""
            Write-Warn "Restart your terminal for PATH changes to take effect"
        }
    } finally {
        # Cleanup
        Remove-Item -Path $TempDir -Recurse -Force -ErrorAction SilentlyContinue
    }
}

Main
