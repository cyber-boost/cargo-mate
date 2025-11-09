# 🚀 Strip Command Documentation

## Overview

**Source**: `cargo-mate/captain/src/cmd/strip.rs` and `cargo-mate/captain/src/strip.rs`

The `strip` command is a powerful Rust source code processor that removes comments, blank lines, attributes, and other non-essential elements from Rust source files. It provides multiple stripping modes ranging from basic comment removal to aggressive code compression.

**Implementation**: `cmd/strip.rs` is a thin wrapper that delegates to `strip.rs:handle_strip_command()`.

## Command Syntax

```bash
cm strip [OPTIONS] <INPUT>
```

**Source**: `strip.rs:StripArgs` struct (verified in `strip.rs:31-56`)

## Arguments

| Argument | Description |
|----------|-------------|
| `<INPUT>` | Input file or directory path (required field: `input: PathBuf`) |

## Options

**All options verified in `strip.rs:StripArgs` struct**:

### Basic Options

| Flag | Description | Source Field |
|------|-------------|--------------|
| `-o, --output <FILE>` | Output file path (defaults to stdout for single files) | `output: Option<PathBuf>` |
| `-t, --target <DIR>` | Target directory to process (alternative to INPUT) | `target: Option<PathBuf>` |
| `-r, --recursive` | Process all .rs files in directory recursively | `recursive: bool` |
| `-b, --remove-blanks` | Remove blank lines | `remove_blanks: bool` |
| `--src` | Process src directory (alias for --input src) | `src: bool` |
| `--max-depth <NUM>` | Maximum depth for recursive processing (default: 10) | `max_depth: usize` (default: 10) |
| `-v, --verbose` | Verbose output | `verbose: bool` |

### Safety Options

| Flag | Description | Source Field |
|------|-------------|--------------|
| `--no-backup` | Disable automatic backup creation | `no_backup: bool` |
| `--force` | Allow overwriting the same file (bypasses safety check) | `force: bool` |

### Aggressive Options

