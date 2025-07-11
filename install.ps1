# Arrowhead Installation Script for Windows
# Usage: iwr https://install.arrowhead.dev/windows | iex

param(
    [string]$InstallDir = "$env:USERPROFILE\.arrowhead\bin",
    [string]$Version = "latest"
)

# Set error action preference
$ErrorActionPreference = "Stop"

# Constants
$GITHUB_REPO = "Jai-Dhiman/arrowhead"
$BINARY_NAME = "arrowhead.exe"

# Helper functions
function Write-ColorOutput {
    param(
        [string]$Message,
        [string]$Color = "White"
    )
    Write-Host $Message -ForegroundColor $Color
}

function Write-Info {
    param([string]$Message)
    Write-ColorOutput "[INFO] $Message" "Blue"
}

function Write-Success {
    param([string]$Message)
    Write-ColorOutput "[SUCCESS] $Message" "Green"
}

function Write-Warn {
    param([string]$Message)
    Write-ColorOutput "[WARN] $Message" "Yellow"
}

function Write-Error {
    param([string]$Message)
    Write-ColorOutput "[ERROR] $Message" "Red"
}

# Get the latest release version from GitHub
function Get-LatestVersion {
    try {
        $url = "https://api.github.com/repos/$GITHUB_REPO/releases/latest"
        $response = Invoke-RestMethod -Uri $url -Method Get
        return $response.tag_name
    }
    catch {
        Write-Error "Could not determine latest version: $_"
        exit 1
    }
}

# Download and extract the binary
function Download-Binary {
    param(
        [string]$Version,
        [string]$TempDir
    )
    
    $filename = "arrowhead-Windows-x86_64.zip"
    $url = "https://github.com/$GITHUB_REPO/releases/download/$Version/$filename"
    $archivePath = Join-Path $TempDir $filename
    
    Write-Info "Downloading Arrowhead $Version for Windows..."
    
    try {
        Invoke-WebRequest -Uri $url -OutFile $archivePath -UseBasicParsing
    }
    catch {
        Write-Error "Download failed: $_"
        exit 1
    }
    
    if (-not (Test-Path $archivePath)) {
        Write-Error "Download failed: archive not found"
        exit 1
    }
    
    Write-Info "Extracting archive..."
    
    try {
        Expand-Archive -Path $archivePath -DestinationPath $TempDir -Force
    }
    catch {
        Write-Error "Failed to extract archive: $_"
        exit 1
    }
    
    $binaryPath = Join-Path $TempDir $BINARY_NAME
    
    if (-not (Test-Path $binaryPath)) {
        Write-Error "Binary not found in archive"
        exit 1
    }
    
    return $binaryPath
}

# Install the binary
function Install-Binary {
    param(
        [string]$BinaryPath,
        [string]$InstallDir
    )
    
    Write-Info "Installing Arrowhead to $InstallDir..."
    
    # Create install directory if it doesn't exist
    if (-not (Test-Path $InstallDir)) {
        try {
            New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
        }
        catch {
            Write-Error "Failed to create install directory: $_"
            exit 1
        }
    }
    
    $installPath = Join-Path $InstallDir $BINARY_NAME
    
    try {
        Copy-Item -Path $BinaryPath -Destination $installPath -Force
    }
    catch {
        Write-Error "Installation failed: $_"
        exit 1
    }
    
    if (-not (Test-Path $installPath)) {
        Write-Error "Installation failed: binary not found at destination"
        exit 1
    }
    
    return $installPath
}

# Add directory to PATH
function Add-ToPath {
    param([string]$Directory)
    
    $currentPath = [Environment]::GetEnvironmentVariable("Path", "User")
    
    if ($currentPath -notlike "*$Directory*") {
        Write-Info "Adding $Directory to PATH..."
        
        try {
            $newPath = "$currentPath;$Directory"
            [Environment]::SetEnvironmentVariable("Path", $newPath, "User")
            $env:Path = "$env:Path;$Directory"
            return $true
        }
        catch {
            Write-Error "Failed to add to PATH: $_"
            return $false
        }
    }
    else {
        Write-Info "$Directory is already in PATH"
        return $false
    }
}

# Test if binary is accessible
function Test-Installation {
    param([string]$InstallPath)
    
    try {
        $output = & $InstallPath --version 2>&1
        if ($LASTEXITCODE -eq 0) {
            Write-Success "Installation verified successfully"
            return $true
        }
        else {
            Write-Warn "Binary installed but may not be working correctly"
            return $false
        }
    }
    catch {
        Write-Warn "Could not verify installation: $_"
        return $false
    }
}

# Setup configuration
function Initialize-Config {
    Write-Info "Setting up configuration..."
    
    $configDir = "$env:USERPROFILE\.arrowhead"
    if (-not (Test-Path $configDir)) {
        try {
            New-Item -ItemType Directory -Path $configDir -Force | Out-Null
        }
        catch {
            Write-Warn "Could not create config directory: $_"
        }
    }
}

# Main installation function
function Main {
    Write-Info "🚀 Installing Arrowhead AI Assistant..."
    
    # Check PowerShell version
    if ($PSVersionTable.PSVersion.Major -lt 5) {
        Write-Error "PowerShell 5.0 or higher is required"
        exit 1
    }
    
    # Get version
    $targetVersion = $Version
    if ($targetVersion -eq "latest") {
        $targetVersion = Get-LatestVersion
    }
    
    Write-Info "Installing version: $targetVersion"
    
    # Create temporary directory
    $tempDir = Join-Path $env:TEMP "arrowhead-install-$(Get-Random)"
    try {
        New-Item -ItemType Directory -Path $tempDir -Force | Out-Null
    }
    catch {
        Write-Error "Failed to create temporary directory: $_"
        exit 1
    }
    
    try {
        # Download binary
        $binaryPath = Download-Binary -Version $targetVersion -TempDir $tempDir
        
        # Install binary
        $installPath = Install-Binary -BinaryPath $binaryPath -InstallDir $InstallDir
        
        # Add to PATH
        $pathAdded = Add-ToPath -Directory $InstallDir
        
        # Initialize configuration
        Initialize-Config
        
        # Test installation
        Test-Installation -InstallPath $installPath | Out-Null
        
        Write-Success "Arrowhead has been successfully installed!"
        
        if ($pathAdded) {
            Write-Warn "⚠️  PATH has been updated. Please restart your terminal or run:"
            Write-Host "    `$env:Path = [System.Environment]::GetEnvironmentVariable('Path', 'User')" -ForegroundColor Yellow
        }
        
        Write-Host ""
        Write-Host "🎉 Installation complete!" -ForegroundColor Green
        Write-Host ""
        Write-Host "To get started:" -ForegroundColor White
        Write-Host "  arrowhead --help     # Show help" -ForegroundColor Cyan
        Write-Host "  arrowhead config     # Configure API keys" -ForegroundColor Cyan
        Write-Host "  arrowhead            # Start interactive mode" -ForegroundColor Cyan
        Write-Host ""
        Write-Host "For more information, visit: https://github.com/$GITHUB_REPO" -ForegroundColor White
    }
    catch {
        Write-Error "Installation failed: $_"
        exit 1
    }
    finally {
        # Cleanup
        if (Test-Path $tempDir) {
            try {
                Remove-Item -Path $tempDir -Recurse -Force
            }
            catch {
                Write-Warn "Could not clean up temporary directory: $tempDir"
            }
        }
    }
}

# Run main function
Main