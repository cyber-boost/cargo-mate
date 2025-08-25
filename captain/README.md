<img src="https://raw.githubusercontent.com/cyber-boost/cargo-mate/refs/heads/main/github.png" alt="Cargo Mate: Rust development companion" width="600">

# Cargo Mate

A Rust development companion that enhances cargo with intelligent workflows, state management, performance optimization, and comprehensive project monitoring.

## Table of Contents

- [Quick Start](#-quick-start)
- [Command Reference](#-command-reference)
  - [Version Commands](#version-commands)
  - [View Commands](#view-commands)
  - [Log Commands](#log-commands)
  - [Journey Commands](#journey-commands)
  - [Anchor Commands](#anchor-commands)
  - [Tide Commands](#tide-commands)
  - [Scrub Commands](#scrub-commands)
  - [Map Commands](#map-commands)
  - [Mutiny Commands](#mutiny-commands)
  - [Config Commands](#config-commands)
  - [Optimize Commands](#optimize-commands)
  - [Checklist Commands](#checklist-commands)
  - [WTF Commands](#wtf-commands)
  - [Idea Commands](#idea-commands)
  - [User Commands](#user-commands)
  - [Tool Commands](#tool-commands)
  - [General Commands](#general-commands)
- [Installation](#-installation)
- [Configuration](#️-configuration)
- [Examples](#-examples)
- [License](#-license)

## 🚀 Installation

### Option 1: Quick & Dirty Installer (Recommended - Works Everywhere... in Theory)
```bash
curl -sSL https://get.cargo.do/mate | bash
```
✅ No compilation needed, auto-detects platform, handles all dependencies

### Option 2: Control & Freaky (Direct Download - Choose Your Platform and Speed)
```bash
# Linux x86_64
wget https://get.cargo.do/linux-x86-64.tar.gz
tar -xzf linux-x86-64.tar.gz && ./install.sh

# Linux ARM64
wget https://get.cargo.do/linux-arm64.tar.gz
tar -xzf linux-arm64.tar.gz && ./install.sh

# macOS Intel
wget https://get.cargo.do/macos-x86-64.tar.gz
tar -xzf macos-x86-64.tar.gz && ./install.sh

# macOS Apple Silicon  
wget https://get.cargo.do/macos-arm64.tar.gz
tar -xzf macos-arm64.tar.gz && ./install.sh

# Windows
wget https://get.cargo.do/windows-x86-64.tar.gz
tar -xzf windows-x86-64.tar.gz
# Run install.ps1 in PowerShell
```

### Option 3: Natural & Confused (Requires Build Tools)
```bash
# Prerequisites: apt install build-essential (or equivalent)
cargo install cargo-mate
cm install && cm activate
```
⚠️ **Note**: Requires C compiler/linker. If you get "linker cc not found", use Option 1 or 2 instead.

### Troubleshooting
- **"linker cc not found"**: Install build-essential first, or use the curl/wget installers
- **"GLIBC_2.32 not found"**: Use the universal installer (Option 1) which auto-selects compatible version
- **Behind firewall**: Use Option 2 to download manually

### Version Commands
```bash
cm version                 # Display overview of version management capabilities and current version
cm version init [<ver>]    # Initialize semantic versioning system with optional starting version number
cm version info            # Show detailed information about current version and version management status
cm version increment <type> # Increase version number by patch, minor, or major increment type
cm version set <version>   # Manually assign a specific version number to the project
cm version history         # Display chronological list of all version changes and releases
cm version update-cargo    # Synchronize version number from version management into Cargo.toml file
cm version config enable   # Activate automatic version incrementation on successful builds
cm version config disable  # Deactivate automatic version incrementation feature
cm version config policy   # Configure the rules for automatic version incrementation behavior
cm version config show     # Display current version management configuration settings

cargo publish and cargo build auto version
```

### View Commands
```bash
cm view                    # Display overview of all available viewing and monitoring capabilities
cm view errors             # Show comprehensive list of all current compilation errors and warnings
cm view artifacts          # Display locations and details of all generated build artifacts and files
cm view scripts            # Show outputs and results from executed build scripts
cm view history            # Present detailed chronological history of all build operations and results
cm view checklist          # Display actionable checklist of errors with suggested fixes and solutions
cm view all                # Show consolidated view of all build results, errors, and artifacts in one interface
cm view latest             # Provide quick overview of most recent build issues and problems
cm view open               # Launch file explorer to navigate and examine build result locations
```

### Captain's Log Commands (Natural language build notes with automatic tagging and search.)
```bash
cm log                     # Display overview of all project log entries and logging capabilities
cm log add <message>       # Record a new timestamped log entry with custom message for project tracking
cm log search <query>      # Find log entries containing specific keywords or phrases
cm log timeline <days>     # Show chronological view of log entries for specified number of recent days
cm log export <path>       # Save all log entries to external file format for backup or analysis
cm log analyze             # Examine log patterns and generate statistics about project activity
```

### Journey Commands
```bash
cm journey                 # Show overview of all recorded command sequences and available actions
cm journey record <name>   # Begin recording a new command sequence for later playback
cm journey play <name>     # Execute a previously recorded command sequence exactly as recorded
cm journey list            # Display all locally stored recorded command sequences
cm journey export <name>   # Save a recorded command sequence to an external file for sharing or backup
cm journey import <path>   # Load a previously exported command sequence into local storage
cm journey publish <name>  # Share a command sequence publicly on the marketplace for others to use
cm journey download <id>   # Download and install a publicly shared command sequence from the marketplace
cm journey search <query>  # Find command sequences in the public marketplace matching your search terms
cm journey published       # Display all command sequences you've published to the marketplace
```

### Anchor Commands
```bash
cm anchor                  # Display overview of all saved project snapshots and available operations
cm anchor save <name>      # Create a complete snapshot of current project state including files and dependencies
cm anchor restore <name>   # Return project to exact state captured in specified snapshot
cm anchor list             # Show all available project snapshots with metadata and timestamps
cm anchor show <name>      # Display detailed information about a specific project snapshot
cm anchor diff <name>      # Compare current project state with saved snapshot to identify changes
cm anchor auto <name>      # Enable automatic background saving of project changes to specified snapshot
cm anchor stop <name>      # Disable automatic saving for the specified snapshot
```


### Tide Commands
```bash
cm tide                    # Display overview of performance tracking and analytics capabilities
cm tide show               # Present interactive charts and graphs of build performance metrics over time
cm tide analyze            # Examine project dependencies and their impact on build performance
cm tide export <path>      # Save performance metrics and analytics data to external file format
```

### Scrub Commands
```bash
cm scrub run --dry-run     # Preview what files and directories would be cleaned without making changes
cm scrub run -v            # Execute project cleanup with detailed verbose output showing all operations
cm scrub run -s /home      # Clean only projects located within the specified directory path
cm scrub run -r web        # Resume cleaning projects whose names contain the specified search term
cm scrub run --min-depth 2 --max-depth 5 # Clean projects within specified directory depth range from root
```

### Map Commands
```bash
cm map                     # Display overview of dependency visualization and analysis tools
cm map show                # Present interactive visual representation of project dependency tree
cm map analyze             # Perform comprehensive analysis of project structure and dependencies
cm map export <path>       # Save dependency map data to external file format for documentation
cm map path <from> <to>    # Show the specific dependency path connecting two specified components
```

### Mutiny Commands
```bash
cm mutiny                  # Display overview of override capabilities and current mutiny status
cm mutiny activate <reason> # Enable mutiny mode to bypass cargo restrictions for specified reason
cm mutiny deactivate       # Disable mutiny mode and restore normal cargo restrictions
cm mutiny allow-warnings   # Temporarily permit compilation warnings without stopping the build
cm mutiny skip-tests       # Bypass test execution during build process in mutiny mode
cm mutiny force            # Override safety checks and force execution of potentially dangerous operations
cm mutiny yolo             # Enable maximum risk mode that disables all safety checks for 30 minutes
cm mutiny status           # Display current status of mutiny mode and active overrides
```

### Config Commands
```bash
cm config                  # Display overview of all configuration options and current settings
cm config set <key> <val>  # Assign a specific value to a configuration key
cm config get <key>        # Retrieve the current value of a specific configuration key
cm config list             # Show all current configuration settings with their values
cm config init             # Create and initialize a new local configuration file for the project
cm config shortcut <name>  # Create a custom command shortcut for frequently used operations
cm config hook <type>      # Add an automated script that triggers on specific build events
```

### Optimize Commands
```bash
cm optimize                # Display overview of optimization capabilities and current optimization settings
cm optimize aggressive     # Apply maximum performance optimizations with potential stability trade-offs
cm optimize balanced       # Implement moderate optimizations balancing performance and stability
cm optimize conservative   # Apply minimal optimizations prioritizing stability over performance
cm optimize custom         # Implement user-defined custom optimization configuration
cm optimize status         # Show current optimization settings and their impact on build performance
cm optimize recommendations # Analyze project and suggest optimal performance improvement strategies
cm optimize restore        # Revert all optimizations and restore original Cargo.toml configuration
```

### Checklist Commands
```bash
cm checklist               # Show current checklist
cm checklist show          # Show current checklist
cm checklist list          # List all checklist items with numbers
cm checklist add <item>    # Add an item to the checklist
cm checklist done <items>  # Mark items as done (e.g., "1,2,3" or "1")
cm checklist clear [target] # Clear checklist items (default: "all", or "done")
```

### WTF Commands (CargoMate AI - Pro only)
```bash
cm wtf                     # Show WTF overview
cm wtf ask <question>      # Ask CargoMate AI a question
cm wtf er [count]          # Send recent errors to CargoMate AI (default: 10)
cm wtf checklist [limit]   # Send recent checklist items to CargoMate AI (default: 10)
cm wtf list [limit]        # List recent conversations (default: 10)
cm wtf show <id>           # Show specific conversation by ID
cm wtf history [limit]     # Show conversation history (default: 10)

# Ollama Integration
cm wtf ollama enable <model> # Enable local Ollama integration (default: llama2)
cm wtf ollama disable        # Disable local Ollama integration
cm wtf ollama status         # Show current Ollama configuration
cm wtf ollama models         # List available Ollama models
```


### Tool Commands
```bash
cm tool                    # Show tool system overview
cm tool list               # List all available tools
cm tool help <name>        # Show help for a specific tool
cm tool run <name> [args]  # Run a specific tool

# Available Tools:
cm tool bench-diff --from <commit> --to <commit> --threshold <percent>
    # Compare benchmark results between commits

cm tool dep-audit --strict --check-security --licenses "MIT,Apache-2.0"
    # Audit dependencies for security and license issues

cm tool test-gen --file <path> --type <unit|integration|property>
    # Generate test boilerplate from Rust function signatures
```

### General Commands
```bash
cm register [--license-key <key>] [--status] [--remaining] # Register or validate Pro license
cm init                    # Set up and initialize cargo-mate for a new project with default configuration
cm install                 # Install cargo-mate shell integration for enhanced command-line experience
cm activate                # Enable cargo-mate shell integration to provide additional functionality
cm checklist               # Display comprehensive project status checklist with actionable items
cm history [<kind>] [<limit>] # Show historical record of commands with optional filtering and size limits
cm idea <idea_text>        # Submit an idea for Cargo Mate development
cm debug                      # Debug command counter status (for testing)
cm user                    # Show user information and license status
cm --help                  # Display comprehensive help information for all available commands
cm --version               # Show current version information for cargo-mate installation
```

### Project Configuration (.cg)
```toml
[project]
name = "my-project"
default_journey = "build"
auto_checklist = true

[version]
auto_increment = true
increment_policy = "patch"

[shortcuts]
build = "build --release"
test = "test --all"
```

### Global Configuration (~/.shipwreck/config.toml)
```toml
[ui]
colors = true
verbose = false

[performance]
parallel_jobs = 4
incremental = true
```

## Some Examples

### Development Workflow
```bash
# Save current state before major changes
cm anchor save "pre-refactor"

# Record your development workflow
cm journey record "dev-workflow"
cargo check
cargo test
cargo build --release
# Ctrl+D to stop recording

# Replay the workflow anytime
cm journey play "dev-workflow"
```

### Performance Optimization
```bash
# Check optimization recommendations
cm optimize recommendations

# Apply balanced optimizations
cm optimize balanced

# Monitor build performance
cm tide show
```

### Version Management
```bash
# Initialize with semantic versioning
cm version init 1.0.0

# Versions auto-increment on builds
cargo build  # 1.0.0 -> 1.0.1
cargo test   # 1.0.1 -> 1.0.2

# Manual version bump for releases
cm version increment minor  # 1.0.2 -> 1.1.0
```

# 🚢 Cargo Mate
- 🚢 Your project is a ship
- ⚓ Anchors save your position
- 🌊 Tide charts track the flow
- 🗺️ Maps show the territory
- 🏴‍☠️ Mutiny overrides the captain
- 📝 Captain's log records the journey
- 🚢 Auto-versioning keeps your ship on course
- 🚀 Build optimization gives you the wind in your sails


## 📄 License

This project is licensed

For more information, visit: [cargo.do/license](https://cargo.do/license)

---

Built with ❤️ for the Rust community. 
No more shipwrecks in the sea of cargo errors!

<img src="https://raw.githubusercontent.com/cyber-boost/cargo-mate/3ebf3ef2f9eb64ec41e343a34e90f3a62f84d506/banner.svg" alt="Cargo Mate: Rust development companion" width="600">


