# 🧹 Sweep

**Source**: `cargo-mate/captain/src/cmd/sweep.rs` and `cargo-mate/captain/src/sweeping/mod.rs`

**Intelligently sweep away the `println!` mess that AI coding assistants leave behind.**

```rust
// Before: What AI assistants do to your code
fn process_data(input: &str) -> Result<String> {
    println!("Starting process_data");  // AI was here
    eprintln!("DEBUG: input = {:?}", input);  // AI was debugging
    
    let result = transform(input)?;
    println!("HERE");  // Classic AI marker
    dbg!(&result);  // AI wanted to check this
    
    println!("got here");  // AI breadcrumb
    Ok(result)
}

// After: sweep clean
fn process_data(input: &str) -> Result<String> {
    let result = transform(input)?;
    Ok(result)
}
```

## 🎯 The Problem

AI coding assistants are incredibly helpful, but they have an annoying habit:
- Adding `println!("HERE")` everywhere as markers
- Spamming `eprintln!("DEBUG: {:?}", variable)` for debugging
- Leaving `dbg!()` macros all over production code
- Creating breadcrumb trails with `println!("entering function")`

**Sweep** is purpose-built to clean up this specific mess, intelligently and safely.

## ✨ Key Features

### 🧠 **Smart Pattern Memory** (NEW!)
```bash
# Sweep with pattern learning - it remembers your decisions!
sweep sweep -p

# When it finds println!("HERE"):
Pattern detected: HERE
What should I do with statements containing this pattern?
  [r] Remove this one
  [k] Keep this one  
  [R] Always remove 'HERE'  # <-- Remember this pattern!
  [K] Always keep 'HERE'
  [s] Skip file
```

Your decisions are saved in `.sweep.toml` and applied automatically next time!

### 🎯 **Intelligent Detection**
- AST-based parsing (not regex) for accuracy
- Context-aware: knows if code is in `main()`, tests, or regular functions
- Recognizes AI patterns: "HERE", "got here", "DEBUG:", etc.

### 🚀 **One-Command Cleanup**
```bash
# Quick sweep with auto-approval
sweep sweep -y

# Interactive mode
sweep sweep -i

# Learn patterns as you go
sweep sweep -p
```

## 📦 Installation

```bash
cargo install sweep

# Or from source
git clone https://github.com/yourusername/sweep
cd sweep
cargo install --path .
```

## 🎮 Quick Start

```bash
# See what needs cleaning
sweep scan

# Dry run first (see what would be removed)
sweep sweep -n

# Clean with pattern memory
sweep sweep -p

# Quick clean (auto-approve all)
sweep sweep -y

# Keep prints in main() and tests
sweep sweep --keep-main --keep-tests
```

## 📖 Commands

**Implementation**: All commands are handled by `sweeping/mod.rs:run_sweep()` which matches on `SweepCommands` enum.

### `cm sweep scan` - Find the mess

**Source**: `sweeping/mod.rs:run_sweep()` - `SweepCommands::Scan` case

**Implementation** (verified in `sweeping/mod.rs:70-90`):
- Creates `Sweeper` instance from `sweeping::src::away::Sweeper`
- Calls `scan_directory(&path, include_tests, include_examples)` to find print statements
- Displays results using `display_statement()` for each found statement
- Exports to JSON if `--export` flag is provided via `export_to_json()`

```bash
cm sweep scan                    # Scan current directory
cm sweep scan ~/my-project      # Scan specific directory
cm sweep scan --include-tests   # Include test files
cm sweep scan --export mess.json # Export findings
```

**Options** (verified in `sweeping/mod.rs:13-22`):
- `path`: Path to scan (default: `.`)
- `--include-tests`: Include test files in scan
- `--include-examples`: Include example files in scan
- `--export <PATH>`: Export results to JSON file

### `cm sweep sweep` - Clean it up (main command!)

**Source**: `sweeping/mod.rs:run_sweep()` - `SweepCommands::Sweep` case

