@echo off
REM 🚢 Cargo Mate Windows Wrapper Script
REM This wrapper decrypts and executes the protected cargo-mate binary
REM Source code is protected - only the compiled binary is distributed

setlocal enabledelayedexpansion

REM Configuration
set "SCRIPT_DIR=%~dp0"
set "PROTECTED_BINARY=%SCRIPT_DIR%windows\cargo-mate-windows-x86_64.exe.protected"
set "KEY=%CARGO_MATE_KEY%"
if "%KEY%"=="" set "KEY=default-protection-key-2024"

REM If we're in the sh/ directory, go up one level to find platform directories
if "%SCRIPT_DIR%"=="*sh*" (
    for %%i in ("%SCRIPT_DIR%..") do set "SCRIPT_DIR=%%~fi\"
    set "PROTECTED_BINARY=%SCRIPT_DIR%windows\cargo-mate-windows-x86_64.exe.protected"
)

REM Check if protected binary exists
if not exist "%PROTECTED_BINARY%" (
    echo ❌ Protected binary not found: %PROTECTED_BINARY%
    echo    Please ensure the cargo-mate package is properly installed.
    exit /b 1
)

REM Create temporary directory
set "TEMP_DIR=%TEMP%\cargo-mate-%RANDOM%"
mkdir "%TEMP_DIR%" 2>nul
set "DECRYPTED_BINARY=%TEMP_DIR%\cargo-mate-decrypted.exe"

REM Decrypt using Python if available
python3 -c "
import sys
import hashlib
import os

# Read encrypted binary
with open(r'%PROTECTED_BINARY%', 'rb') as f:
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
key_hash = hashlib.sha256('%KEY%'.encode()).digest()

# XOR decryption
decrypted_data = bytearray()
for i, byte in enumerate(encrypted_data):
    decrypted_data.append(byte ^ key_hash[i % len(key_hash)])

# Write decrypted binary
with open(r'%DECRYPTED_BINARY%', 'wb') as f:
    f.write(decrypted_data)

print('Binary decrypted successfully')
" 2>nul

if %errorlevel% neq 0 (
    echo ⚠️  Python decryption failed, using fallback method
    REM Simple fallback - just copy the binary (if it's not actually encrypted)
    copy "%PROTECTED_BINARY%" "%DECRYPTED_BINARY%" >nul
) else (
    echo ✅ Binary decrypted successfully
)

REM Execute the decrypted binary with all remaining arguments
"%DECRYPTED_BINARY%" %*

REM Cleanup
rd /s /q "%TEMP_DIR%" 2>nul

endlocal
