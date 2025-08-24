#!/bin/bash

# 🚢 Cargo Mate macOS Wrapper Script
# This wrapper decrypts and executes the protected cargo-mate binary
# Source code is protected - only the compiled binary is distributed

set -euo pipefail

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROTECTED_BINARY=""
KEY="${CARGO_MATE_KEY:-default-protection-key-2024}"

# Detect architecture
detect_arch() {
    local arch=$(uname -m)
    case $arch in
        x86_64|amd64)
            PROTECTED_BINARY="$SCRIPT_DIR/macos/cargo-mate-macos-x86_64.protected"
            ;;
        aarch64|arm64)
            PROTECTED_BINARY="$SCRIPT_DIR/macos/cargo-mate-macos-aarch64.protected"
            ;;
        *)
            echo "❌ Unsupported architecture: $arch"
            echo "   Supported: x86_64, aarch64 (Apple Silicon)"
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
        echo "✅ Executing protected binary directly"
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