**Implementation** (verified in `sweeping/mod.rs:100-140`):
- Creates `Sweeper` instance
- Scans directory with `scan_directory(&path, !keep_tests, !keep_examples)`
- Loads config from `.sweep.toml` via `load_config()`
- Creates `SweepOptions` struct with all flags
- Calls `sweep_files()` to perform the actual cleaning

```bash
cm sweep sweep              # Interactive cleaning
cm sweep sweep --dry-run    # Dry run
cm sweep sweep --prompt     # Prompt mode with pattern memory
cm sweep sweep --yes        # Auto-approve all removals
cm sweep sweep --interactive # Interactive (confirm each)
cm sweep sweep --backup     # Create .bak files first
```

**Options** (verified in `sweeping/mod.rs:24-48`):
- `path`: Path to clean (default: `.`)
- `--dry-run`: Show what would be changed without modifying files
- `--interactive`: Confirm each removal
- `--prompt`: Prompt mode with pattern memory
- `--keep-main`: Keep prints in main() functions
- `--keep-tests`: Keep prints in test functions
- `--keep-examples`: Keep prints in example functions
- `--backup`: Create backup files before modification
- `--yes`: Auto-approve all removals
- `--pristine`: Additional cleaning mode
- `--format`: Format code after cleaning
- `--organize-imports`: Organize imports after cleaning
- `--add-docs`: Add documentation
- `--fix-clippy`: Fix clippy warnings

### `cm sweep convert` - Upgrade to proper logging

**Source**: `sweeping/mod.rs:run_sweep()` - `SweepCommands::Convert` case

**Implementation** (verified in `sweeping/mod.rs:142-200`):
- Scans directory for print statements
- Maps `println_level` and `eprintln_level` strings to `LogLevel` enum (Trace, Info, Warn, Error, Debug)
- Calls `convert_statement_in_file()` for each statement found
- Optionally adds log dependency to Cargo.toml if `--add-dependency` is set

```bash
cm sweep convert                        # Convert to log statements
cm sweep convert --add-dependency      # Also add log to Cargo.toml
cm sweep convert --println-level info  # Custom log levels
```

**Options** (verified in `sweeping/mod.rs:50-58`):
- `path`: Path to convert (default: `.`)
- `--println-level <LEVEL>`: Log level for println! (trace, info, warn, error, debug)
- `--eprintln-level <LEVEL>`: Log level for eprintln! (trace, info, warn, error, debug)
- `--dry-run`: Show what would be converted without modifying files
- `--add-dependency`: Add log crate to Cargo.toml

### `cm sweep analyze` - Understand the problem

**Source**: `sweeping/mod.rs:run_sweep()` - `SweepCommands::Analyze` case

**Implementation** (verified in `sweeping/mod.rs:202-215`):
- Scans directory for print statements
- Calls `analyze_patterns(&statements, top)` to generate analysis report
- Displays report using `display_report()`

```bash
cm sweep analyze           # Show statistics and patterns
cm sweep analyze --top 20  # Show top 20 messiest files
```

**Options** (verified in `sweeping/mod.rs:60-65`):
- `path`: Path to analyze (default: `.`)
- `--top <NUM>`: Number of top results to show (default: 10)

### `cm sweep init` - Initialize configuration

**Source**: `sweeping/mod.rs:run_sweep()` - `SweepCommands::Init` case

**Implementation** (verified in `sweeping/mod.rs:217-230`):
- Checks if `.sweep.toml` exists
- If exists and `--force` not set, warns and exits
- Calls `create_default_config(&config_path)` to create default configuration

```bash
cm sweep init              # Create .sweep.toml
cm sweep init --force      # Overwrite existing config
```

**Options** (verified in `sweeping/mod.rs:66-67`):
- `--force`: Overwrite existing configuration file

## ⚙️ Configuration (.sweep.toml)

Sweep automatically creates and updates `.sweep.toml`:

