# 🔍 Stub - Stub/Placeholder/TODO Finder

**Source**: `cargo-mate/captain/src/cmd/stub.rs` and `cargo-mate/captain/src/cmd/smune.rs:StubAction`

Stub is a powerful command that scans your codebase for stubs, placeholders, TODOs, FIXMEs, and other unimplemented code patterns. It helps you identify areas that need attention and generates comprehensive reports.

## Overview

**Main Handler**: `stub.rs:handle_stub()` (verified in `stub.rs:311-345`)

**Implementation**:
- Default action (no subcommand): Calls `handle_stub_find()` (verified in `stub.rs:344`)
- `StubAction::History` → `handle_stub_history()` (verified in `stub.rs:323-324`)
- `StubAction::Show { name }` → `handle_stub_show(&name)` (verified in `stub.rs:326-327`)
- `StubAction::Delete { all }` → `handle_stub_delete(all)` (verified in `stub.rs:329-330`)
- `StubAction::Find { pattern }` → `handle_stub_find()` with pattern (verified in `stub.rs:332-335`)
- `StubAction::Skip { patterns }` → `handle_stub_find()` with skip patterns (verified in `stub.rs:337-338`)

Stub recursively scans your project files and finds common patterns indicating incomplete code, such as:
- TODO/FIXME comments
- Stub functions and placeholders
- Unimplemented code blocks
- Temporary/mock implementations
- Work-in-progress markers

## Usage

### Basic Usage

```bash
# Scan current directory for stubs
cm stub

# Scan specific directory
cm stub -t ./src

# Specify output file
cm stub -o stubs-report.md

# Scan specific file extensions
cm stub --ext rs,py,js,html

# Custom pattern search
cm stub --custom "my_pattern"

# Skip certain patterns
cm stub --skip "temp,mock"
```

### Command Options

- `-t, --target <PATH>`: Target directory to scan (default: `.`)
- `-o, --out <PATH>`: Output file path (default: `cm-stubs-[timestamp].md`)
- `--ext <EXTENSIONS>`: Comma-separated file extensions to scan (default: `rs,py,js,html`)
- `--custom <PATTERN>`: Custom pattern(s) to search for (comma-separated)
- `--find <PATTERN>`: Alias for `--custom` (same functionality)
- `--skip <PATTERNS>`: Patterns to skip/exclude (comma-separated)

### Subcommands

#### Find

Search for stubs with optional custom pattern:

```bash
cm stub find
cm stub find "my_custom_pattern"
```

#### Skip

Search for stubs while skipping specific patterns:

```bash
cm stub skip "temp,mock,test"
```

#### History

View all previously generated stub reports:

```bash
cm stub history
```

Lists all reports saved in `~/.shipwreck/stubs/` with timestamps.

#### Show

Display a specific stub report from history:

```bash
cm stub show cm-stubs-20241201_143022
cm stub show 20241201  # Partial match works too
```

#### Delete

Delete stub reports from history:

```bash
# Delete all reports
cm stub delete --all

# Without --all, shows help message
cm stub delete
```

## Default Patterns

Stub searches for these common patterns (case-insensitive):

### Comments
- `TODO`, `FIXME`, `XXX`, `HACK`
- `# stub`, `# placeholder`, `# mock`
- `# not implemented`, `# unimplemented`
- `# temporary`, `# temp`, `# wip`
- `# work in progress`

### Code Patterns
- `stub`, `placeholder`, `mock`
- `unimplemented!()`, `todo!()`
- `raise NotImplementedError`, `raise NotImplemented`
- `return None # stub`, `return [] # stub`
- `pass # implement`, `... # implement`
- `Some(`, `None # placeholder`

### Phrases
- "in real implementation"
- "need to implement"
- "implement later"
- "not yet implemented"
- "to be implemented"
- "fix later", "fix soon"

## Examples

### Example 1: Basic Scan

```bash
cm stub
```

Scans current directory for all default patterns in Rust, Python, JavaScript, and HTML files.

### Example 2: Rust Only

```bash
cm stub --ext rs -t ./src
```

Scans only Rust files in the `src` directory.

### Example 3: Custom Pattern

```bash
cm stub --custom "REVIEW,REFACTOR" --ext rs,py
```

Searches for custom patterns "REVIEW" and "REFACTOR" in Rust and Python files.

### Example 4: Skip Patterns

```bash
cm stub --skip "test,mock,temp"
```

Finds stubs but excludes matches containing "test", "mock", or "temp".

### Example 5: Combined Options

```bash
cm stub -t ./src --ext rs --custom "FIXME" --skip "test" -o my-stubs.md
```

Scans `./src` for Rust files, searches for "FIXME" pattern, skips "test" matches, and outputs to `my-stubs.md`.

## Output Format

The generated markdown report includes:

1. **Summary**: Total stubs found and breakdown by file
2. **Detailed Matches**: For each stub found:
   - File path and line number
   - Pattern that matched
   - Code snippet with context (3 lines before/after)
   - Highlighted match line

### Example Output

```markdown
# 🔍 Stub/Placeholder/TODO Finder Report

**Total stubs found**: 5

## Summary by File

- **src/main.rs**: 2 stub(s)
- **src/utils.rs**: 3 stub(s)

## Detailed Matches

### 1. src/main.rs

#### Match 1 (Line 42)

**Pattern**: `TODO`

**Code**:

```
   39 |     let config = load_config();
   40 |     let data = fetch_data();
   41 |     // TODO: Add error handling here
   42 |     process_data(data);
   43 |     save_results();
   44 |     Ok(())
```

---
```

## Stub History

All generated stub reports are automatically saved to `~/.shipwreck/stubs/` for future reference. This allows you to:

- Track stub count over time
- Compare different scans
- Review historical reports
- Monitor code quality improvements

### History Management

```bash
# List all reports
cm stub history

# Show a specific report
cm stub show <name>

# Delete all reports
cm stub delete --all
```

## Use Cases

### 1. Code Review Preparation

Before a code review, generate a stub report to identify areas that need attention:

```bash
cm stub -t ./src -o review-stubs.md
```

### 2. Project Cleanup

Find all TODOs and FIXMEs for a cleanup sprint:

```bash
cm stub --custom "TODO,FIXME" --ext rs
```

### 3. Quality Assurance

Regularly scan for stubs to maintain code quality:

```bash
cm stub --ext rs,py,js
```

### 4. Custom Pattern Tracking

Track specific patterns in your codebase:

```bash
cm stub --custom "REVIEW,OPTIMIZE,REFACTOR"
```

## Performance

- **Fast**: Pattern matching is efficient even on large codebases
- **Recursive**: Scans all subdirectories automatically
- **Selective**: Only scans files with specified extensions
- **Context-Aware**: Provides code context around matches

## Tips

1. **Regular Scans**: Run `cm stub` regularly to track technical debt
2. **Custom Patterns**: Use `--custom` to track project-specific patterns
3. **Skip Patterns**: Use `--skip` to filter out false positives
4. **History**: Check `cm stub history` to see improvement over time
5. **Extensions**: Specify `--ext` to focus on relevant file types

## Integration

Stub is integrated with Cargo Mate's license system and requires a valid license to use. All generated reports are automatically saved to the history directory for easy access later.

## See Also

- `cm tree` - Generate directory trees
- `cm liberate` - Generate lib.rs files
- `cm sweep` - Remove debug statements

---

**Note**: Stub helps you "hate thy stubs" less by making them visible and trackable! Use it regularly to maintain clean, production-ready code.

