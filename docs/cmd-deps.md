## Deps Commands

The dependency analysis system provides fast visibility into which external crates are actually used versus declared in Cargo.toml. This helps reduce dependency bloat, identify missing dependencies, and aid in project onboarding.

### Key Features
- **Smart Filtering**: Automatically filters out reserved roots and local modules
- **Robust Parsing**: Uses `syn` crate for accurate Rust AST parsing
- **Network Checks**: Queries crates.io to verify missing dependencies exist
- **Rename Support**: Handles renamed dependencies in Cargo.toml
- **Multiple Dependency Types**: Checks `[dependencies]`, `[dev-dependencies]`, and `[build-dependencies]`
- **JSON Output**: Machine-readable output for tooling and CI integration

### `cm deps [--path <PATH>] [--json]`
**Description**: Analyze project dependencies - find which external crates are actually used versus declared in Cargo.toml

**Usage**:
```bash
# Analyze current directory
cm deps

# Analyze specific directory
cm deps --path ./my-crate

# Output as JSON
cm deps --json
```

**What it does**:
- Scans all Rust source files (`src/**/*.rs` and `build.rs`)
- Extracts `use` statements and `extern crate` declarations
- Parses `Cargo.toml` to get declared dependencies
- Compares used crates vs declared dependencies
- Queries crates.io to verify missing dependencies exist
- Reports unused declared dependencies and missing used dependencies

**Output Format**:

**Human-readable** (default):
```
📦 Crate root: /path/to/project

🔎 Detected external crates (from use statements):
  - serde
  - tokio
  - reqwest

🧾 Declared dependencies:
  [dependencies]:
    - serde
    - tokio
    - reqwest
    - unused-crate

❗ Missing dependencies (not declared in Cargo.toml):
  - missing-crate [https://crates.io/crates/missing-crate]
```

**JSON format** (`--json`):
```json
{
  "crate_root": "/path/to/project",
  "used_crates": ["serde", "tokio", "reqwest"],
  "declared_deps": {
    "normal": ["serde", "tokio", "reqwest", "unused-crate"],
    "dev": [],
    "build": [],
    "renamed": {}
  },
  "missing_deps": [
    {
      "name": "missing-crate",
      "crates_io": "https://crates.io/crates/missing-crate"
    }
  ]
}
```

**Features**:
- **Smart filtering**: Automatically filters out:
  - Reserved roots: `crate`, `self`, `super`, `std`, `core`, `alloc`
  - Local modules (detected via `src/<name>.rs` or `src/<name>/mod.rs`)
- **Robust parsing**: Uses `syn` crate for accurate Rust AST parsing
- **Network checks**: Queries crates.io with 5-second timeout to verify missing dependencies
- **Rename support**: Handles renamed dependencies in Cargo.toml
- **Multiple dependency types**: Checks `[dependencies]`, `[dev-dependencies]`, and `[build-dependencies]`

**Options**:
- `--path <PATH>`: Analyze specific directory (default: current directory)
- `--json`: Output results as JSON instead of human-readable format

**Use Cases**:
1. **Dependency cleanup**: Find unused dependencies to remove
2. **Onboarding**: Quickly see what external crates a project uses
3. **CI/CD**: Integrate into build pipelines to track dependency usage
4. **Code review**: Verify all used dependencies are properly declared

**Limitations**:
- Proc-macro-only dependencies (derives, attributes) won't appear from `use` statements
- Workspace support: v1 analyzes single crate (nearest Cargo.toml)
- Network checks are best-effort; failures are labeled as `error` in report
- Aliased crates via `extern crate foo as bar;` are handled by scanning `extern crate`

**Examples**:
```bash
# Quick check of current project
cm deps

# Analyze specific crate in workspace
cm deps --path ./crates/my-crate

# Get JSON output for tooling
cm deps --json > deps-report.json

# Use in CI pipeline
cm deps --json | jq '.missing_deps | length'  # Count missing deps
```

