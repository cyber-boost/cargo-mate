## View Commands

The view command system provides comprehensive access to build results, artifacts, errors, warnings, and build history. This system integrates with the enhanced display system to provide detailed information about your builds in an organized, easy-to-access format.

### Key Features
- **Comprehensive Results**: Access errors, warnings, artifacts, scripts, and history
- **Organized Storage**: Results automatically saved to `~/.shipwreck/` directory
- **Multiple View Options**: Different ways to access build information
- **Real-Time Updates**: Results updated automatically after each build
- **Easy Navigation**: Simple commands to access different result types

### Result Storage System

All build results are automatically stored in the `~/.shipwreck/` directory:

- **Errors**: `~/.shipwreck/errors/latest.txt`
- **Warnings**: `~/.shipwreck/warnings/latest.txt`
- **Artifacts**: `~/.shipwreck/artifacts/latest.txt`
- **Build Scripts**: `~/.shipwreck/scripts/latest.txt`
- **History**: Integrated with existing history system

### `cm view errors`
**Description**: View all build errors

**Usage**:
```bash
cm view errors
```

**Output**:
```
🔴 Latest Errors:
══════════════════════════════════════════════════
error[E0308]: mismatched types
  --> src/main.rs:42:5
  |
42 |     let result: String = 42;
  |         ^^^^^ expected String, found integer
  |
  = note: expected due to previous error

error[E0433]: failed to resolve: use of undeclared crate
  --> src/main.rs:7:5
  |
7 | use nonexistent_crate;
  |     ^^^^^^^^^^^^^^^^
══════════════════════════════════════════════════
```

---

### `cm view warnings`
**Description**: View all build warnings

**Usage**:
```bash
cm view warnings
```

---

### `cm view artifacts`
**Description**: View generated files and locations

**Usage**:
```bash
cm view artifacts
```

**Output**:
```
📦 Generated Artifacts:
══════════════════════════════════════════════════
🔨 Build Scripts:
  cargo-mate -> libs: 0, paths: 0, cfgs: 0

📁 Target Directory: target/
  ├── debug/
  │   ├── cm (executable)
  │   └── deps/
  └── release/
      ├── cm (executable)
      └── deps/

📋 Cargo.toml: Updated to version 1.0.1
══════════════════════════════════════════════════
```

---

### `cm view scripts`
**Description**: View build script outputs

**Usage**:
```bash
cm view scripts
```

---

### `cm view history`
**Description**: View detailed build history

**Usage**:
```bash
cm view history
```

---

### `cm view checklist`
**Description**: View checklist and fixes

**Usage**:
```bash
cm view checklist
```

---

### `cm view all`
**Description**: View all results in one place

**Usage**:
```bash
cm view all
```

**Output**:
```
🔍 Complete Build Results View:
══════════════════════════════════════════════════

🔴 ERRORS (2):
error[E0308]: mismatched types
  --> src/main.rs:42:5
  |
42 |     let result: String = 42;
  |         ^^^^^ expected String, found integer

⚠️  WARNINGS (1):
warning: unused variable
  --> src/main.rs:15:5
  |
15 |     let unused = "hello";
  |     ^^^^^^^^^

📦 ARTIFACTS:
🔨 Build Scripts:
  cargo-mate -> libs: 0, paths: 0, cfgs: 0

📁 Target Directory: target/
  ├── debug/
  │   ├── cm (executable)
  │   └── deps/
  └── release/
      ├── cm (executable)
      └── deps/

📋 Cargo.toml: Updated to version 1.0.1
══════════════════════════════════════════════════
```

---

### `cm view latest`
**Description**: Quick view of latest issues

**Usage**:
```bash
cm view latest
```

---

### `cm view open`
**Description**: Open results in file explorer

**Usage**:
```bash
cm view open
---

### `cm view all`
**Description**: View all results in one place

**Usage**:
```bash
cm view all
```

**Output**:
```
🔍 Complete Build Results View:
══════════════════════════════════════════════════

🔴 ERRORS (2):
error[E0308]: mismatched types
  --> src/main.rs:42:5
  |
42 |     let result: String = 42;
  |         ^^^^^ expected String, found integer

error[E0433]: failed to resolve: use of undeclared crate
  --> src/main.rs:7:5
  |
7 | use nonexistent_crate;
  |     ^^^^^^^^^^^^^^^^

