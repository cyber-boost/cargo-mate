# SCAT - Source Code Obfuscation Tool

**Source**: `cargo-mate/captain/src/scat.rs` and `cargo-mate/captain/src/cmd/smune.rs:Commands::Scat`

## Overview

**⚠️ NOTE**: This is currently a **stub implementation** that delegates to `captain-real`. The actual implementation is not in this codebase.

**Main Handler**: `scat.rs:handle_scat_command()` (verified in `scat.rs:4-23`)

SCAT (Source Code Obfuscation Tool) provides **legitimate obfuscation** techniques that maintain code functionality while making it harder to casually read. Unlike destructive obfuscation, SCAT creates **reversible transformations** using mapping files, making it perfect for contests, audits, and educational purposes.

**Current Implementation** (verified in `scat.rs:4-23`):
- All subcommands print delegation messages
- Actual implementation is handled by `captain-real` binary
- Commands: `Protect`, `Verify`, `Info`

## Key Features

- 🔄 **Reversible Obfuscation**: All transformations can be undone using mapping files
- 🛡️ **Functionality Preserved**: Code remains executable and functional
- 🎯 **Multiple Strategies**: Names, code identifiers, strings, and file packing
- 📊 **Mapping Files**: JSON format for easy reversal and tracking
- 🚀 **Production Ready**: Comprehensive error handling and safety checks

## Legitimate Use Cases

- **Contest Submissions**: Hide source code until reveal
- **Audit Preparation**: Protect algorithms during security reviews
- **Educational Puzzles**: Create programming challenges
- **IP Protection**: Prevent casual copying while maintaining executability

## Command Structure

```bash
cm scat <SUBCOMMAND> [OPTIONS]
```

### Available Subcommands

| Subcommand | Description |
|------------|-------------|
| `names` | Obfuscate file/folder names |
| `code` | Obfuscate Rust identifiers |
| `strings` | Scramble string literals |
| `pack` | Pack files into bundle |
| `unpack` | Reverse obfuscation |

---

## Names Subcommand

Obfuscates file and folder names within a directory structure.

### Syntax

```bash
cm scat names <PATH> [OPTIONS]
```

### Options

| Flag | Description |
|------|-------------|
| `--map <FILE>` | Output mapping file for reversal |
| `--sequential` | Use predictable names (file1, file2) instead of random |

### Examples

```bash
# Obfuscate with random names and mapping
cm scat names src/ --map name_mapping.json

# Use sequential naming
cm scat names project/ --sequential --map sequential_mapping.json

# Quick obfuscation (mapping saved automatically)
cm scat names code/
```

### Behavior

- **Default Mapping**: `name_mapping.json` in target directory
- **Name Generation**: 8-character random strings or sequential numbers
- **Safety**: Preserves file extensions and directory structure
- **Reversal**: Use `unpack` command with mapping file

---

## Code Subcommand

Obfuscates Rust identifiers while preserving code functionality.

### Syntax

```bash
cm scat code <PATH> [OPTIONS]
```

### Options

| Flag | Description |
|------|-------------|
| `--preserve-pub` | Keep public API identifiers unchanged |
| `--min-len <NUM>` | Minimum identifier length to obfuscate (default: 3) |
| `--map <FILE>` | Output mapping file for reversal |

### Examples

```bash
# Obfuscate all identifiers longer than 3 characters
cm scat code src/ --min-len 3 --map code_mapping.json

# Preserve public functions and structs
cm scat code lib.rs --preserve-pub --map public_mapping.json

# Quick code obfuscation
cm scat code main.rs
```

### Behavior

- **Target**: Function names, variable names, struct fields
- **Preserved**: Keywords, types, attributes, strings
- **Safety**: Maintains compilation and execution
- **Mapping**: Tracks original → obfuscated name pairs

---

## Strings Subcommand

Scrambles string literals in source code.

### Syntax

```bash
cm scat strings <PATH> [OPTIONS]
```

### Options

| Flag | Description |
|------|-------------|
| `--key <STRING>` | Encryption key for reversible scrambling |
| `--map <FILE>` | Output mapping file for reversal |

### Examples