```toml
# Patterns to always keep
keep_patterns = ["Error:", "Warning:", "Usage:", "Version:"]

# Patterns to always remove  
remove_patterns = ["HERE", "got here", "DEBUG:", "TODO:", "FIXME:"]

# Skip these directories
skip_dirs = ["target", ".git", "vendor"]

# Default behaviors
keep_in_main = true      # Keep prints in main()
keep_in_tests = true     # Keep prints in tests
keep_in_examples = true  # Keep prints in examples

# Pattern memory from -p flag (automatically managed)
[remembered_patterns]
"HERE" = { pattern = "HERE", action = "AlwaysRemove", created_at = "2024-01-20T10:30:00Z" }
"Error:" = { pattern = "Error:", action = "AlwaysKeep", created_at = "2024-01-20T10:31:00Z" }
```

## 🎯 Real-World Workflows

### After AI Pair Programming
```bash
# 1. See the damage
sweep analyze

# 2. Clean with pattern learning
sweep sweep -p

# 3. Next time, it remembers your preferences!
sweep sweep -y
```

### Prepare for Production
```bash
# Remove debug prints but keep important ones
sweep sweep --keep-main --backup

# Convert remaining to proper logs
sweep convert --add-dependency
```

### Quick Cleanup
```bash
# Just sweep it all away
sweep sweep -y
```

## 🤖 Why "Sweep"?

- **Short** - Easy to type, easy to remember
- **Clear** - You're sweeping away the mess
- **Action-oriented** - It's not analyzing or thinking, it's doing

## 🛡️ Safety Features