⚠️  WARNINGS (1):
warning: unused variable
  --> src/main.rs:15:5
  |
15 |     let unused = "hello";
  |     ^^^^^^^^^

📦 ARTIFACTS:
🔨 Build Scripts:
  cargo-mate -> libs: 0, paths: 0, cfgs: 0

📁 Target Directory: target/
  ├── debug/
  │   ├── cm (executable)
  │   └── deps/
  └── release/
      ├── cm (executable)
      └── deps/

📋 Cargo.toml: Updated to version 1.0.1
══════════════════════════════════════════════════
```

---

### `cm view latest`
**Description**: Quick view of latest issues

**Usage**:
```bash
cm view latest
```

---

### `cm view open`
**Description**: Open results in file explorer

**Usage**:
```bash
cm view open
```

---

---

## Best Practices

### Regular Build Review

```bash
# After each build, review results
cm view all

# Check for errors first
cm view errors

# Review warnings
cm view warnings

# Check generated artifacts
cm view artifacts
```

### Quick Issue Check

```bash
# Quick view of latest issues
cm view latest

# Detailed error analysis
cm view errors | grep -A 5 "error\["

# Check build script issues
cm view scripts
```

### Build History Tracking

```bash
# View build history
cm view history

# Track build trends over time
cm view history | grep "Success\|Failed"
```

## Troubleshooting

### No Results Available

```bash
# Check if build has been run
cargo build

# Verify storage directory exists
ls -la ~/.shipwreck/

# Check file permissions
chmod -R 755 ~/.shipwreck/
```

### Results Not Updating

```bash
# Run a build to generate new results
cm build

# Check if results are being saved
ls -la ~/.shipwreck/errors/
ls -la ~/.shipwreck/artifacts/
```

### File Not Found Errors

```bash
# Create storage directories if missing
mkdir -p ~/.shipwreck/{errors,warnings,artifacts,scripts}

# Run a build to populate results
cm build
```

## Integration with Build System

### Automatic Result Collection

The view system automatically collects results during builds:

- **Error Collection**: Real-time error and warning collection during compilation
- **Artifact Tracking**: Monitors `CompilerArtifact` and `BuildScriptExecuted` messages
- **File Management**: Automatic creation of result storage directories
- **JSON Parsing**: Parses JSON-formatted cargo output for structured data

### Enhanced Display Integration

The view system integrates with the enhanced display system:

- **Live Updates**: Results updated in real-time during builds
- **Progress Tracking**: Live error counts and file processing
- **Summary Display**: Comprehensive build summary after completion
- **View Options**: Clear display of available viewing commands

## Advanced Usage

### Filtering Results

```bash
# View only specific error types
cm view errors | grep "E0308"

# Count errors
cm view errors | grep -c "error\["

# View artifacts for specific target
cm view artifacts | grep "release"
```

### Exporting Results

```bash
# Export errors to file
cm view errors > build-errors.txt

# Export artifacts list
cm view artifacts > artifacts-list.txt

# Export complete build summary
cm view all > build-summary.txt
```

### CI/CD Integration

```bash
# In CI pipeline
cm build
cm view errors > ci-errors.txt
cm view artifacts > ci-artifacts.txt

# Fail build if errors found
if [ -s ci-errors.txt ]; then
    echo "Build failed with errors"
    exit 1
fi
```

## Technical Details

### Data Collection

**Cargo Messages**:
- Parses JSON-formatted cargo output
- Monitors `CompilerArtifact` messages
- Tracks `BuildScriptExecuted` events
- Real-time error and warning collection

**File Management**:
- Automatic creation of result storage directories
- Timestamped result files
- Latest results always available
- Historical results preserved

### Error Handling

**Graceful Degradation**:
- Continues operation if view commands fail
- File validation before reading
- Clear error messages for missing files
- Non-blocking integration with builds

### Performance

**Fast Access**:
- Results stored locally for instant access
- Efficient file reading and parsing
- Minimal overhead on build operations
- Quick command execution

## Future Enhancements

Planned features:
- **Result Filtering**: Advanced filtering options for large result sets
- **Export Formats**: Support for JSON, CSV, or HTML export
- **Custom Views**: User-configurable view layouts
- **Result Archiving**: Automatic archiving of old build results
- **Search Functionality**: Search through historical results
- **Comparison Tools**: Compare results between builds

---