## Implementation Details

### Scanning Process

1. **File Discovery**: Recursively scans `src/**/*.rs` and optional `build.rs`
2. **AST Parsing**: Uses `syn` crate to parse Rust source files
3. **Use Statement Extraction**: Extracts `use` statements and `extern crate` declarations
4. **Dependency Parsing**: Parses `Cargo.toml` to get declared dependencies
5. **Comparison**: Compares used crates vs declared dependencies
6. **Network Verification**: Queries crates.io with 5-second timeout to verify missing dependencies

### Filtering Logic

**Reserved Roots (Automatically Filtered)**:
- `crate`, `self`, `super`, `std`, `core`, `alloc`

**Local Modules (Automatically Detected)**:
- Files matching `src/<name>.rs` or `src/<name>/mod.rs` patterns
- Automatically identified as local modules, not external dependencies

### Dependency Types Analyzed

- **Normal Dependencies**: `[dependencies]` section
- **Dev Dependencies**: `[dev-dependencies]` section  
- **Build Dependencies**: `[build-dependencies]` section
- **Renamed Dependencies**: Handles `package = "..."` renames in Cargo.toml

## Best Practices

### Regular Dependency Audits

```bash
# Run dependency analysis regularly
cm deps

# Check for unused dependencies
cm deps | grep "unused"

# Export for tracking
cm deps --json > deps-audit-$(date +%Y%m%d).json
```

### CI/CD Integration

```bash
# Fail build if missing dependencies found
cm deps --json | jq -e '.missing_deps | length == 0' || exit 1

# Track dependency changes over time
cm deps --json > deps-snapshot.json
```

### Project Onboarding

```bash
# Quick overview of project dependencies
cm deps

# See what external crates are actually used
cm deps | grep "Detected external crates"
```

## Troubleshooting

### Missing Dependencies Not Detected

**Proc-macro Dependencies**:
- Proc-macro-only dependencies (like `serde_derive`) won't appear from `use` statements
- These are typically detected through their usage in `#[derive(...)]` attributes
- Check `Cargo.toml` manually for proc-macro dependencies

**Conditional Compilation**:
- Dependencies used only in `#[cfg(...)]` blocks may not be detected
- Review conditional compilation blocks manually

### False Positives

**Local Modules**:
- If a local module is incorrectly identified as external, check file structure
- Ensure modules follow standard Rust module conventions

**Renamed Dependencies**:
- Renamed dependencies are handled automatically
- Check `Cargo.toml` for `package = "..."` entries

### Network Check Failures

```bash
# Network checks are best-effort
# Failures are labeled as 'error' in report
# Check crates.io manually if needed
curl -s "https://crates.io/api/v1/crates/missing-crate" | jq
```

## Technical Details

### Parsing Implementation

- **AST-Based**: Uses `syn` crate for robust Rust code parsing
- **Pattern Matching**: Handles `use` groups, globs, and renames
- **File Traversal**: Uses `walkdir` to efficiently traverse source directories

### Performance

- **Fast Execution**: Typically completes in < 1 second for most projects
- **Efficient Scanning**: Only processes `.rs` files in `src/` directory
- **Network Timeout**: 5-second timeout for crates.io queries

### Output Formats

**Human-Readable**:
- Color-coded output with emojis
- Clear sections for different information types
- Easy to read and understand

**JSON Format**:
- Machine-readable structured data
- Suitable for scripting and CI integration
- Includes all analysis results

## Future Enhancements

Planned features for future versions:
- **`--include-tests`**: Include test files in analysis
- **Workspace-Wide Mode**: Analyze entire workspace, not just single crate
- **`--add` Flag**: Automatically write missing dependencies to Cargo.toml
- **Proc-Macro Detection**: Better detection of proc-macro dependencies
- **Conditional Compilation**: Analysis of `#[cfg(...)]` blocks

---

