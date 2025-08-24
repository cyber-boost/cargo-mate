# Tests

This directory contains tests for the Cargo Mate published wrapper crate.

## 📁 Test Files

### [`integration_test.rs`](./integration_test.rs)
Integration tests that verify the overall functionality of the published crate:

- **Binary Availability**: Verifies cargo-mate is properly installed
- **File Structure**: Ensures all required wrapper scripts exist
- **Platform Support**: Checks protected binaries for supported platforms
- **Configuration**: Validates Cargo.toml configuration
- **Documentation**: Ensures README completeness
- **Directory Structure**: Verifies proper project layout

### [`wrapper_test.rs`](./wrapper_test.rs)
Unit tests focused on the wrapper script functionality:

- **Script Permissions**: Verifies scripts are executable with proper shebangs
- **Platform Wrappers**: Tests Windows batch and PowerShell scripts
- **Binary References**: Ensures scripts reference correct binary names
- **Platform Detection**: Tests architecture detection logic
- **Error Handling**: Verifies proper error handling patterns
- **Documentation**: Checks script documentation quality

## 🏗️ Test Architecture

### Published vs Source Code
- These tests run against the **published wrapper crate**
- The real Cargo Mate source code (3,300+ lines) is protected and separate
- Tests verify the wrapper functionality, not the core implementation

### Test Categories

#### **Structure Tests**
- Verify file and directory structure
- Check script permissions and executability
- Validate configuration files

#### **Integration Tests**
- Test end-to-end functionality (when cargo-mate is installed)
- Verify platform-specific behavior
- Check installation and setup processes

#### **Validation Tests**
- Ensure wrapper scripts reference correct binaries
- Validate platform detection logic
- Check error handling and edge cases

## 🚀 Running Tests

### Local Development
```bash
# Run all tests
cargo test

# Run specific test file
cargo test --test integration_test
cargo test --test wrapper_test

# Run with verbose output
cargo test -- --nocapture
```

### CI/CD Integration
These tests are designed to run in automated environments:
- **GitHub Actions** can verify the published crate structure
- **Integration tests** ensure wrapper functionality
- **Validation tests** catch configuration issues

## 📊 Test Coverage

### What These Tests Cover
- ✅ **Wrapper Script Integrity**: Permissions, shebangs, error handling
- ✅ **Platform Support**: Linux, macOS, Windows compatibility
- ✅ **File Structure**: Required directories and files exist
- ✅ **Configuration**: Cargo.toml and README validation
- ✅ **Binary References**: Scripts point to correct protected binaries

### What These Tests Don't Cover
- ❌ **Core Functionality**: The protected binary's actual features
- ❌ **Source Code**: Real implementation is separate and protected
- ❌ **Advanced Features**: AI assistance, complex workflows (protected)

## 🔒 Security Note

The tests respect the protected binary architecture:
- They don't attempt to access or test the protected source code
- They verify the wrapper layer functions correctly
- They ensure the published crate structure is sound

## 🤝 Contributing

When modifying the published crate structure:
1. Update tests to reflect new requirements
2. Ensure cross-platform compatibility
3. Maintain the wrapper/protected separation
4. Test on all supported platforms

The tests serve as both validation and documentation of the published crate's expected structure and behavior.
