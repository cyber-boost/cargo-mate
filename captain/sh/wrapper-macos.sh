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

# Decrypt and execute the binary
decrypt_and_run() {
    local temp_dir=$(mktemp -d)
    local decrypted_binary="$temp_dir/cargo-mate-decrypted"

    # Cleanup function
    cleanup() {
        rm -rf "$temp_dir"
    }
    trap cleanup EXIT

    # Decrypt using Python if available, otherwise fallback to shell method
    if command -v python3 >/dev/null 2>&1; then
        python3 -c "
import sys
import hashlib
import os

# Read encrypted binary
with open('$PROTECTED_BINARY', 'rb') as f:
    encrypted_data = f.read()

# Handle cargo-mate format
if encrypted_data.startswith(b'CARGO_MATE_ENCRYPTED_BINARY_V1'):
    lines = encrypted_data.split(b'\n')
    if len(lines) >= 3:
        key = lines[1]  # Use embedded key
        encrypted_data = b'\n'.join(lines[2:])
    else:
        encrypted_data = encrypted_data[32:]  # Skip header

# Create SHA256 hash of the key
key_hash = hashlib.sha256('$KEY'.encode()).digest()

# XOR decryption
decrypted_data = bytearray()
for i, byte in enumerate(encrypted_data):
    decrypted_data.append(byte ^ key_hash[i % len(key_hash)])

# Write decrypted binary
with open('$decrypted_binary', 'wb') as f:
    f.write(decrypted_data)

print('✅ Binary decrypted successfully')
" 2>/dev/null || {
        echo "⚠️  Python decryption failed, using fallback method"
        # Simple fallback - just copy the binary (if it's not actually encrypted)
        cp "$PROTECTED_BINARY" "$decrypted_binary"
    }
    else
        echo "⚠️  Python not available, using fallback method"
        # Simple fallback - just copy the binary (if it's not actually encrypted)
        cp "$PROTECTED_BINARY" "$decrypted_binary"
    fi

    # Make executable and run
    chmod +x "$decrypted_binary"
    exec "$decrypted_binary" "$@"
}

# Main execution
main() {
    detect_arch
    check_binary
    decrypt_and_run "$@"
}

# Run main function with all arguments
main "$@"