- **Dry run by default** for scanning
- **Pattern memory** reduces mistakes over time
- **Context awareness** (won't remove prints from main/tests unless told)
- **Backup option** for paranoid days
- **Interactive modes** for careful review

## 📊 Example Output

```bash
$ sweep analyze

📊 Sweep Analysis Report
════════════════════════════════════════════════════════════

Total Statements: 127

Distribution by Type:
  println! 87
  eprintln! 23  
  dbg! 17

Top Files:
  src/processor.rs - 23 statements
  src/handler.rs - 18 statements

Common Patterns:
  HERE markers: 34
  DEBUG markers: 28
  Error messages: 12

💡 Recommendations:
  🤖 Looks like AI assistants have been here!
  🧹 Run 'sweep sweep -y' to quickly clean these up
```

## 🔄 Integration Ideas

### Git Pre-commit Hook
```bash
#!/bin/sh
if [ $(sweep scan | grep "Total:" | awk '{print $2}') -gt 20 ]; then
    echo "⚠️  Too many debug prints! Run 'sweep sweep' to clean up."
    exit 1
fi
```

### VS Code Task
```json
{
    "label": "Sweep Clean",
    "type": "shell",
    "command": "sweep sweep -p",
    "problemMatcher": []
}
```

### Cargo Alias
```toml
# In .cargo/config.toml
[alias]
clean-prints = "sweep sweep -y"
```

## 🎉 The Sweep Promise

**No more manual hunting for debug statements!** 

Sweep learns your patterns, respects your code structure, and makes cleaning up after AI assistants a one-command operation.

## 📄 License

MIT OR Apache-2.0

## 🙏 Credits

Built out of frustration with AI assistants who think every line needs a `println!`. 

Special thanks to all the `println!("HERE")` statements that died so this tool could live. You won't be missed. 😄

---

**Remember:** A clean codebase is a happy codebase. Sweep early, sweep often! 🧹

# 🧹 Sweep Pattern Memory - Example Session

## First Time Using Sweep

```bash
$ sweep sweep -p

🔍 Sweeping for print statements...
✅ Found 47 print statements in 12 files

🧹 Sweeping: src/handler.rs

📍 src/handler.rs
  println! line 23 col 5 - println!("HERE") in fn process

Pattern detected: HERE
What should I do with statements containing this pattern?
  [r] Remove this one
  [k] Keep this one
  [R] Always remove 'HERE'
  [K] Always keep 'HERE'
  [s] Skip file
Choice: R

💾 Remembering: always remove patterns with 'HERE'

📍 src/handler.rs
  eprintln! line 45 col 9 - eprintln!("Error: {}", e) in fn handle_error

Pattern detected: Error:
What should I do with statements containing this pattern?
  [r] Remove this one
  [k] Keep this one
  [R] Always remove 'Error:'
  [K] Always keep 'Error:'
  [s] Skip file
Choice: K

💾 Remembering: always keep patterns with 'Error:'

📍 src/processor.rs
  println! line 12 col 5 - println!("DEBUG: processing {}", id) in fn process

Pattern detected: DEBUG:
What should I do with statements containing this pattern?
  [r] Remove this one
  [k] Keep this one
  [R] Always remove 'DEBUG:'
  [K] Always keep 'DEBUG:'
  [s] Skip file
Choice: R

💾 Remembering: always remove patterns with 'DEBUG:'

[... continues through files ...]

🧹 Sweep Summary
════════════════════════════════════════════════════════════
  🗑️ Removed: 38
  ✅ Kept: 9
  📝 Files modified: 8
```

## Second Time - It Remembers!

```bash
$ sweep sweep -y

🔍 Sweeping for print statements...
✅ Found 23 print statements in 5 files

🧹 Found 23 statements to potentially sweep

[Automatically applies remembered patterns:
 - Removes all "HERE" prints
 - Removes all "DEBUG:" prints  
 - Keeps all "Error:" prints
 No prompting needed!]

✨ Sweep complete!

🧹 Sweep Summary
════════════════════════════════════════════════════════════
  🗑️ Removed: 21
  ✅ Kept: 2
  📝 Files modified: 5
```

## Check Your Pattern Memory

```bash
$ cat .sweep.toml

# ... other config ...

[remembered_patterns]
"HERE" = { pattern = "HERE", action = "AlwaysRemove", created_at = "2024-01-20T10:30:00Z" }
"Error:" = { pattern = "Error:", action = "AlwaysKeep", created_at = "2024-01-20T10:31:00Z" }
"DEBUG:" = { pattern = "DEBUG:", action = "AlwaysRemove", created_at = "2024-01-20T10:32:00Z" }
"got here" = { pattern = "got here", action = "AlwaysRemove", created_at = "2024-01-20T10:33:00Z" }
"Warning:" = { pattern = "Warning:", action = "AlwaysKeep", created_at = "2024-01-20T10:34:00Z" }
```

## Reset Pattern Memory

```bash
# Edit .sweep.toml and remove entries from [remembered_patterns]
# Or reinitialize completely:
$ sweep init --force
```

## Pattern Memory Benefits

1. **Learns Your Preferences** - After the first run, it knows what you want
2. **Speeds Up Cleaning** - No more answering the same questions
3. **Team Consistency** - Share `.sweep.toml` with your team
4. **AI-Specific** - Perfect for repeated AI patterns like "HERE", "DEBUG:", etc.
5. **Override Anytime** - Use `-i` flag to go back to manual mode



SWEEP
[package]
name = "sweep"
version = "0.1.0"
edition = "2021"
authors = ["Your Name <your.email@example.com>"]
description = "🧹 Intelligently sweep away println! and eprintln! debug statements that AI assistants love to spam"
license = "MIT OR Apache-2.0"
keywords = ["rust", "println", "cleanup", "debugging", "ai-cleanup"]
categories = ["development-tools", "command-line-utilities"]

[dependencies]
anyhow = "1.0"
clap = { version = "4.5", features = ["derive"] }
colored = "2.1"
indicatif = "0.17"
regex = "1.10"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
syn = { version = "2.0", features = ["full", "parsing", "visit", "visit-mut", "extra-traits"] }
quote = "1.0"
proc-macro2 = "1.0"
toml = "0.8"
walkdir = "2.5"

[dev-dependencies]
tempfile = "3.10"
assert_cmd = "2.0"
predicates = "3.1"

[profile.release]
opt-level = 3
lto = true
codegen-units = 1
strip = true