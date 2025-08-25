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

REM The .protected files are the actual binaries - no decryption needed
REM Execute the protected binary directly
"%PROTECTED_BINARY%" %*

REM Cleanup
rd /s /q "%TEMP_DIR%" 2>nul

endlocal