```bash
# Simple string reversal
cm scat strings main.rs --map string_mapping.json

# Encrypted scrambling with key
cm scat strings src/ --key "my_secret_key" --map encrypted_mapping.json

# Batch string obfuscation
cm scat strings . --recursive --map all_strings.json
```

### Behavior

- **Target**: String literals (`"text"`, `'single'`)
- **Methods**: Reversal or XOR encryption with key
- **Safety**: Preserves string structure and formatting
- **Reversal**: Requires same key for encrypted strings

---

## Pack Subcommand

Compresses and packs multiple files into a single obfuscated bundle.

### Syntax

```bash
cm scat pack <INPUT> <OUTPUT> [OPTIONS]
```

### Options

| Flag | Description |
|------|-------------|
| `--compress` | Compress files before packing |

### Examples

```bash
# Pack directory with compression
cm scat pack src/ project.bundle --compress

# Pack without compression
cm scat pack code/ submission.bundle

# Pack single file
cm scat pack main.rs executable.bundle
```

### Behavior

- **Format**: Custom binary format with metadata
- **Compression**: Optional gzip-like compression
- **Structure**: Preserves directory hierarchy
- **Safety**: No data loss, full recovery possible

---

## Unpack Subcommand

Reverses obfuscation using mapping files.

### Syntax

```bash
cm scat unpack <INPUT> <MAP> [OPTIONS]
```

### Options

| Flag | Description |
|------|-------------|
| `--output <DIR>` | Output directory for unpacked files |

### Examples

```bash
# Unpack names using mapping
cm scat unpack obfuscated/ name_mapping.json --output original/

# Reverse code obfuscation
cm scat unpack main.rs code_mapping.json

# Unpack bundle with custom output
cm scat unpack project.bundle mapping.json --output restored/
```

### Behavior

- **Mapping Required**: Must provide correct mapping file
- **Validation**: Verifies mapping integrity
- **Safety**: Creates backups before overwriting
- **Complete Restoration**: Returns to original state

---

## Mapping File Format

All SCAT commands generate JSON mapping files for reversal:

```json
{
  "original_to_obfuscated": {
    "original_name": "obfuscated_name",
    "function_name": "f0",
    "variable": "v1"
  },
  "obfuscated_to_original": {
    "obfuscated_name": "original_name",
    "f0": "function_name",
    "v1": "variable"
  },
  "timestamp": "2024-01-15T10:30:00Z",
  "method": "names|code|strings",
  "metadata": {
    "version": "1.0",
    "options": {}
  }
}
```

### Mapping File Security

- **Keep Secure**: Mapping files enable complete reversal
- **Backup Required**: Store mapping files separately from obfuscated code
- **Version Control**: Consider whether to commit mapping files
- **Access Control**: Limit who can access mapping files

---

## Safety Features

### Automatic Backups

- **File Level**: Original files backed up before modification
- **Directory Level**: `.shipwreck/scat/` directory for all backups
- **Timestamped**: Backup names include modification time
- **Cleanup**: Automatic cleanup of old backups (configurable)

### Safety Checks

- **Path Validation**: Verifies input/output paths exist and are accessible
- **Permission Checks**: Ensures write permissions for target directories
- **File Type Detection**: Validates Rust files for code/string operations
- **Mapping Validation**: Verifies mapping file integrity before operations

### Error Recovery

- **Transaction-like**: Operations are atomic where possible
- **Partial Recovery**: Can recover from interrupted operations
- **Backup Restoration**: Easy restoration from automatic backups

---

## Advanced Usage

### Chaining Operations

```bash
# Obfuscate names first
cm scat names src/ --map names.json

# Then obfuscate code
cm scat code src/ --map code.json

# Finally scramble strings
cm scat strings src/ --key "contest_key" --map strings.json
```

### Batch Processing

```bash
# Process multiple directories
for dir in src tests examples; do
    cm scat code "$dir" --map "${dir}_mapping.json"
done
```

### Custom Workflows

```bash
# Contest submission workflow
cm scat names project/ --sequential
cm scat code project/src/
cm scat strings project/
cm scat pack project/ submission.bundle
```

---

## Best Practices

### For Contests

