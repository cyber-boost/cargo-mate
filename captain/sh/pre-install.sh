#!/bin/bash

# 🚢 Cargo Mate Pre-Install Script
# Checks and installs build dependencies before cargo install
# Run this BEFORE: cargo install cargo-mate

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

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

# Main function
main() {
    echo "🚢 Cargo Mate - Pre-Install Check"
    echo "==================================="
    echo ""
    echo "This script will check and install build dependencies"
    echo "required for: cargo install cargo-mate"
    echo ""

    check_build_dependencies

    echo ""
    echo "🎉 Pre-install check completed!"
    echo ""
    echo "🚀 You can now safely run:"
    echo "   cargo install cargo-mate"
    echo ""
    echo "📝 After installation, run:"
    echo "   cm install && cm activate"
}

# Run main
main "$@"
