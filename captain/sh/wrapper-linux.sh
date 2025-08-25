#!/bin/bash

# 🚢 Cargo Mate Linux Wrapper Script
# This wrapper decrypts and executes the protected cargo-mate binary
# Source code is protected - only the compiled binary is distributed

set -euo pipefail

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROTECTED_BINARY=""
KEY="${CARGO_MATE_KEY:-default-protection-key-2024}"

# Handle different installation scenarios:
# 1. During development: scripts are in sh/ subdirectory
# 2. After installation: scripts are in .cargo-mate/ directory
if [[ "$SCRIPT_DIR" == *"/sh" ]]; then
    # We're in development mode - go up to find platform directories
    SCRIPT_DIR="$(dirname "$SCRIPT_DIR")"
elif [[ "$SCRIPT_DIR" == *"/.cargo-mate" ]]; then
    # We're installed - SCRIPT_DIR is already correct
    true
else
    # Fallback - assume we're in the right place
    true
fi

# Detect architecture and find protected binary
detect_arch() {
    local arch=$(uname -m)
    
    # First try to find the binary in .shipwreck/bin (new structure)
    # CRITICAL: Binary MUST be named 'cm' to enhance cargo properly
    local shipwreck_bin="$HOME/.shipwreck/bin/cm"
    if [[ -f "$shipwreck_bin" ]]; then
        PROTECTED_BINARY="$shipwreck_bin"
        return 0
    fi
    
    # Fallback to old structure (for development/testing)
    case $arch in
        x86_64|amd64)
            PROTECTED_BINARY="$SCRIPT_DIR/linux/cargo-mate-linux-x86_64.protected"
            ;;
        aarch64|arm64)
            PROTECTED_BINARY="$SCRIPT_DIR/linux/cargo-mate-linux-aarch64.protected"
            ;;
        *)
            echo "❌ Unsupported architecture: $arch"
            echo "   Supported: x86_64, aarch64"
            exit 1
            ;;
    esac
}

# Check if protected binary exists
check_binary() {
    if [[ ! -f "$PROTECTED_BINARY" ]]; then
        echo "❌ Protected binary not found: $PROTECTED_BINARY"
        echo "   Please ensure the cargo-mate package is properly installed."
        exit 1
    fi
}

# Execute the protected binary directly
execute_binary() {
    # The .protected files are the actual binaries - no decryption needed
    if [[ -x "$PROTECTED_BINARY" ]]; then
        exec "$PROTECTED_BINARY" "$@"
    else
        echo "❌ Protected binary not executable: $PROTECTED_BINARY"
        exit 1
    fi
}

# Main execution
main() {
    detect_arch
    check_binary
    execute_binary "$@"
}

# Run main function with all arguments
main "$@"
