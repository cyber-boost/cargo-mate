#!/bin/bash

# 🚢 Cargo Mate Universal Installer
# Installs cargo-mate with source code protection
# Supports Linux, macOS, and Windows (via Git Bash/WSL)

set -euo pipefail

# Check if this is being run as a pre-install check
if [[ "${1:-}" == "--pre-install-check" ]]; then
    echo "🔧 Running pre-install dependency check..."
    check_build_dependencies
    echo "✅ Build dependencies are ready!"
    echo ""
    echo "🚀 You can now safely run:"
    echo "   cargo install cargo-mate"
    exit 0
fi

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
INSTALL_DIR="${CARGO_MATE_INSTALL_DIR:-$HOME/.cargo/bin}"
DOWNLOAD_URL="https://get.cargo.do/latest.tar.gz"

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

# Check and install build dependencies
check_build_dependencies() {
    log_info "Checking for build dependencies..."

    # Check if we have a C compiler
    if command -v cc >/dev/null 2>&1 || command -v gcc >/dev/null 2>&1; then
        log_success "C compiler found: $(command -v cc || command -v gcc)"
        return 0
    fi

    log_warning "C compiler not found. Installing build tools..."

    # Detect the Linux distribution for better package manager handling
    detect_linux_distro() {
        if [[ -f /etc/os-release ]]; then
            . /etc/os-release
            echo "$ID"
        elif [[ -f /etc/lsb-release ]]; then
            . /etc/lsb-release
            echo "$DISTRIB_ID" | tr '[:upper:]' '[:lower:]'
        elif [[ -f /etc/debian_version ]]; then
            echo "debian"
        elif [[ -f /etc/redhat-release ]]; then
            echo "rhel"
        else
            echo "unknown"
        fi
    }

    # Try multiple package managers in order of preference
    local distro=$(detect_linux_distro)
    log_info "Detected Linux distribution: $distro"

    # Try apt (Ubuntu, Debian)
    if command -v apt >/dev/null 2>&1; then
        log_info "Using apt to install build-essential..."
        if sudo apt update && sudo apt install -y build-essential gcc g++; then
            log_success "Build tools installed successfully with apt!"
            return 0
        else
            log_error "Failed to install build tools with apt"
        fi
    fi

    # Try yum/dnf (RHEL, CentOS, Fedora)
    if command -v dnf >/dev/null 2>&1; then
        log_info "Using dnf to install development tools..."
        if sudo dnf group install -y "Development Tools" && sudo dnf install -y gcc gcc-c++; then
            log_success "Build tools installed successfully with dnf!"
            return 0
        else
            log_error "Failed to install build tools with dnf"
        fi
    elif command -v yum >/dev/null 2>&1; then
        log_info "Using yum to install development tools..."
        if sudo yum groupinstall -y "Development Tools" && sudo yum install -y gcc gcc-c++; then
            log_success "Build tools installed successfully with yum!"
            return 0
        else
            log_error "Failed to install build tools with yum"
        fi
    fi

    # Try pacman (Arch Linux)
    if command -v pacman >/dev/null 2>&1; then
        log_info "Using pacman to install base-devel..."
        if sudo pacman -S --noconfirm --needed base-devel gcc; then
            log_success "Build tools installed successfully with pacman!"
            return 0
        else
            log_error "Failed to install build tools with pacman"
        fi
    fi

    # Try apk (Alpine Linux)
    if command -v apk >/dev/null 2>&1; then
        log_info "Using apk to install build tools..."
        if sudo apk add build-base gcc g++; then
            log_success "Build tools installed successfully with apk!"
            return 0
        else
            log_error "Failed to install build tools with apk"
        fi
    fi

    # Try brew (macOS or Linux with Homebrew)
    if command -v brew >/dev/null 2>&1; then
        log_info "Using brew to install build tools..."
        if brew install gcc; then
            log_success "Build tools installed successfully with brew!"
            return 0
        else
            log_error "Failed to install build tools with brew"
        fi
    fi

    # If all package managers failed, provide manual instructions
    log_error "All automated installation methods failed."
    log_error "Please install a C compiler manually:"
    log_error ""
    log_error "🐧 Ubuntu/Debian:"
    log_error "   sudo apt update && sudo apt install -y build-essential gcc g++"
    log_error ""
    log_error "🐧 CentOS/RHEL/Fedora:"
    log_error "   sudo dnf group install -y 'Development Tools' && sudo dnf install -y gcc gcc-c++"
    log_error "   OR"
    log_error "   sudo yum groupinstall -y 'Development Tools' && sudo yum install -y gcc gcc-c++"
    log_error ""
    log_error "🐧 Arch Linux:"
    log_error "   sudo pacman -S --noconfirm base-devel gcc"
    log_error ""
    log_error "🐧 Alpine Linux:"
    log_error "   sudo apk add build-base gcc g++"
    log_error ""
    log_error "🍎 macOS:"
    log_error "   brew install gcc"
    log_error ""
    log_error "After installing, run: cargo install cargo-mate"
    exit 1
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

# Download and install binaries
download_and_install() {
    local temp_dir
    temp_dir=$(mktemp -d)
    local tarball_path="$temp_dir/cargo-mate.tar.gz"

    log_info "Downloading cargo-mate binaries from $DOWNLOAD_URL"

    # Download the tarball
    if command -v curl >/dev/null 2>&1; then
        curl -L -o "$tarball_path" "$DOWNLOAD_URL"
    elif command -v wget >/dev/null 2>&1; then
        wget -O "$tarball_path" "$DOWNLOAD_URL"
    else
        log_error "Neither curl nor wget found. Please install one of them."
        exit 1
    fi

    # Extract the tarball
    log_info "Extracting binaries..."
    tar -xzf "$tarball_path" -C "$temp_dir"

    # Find the binary for this platform
    local binary_name
    case $PLATFORM in
        linux)
            binary_name="cargo-mate-linux-${ARCH}.protected"
            ;;
        macos)
            binary_name="cargo-mate-macos-${ARCH}.protected"
            ;;
        windows)
            binary_name="cargo-mate-windows-x86_64.exe.protected"
            ;;
    esac

    local wrapper_name
    case $PLATFORM in
        linux)
            wrapper_name="wrapper-linux.sh"
            ;;
        macos)
            wrapper_name="wrapper-macos.sh"
            ;;
        windows)
            wrapper_name="wrapper-windows.bat"
            ;;
    esac

    local binary_path="$INSTALL_DIR/$binary_name"
    local wrapper_path="$INSTALL_DIR/cm"

    # Install the binary
    if [[ -f "$temp_dir/pkg/releases/$binary_name" ]]; then
        cp "$temp_dir/pkg/releases/$binary_name" "$binary_path"
        log_success "Installed protected binary: $binary_path"
    else
        log_error "Binary not found in download: $binary_name"
        log_info "Available binaries:"
        ls -la "$temp_dir/pkg/releases/"
        exit 1
    fi

    # Install the wrapper script
    if [[ -f "$SCRIPT_DIR/sh/$wrapper_name" ]]; then
        cp "$SCRIPT_DIR/sh/$wrapper_name" "$wrapper_path"
        chmod +x "$wrapper_path"
        log_success "Installed wrapper script: $wrapper_path"
    else
        log_error "Wrapper script not found: $SCRIPT_DIR/sh/$wrapper_name"
        exit 1
    fi

    # Clean up
    rm -rf "$temp_dir"

    log_success "Cargo Mate installed successfully!"
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
    check_build_dependencies
    create_install_dir
    download_and_install
    verify_installation
}

# Run installation
main "$@"
