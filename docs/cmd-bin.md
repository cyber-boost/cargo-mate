# 🔬 Bin - Systematic Binary Testing

**Source**: `cargo-mate/captain/src/cmd/bin.rs` and `cargo-mate/captain/src/cmd/smune.rs:BinAction`

Bin is a powerful command that systematically tests any binary by parsing its help output and testing all possible command and flag combinations. It's perfect for understanding binary behavior, testing CLI tools, and generating comprehensive test reports.

## Overview

**Main Handler**: `bin.rs:handle_bin()` (verified in `bin.rs:514-570`)

Bin performs systematic testing of binaries by:
1. Running `--help`, `-h`, or `help` to discover commands
2. Parsing the help output to extract available commands
3. For each command, getting its help to extract flags
4. Testing each command with each flag combination
5. Collecting all outputs, exit codes, and errors
6. Generating a comprehensive markdown report

## Usage

### Basic Usage

```bash
# Test a binary by name (searches PATH)
cm bin --name cargo

# Test a binary by path
cm bin --path /usr/bin/git

# Specify output file
cm bin --name cargo -o cargo-test.md

# Set timeout (default: 10 seconds)
cm bin --name cargo --timeout-seconds 30

# Limit command depth (useful for binaries with many commands)
cm bin --name cargo --max-depth 5
```

### Command Options

- `-p, --path <PATH>`: Path to the binary file
- `-n, --name <NAME>`: Binary name (searches PATH)
- `-o, --out <PATH>`: Output file path (default: `cm-bin-{name}-{timestamp}.md`)
- `--timeout-seconds <SECONDS>`: Timeout for each command (default: `10`)
- `--max-depth <DEPTH>`: Maximum number of commands to test (useful for limiting scope)

### Subcommands

**Source**: `bin.rs:handle_bin()` matches on `BinAction` enum (verified in `bin.rs:520-537`)

#### History

**Source**: `bin.rs:handle_bin_history()` (verified in `bin.rs:584-610`)

View all previously generated test reports:

```bash
cm bin history
```

**Implementation**:
- Calls `list_bin_history()` to get all reports from `~/.shipwreck/test-bin/`
- Displays each report with filename and path
- Shows helpful hints if no history found

Lists all reports saved in `~/.shipwreck/test-bin/` with timestamps.

#### Show

**Source**: `bin.rs:handle_bin_show()` (verified in `bin.rs:613-635`)

Display a specific test report from history:

```bash
cm bin show cm-bin-cargo-20241201_143022
cm bin show cargo  # Partial match works too
```

**Implementation**:
- Searches history for reports matching the name (partial match supported)
- Reads and displays the full report content
- Falls back to first report if exact match not found

#### Find

**Source**: `bin.rs:handle_bin_find()` (verified in `bin.rs:636-668`)

Search through test reports for specific content:

```bash
cm bin find "error"
cm bin find "cargo"
```

**Implementation**:
- Reads all report files from history
- Performs case-insensitive search for query string
- Displays matching reports with filenames and paths

Searches the content of all saved reports for the query string.

#### Delete

**Source**: `bin.rs:handle_bin_delete()` (verified in `bin.rs:669-690`)

Delete test reports from history:

```bash
# Delete all reports
cm bin delete --all

# Without --all, shows help message
cm bin delete
```

**Implementation**:
- Requires `--all` flag to actually delete
- Without `--all`, shows helpful message with example
- Deletes all files in `~/.shipwreck/test-bin/` directory

## How It Works

### Step 1: Discovery

Bin first runs the binary with help flags (`--help`, `-h`, or `help`) to discover available commands:

```bash
binary --help
```

### Step 2: Command Parsing

The help output is parsed to extract command names. Bin looks for:
- "COMMANDS:" sections
- "SUBCOMMANDS:" sections
- Common command listing patterns

### Step 3: Flag Discovery

For each discovered command, Bin runs:

```bash
binary <command> --help
```

And parses the output to extract available flags.

### Step 4: Systematic Testing

Bin then tests each combination:
- `binary <command>` (no flags)
- `binary <command> <flag>` (each flag individually)
- `binary <command> --help` (help output)

### Step 5: Report Generation

