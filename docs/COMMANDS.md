# 📚 Cargo Mate (cm) - Complete Command Reference

## Table of Contents
1. [Basic Commands](#basic-commands)
2. [Journey Commands](#journey-commands)
3. [Anchor Commands](#anchor-commands)
4. [Captain's Log Commands](#captains-log-commands)
5. [Tide Charts Commands](#tide-charts-commands)
6. [Treasure Map Commands](#treasure-map-commands)
7. [Mutiny Mode Commands](#mutiny-mode-commands)
8. [Version Management Commands](#version-management-commands)
9. [Build Optimization Commands](#build-optimization-commands)
10. [View Commands](#view-commands)
11. [Config Commands](#config-commands)
12. [Utility Commands](#utility-commands)
13. [Shell Integration](#shell-integration)

---

## Basic Commands

### `cm`
**Description**: Auto-builds release if Cargo.toml exists, or runs default journey if configured

**Usage**:
```bash
cm
```

**Behavior**:
- If `Cargo.toml` exists and no default journey: runs `cargo build --release`
- If default journey configured: runs that journey
- If no `Cargo.toml`: shows help

---

### `cm init`
**Description**: Initialize cargo-mate and automatically set up shell integration

**Usage**:
```bash
cm init
```

**What it does**:
- Creates local `.cg` config file
- Automatically installs shell integration
- Backs up your shell RC file
- Adds cargo() function to intercept cargo commands
- Creates aliases: `cm` and `cg`

**Shell Integration Added**:
```bash
# === Cargo Mate (cm) Integration ===
cargo() {
    cm exec "$@"
}
alias cm='cargo-mate'
alias cg='cm'
# === End Cargo Mate Integration ===
```

**Output**:
```
🚢 Initializing Cargo Mate...
✅ Local config created: .cg
🔧 Setting up shell integration...
📋 Backed up ~/.bashrc to ~/.bashrc.bak.cargo-mate
✅ Shell integration added to ~/.bashrc
📁 Error logs will be stored in ~/.shipwreck/

🎉 Cargo Mate initialized successfully!

⚡ Shell integration added. The following are now available:
   cargo - Run cargo through cargo-mate
   cm - Direct cargo-mate access
   cg - Alias for cargo-mate

🔄 Please run: source ~/.bashrc
   Or restart your terminal to activate
```

**After Running**:
- All `cargo` commands will be intercepted by cm
- You can use `cm` directly for cargo-mate commands
- `cg` is available as a shortcut

---

### `cm help`
**Description**: Show help information

**Usage**:
```bash
cm help
cm --help
cm <command> --help
```

---

### `cm --version`
**Description**: Show version information

**Usage**:
```bash
cm --version
```

---

### `cm activate`
**Description**: Activate shell integration immediately (alternative to manual sourcing)

**Usage**:
```bash
cm activate
```

**What it does**:
- Sources your shell configuration file (`.bashrc`, `.zshrc`, etc.)
- Activates cargo-mate integration without restarting terminal
- Checks if integration exists before attempting to activate
- Provides clear feedback about activation status

**Example**:
```bash
cm init              # Sets up shell integration
cm activate          # Activates it immediately
cargo --version      # Now uses cargo-mate
```

**Output**:
```
⚡ Activating Cargo Mate shell integration...
🔄 Sourcing /root/.bashrc
✅ Shell integration activated successfully!

🚢 You can now use:
   cargo - cargo commands go through cargo-mate
   cm - direct cargo-mate access
   cg - quick alias

🎯 Try it:
   cargo --version
   cm --help
```

---

## Journey Commands

### `cm journey record <name>`
**Description**: Start recording a journey (sequence of commands)

**Usage**:
```bash
cm journey record deploy-flow
# Execute your commands...
# Press Ctrl+D to stop recording
```

**Features**:
- Records all terminal I/O via PTY
- Captures command timing
- Stores working directory
- Optimizes by removing redundant commands

**Example**:
```bash
cm journey record build-and-test
cargo fmt
cargo build --release
cargo test --all
# Ctrl+D
```

---

### `cm journey play <name> [--dry-run]`
**Description**: Replay a recorded journey

**Usage**:
```bash
cm journey play deploy-flow
cm journey play deploy-flow --dry-run  # Preview without executing
```

**Features**:
- Variable substitution with `{{variables}}`
- Interactive mode with pause points
- Checkpoint validation
- Dry-run mode for preview

---

### `cm journey list`
**Description**: List all available journeys

**Usage**:
```bash
cm journey list
```

**Output**:
```
📚 Available journeys:
  • build-flow
  • deploy-prod
  • test-suite
```

---

### `cm journey export <name> <output>`
**Description**: Export a journey to a file

**Usage**:
```bash
cm journey export deploy-flow ./deploy.journey.json
```

---

### `cm journey import <path>`
**Description**: Import a journey from a file

**Usage**:
```bash
cm journey import ./shared-journey.json
```

---

### `cm journey publish <name> [--tags <tags>...]`
**Description**: Publish a journey to the marketplace (GitHub Gists)

**Usage**:
```bash
cm journey publish build-flow --tags rust cargo build
cm journey publish test-suite
```

**Options**:
- `--tags`: Add searchable tags to your journey

**Features**:
- Uses GitHub CLI (`gh`) to create public gists
- Adds metadata (author, tags, description)
- Returns shareable gist ID and URL
- Tracks published journeys locally

**Output**:
```
📤 Publishing journey 'build-flow' to GitHub Gist...
✅ Journey published successfully!
🔗 Gist URL: https://gist.github.com/username/abc123def456
📋 Share ID: abc123def456
```

---

### `cm journey download <gist-id>`
**Description**: Download a journey from the marketplace

**Usage**:
```bash
cm journey download abc123def456
```

**Features**:
- Downloads journey from GitHub gist
- Validates journey format
- Shows author and metadata
- Adds to local journey library

**Output**:
```
📥 Downloading journey from gist abc123def456...
✅ Journey 'build-flow' downloaded successfully!
📝 Description: Optimized Rust build workflow
👤 Author: username
🏷️ Tags: rust, cargo, build
```

---

### `cm journey search <query>`
**Description**: Search the marketplace for journeys

**Usage**:
```bash
cm journey search "rust build"
cm journey search testing
```

**Features**:
- Searches public GitHub gists
- Filters for Cargo Mate journeys
- Shows author, description, and gist ID
- Sorted by relevance

**Output**:
```
🔍 Searching for journeys matching 'rust build'...
Found 3 journey(s):

1. build-flow by alice
   Optimized Rust build workflow
   ID: abc123def456

2. release-process by bob
   Complete release automation
   ID: ghi789jkl012
```

---

### `cm journey published`
**Description**: List your published journeys

**Usage**:
```bash
cm journey published
```

**Features**:
- Shows all journeys you've published
- Includes gist IDs for reference
- Stored in `~/.shipwreck/journeys/.published.json`

**Output**:
```
📤 Your published journeys:
  • build-flow (abc123def456)
  • test-suite (ghi789jkl012)
  • release-process (mno345pqr678)
```

---

## Anchor Commands

### `cm anchor save <name> [--message <msg>]`
**Description**: Save current project state as an anchor point

**Usage**:
```bash
cm anchor save before-refactor
cm anchor save v1.0 --message "Pre-release state"
```

**What it saves**:
- Git commit hash
- Cargo.lock snapshot
- Source files (SHA256 verified)
- Environment variables
- Project metadata

---

### `cm anchor restore <name>`
**Description**: Restore project to a saved anchor point

**Usage**:
```bash
cm anchor restore before-refactor
```

**What it restores**:
- Git checkout to saved commit
- Cargo.lock file
- Modified/deleted files
- Working directory state

---

### `cm anchor list`
**Description**: List all saved anchors

**Usage**:
```bash
cm anchor list
```

**Output**:
```
⚓ Saved anchors:
⚓ before-refactor - 2024-01-20 14:30:00 (15 files)
   Major refactoring checkpoint
⚓ v1.0 - 2024-01-19 10:00:00 (23 files)
   Pre-release state
```

---

### `cm anchor show <name>`
**Description**: Show detailed information about an anchor

**Usage**:
```bash
cm anchor show before-refactor
```

---

### `cm anchor diff <name>`
**Description**: Show differences between current state and anchor

**Usage**:
```bash
cm anchor diff before-refactor
```

**Output**:
```
=== Diff from anchor 'before-refactor' ===

✨ Added files:
   + src/new_module.rs

📝 Modified files:
   ~ src/main.rs
   ~ Cargo.toml

🗑️  Deleted files:
   - src/old_module.rs
```

---

### `cm anchor auto [OPTIONS] <name>`
**Description**: Start auto-update mode for anchor (runs in background by default)

**Usage**:
```bash
# Default background mode (recommended)
cm anchor auto my-project
cargo anchor auto my-project

# Foreground mode (blocking)
cm anchor auto my-project --foreground
cargo anchor auto my-project --foreground
```

**Options**:
- `--foreground`: Run in blocking foreground mode instead of background

**Background Mode Features**:
- ✅ Non-blocking: Doesn't interfere with your terminal workflow
- 🔄 Real-time: Updates files immediately when changed
- ⚡ Efficient: Only updates specifically modified files
- 📊 Smart: Monitors only relevant directories automatically
- 🛑 Controllable: Easy start/stop management

**Background Mode Output**:
```
🚀 Starting auto-update for anchor: my-project
📁 Setting up file monitoring...
✅ Auto-update STARTED successfully!
🔄 Files will be updated automatically when changed
🛑 Use 'cargo anchor stop my-project' to stop monitoring

💡 Background daemon running for anchor 'my-project'
```

**Foreground Mode Output**:
```
📁 Monitoring 19 files for changes...
💡 Press Ctrl+C to stop auto-update

👀 Watching 2 directories
✅ Auto-update started! Files will be updated automatically.

🔄 Updated src/main.rs in anchor 'my-project'
```

**File Change Notifications**:
When files are modified in background mode, you'll see:
```
🔄 [14:32:15] Updated src/main.rs in anchor 'my-project'
🔄 [14:33:22] Updated Cargo.toml in anchor 'my-project'
```

**Examples**:
```bash
# Start monitoring your project (background by default)
cargo anchor auto my-rust-project

# Start in foreground if you want to see real-time updates
cargo anchor auto my-rust-project --foreground

# The auto-update runs continuously until stopped
# Edit files, and they'll be automatically saved to the anchor!
```

---

### `cm anchor stop <name>`
**Description**: Stop auto-update mode for anchor

**Usage**:
```bash
cm anchor stop my-project
cargo anchor stop my-project
```

**Output**:
```
🛑 Stopping auto-update for anchor: my-project
⚠️  Note: In this implementation, stopping requires restarting the shell
💡 Future versions will have proper daemon management
```

**Examples**:
```bash
# Stop auto-update for your project
cargo anchor stop my-rust-project
```

---

## Captain's Log Commands

### `cm log add <message> [--tags <tags>...]`
**Description**: Add an entry to the captain's log

**Usage**:
```bash
cm log add "Switched to async runtime"
cm log add "Fixed memory leak in parser" --tags bug performance
```

---

### `cm log search <query>`
**Description**: Search through log entries

**Usage**:
```bash
cm log search "memory"
cm log search "performance"
```

---

### `cm log timeline [days]`
**Description**: Show log timeline for specified days (default: 7)

**Usage**:
```bash
cm log timeline        # Last 7 days
cm log timeline 30     # Last 30 days
```

**Output**:
```
=== Captain's Log - Last 7 Days ===

📅 Saturday, January 20, 2024
  📝 14:30:00 - Added async support
      🏷️  async, feature
  ⚙️ 15:45:00 - cargo build --release
      ✅ Success (45.2s)
```

---

### `cm log export <path> [--format <format>]`
**Description**: Export logs to file (formats: json, markdown, html)

**Usage**:
```bash
cm log export ./project-log.md --format markdown
cm log export ./log.json --format json
cm log export ./report.html --format html
```

---

### `cm log analyze`
**Description**: Analyze log patterns and statistics

**Usage**:
```bash
cm log analyze
```

**Output**:
```
=== Captain's Log Analysis ===
📊 Total entries: 142
⚙️  Total commands: 89
✅ Successful builds: 67
❌ Failed builds: 22
📈 Success rate: 75.3%
⏱️  Average build time: 23.4s

🏷️  Most common tags:
   bug (15)
   performance (12)
   feature (10)
```

---

## Tide Charts Commands

### `cm tide show`
**Description**: Show interactive performance charts (TUI)

**Usage**:
```bash
cm tide show
```

**Controls**:
- `Tab` / `Shift+Tab`: Switch between views
- `q` / `Esc`: Quit
- Views: Overview, Performance, Dependencies, Trends

---

### `cm tide analyze`
**Description**: Analyze build performance and dependencies

**Usage**:
```bash
cm tide analyze
```

**Output**:
```
🔍 Analyzing dependency compile times...
✅ Timing data collected. Check target/cargo-timings/ for detailed report.
```

---

### `cm tide export <path>`
**Description**: Export build metrics to CSV

**Usage**:
```bash
cm tide export ./metrics.csv
```

**CSV Format**:
```csv
timestamp,command,duration,success,errors,warnings
2024-01-20T14:30:00Z,build,45.2,true,0,3
```

---

## Treasure Map Commands

### `cm map show`
**Description**: Display dependency tree visualization

**Usage**:
```bash
cm map show
```

**Output**:
```
🗺️  Treasure Map - Dependency Visualization

📦 my-project v0.1.0
├── 📚 serde v1.0.195
│   └── 📚 serde_derive v1.0.195
├── 📚 tokio v1.35.1
│   ├── 📚 mio v0.8.10
│   └── 📚 bytes v1.5.0
└── 📚 reqwest v0.11.23
```

---

### `cm map analyze`
**Description**: Analyze dependencies for issues

**Usage**:
```bash
cm map analyze
```

**Output**:
```
=== Dependency Analysis ===
📊 Total dependencies: 42
   Direct: 8
   Dev: 5
   Max depth: 4
💾 Total size: 15.3 MB

⚠️  2 duplicate dependencies found:
   rand has versions: 0.7.3, 0.8.5

📦 Largest dependencies:
   tokio v1.35.1 - 2.1 MB
   reqwest v0.11.23 - 1.8 MB
```

---

### `cm map export <path>`
**Description**: Export dependency graph as DOT file

**Usage**:
```bash
cm map export ./deps.dot
dot -Tpng deps.dot -o deps.png  # Generate image with graphviz
```

---

### `cm map path <from> <to>`
**Description**: Find dependency path between two crates

**Usage**:
```bash
cm map path serde tokio
```

**Output**:
```
📍 Path from serde to tokio:
  1. serde
  2. serde_json
  3. tower
  4. tokio
```

---

## Mutiny Mode Commands

### `cm mutiny activate <reason>`
**Description**: Activate mutiny mode to override cargo restrictions

**Usage**:
```bash
cm mutiny activate "Quick prototype testing"
```

---

### `cm mutiny deactivate`
**Description**: Deactivate mutiny mode

**Usage**:
```bash
cm mutiny deactivate
```

---

### `cm mutiny allow-warnings`
**Description**: Allow warnings for 1 hour

**Usage**:
```bash
cm mutiny allow-warnings
```

---

### `cm mutiny skip-tests`
**Description**: Skip tests when building

**Usage**:
```bash
cm mutiny skip-tests
```

---

### `cm mutiny force`
**Description**: Force build ignoring dirty state and lockfile

**Usage**:
```bash
cm mutiny force
```

---

### `cm mutiny yolo`
**Description**: DANGEROUS - Disable ALL checks for 30 minutes

**Usage**:
```bash
cm mutiny yolo
```

**⚠️ WARNING**: This disables:
- All lints
- All tests
- Format checking
- Security audits
- Lockfile checking

---

### `cm mutiny status`
**Description**: Show current mutiny mode status

**Usage**:
```bash
cm mutiny status
```

**Output**:
```
=== Mutiny Mode Status ===
Status: 🏴‍☠️ ACTIVE

📋 Active Overrides:
   allow_warnings - Temporarily allowing warnings
      Expires in: 45 minutes

🚩 Forced Flags:
   --cap-lints=warn

⏭️  Skipped Checks:
   test
```

---

## Version Management Commands

### `cm version init [version]`
**Description**: Initialize auto-versioning for the project

**Usage**:
```bash
cm version init              # Use default 1.0.0
cm version init 2.0.0        # Start with specific version
```

**What it does**:
- Creates `.v` file in project root
- Sets up auto-incrementing on cargo operations
- Configures versioning policies
- Integrates with Cargo.toml version field

**Output**:
```
🚢 Setting up auto-versioning for your project
Enter initial version number (default: 1.0.0): 2.0.0
Enable auto-incrementing? (Y/n): Y
Select increment policy:
1) Patch (1.0.0 -> 1.0.1) - Default
2) Minor (1.0.0 -> 1.1.0)
3) Major (1.0.0 -> 2.0.0)
Enter choice (1-3): 1
✅ Auto-versioning initialized with version 2.0.0
```

---

### `cm version info`
**Description**: Show current version information

**Usage**:
```bash
cm version info
```

**Output**:
```
🚢 Version Information:
══════════════════════════════════════════════════
📦 Current Version: 2.0.1
🔄 Auto-increment: Enabled
📊 Increment Policy: Patch
📁 Version File: .v
📋 Cargo.toml: 2.0.1
══════════════════════════════════════════════════
```

---

### `cm version increment <type>`
**Description**: Manually increment version

**Usage**:
```bash
cm version increment patch    # 2.0.1 -> 2.0.2
cm version increment minor    # 2.0.2 -> 2.1.0
cm version increment major    # 2.1.0 -> 3.0.0
```

---

### `cm version config <action> [value]`
**Description**: Configure versioning behavior

**Usage**:
```bash
cm version config show        # Show current configuration
cm version config enable       # Enable auto-incrementing
cm version config disable      # Disable auto-incrementing
cm version config policy patch # Set increment policy
```

**Policies**:
- `patch`: Increment patch version (1.0.0 -> 1.0.1)
- `minor`: Increment minor version (1.0.0 -> 1.1.0)
- `major`: Increment major version (1.0.0 -> 2.0.0)

---

## Build Optimization Commands

### `cm optimize aggressive`
**Description**: Apply aggressive optimizations for maximum build speed

**Usage**:
```bash
cm optimize aggressive
```

**What it applies**:
- Parallel jobs: CPU core count (8 on 8-core system)
- Incremental compilation: true
- Optimization level: 1 (basic optimizations)
- Codegen units: 256 (maximum parallelism)
- Debug level: 1 (reduced debug info)
- LTO: false (faster dev builds)

**Output**:
```
📋 Backed up Cargo.toml to Cargo.toml.backup
✅ Applied Aggressive optimizations to Cargo.toml

🚀 Build Optimization Summary:
══════════════════════════════════════════════════
📊 Parallel Jobs: 8
🔄 Incremental: true
⚡ Optimization Level: 1
🐛 Debug Level: 1
🏗️  Codegen Units: 256
🔗 Link-Time Optimization: false

🌍 Environment Variables:
  CARGO_BUILD_JOBS = "8"
  CARGO_INCREMENTAL = "1"
══════════════════════════════════════════════════
💡 Run 'cargo build' to see the speed improvements!
```

---

### `cm optimize balanced`
**Description**: Apply balanced optimizations for good speed/stability

**Usage**:
```bash
cm optimize balanced
```

**What it applies**:
- Parallel jobs: CPU core count / 2 (4 on 8-core system)
- Incremental compilation: true
- Optimization level: 1 (basic optimizations)
- Codegen units: 128 (moderate parallelism)
- Debug level: 1 (reduced debug info)
- LTO: false (faster dev builds)

---

### `cm optimize conservative`
**Description**: Apply conservative optimizations for maximum stability

**Usage**:
```bash
cm optimize conservative
```

**What it applies**:
- Parallel jobs: 2
- Incremental compilation: true
- Optimization level: 0 (no optimizations)
- Codegen units: 64 (minimal parallelism)
- Debug level: 2 (full debug info)
- LTO: false (faster dev builds)

---

### `cm optimize custom [JOBS] [INCREMENTAL] [OPT_LEVEL] [DEBUG_LEVEL] [CODEGEN_UNITS]`
**Description**: Apply custom optimizations with specific values

**Usage**:
```bash
cm optimize custom 16 true 2 1 512
```

**Parameters**:
- `JOBS`: Number of parallel jobs (default: 4)
- `INCREMENTAL`: Enable incremental compilation (default: true)
- `OPT_LEVEL`: Optimization level 0-3 (default: 1)
- `DEBUG_LEVEL`: Debug level 0-2 (default: 1)
- `CODEGEN_UNITS`: Codegen units for parallelism (default: 128)

---

### `cm optimize status`
**Description**: Show current optimization status

**Usage**:
```bash
cm optimize status
```

**Output**:
```
🔍 Current Build Optimization Status:
══════════════════════════════════════════════════
📊 Build Configuration:
  incremental: true
  jobs: 8

⚡ Dev Profile:
  codegen-units: 256
  debug: 1
  lto: false
  opt-level: 1

🌍 Environment Variables:
  CARGO_BUILD_JOBS: "8"
  CARGO_INCREMENTAL: "1"
══════════════════════════════════════════════════
```

---

### `cm optimize recommendations`
**Description**: Show optimization recommendations based on your system

**Usage**:
```bash
cm optimize recommendations
```

**Output**:
```
💡 Build Optimization Recommendations:
══════════════════════════════════════════════════
🖥️  CPU Cores: 8
📊 Recommended Jobs: 8

🚀 Aggressive Profile:
  - Parallel jobs: 8
  - Incremental: true
  - Opt level: 1 (basic optimizations)
  - Codegen units: 256 (maximum parallelism)
  - Debug: 1 (reduced debug info)

⚖️  Balanced Profile:
  - Parallel jobs: 4
  - Incremental: true
  - Opt level: 1 (basic optimizations)
  - Codegen units: 128 (moderate parallelism)
  - Debug: 1 (reduced debug info)

🛡️  Conservative Profile:
  - Parallel jobs: 2
  - Incremental: true
  - Opt level: 0 (no optimizations)
  - Codegen units: 64 (minimal parallelism)
  - Debug: 2 (full debug info)

💡 Use 'cm optimize aggressive' for maximum speed
💡 Use 'cm optimize balanced' for good speed/stability
💡 Use 'cm optimize conservative' for maximum stability
```

---

### `cm optimize restore`
**Description**: Restore original Cargo.toml from backup

**Usage**:
```bash
cm optimize restore
```

**Output**:
```
✅ Restored Cargo.toml from backup
```

---

## View Commands

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

### `cm activate`
**Description**: Activate shell integration immediately

**Usage**:
```bash
cm activate
```

**What it does**:
- Sources your shell configuration file (`.bashrc`, `.zshrc`, etc.)
- Activates cargo-mate integration without restarting terminal
- Checks if integration exists before attempting to activate
- Provides clear feedback about activation status

**Example**:
```bash
cm init              # Sets up shell integration
cm activate          # Activates it immediately
cargo --version      # Now uses cargo-mate
```

**Output**:
```
⚡ Activating Cargo Mate shell integration...
🔄 Sourcing /root/.bashrc
✅ Shell integration activated successfully!

🚢 You can now use:
   cargo - cargo commands go through cargo-mate
   cm - direct cargo-mate access
   cg - quick alias

🎯 Try it:
   cargo --version
   cm --help
```

---

## Config Commands

### `cm config init`
**Description**: Initialize local project config

**Usage**:
```bash
cm config init
```

**Creates**: `.cg` file in project root

---

### `cm config set <key> <value> [--local]`
**Description**: Set a configuration value

**Usage**:
```bash
cm config set project.name "my-app" --local
cm config set build.default_profile release
cm config set auto_fix.format_on_save true
```

**Available Keys**:
- `project.name`
- `project.default_journey`
- `project.theme`
- `project.auto_checklist`
- `auto_fix.format_on_save`
- `auto_fix.clippy_on_build`
- `build.default_profile`
- `build.incremental`

---

### `cm config get <key>`
**Description**: Get a configuration value

**Usage**:
```bash
cm config get project.name
cm config get build.default_profile
```

---

### `cm config list`
**Description**: List all configuration

**Usage**:
```bash
cm config list
```

---

### `cm config shortcut <name> <command> [--local]`
**Description**: Add a command shortcut

**Usage**:
```bash
cm config shortcut b "build --release" --local
cm config shortcut t "test --all"
cm config shortcut d "doc --open"
```

---

### `cm config hook <type> <command> [--local]`
**Description**: Add a build hook

**Usage**:
```bash
cm config hook pre_build "cargo fmt"
cm config hook post_build "cargo test"
cm config hook on_error "cm checklist"
cm config hook on_success "echo 'Build successful!'"
```

**Hook Types**:
- `pre_build`: Run before cargo commands
- `post_build`: Run after successful builds
- `on_error`: Run when build fails
- `on_success`: Run when build succeeds

---

## Utility Commands

### `cm checklist`
**Description**: Show current error/warning checklist

**Usage**:
```bash
cm checklist
```

**Output**:
```
=== Build Checklist [3 errors, 2 warnings] ===
Generated: 2024-01-20 14:30:00

ERRORS (must fix):
[ ] Fix E0308 in src/main.rs:42 - mismatched types: expected String, found &str
[ ] Fix E0384 in src/lib.rs:13 - cannot assign twice to immutable variable
[ ] Fix E0433 in src/parser.rs:7 - failed to resolve: use of undeclared crate

WARNINGS (consider fixing):
[ ] dead_code in src/utils.rs:23 - function `helper` is never used
[ ] unused_imports in src/main.rs:3 - unused import: `std::fs`
```

---

### `cm history <type> [limit]`
**Description**: Show build history

**Usage**:
```bash
cm history summary 20    # Last 20 builds summary
cm history errors 10     # Last 10 errors
cm history warnings 10   # Last 10 warnings
```

**Output (summary)**:
```
=== Build History Summary ===
📊 Last 20 builds:
  ✅ Successful: 15
  ❌ Failed: 5

📈 Recent builds:
  ✅ 2024-01-20 14:30:00 - cargo build - 🔴 0 ⚠️ 3
  ❌ 2024-01-20 14:00:00 - cargo test - 🔴 2 ⚠️ 1
```

---

### `cm exec <cargo-command> [args...]`
**Description**: Execute cargo command through cm wrapper

**Usage**:
```bash
cm exec build --release
cm exec test --all
cm exec clippy
```

**Note**: Any unrecognized cm command automatically falls through to cargo

---

## Shell Integration

### `cm install`
**Description**: Install shell integration (bash, zsh, fish)

**Usage**:
```bash
cm install
```

**What it does**:
1. Detects your shell
2. Backs up your RC file
3. Adds cargo function override
4. Creates aliases (cm, cg)
5. Installs completions
6. Sets up auto-config loading

**After installation**:
```bash
source ~/.bashrc  # or ~/.zshrc
# OR use the new command:
cm activate
```

**Result**:
- `cargo` commands routed through cm
- `cm` available directly
- `cg` as quick alias
- Tab completion enabled
- `.cg` files auto-loaded in directories

---

## Environment Variables

### Configuration
- `CM_PROJECT_CONFIG`: Path to project config file
- `CM_DEFAULT_PROFILE`: Default build profile
- `CM_PARALLEL_JOBS`: Number of parallel build jobs
- `CM_AUTO_FIX`: Enable auto-fix features
- `CM_THEME`: UI theme (nautical by default)

### Example:
```bash
export CM_DEFAULT_PROFILE=release
export CM_AUTO_FIX=true
cm  # Will use these settings
```

---

## Config File Examples

### Global Config (`~/.shipwreck/config.toml`)
```toml
[project]
theme = "nautical"
auto_checklist = true
track_performance = true

[shortcuts]
b = "build --release"
t = "test --all"
c = "check"

[auto_fix]
format_on_save = true
clippy_on_build = false

[build]
default_profile = "dev"
incremental = true
```

### Project Config (`.cg`)
```toml
[project]
name = "my-awesome-project"
default_journey = "dev-cycle"

[shortcuts]
dev = "run --bin dev-server"
prod = "build --release --target x86_64-unknown-linux-musl"

[hooks]
pre_build = ["cargo fmt --check"]
post_build = ["cargo test --quiet"]
on_error = ["cm checklist"]

[version]
auto_increment = true
increment_policy = "patch"
```

---

## Tips & Tricks

### 1. Quick Development Cycle
```bash
cm journey record dev-cycle
cargo fmt
cargo clippy
cargo test
cargo run
# Ctrl+D

# Later:
cm  # If default_journey = "dev-cycle"
```

### 2. Safe Experimentation
```bash
cm anchor save safe-point
cm mutiny yolo  # Go wild for 30 min
# ... experiment ...
cm anchor restore safe-point  # If things go wrong
```

### 3. Team Workflow Sharing
```bash
cm journey record onboarding
# Show new developer the full setup
cm journey export onboarding team-onboarding.json
# Share the file
```

### 4. Performance Tracking
```bash
# After each build, cm automatically tracks metrics
cm tide show  # View beautiful charts
cm tide export metrics.csv  # For external analysis
```

### 5. Dependency Auditing
```bash
cm map analyze  # Quick dependency check
cm map path vulnerable-crate my-crate  # Trace dependencies
```

### 6. Build Optimization
```bash
# Get recommendations for your system
cm optimize recommendations

# Apply aggressive optimizations
cm optimize aggressive

# Check what was changed
cm optimize status

# Restore if needed
cm optimize restore
```

### 7. Auto-Versioning
```bash
# Initialize with custom version
cm version init 2.0.0

# Version auto-increments on every build
cm check    # 2.0.0 -> 2.0.1
cm build    # 2.0.1 -> 2.0.2

# Manual version bump
cm version increment minor  # 2.0.2 -> 2.1.0
```

---

## Error Codes

| Code | Description | Solution |
|------|-------------|----------|
| File not found | Journey/Anchor doesn't exist | Check name with `list` command |
| Permission denied | Can't write to ~/.shipwreck | Check directory permissions |
| Invalid config | Config file malformed | Check TOML syntax |
| Cargo not found | Cargo not in PATH | Install Rust toolchain |

---

## Troubleshooting

### Issue: Commands not working after install
**Solution**: Source your shell RC file or restart terminal
```bash
source ~/.bashrc  # or ~/.zshrc
```

### Issue: Tide charts not displaying
**Solution**: Requires terminal with UTF-8 support and 80+ columns

### Issue: Journey recording captures too much
**Solution**: Edit journey file in ~/.shipwreck/journeys/ to remove unwanted commands

### Issue: Mutiny mode won't deactivate
**Solution**: Settings expire automatically, or run:
```bash
cm mutiny deactivate
rm ~/.shipwreck/mutiny.toml  # Nuclear option
```

### Issue: Version not auto-incrementing
**Solution**: Check if auto-increment is enabled:
```bash
cm version config show
cm version config enable  # If disabled
```

### Issue: Build optimizations not working
**Solution**: Check current status and restore if needed:
```bash
cm optimize status
cm optimize restore  # If configuration is corrupted
```

---

## More Information

- GitHub: [cargo-mate repository](#)
- Issues: [Report bugs](#)
- Docs: See README.md for overview
- Version: Run `cm --version`

---

*Built with ❤️ for the Rust community*