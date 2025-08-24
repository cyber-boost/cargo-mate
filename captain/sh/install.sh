#!/bin/bash

# 🚢 Cargo Mate Universal Installer
# Installs cargo-mate with source code protection
# Supports Linux, macOS, and Windows (via Git Bash/WSL)

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
INSTALL_DIR="${CARGO_MATE_INSTALL_DIR:-$HOME/.cargo/bin}"
REPO_URL="https://github.com/cyber-boost/cargo-mate/releases/latest/download"

# If we're in the sh/ directory, go up one level to find platform directories
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
if [[ "$SCRIPT_DIR" == *"/sh" ]]; then
    SCRIPT_DIR="$(dirname "$SCRIPT_DIR")"
fi

# Logging functions
log_info() {
    echo -e "${BLUE}ℹ️  $1${NC}"
}

log_success() {
    echo -e "${GREEN}✅ $1${NC}"
}

log_warning() {
    echo -e "${YELLOW}⚠️  $1${NC}"
}

log_error() {
    echo -e "${RED}❌ $1${NC}"
}

# Detect platform and architecture
detect_platform() {
    OS=$(uname -s | tr '[:upper:]' '[:lower:]')
    ARCH=$(uname -m)

    case $OS in
        linux)
            PLATFORM="linux"
            ;;
        darwin)
            PLATFORM="macos"
            ;;
        msys*|mingw*|cygwin*)
            PLATFORM="windows"
            ;;
        *)
            log_error "Unsupported operating system: $OS"
            exit 1
            ;;
    esac

    case $ARCH in
        x86_64|amd64)
            ARCH="x86_64"
            ;;
        aarch64|arm64)
            ARCH="aarch64"
            ;;
        *)
            log_error "Unsupported architecture: $ARCH"
            log_info "Supported architectures: x86_64, aarch64"
            exit 1
            ;;
    esac

    log_info "Detected platform: $PLATFORM-$ARCH"
}

# Create install directory
create_install_dir() {
    if [[ ! -d "$INSTALL_DIR" ]]; then
        log_info "Creating install directory: $INSTALL_DIR"
        mkdir -p "$INSTALL_DIR"
    fi

    # Add to PATH if not already there
    if [[ ":$PATH:" != *":$INSTALL_DIR:"* ]]; then
        log_warning "Please add $INSTALL_DIR to your PATH"
        log_info "Add this to your shell profile:"
        echo "export PATH=\"\$PATH:$INSTALL_DIR\""
    fi
}

# Download and install wrapper script
install_wrapper() {
    local wrapper_name
    local binary_name

    case $PLATFORM in
        linux)
            wrapper_name="wrapper-linux.sh"
            binary_name="cargo-mate-linux-${ARCH}.protected"
            ;;
        macos)
            wrapper_name="wrapper-macos.sh"
            binary_name="cargo-mate-macos-${ARCH}.protected"
            ;;
        windows)
            wrapper_name="wrapper-windows.bat"
            binary_name="cargo-mate-windows-x86_64.exe.protected"
            ;;
    esac

    local wrapper_path="$INSTALL_DIR/cm"
    local binary_path="$INSTALL_DIR/$binary_name"

    log_info "Installing wrapper script to: $wrapper_path"

    # Copy wrapper script (scripts don't need .protected extension)
    if [[ -f "$SCRIPT_DIR/sh/$wrapper_name" ]]; then
        cp "$SCRIPT_DIR/sh/$wrapper_name" "$wrapper_path"
    else
        log_error "Wrapper script not found: $SCRIPT_DIR/sh/$wrapper_name"
        exit 1
    fi

    # Copy protected binary to same directory as wrapper (for Windows compatibility)
    if [[ -f "$SCRIPT_DIR/$PLATFORM/$binary_name" ]]; then
        cp "$SCRIPT_DIR/$PLATFORM/$binary_name" "$binary_path"
        log_success "Installed protected binary: $binary_path"
    else
        log_error "Protected binary not found: $SCRIPT_DIR/$PLATFORM/$binary_name"
        exit 1
    fi

    # Make wrapper executable
    chmod +x "$wrapper_path"

    # All platforms now handle path resolution automatically
    log_info "Wrapper script handles path resolution automatically"

    log_success "Installed cargo-mate wrapper: $wrapper_path"
}

# Verify installation
verify_installation() {
    if [[ -x "$INSTALL_DIR/cm" ]]; then
        log_info "Testing installation..."
        if "$INSTALL_DIR/cm" --version >/dev/null 2>&1; then
            $INSTALL_DIR/cm install captain
            $INSTALL_DIR/cm activate 
            log_success "Installation verified successfully!"
        else
            log_warning "Installation test failed, but binary is installed"
        fi
    else
        log_error "Installation failed - wrapper not found"
        exit 1
    fi
}

# Main installation process
main() {
    log_info "🚢 Installing Cargo Mate (Source Protected)"
    detect_platform
    create_install_dir
    install_wrapper
    verify_installation
}

# Run installation
main "$@"
