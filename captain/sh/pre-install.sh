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
    if ! command -v cc >/dev/null 2>&1 && ! command -v gcc >/dev/null 2>&1; then
        log_warning "C compiler not found. Installing build tools..."

        # Detect package manager and install build tools
        if command -v apt >/dev/null 2>&1; then
            log_info "Using apt to install build-essential..."
            if sudo apt update && sudo apt install -y build-essential; then
                log_success "Build tools installed successfully!"
            else
                log_error "Failed to install build tools. Please run: sudo apt install build-essential"
                exit 1
            fi
        elif command -v yum >/dev/null 2>&1; then
            log_info "Using yum to install development tools..."
            if sudo yum groupinstall -y "Development Tools"; then
                log_success "Build tools installed successfully!"
            else
                log_error "Failed to install build tools. Please run: sudo yum groupinstall 'Development Tools'"
                exit 1
            fi
        elif command -v pacman >/dev/null 2>&1; then
            log_info "Using pacman to install base-devel..."
            if sudo pacman -S --noconfirm base-devel; then
                log_success "Build tools installed successfully!"
            else
                log_error "Failed to install build tools. Please run: sudo pacman -S base-devel"
                exit 1
            fi
        elif command -v brew >/dev/null 2>&1; then
            log_info "Using brew to install build tools..."
            if brew install gcc; then
                log_success "Build tools installed successfully!"
            else
                log_error "Failed to install build tools. Please run: brew install gcc"
                exit 1
            fi
        else
            log_error "No supported package manager found."
            log_error "Please install a C compiler manually:"
            log_error "  Ubuntu/Debian: sudo apt install build-essential"
            log_error "  CentOS/RHEL: sudo yum groupinstall 'Development Tools'"
            log_error "  Arch: sudo pacman -S base-devel"
            log_error "  macOS: brew install gcc"
            exit 1
        fi
    else
        log_success "Build tools already available!"
    fi
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
