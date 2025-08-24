# Cargo Mate Tools System

## Overview

The Cargo Mate Tools system provides a modular framework for adding utility tools that can be accessed via the `cm tool` command. This system allows developers to extend Cargo Mate's functionality with custom tools while maintaining consistency and discoverability.

## Architecture

### Core Components

- **`mod.rs`** - Defines the `Tool` trait and manages tool registration
- **`Tool` trait** - Standard interface that all tools must implement
- **Registry system** - Auto-discovers and registers all available tools
- **CLI integration** - Seamlessly integrates with Cargo Mate's command-line interface

### Tool Structure

Each tool must implement the `Tool` trait:

```rust
pub trait Tool {
    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str;
    fn command(&self) -> Command;
    fn execute(&self, matches: &ArgMatches) -> Result<()>;
}
```

## Available Tools

### 🔍 `bench-diff` - Benchmark Comparison
Compares cargo benchmark results between commits to identify performance changes.

```bash
cm tool bench-diff --from HEAD~1 --to HEAD --threshold 5.0
```

**Features:**
- Compares performance between git commits
- Color-coded output (red for regressions, green for improvements)
- Configurable threshold for reporting changes
- Multiple output formats: human, json, table
- Results stored in `.cargo-mate/benchmarks/`

### 🔒 `dep-audit` - Dependency Security Audit
Audits Rust dependencies for security vulnerabilities, license compatibility, and maintenance status.

```bash
cm tool dep-audit --strict --check-security --licenses "MIT,Apache-2.0"
```

**Features:**
- Parses `cargo tree` output for comprehensive analysis
- License compatibility checking
- Security vulnerability scanning framework
- Maintenance status evaluation
- Exit codes based on severity levels

### 🧪 `test-gen` - Test Generation from AST
Generates test boilerplate by parsing Rust code and analyzing function signatures.

```bash
cm tool test-gen --file src/lib.rs --type unit --output tests/generated_tests.rs
```

**Features:**
- Parses Rust AST using `syn` crate
- Supports unit, integration, and property-based tests
- Generates realistic mock values for different data types
- Async function support with `tokio::test`
- Property-based testing for numeric functions

## Usage

### List All Available Tools
```bash
cm tool list
```

### Get Help for a Specific Tool
```bash
cm tool help <tool-name>
cm tool help bench-diff
```

### Run a Tool
```bash
cm tool run <tool-name> [options]
cm tool bench-diff --from HEAD~5 --to HEAD
```

### Direct Tool Execution
```bash
cm tool <tool-name> [options]
```

## Adding New Tools

### 1. Create Tool Module
Create a new file `src/tools/your_tool.rs`:

```rust
use super::{Tool, Result, ToolError, common_options, parse_output_format, OutputFormat};
use clap::{Arg, ArgMatches, Command};
use colored::*;

pub struct YourTool;

impl Tool for YourTool {
    fn name(&self) -> &'static str {
        "your-tool"
    }

    fn description(&self) -> &'static str {
        "Brief description of what your tool does"
    }

    fn command(&self) -> Command {
        Command::new(self.name())
            .about(self.description())
            .long_about("Detailed description with usage examples")
            .args(&[
                Arg::new("input")
                    .long("input")
                    .short('i')
                    .help("Input file path")
                    .required(true),
            ])
            .args(&common_options())
    }

    fn execute(&self, matches: &ArgMatches) -> Result<()> {
        let input = matches.get_one::<String>("input").unwrap();
        let output_format = parse_output_format(matches);
        let verbose = matches.get_flag("verbose");

        // Your tool implementation here
        println!("Processing: {}", input);

        Ok(())
    }
}

impl Default for YourTool {
    fn default() -> Self {
        Self::new()
    }
}

impl YourTool {
    pub fn new() -> Self {
        Self
    }
}
```

### 2. Register the Tool
Add your tool to `src/tools/mod.rs`:

```rust
pub fn create_registry() -> ToolRegistry {
    let registry = ToolRegistry::new();

    // Register all available tools
    registry
        .register(bench_diff::BenchDiffTool::new())
        .register(dep_audit::DepAuditTool::new())
        .register(test_gen::TestGenTool::new())
        .register(your_tool::YourTool::new())  // Add new tool here
}
```

### 3. Add Module Declaration
Add the module to `src/tools/mod.rs`:

```rust
pub mod bench_diff;
pub mod dep_audit;
pub mod test_gen;
pub mod your_tool;  // Add new module here
```

## Tool Development Guidelines

### Error Handling
- Use `ToolError` enum with contextual messages
- Provide suggestions for fixing errors
- Use proper error propagation with `?` operator

### Output Formats
- Support `--output human` (default), `--output json`, `--output table`
- Use `parse_output_format(matches)` helper
- Colorize human output with `colored` crate

### Performance
- Keep execution time under 1 second for simple operations
- Use efficient algorithms and data structures
- Add progress indicators for long operations

### Configuration
- Support `--verbose` flag for detailed output
- Support `--dry-run` flag for testing
- Consider adding `.cargo-mate.toml` configuration support

### Testing
- Add unit tests for core functionality
- Test error conditions and edge cases
- Test different output formats

### Documentation
- Provide clear help text and usage examples
- Document all command-line options
- Add code comments for complex logic

## Tool Categories

### Suggested Tool Types:
1. **Code Analysis**: Complexity analysis, dead code detection, API usage analysis
2. **Performance**: Memory profiling, CPU optimization suggestions, compilation analysis
3. **Security**: Vulnerability scanning, dependency security checks, code security analysis
4. **Development**: Documentation generation, API testing, database schema tools
5. **CI/CD**: Release management, changelog generation, deployment helpers
6. **Project Management**: TODO tracking, milestone planning, issue analysis

## Build Integration

Tools are automatically copied to `cargo-git/tools/` during the build process via `build.sh`, making them available for public distribution outside the main encrypted binary.

## Future Enhancements

- Tool chaining: `cm tool dep-audit --json | cm tool report-gen`
- Custom tool paths: Load external tools from `~/.cargo-mate/tools/`
- Tool aliases in configuration
- Plugin system for third-party tools
- Web-based tool marketplace