All results are collected and a comprehensive markdown report is generated with:
- Summary statistics
- Commands found
- Flags discovered
- Detailed test results for each combination
- Exit codes, stdout, stderr
- Execution times

## Examples

### Example 1: Test Cargo

```bash
cm bin --name cargo
```

Systematically tests all cargo commands and flags.

### Example 2: Test Custom Binary

```bash
cm bin --path ./target/release/my-binary
```

Tests a custom binary at a specific path.

### Example 3: Limited Testing

```bash
cm bin --name git --max-depth 3 --timeout-seconds 5
```

Tests only the first 3 git commands with a 5-second timeout per command.

### Example 4: Save to Specific Location

```bash
cm bin --name docker -o docker-complete-test.md
```

Tests Docker and saves the report to a specific file.

## Output Format

The generated markdown report includes:

1. **Header**: Binary name, path, test duration
2. **Summary**: Total commands, tests, passes, failures, errors
3. **Commands Found**: List of all discovered commands
4. **Flags by Command**: Flags discovered for each command
5. **Test Results**: Detailed results for each test:
   - Status (PASS/FAIL)
   - Exit code
   - Duration
   - Stdout (truncated if > 500 chars)
   - Stderr (truncated if > 500 chars)
   - Error messages

### Example Output

```markdown
# 🔬 Binary Test Report

**Binary**: `cargo`
**Path**: `/home/user/.cargo/bin/cargo`
**Test Duration**: 45.23s

## Summary

- **Total Commands Found**: 15
- **Total Tests Run**: 127
- **✅ Passed**: 98
- **❌ Failed**: 29
- **⚠️ Errors**: 12

## Commands Found

- `build`
- `test`
- `run`
- `doc`
...

## Test Results

### Command: `build`

#### Test 1: `build` (no flags) ✅

- **Status**: PASS
- **Exit Code**: 0
- **Duration**: 2.345s

**Stdout**:
```
Compiling...
Finished...
```

---
```

## Test History

All generated test reports are automatically saved to `~/.shipwreck/test-bin/` for future reference. This allows you to:

- Track binary behavior over time
- Compare different versions
- Review historical test results
- Monitor command compatibility

### History Management

```bash
# List all reports
cm bin history

# Show a specific report
cm bin show <name>

# Search reports
cm bin find <query>

# Delete all reports
cm bin delete --all
```

## Use Cases

### 1. CLI Tool Documentation

Generate comprehensive documentation of a CLI tool's behavior:

```bash
cm bin --name mytool -o tool-docs.md
```

### 2. Binary Testing

Systematically test a binary to understand all its commands and flags:

```bash
cm bin --path ./my-binary
```

### 3. Compatibility Testing

Test if a binary works correctly with all its flags:

```bash
cm bin --name cargo --timeout-seconds 30
```

### 4. Discovery

Discover all available commands in an unfamiliar binary:

```bash
cm bin --name newtool
```

## Performance

- **Timeout Protection**: Each command has a configurable timeout (default: 10s)
- **Efficient**: Tests run sequentially but efficiently
- **Scalable**: Use `--max-depth` to limit testing scope
- **Safe**: Commands run in isolated processes with timeouts

## Tips

1. **Start Small**: Use `--max-depth` to test a few commands first
2. **Adjust Timeout**: Increase `--timeout-seconds` for slow commands
3. **Review Output**: Check the generated report for unexpected behavior
4. **History**: Use `cm bin history` to track changes over time
5. **Search**: Use `cm bin find` to locate specific test results

## Limitations

1. **Help Parsing**: Relies on standard help output formats - may miss commands in non-standard formats
2. **Flag Combinations**: Tests flags individually, not all combinations (would be exponential)
3. **Interactive Commands**: Commands that require interactive input may timeout
4. **Platform Specific**: Some binaries may behave differently on different platforms

## Integration

Bin is integrated with Cargo Mate's license system and requires a valid license to use. All generated reports are automatically saved to the history directory for easy access later.

## See Also

- `cm stub` - Find stubs and TODOs
- `cm tree` - Generate directory trees
- `cm liberate` - Generate lib.rs files

---

**Note**: Bin is perfect for systematically understanding any binary's behavior. Use it to test CLI tools, generate documentation, or discover all available commands and flags!