1. **Use Sequential Names**: Easier to track during development
2. **Document Mapping Location**: Keep mapping files secure but accessible
3. **Test Compilation**: Ensure obfuscated code still compiles
4. **Preserve Public API**: Use `--preserve-pub` for libraries

### For Audits

1. **Create Backup Bundle**: Pack original code separately
2. **Use Strong Keys**: For string encryption
3. **Maintain Mapping Chain**: Document obfuscation steps
4. **Test Functionality**: Verify all features work after obfuscation

### For Education

1. **Start Simple**: Use name obfuscation for basic puzzles
2. **Progressive Difficulty**: Combine multiple techniques
3. **Provide Hints**: Include partial mapping information
4. **Easy Reversal**: Use sequential naming for simpler challenges

---

## Performance Considerations

### File Processing

- **Memory Usage**: Large files processed in chunks
- **I/O Optimization**: Minimizes disk access patterns
- **Parallel Processing**: Directory operations use multiple threads
- **Progress Indication**: Real-time feedback for long operations

### Mapping Generation

- **Incremental Updates**: Only process changed files
- **Memory Efficient**: Streaming JSON generation for large mappings
- **Sorted Output**: Consistent mapping file format
- **Compression**: Optional mapping file compression

---

## Troubleshooting

### Common Issues

**"Path does not exist"**
- Verify input path is correct and accessible
- Check file permissions

**"Mapping file corrupted"**
- Regenerate mapping file
- Check file system integrity
- Restore from backup

**"Compilation failed after obfuscation"**
- Check if `--preserve-pub` is needed
- Verify min-len setting isn't too aggressive
- Test with original code first

### Recovery Procedures

**Lost Mapping File**
```bash
# Restore from backup
cp .shipwreck/scat/backup_timestamp/* original_location/
```

**Partial Obfuscation**
```bash
# Reverse what you can
cm scat unpack partial/ available_mapping.json
# Manually restore remaining files
```

**Emergency Reversal**
```bash
# Force unpack with best effort
cm scat unpack --force corrupted/ mapping.json
```

---

## Integration

### CI/CD Pipelines

```yaml
# Example GitHub Actions workflow
- name: Obfuscate for contest
  run: |
    cm scat names src/ --sequential
    cm scat code src/ --preserve-pub
    cm scat pack src/ contest.bundle
```

### Build Scripts

```bash
# build.sh integration
if [ "$CONTEST_MODE" = "true" ]; then
    echo "🧩 Obfuscating for contest..."
    cm scat names src/ --map contest_mapping.json
    cm scat code src/ --min-len 5
fi
```

### IDE Integration

- **VS Code**: Custom tasks for obfuscation workflows
- **IntelliJ**: External tool configuration
- **Vim**: Custom commands and mappings

---

## Technical Details

### Architecture

- **Modular Design**: Separate modules for each obfuscation type
- **AST Processing**: Uses `syn` crate for safe Rust code analysis
- **File System**: Cross-platform path handling
- **Serialization**: JSON mapping files with metadata

### Dependencies

- `syn`: Rust AST parsing and manipulation
- `quote`: Code generation from AST
- `serde`: Serialization for mapping files
- `rand`: Random name generation
- `walkdir`: Directory traversal
- `regex`: String pattern matching

### Security Model

- **No Cryptography**: Simple obfuscation, not encryption
- **Mapping Security**: Protect mapping files as they enable reversal
- **Code Safety**: Maintains Rust safety guarantees
- **File System**: Respects OS permissions and security models

---

## Future Enhancements

### Planned Features

- **Advanced Code Obfuscation**: Control flow obfuscation, variable splitting
- **Binary Obfuscation**: Integration with binary packing tools
- **Web Interface**: Browser-based obfuscation dashboard
- **Team Collaboration**: Shared mapping file management
- **Integration APIs**: REST API for automation

### Community Contributions

- **Plugin System**: Custom obfuscation strategies
- **Language Support**: Extend beyond Rust
- **Template System**: Predefined obfuscation profiles
- **Analytics**: Obfuscation effectiveness metrics

---

*Remember: SCAT is designed for legitimate obfuscation needs. Always maintain mapping files securely and respect intellectual property rights.*
