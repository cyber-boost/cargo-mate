# 🚢 Cargo Mate Windows PowerShell Wrapper Script
# This wrapper decrypts and executes the protected cargo-mate binary
# Source code is protected - only the compiled binary is distributed

param(
    [Parameter(ValueFromRemainingArguments = $true)]
    [string[]]$CargoArgs
)

# Configuration
$ScriptDir = Split-Path -Parent $PSCommandPath
$ProtectedBinary = ""
$Key = $env:CARGO_MATE_KEY
if (-not $Key) {
    $Key = "default-protection-key-2024"
}

# Detect architecture
$Arch = $env:PROCESSOR_ARCHITECTURE
switch ($Arch) {
    "AMD64" {
        $ProtectedBinary = Join-Path $ScriptDir "windows\cargo-mate-windows-x86_64.exe.protected"
    }
    default {
        Write-Error "❌ Unsupported architecture: $Arch"
        Write-Host "   Supported: AMD64 (x86_64)"
        exit 1
    }
}

# Check if protected binary exists
if (-not (Test-Path $ProtectedBinary)) {
    Write-Error "❌ Protected binary not found: $ProtectedBinary"
    Write-Host "   Please ensure the cargo-mate package is properly installed."
    exit 1
}

# Create temporary directory
$TempDir = Join-Path $env:TEMP "cargo-mate-$(Get-Random)"
New-Item -ItemType Directory -Path $TempDir -Force | Out-Null
$DecryptedBinary = Join-Path $TempDir "cargo-mate-decrypted.exe"

# The .protected files are the actual binaries - no decryption needed
Write-Host "✅ Executing protected binary directly"

# Execute the protected binary directly
try {
    & $ProtectedBinary @CargoArgs
}
finally {
    # Cleanup temp directory (even though we don't use it anymore)
    if (Test-Path $TempDir) {
        Remove-Item $TempDir -Recurse -Force
    }
}
