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

try {
    # Read encrypted binary
    $EncryptedData = [System.IO.File]::ReadAllBytes($ProtectedBinary)

    # Handle cargo-mate format
    $Header = [System.Text.Encoding]::ASCII.GetString($EncryptedData[0..31])
    if ($Header -eq "CARGO_MATE_ENCRYPTED_BINARY_V1") {
        $Lines = [System.Text.Encoding]::UTF8.GetString($EncryptedData) -split "`n"
        if ($Lines.Length -ge 3) {
            $Key = $Lines[1]  # Use embedded key
            $EncryptedData = [System.Text.Encoding]::UTF8.GetBytes(($Lines[2..($Lines.Length-1)] -join "`n"))
        } else {
            $EncryptedData = $EncryptedData[32..($EncryptedData.Length-1)]
        }
    }

    # Create SHA256 hash of the key
    $Sha256 = [System.Security.Cryptography.SHA256]::Create()
    $KeyBytes = [System.Text.Encoding]::UTF8.GetBytes($Key)
    $KeyHash = $Sha256.ComputeHash($KeyBytes)

    # XOR decryption
    $DecryptedData = New-Object byte[] $EncryptedData.Length
    for ($i = 0; $i -lt $EncryptedData.Length; $i++) {
        $DecryptedData[$i] = $EncryptedData[$i] -bxor $KeyHash[$i % $KeyHash.Length]
    }

    # Write decrypted binary
    [System.IO.File]::WriteAllBytes($DecryptedBinary, $DecryptedData)

    Write-Host "✅ Binary decrypted successfully"
}
catch {
    Write-Warning "⚠️  PowerShell decryption failed, using fallback method"
    # Simple fallback - just copy the binary (if it's not actually encrypted)
    Copy-Item $ProtectedBinary $DecryptedBinary -Force
}

# Execute the decrypted binary
try {
    & $DecryptedBinary @CargoArgs
}
finally {
    # Cleanup
    if (Test-Path $TempDir) {
        Remove-Item $TempDir -Recurse -Force
    }
}
