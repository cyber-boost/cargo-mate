# 🚢 Cargo Mate Protected Distribution

This directory contains wrapper scripts for distributing cargo-mate with source code protection. These wrappers decrypt and execute the protected binaries, allowing you to distribute cargo-mate on crates.io while keeping your source code secure.

## 📁 Directory Structure

```
captain/
├── linux/
│   ├── cargo-mate-linux-x86_64.protected
│   ├── cargo-mate-linux-aarch64.protected
│   └── cargo-mate-linux-x86_64.secure.protected
├── macos/
│   ├── cargo-mate-macos-x86_64.protected
│   └── cargo-mate-macos-aarch64.protected
├── windows/
│   └── cargo-mate-windows-x86_64.exe.protected
├── wrapper-linux.sh
├── wrapper-macos.sh
├── wrapper-windows.bat
├── install.sh
└── README-wrappers.md (this file)
```

## 🔧 How It Works

1. **Protected Binaries**: The actual cargo-mate binaries are encrypted using XOR encryption with a secret key
2. **Wrapper Scripts**: Platform-specific wrapper scripts that decrypt the binary in memory and execute it
3. **Runtime Decryption**: The source code is never exposed - decryption happens at runtime in memory

## 🚀 Installation for Users

### Automatic Installation (Recommended)

```bash
# Download and run the installer
curl -fsSL https://github.com/yourusername/cargo-mate/releases/latest/download/install.sh | bash
```

### Manual Installation

1. Download the appropriate wrapper script for your platform
2. Download the corresponding protected binary
3. Place them in the same directory
4. Make the wrapper executable: `chmod +x wrapper-*.sh`
5. Run: `./wrapper-linux.sh --help` (or appropriate platform)

## 🛠️ For Publishers (You)

### Creating Protected Binaries

Use the cargo-mate build tools to create protected binaries:

```bash
# Build and protect your cargo-mate binary
cargo build --release
./cargo-mate create_secure_binary target/release/cargo-mate my-secret-key
```

### Publishing to crates.io

1. Create a crate that includes:
   - The wrapper scripts
   - The protected binaries for each platform
   - The install.sh script
   - This README

2. Users install via: `cargo install cargo-mate-protected`

3. The wrapper automatically detects the platform and decrypts the appropriate binary

### Environment Variables

- `CARGO_MATE_KEY`: Override the decryption key (default: built-in key)
- `CARGO_MATE_INSTALL_DIR`: Custom installation directory (default: ~/.cargo/bin)

## 🔒 Security Features

- **Source Code Protection**: Source code is never distributed
- **Runtime Decryption**: Decryption happens in memory only
- **Key Management**: Uses SHA256 hash of your secret key for XOR encryption
- **Platform Detection**: Automatically selects correct binary for user's platform
- **Fallback Support**: Works even without Python (basic functionality)

## 🏗️ Architecture Support

| Platform | Architectures | Status |
|----------|---------------|---------|
| Linux | x86_64, aarch64 | ✅ Full Support |
| macOS | x86_64, aarch64 (Apple Silicon) | ✅ Full Support |
| Windows | x86_64 | ✅ Full Support (via .bat) |

## 🚨 Important Notes

1. **Key Security**: Keep your encryption key secure - it's required for decryption
2. **Python Dependency**: The wrappers prefer Python for decryption but have shell fallbacks
3. **Temporary Files**: Decryption creates temporary files that are cleaned up automatically
4. **Performance**: There's a small startup delay due to decryption (usually <100ms)

## 🔄 Update Process

When you release new versions:

1. Build new protected binaries with your key
2. Update the wrapper scripts if needed
3. Create a new release with updated binaries
4. Users automatically get updates via `cargo update`

## 📝 Example Usage

```bash
# After installation, users can use cargo-mate normally:
cm --version
cm init
cm build --release
cm anchor save my-project
cm journey record development-workflow
```

## 🆘 Troubleshooting

### "Binary not found"
- Ensure the protected binary is in the correct platform subdirectory
- Check file permissions

### "Decryption failed"
- Verify the encryption key is correct
- Ensure Python is available (optional but recommended)

### "Permission denied"
- Make wrapper script executable: `chmod +x wrapper-*.sh`

## 📄 License

This distribution maintains the same license as cargo-mate. See the main cargo-mate LICENSE file for details.

---

**Built with ❤️ for the Rust community**
*Protect your intellectual property while sharing amazing tools!*
