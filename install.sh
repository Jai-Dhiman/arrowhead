#!/bin/bash

# Arrowhead Installation Script
# Usage: curl -fsSL https://install.arrowhead.dev | sh

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Constants
GITHUB_REPO="Jai-Dhiman/arrowhead"
BINARY_NAME="arrowhead"
INSTALL_DIR="/usr/local/bin"

# Helper functions
print_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Detect OS and architecture
detect_platform() {
    local os arch
    
    # Detect OS
    case "$(uname -s)" in
        Linux*)  os="Linux" ;;
        Darwin*) os="Darwin" ;;
        *)
            print_error "Unsupported operating system: $(uname -s)"
            exit 1
            ;;
    esac
    
    # Detect architecture
    case "$(uname -m)" in
        x86_64)  arch="x86_64" ;;
        arm64)   arch="arm64" ;;
        aarch64) arch="arm64" ;;
        *)
            print_error "Unsupported architecture: $(uname -m)"
            exit 1
            ;;
    esac
    
    echo "${os}-${arch}"
}

# Get the latest release version from GitHub
get_latest_version() {
    local url="https://api.github.com/repos/${GITHUB_REPO}/releases/latest"
    local version
    
    if command -v curl >/dev/null 2>&1; then
        version=$(curl -s "$url" | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/')
    elif command -v wget >/dev/null 2>&1; then
        version=$(wget -qO- "$url" | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/')
    else
        print_error "Neither curl nor wget is available"
        exit 1
    fi
    
    if [ -z "$version" ]; then
        print_error "Could not determine latest version"
        exit 1
    fi
    
    echo "$version"
}

# Download and verify the binary
download_binary() {
    local version="$1"
    local platform="$2"
    local filename="${BINARY_NAME}-${platform}.tar.gz"
    local url="https://github.com/${GITHUB_REPO}/releases/download/${version}/${filename}"
    local tmp_dir="/tmp/arrowhead-install"
    
    print_info "Downloading Arrowhead ${version} for ${platform}..."
    
    # Create temporary directory
    mkdir -p "$tmp_dir"
    cd "$tmp_dir"
    
    # Download the archive
    if command -v curl >/dev/null 2>&1; then
        curl -fsSL "$url" -o "$filename"
    elif command -v wget >/dev/null 2>&1; then
        wget -q "$url" -O "$filename"
    else
        print_error "Neither curl nor wget is available"
        exit 1
    fi
    
    # Verify download
    if [ ! -f "$filename" ]; then
        print_error "Download failed"
        exit 1
    fi
    
    # Extract the archive
    print_info "Extracting archive..."
    tar -xzf "$filename"
    
    # Verify binary exists
    if [ ! -f "$BINARY_NAME" ]; then
        print_error "Binary not found in archive"
        exit 1
    fi
    
    # Make binary executable
    chmod +x "$BINARY_NAME"
    
    echo "$tmp_dir/$BINARY_NAME"
}

# Install the binary
install_binary() {
    local binary_path="$1"
    local install_path="$INSTALL_DIR/$BINARY_NAME"
    
    print_info "Installing Arrowhead to $install_path..."
    
    # Check if we need sudo
    if [ ! -w "$INSTALL_DIR" ]; then
        if command -v sudo >/dev/null 2>&1; then
            sudo cp "$binary_path" "$install_path"
        else
            print_error "No write permission to $INSTALL_DIR and sudo is not available"
            print_info "Please run as root or install manually"
            exit 1
        fi
    else
        cp "$binary_path" "$install_path"
    fi
    
    # Verify installation
    if [ ! -f "$install_path" ]; then
        print_error "Installation failed"
        exit 1
    fi
    
    # Make sure it's executable
    if [ ! -w "$INSTALL_DIR" ]; then
        sudo chmod +x "$install_path"
    else
        chmod +x "$install_path"
    fi
}

# Cleanup temporary files
cleanup() {
    if [ -d "/tmp/arrowhead-install" ]; then
        rm -rf "/tmp/arrowhead-install"
    fi
}

# Check if PATH includes install directory
check_path() {
    if echo "$PATH" | grep -q "$INSTALL_DIR"; then
        return 0
    else
        return 1
    fi
}

# Main installation function
main() {
    print_info "🚀 Installing Arrowhead AI Assistant..."
    
    # Check dependencies
    if ! command -v curl >/dev/null 2>&1 && ! command -v wget >/dev/null 2>&1; then
        print_error "Either curl or wget is required for installation"
        exit 1
    fi
    
    if ! command -v tar >/dev/null 2>&1; then
        print_error "tar is required for installation"
        exit 1
    fi
    
    # Detect platform
    local platform
    platform=$(detect_platform)
    print_info "Detected platform: $platform"
    
    # Get latest version
    local version
    version=$(get_latest_version)
    print_info "Latest version: $version"
    
    # Download binary
    local binary_path
    binary_path=$(download_binary "$version" "$platform")
    
    # Install binary
    install_binary "$binary_path"
    
    # Cleanup
    cleanup
    
    print_success "Arrowhead has been successfully installed!"
    
    # Check PATH
    if ! check_path; then
        print_warn "⚠️  $INSTALL_DIR is not in your PATH"
        print_info "Add the following to your shell profile (~/.bashrc, ~/.zshrc, etc.):"
        echo "export PATH=\"$INSTALL_DIR:\$PATH\""
    fi
    
    echo ""
    echo "🎉 Installation complete!"
    echo ""
    echo "To get started:"
    echo "  arrowhead --help     # Show help"
    echo "  arrowhead config     # Configure API keys"
    echo "  arrowhead            # Start interactive mode"
    echo ""
    echo "For more information, visit: https://github.com/${GITHUB_REPO}"
}

# Set up signal handlers for cleanup
trap cleanup EXIT

# Run main function
main "$@"