| Flag | Description | Source Field |
|------|-------------|--------------|
| `-a, --aggressive` | Aggressive mode: maximum stripping | `aggressive: bool` |
| `--minify` | Minify: single line where possible | `minify: bool` |
| `-t, --tease` | TEASE mode (remove all comments + blanks) | `tease: bool` |
| `--strip-attrs` | Remove all attributes (#[...]) | `strip_attrs: bool` |
| `--strip-docs` | Remove doc comments specifically | `strip_docs: bool` |
| `--inline-uses` | Remove all use statements and inline them | `inline_uses: bool` |

## Usage Examples

### Basic Usage

```bash
# Strip comments from a single file to stdout
cm strip src/main.rs

# Strip comments and save to new file
cm strip src/main.rs --output main.stripped.rs

# Strip comments and blank lines
cm strip src/main.rs -b

# Process entire src directory recursively
cm strip src/ --recursive
```

### Directory Processing

```bash
# Process src directory with automatic backups (default)
cm strip src/ -r

# Process target directory with custom depth
cm strip src/ --recursive --max-depth 5

# Use src alias
cm strip --src --recursive

# Process without backups (dangerous!)
cm strip src/ -r --no-backup
```

### Aggressive Stripping

```bash
# Maximum aggressive stripping
cm strip src/ -r -a

# Remove all attributes
cm strip src/main.rs --strip-attrs

# Remove only doc comments
cm strip src/main.rs --strip-docs

# Minify to single line
cm strip src/main.rs --minify

# Combine multiple aggressive options
cm strip src/ -r --strip-attrs --strip-docs --minify
```

### Safety Features

```bash
# Process with automatic backups (default behavior)
cm strip src/ -r

# Process without backups (dangerous!)
cm strip src/ -r --no-backup

# Force overwrite same file (allows bypassing safety check)
cm strip src/main.rs --force --output src/main.rs

# Disable backups AND force overwrite (maximum danger!)
cm strip src/main.rs --force --no-backup --output src/main.rs
```

## Stripping Modes

### 1. Basic Mode
- Removes all comments (//, /* */, ///)
- Preserves code structure and formatting
- Optional blank line removal

### 2. Minify Mode (`--minify`)
- Converts code to single-line format where possible
- Maintains syntactic correctness
- Reduces file size significantly

### 3. Aggressive Mode (`-a, --aggressive`)
- Removes all attributes except essential ones (`#[test]`, `#[cfg]`)
- Strips doc comments
- Compresses whitespace around punctuation
- Applies maximum compression techniques

### 4. Selective Stripping
- `--strip-attrs`: Remove all attributes
- `--strip-docs`: Remove only documentation comments
- `--inline-uses`: Inline use statements (planned feature)

## Implementation Details

**Main Handler**: `strip.rs:handle_strip_command()` (verified in `strip.rs:64-88`)

**Key Functions**:
- `determine_input_path()`: Resolves input path from `--src`, `--target`, or `input` argument (verified in `strip.rs:134-142`)
- `validate_rust_file()`: Ensures only `.rs` files are processed (verified in `strip.rs:143-150`)
- `create_backup_directory()`: Creates `~/.shipwreck/strip/` directory (verified in `strip.rs:155-163`)
- `create_backup()`: Creates timestamped backup files (verified in `strip.rs:164-175`)
- `strip_rust()`: Core stripping logic using `syn` crate for AST parsing (verified in `strip.rs:176-230`)
- `process_single_file()`: Handles single file processing (verified in `strip.rs:383-420`)
- `process_directory()`: Handles recursive directory processing (verified in `strip.rs:433-500`)

**Stripping Logic** (verified in `strip.rs:176-230`):
- Uses `syn::parse_file()` to parse Rust source into AST
- `strip_attributes()`: Removes attributes except `#[test]` and `#[cfg]` (verified in `strip.rs:232-260`)
- `strip_doc_comments()`: Removes documentation comments (verified in `strip.rs:262-280`)
- `inline_use_statements()`: Inlines use statements (verified in `strip.rs:282-320`)
- Uses `prettyplease::unparse()` to convert AST back to source code
- Aggressive mode applies additional whitespace compression

## Backup System

**Source**: `strip.rs:create_backup_directory()` and `strip.rs:create_backup()` (verified in `strip.rs:155-175`)

The strip command includes a comprehensive backup system:

### Backup Location
```
~/.shipwreck/strip/
```

**Implementation**: Created automatically if it doesn't exist (verified in `strip.rs:155-163`)

### Backup Naming
```
<original_filename>_<timestamp>.backup
```

**Format**: `{filename}_{YYYYMMDD_HHMMSS}.backup` (verified in `strip.rs:167-175`)

### Examples
```bash
# Creates: ~/.shipwreck/strip/main.rs_20231222_143052.backup (default behavior)
cm strip src/main.rs

# No backup created, direct processing
cm strip src/main.rs --no-backup

# Force overwrite same file (backup still created unless --no-backup)
cm strip src/main.rs --force --output src/main.rs
```

## Processing Statistics

After processing, the command displays:
- Original line count
- Stripped line count
- Reduction percentage
- Files processed (for directories)

## Error Handling

The command handles various error conditions:

### File Errors
- Missing input files
- Permission issues
- Invalid Rust syntax

### Directory Errors
- Non-existent directories
- Maximum depth exceeded
- File system errors

### Backup Errors
- Insufficient disk space
- Permission issues in backup directory

## Integration with Cargo Mate

The strip command integrates seamlessly with other Cargo Mate features:

### With Anchors
```bash
# Create anchor, then strip code
cm anchor save before-strip
cm strip src/ -r -a --backup
cm anchor restore before-strip  # if needed
```

### With Journeys
```bash
# Record stripping workflow
cm journey record code-stripping
cm strip src/ -r --aggressive --backup
cm strip tests/ -r --strip-docs
cm journey play code-stripping  # repeat workflow
```

## Best Practices

### 1. Always Use Backups
```bash
# ✅ Good - creates backup automatically (default)
cm strip src/ -r

# ✅ Good - explicit control when needed
cm strip src/main.rs --no-backup  # Only if you really know what you're doing

# ❌ Bad - no backup protection
cm strip src/main.rs --force --no-backup --output src/main.rs
```

### 2. Test Before Aggressive Stripping
```bash
# First test on single file
cm strip src/main.rs --aggressive --output main.test.rs

# Then process directory
cm strip src/ -r --aggressive --backup
```

### 3. Use Selective Stripping
```bash
# Remove only doc comments (safer)
cm strip src/ -r --strip-docs --backup

# Full aggressive stripping (more aggressive)
cm strip src/ -r -a --backup
```

## Performance Considerations

- **Single files**: Near-instantaneous processing
- **Large directories**: Processing time scales with file count
- **Aggressive mode**: Slower due to AST transformations
- **Backup creation**: Adds I/O overhead

## Troubleshooting

### Common Issues

#### "Permission denied"
```bash
# Solution: Check file permissions
ls -la src/main.rs
chmod 644 src/main.rs
```

#### "Invalid Rust syntax"
```bash
# Solution: Check for syntax errors first
cargo check
```

#### "Backup directory not writable"
```bash
# Solution: Fix permissions
chmod 755 ~/.shipwreck/
```

### Recovery

#### Restore from Backup
```bash
# List available backups
ls -la ~/.shipwreck/strip/

# Restore specific file
cp ~/.shipwreck/strip/main.rs_20231222_143052.backup src/main.rs
```

## Technical Details

### AST Processing
The command uses `syn` crate to parse Rust code into Abstract Syntax Tree (AST), allowing precise removal of:
- Comments (all types)
- Attributes (configurable)
- Doc comments (configurable)
- Whitespace (in aggressive mode)

### File Handling
- Preserves file permissions
- Maintains executable bits
- Creates atomic writes where possible
- Safe error recovery

### Cross-Platform Compatibility
- Works on Linux, macOS, Windows
- Handles platform-specific path separators
- Consistent behavior across platforms

## Future Enhancements

### Planned Features
- `--inline-uses`: Inline use statements across files
- `--obfuscate`: Code obfuscation (limited use case)
- `--compress`: Additional compression techniques
- `--format`: Custom output formatting options

### Integration Opportunities
- Integration with `cargo build` pipeline
- CI/CD workflow integration
- IDE plugin support

## Conclusion

The `strip` command provides a comprehensive solution for code cleaning and optimization, with safety features and multiple stripping modes to suit different use cases. Always use backups when working with important codebases!

For questions or issues, refer to the main Cargo Mate documentation or create an issue in the project repository.
