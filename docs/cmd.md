# Cargo Mate (cm)
Run cm install && cm activate > then cm can run cargo and cargo can run cm 

## CM Commands
cm --help
cm --version

### Usage: cm journey <COMMAND>

Commands:
  (no args)         Show journey overview
  record <name>     Record a new journey entry
  play <name>       Replay a recorded journey
  list              List all recorded journeys
  export <name> <output> Export a journey to external format
  import <path>     Import a journey from external format
  publish <name>    Publish a journey for sharing
  download <gist_id> Download a published journey
  search <query>    Search through journey entries
  published         List published journeys
  help              Print this message or the help of the given subcommand

### Usage: cm anchor <COMMAND>

Commands:
  (no args)         Show anchor overview
  save <name>       Save current project state as anchor
  restore <name>    Restore project to saved anchor state
  list              List all saved anchors
  show <name>       Show details of a specific anchor
  diff <name>       Compare current state with anchor
  auto <name>       Auto-save anchor for project
  stop <name>       Stop auto-update mode for anchor
  help              Print this message or the help of the given subcommand

### Usage: cm log <COMMAND>

Commands:
  (no args)         Show log overview
  add <message>     Add a new log entry
  search <query>    Search through log entries
  timeline [days]   Display log entries in timeline view (default: 7 days)
  export <path>     Export logs to external format
  analyze           Analyze log patterns and statistics
  help              Print this message or the help of the given subcommand

### Usage: cm tide <COMMAND>

Commands:
  (no args)         Show tide overview
  show              Display tide information
  analyze           Analyze tide patterns
  export <path>     Export tide data
  help              Print this message or the help of the given subcommand

### Usage: cm map <COMMAND>

Commands:
  (no args)         Show map overview
  show              Display project dependency map
  analyze           Analyze project structure
  export <path>     Export map data
  path <from> <to>  Show dependency paths
  help              Print this message or the help of the given subcommand

### Usage: cm mutiny <COMMAND>

Commands:
  (no args)         Show mutiny overview
  activate <reason> Activate mutiny mode
  deactivate        Deactivate mutiny mode
  allow-warnings    Allow warnings in mutiny mode
  skip-tests        Skip tests in mutiny mode
  force             Force operations in mutiny mode
  yolo              Enable YOLO mode (maximum risk)
  status            Show current mutiny status
  help              Print this message or the help of the given subcommand

### Usage: cm config <COMMAND>

Commands:
  (no args)         Show config overview
  set <key> <value> Set a config value
  get <key>         Get a config value
  list              List all config
  init              Initialize local config
  shortcut <name> <command> Add a shortcut
  hook <type> <command> Add a hook
  help              Print this message or the help of the given subcommand

### Usage: cm version <COMMAND>

Commands:
  (no args)         Show version overview
  init [version]    Initialize version management (default: 0.1.0)
  info              Show version information
  increment [type]  Increment version number (patch/minor/major)
  set <version>     Set specific version
  history           Show version history
  update-cargo      Update Cargo.toml version
  config <action>   Manage version configuration
  help              Print this message or the help of the given subcommand

### Usage: cm version config <COMMAND>

Commands:
  enable            Enable auto-increment
  disable           Disable auto-increment
  policy <type>     Set increment policy (patch/minor/major)
  show              Show version configuration
  help              Print this message or the help of the given subcommand

### Usage: cm view <COMMAND>

Commands:
  (no args)         Show view overview
  errors            View all errors and warnings
  artifacts         View generated files and locations
  scripts           View build script outputs
  history           View detailed build history
  checklist         View checklist and fixes
  all               View all results in one place
  latest            Quick view of latest issues
  open              Open results in file explorer
  help              Print this message or the help of the given subcommand

### Usage: cm optimize <COMMAND>

Commands:
  (no args)         Show optimization overview
  aggressive        Apply aggressive optimizations for maximum speed
  balanced          Apply balanced optimizations for good speed/stability
  conservative      Apply conservative optimizations for maximum stability
  custom            Apply custom optimizations with specific values
  status            Show current optimization status
  recommendations   Show optimization recommendations
  restore           Restore original Cargo.toml from backup
  help              Print this message or the help of the given subcommand

### Usage: cm optimize custom <OPTIONS>

Options:
  --jobs <number>           Number of parallel jobs (default: 4)
  --incremental <bool>      Enable incremental compilation (default: true)
  --opt-level <0-3>         Optimization level (default: 1)
  --debug-level <0-2>       Debug level (default: 1)
  --codegen-units <number>  Codegen units for parallelism (default: 128)

### Usage: cm checklist <COMMAND>

Commands:
  (no args)         Show current checklist
  show              Show current checklist
  list              List all checklist items with numbers
  add <item>        Add an item to the checklist
  done <items>      Mark items as done (e.g., "1,2,3" or "1")
  clear [target]    Clear checklist items (default: "all", or "done")
  help              Print this message or the help of the given subcommand

### Usage: cm wtf <COMMAND> (CargoMate AI - Pro only)

Commands:
  (no args)         Show WTF overview
  ask <question>    Ask CargoMate AI a question
  er [count]        Send recent errors to CargoMate AI (default: 10)
  ollama <command>  Configure local Ollama integration
  list [limit]      List recent conversations (default: 10)
  show <id>         Show specific conversation by ID
  history [limit]   Show conversation history (default: 10)
  checklist [limit] Send recent checklist items to CargoMate AI (default: 10)
  help              Print this message or the help of the given subcommand

### Usage: cm wtf ollama <COMMAND>

Commands:
  enable [model]    Enable local Ollama integration (default: llama2)
  disable           Disable local Ollama integration
  status            Show current Ollama configuration
  models            List available Ollama models
  help              Print this message or the help of the given subcommand

### Usage: cm register <OPTIONS>

Commands:
  (no args)         Show registration help
  <license_key>     Register with license key (CM-XXXXX-XXXXX-XXXXX)
  --status          Check current license status without registering
  --remaining       Show only remaining commands count
  help              Print this message or the help of the given subcommand

### Usage: cm idea <OPTIONS>

Commands:
  <idea_text>       Submit your idea directly
  help              Print this message or the help of the given subcommand

Description: Submit ideas and suggestions for Cargo Mate development. Ideas are sent to the CargoMate API and stored in your local idea history.

### Usage: cm user

Commands:
  (no args)         Show user information and license status

Description: Display your user information, license status, and CargoMate Pro features availability.

### Usage: cm deps [OPTIONS]

Options:
  --path <PATH>     Analyze specific directory (default: current)
  --json            Output results in JSON format

Description: Analyze project dependencies - find which external crates are actually used vs declared

### Usage: cm liberate [OPTIONS]

Options:
  -t, --target <PATH>  Target directory to scan (default: current)
  -o, --out <PATH>     Output file path (default: .LIBerate-[timestamp].rs)

Description: Generate lib.rs by scanning all Rust files and extracting public items

### Usage: cm tree [OPTIONS]

Options:
  -t, --target <PATH>  Target directory to scan (default: current)
  -o, --out <PATH>     Output file path (default: cm-tree-[timestamp].md)
  --file-size          Include file sizes
  --line-count         Count lines in files
  --dates              Include modification dates
  --style <STYLE>      Choose style: basic, readme, cm, hard, easy
  --yolo               Activate YOLO mode 🎉

Commands:
  history            View all previously generated trees
  show <name>        Display a specific tree from history
  find <query>       Search through tree history

Description: Generate beautiful markdown-formatted directory tree

### Usage: cm stub [OPTIONS]

Options:
  -t, --target <PATH>  Target directory to scan (default: current)
  -o, --out <PATH>     Output file path (default: cm-stubs-[timestamp].md)
  --ext <EXT>          File extensions to scan (default: rs,py,js,html)
  --custom <PATTERN>   Custom pattern(s) to search for
  --skip <PATTERNS>    Patterns to skip/exclude

Commands:
  find [<pattern>]   Search for stubs with optional custom pattern
  skip <patterns>    Search while skipping specific patterns
  history            View all previously generated stub reports
  show <name>        Display a specific stub report from history
  delete --all       Delete all stub reports from history

Description: Scan for stubs, placeholders, TODOs, FIXMEs, and unimplemented code patterns

### Usage: cm bin [OPTIONS]

Options:
  -p, --path <PATH>           Path to the binary file
  -n, --name <NAME>           Binary name (searches PATH)
  -o, --out <PATH>            Output file path
  --timeout-seconds <SECONDS> Timeout for each command (default: 10)
  --max-depth <DEPTH>         Maximum number of commands to test

Commands:
  history            View all previously generated test reports
  show <name>        Display a specific test report from history
  find <query>       Search through test reports
  delete --all       Delete all test reports from history

Description: Systematically test any binary by parsing help output and testing all command/flag combinations

### Usage: cm probe <COMMAND>

Commands:
  flake [options]    Detect flaky probes that pass/fail inconsistently
  impact [options]   Run only probes affected by recent Git changes
  coverage [options] Generate coverage reports with automatic backend detection
  profile [options]  Measure execution time of individual probes
  tag [options]      Custom probe categorization and selective execution
  ci-gen [options]   One-click generation of CI pipelines
  env [options]      Manage Docker containers for probe dependencies
  replay [options]   Deterministically replay failed probe runs
  order [options]    Detect probe ordering dependencies
  doc [options]      Generate comprehensive markdown documentation
  help               Print this message or the help of the given subcommand

Description: Intelligent probe suite management for Rust testing

### Usage: cm strip [OPTIONS] <INPUT>

Options:
  -o, --output <FILE>    Output file path (defaults to stdout)
  -r, --recursive        Process directory recursively
  -b, --remove-blanks    Remove blank lines
  -a, --aggressive       Aggressive mode: maximum stripping
  --minify               Minify to single line where possible
  --strip-attrs          Remove all attributes (#[...])
  --strip-docs           Remove documentation comments
  --tease                Strip meticulously comments and blank lines
  --no-backup            Disable automatic backup creation
  --force                Allow overwriting the same file

Description: Remove comments and non-essential elements from Rust files

### Usage: cm scat <SUBCOMMAND>

Commands:
  names <PATH>       Obfuscate file/folder names with mapping file
  code <PATH>        Obfuscate Rust identifiers while preserving functionality
  strings <PATH>     Scramble string literals with encryption key
  pack <INPUT> <OUTPUT> Pack files into obfuscated bundle
  unpack <INPUT> <MAP> Reverse obfuscation using mapping files
  help               Print this message or the help of the given subcommand

Description: Source Code Obfuscation Tool (STILL UNDER DEVELOPMENT - SCAT AT YOUR OWN RISK)

### Usage: cm sweep <COMMAND>

Commands:
  (no args)          Display overview of sweep capabilities
  scan               Scan for println!, eprintln!, and dbg! statements
  sweep [options]    Clean up debug statements with intelligent pattern recognition
  convert            Convert debug prints to proper logging statements
  analyze            Analyze debug statement patterns and statistics
  init               Initialize sweep configuration file
  help               Print this message or the help of the given subcommand

Options:
  -n                 Dry run (show what would be removed)
  -p                 Interactive mode with pattern memory learning
  -y                 Auto-approve all removals
  -i                 Interactive confirmation for each statement
  --backup           Create backup files before cleaning

Description: Intelligently sweep away println! mess that AI coding assistants leave behind

### Usage: cm scrub <COMMAND>

Commands:
  run [OPTIONS]      Run scrub operation

Options:
  -d, --directory <dir>    Directory to start searching from (default: current)
  -n, --dry-run            Show what would be cleaned without actually cleaning
  -v, --verbose            Enable verbose output
  -r, --resume-from <str>  Resume from projects containing this string
  --min-depth <num>        Minimum directory depth to search
  --max-depth <num>        Maximum directory depth to search
  -j, --jobs <num>         Number of parallel workers (default: 4)
  --min-size <MB>          Only show projects larger than this size
  -s, --sort-by-size       Sort results by size
  --export-json <file>     Export results to JSON file
  -i, --interactive        Ask before cleaning each project
  -e, --exclude <pattern>  Exclude directories matching patterns (multiple)
  --stats-only             Only show statistics without cleaning

Description: Clean up target directories across your entire system

### Usage: cm ddr <COMMAND>

Commands:
  (no args)          Display overview of DDR capabilities
  build [OPTIONS]    Build Rust project using Docker containers

Options:
  --target <target>  Cross-compile for specific target architecture
  --jobs <num>       Set maximum parallel build jobs (default: 16)
  --image <img>      Use custom Docker image for builds
  --profile <profile> Build with specific profile (debug/release)
  -n                 Dry run (show what would be built)
  --verbose          Verbose output with detailed Docker commands
  --no-cache         Disable Docker layer caching
  --clean            Clean build artifacts before building
  --watch            Watch mode (rebuild on changes)

Description: Docker Dock Rust - Parallel Docker-based Rust build orchestration (UNDER DEVELOPMENT)

### Usage: cm tool <COMMAND>

Commands:
  (no args)          Show tool system overview
  list               List all available tools
  help <name>        Show help for a specific tool
  run <name> [args]  Run a specific tool
  help               Print this message or the help of the given subcommand

Description: Tools are just helper files / base files for you to use if wanted

### Usage: cm debug

Description: Enable debug mode for Cargo Mate (for testing and troubleshooting)

### Usage: cm <COMMAND>

Commands:
  init              Initialize a new cargo-mate project
  install           Install cargo-mate system-wide
  activate          Activate cargo-mate for current project
  checklist         Show project checklist
  history [kind] [limit] Show command history (default: summary, 50)
  exec <cargo_args> Execute cargo command (fallback for standard cargo commands)
  test              Test command for error handling
  debug             Debug command counter status (for testing)
  help              Print this message or the help of the given subcommands

### Standard Cargo Commands

All standard cargo commands work normally through cm:
- cm build
- cm test
- cm run
- cm check
- cm clippy
- cm fmt
- cm clean
- cm update
- cm add
- cm remove
- And all other cargo commands

### Direct WTF Questions

You can ask CargoMate AI questions directly without the `ask` subcommand:
```bash
cm wtf "How do I optimize my Rust code?"
cm wtf "What's wrong with this error message?"
```

### Examples

```bash
# Journey management
cm journey record build-flow
cm anchor save before-refactor
cm log add "Fixed memory leak in async module"

# Performance and analysis
cm tide show
cm map show
cm optimize aggressive
cm optimize custom --jobs 8 --opt-level 3
cm deps                    # Analyze dependencies
cm probe flake -i 30       # Detect flaky tests
cm probe coverage --open   # Generate coverage report

# AI assistance (Pro only)
cm wtf ask "How do I optimize my Rust code?"
cm wtf er 10
cm wtf checklist 5
cm wtf ollama enable llama2
cm wtf "What's causing this compilation error?"

# Version management
cm version init              # Initialize with default version 0.1.0
cm version init 1.0.0       # Initialize with specific version
cm version increment patch
cm version config enable
cm version config policy minor

# Project management
cm checklist add "Fix the async bug"
cm checklist done 1,2,3
cm view errors
cm mutiny allow-warnings

# Configuration
cm config set build.jobs 8
cm config shortcut fast "cargo build --release"
cm config hook pre-build "echo 'Starting build...'"

# License and registration
cm register CM-12345-67890-ABCDE
cm register --status
cm user

# Ideas and feedback
cm idea "Add support for workspace-level optimizations"

# Code analysis and utilities
cm liberate -t ./src -o lib.rs    # Generate lib.rs from project files
cm tree --file-size --line-count  # Generate directory tree
cm stub --ext rs                  # Find TODOs and stubs
cm bin --name cargo               # Test binary systematically
cm strip src/ -r --aggressive     # Strip comments and blank lines
cm sweep sweep -y                 # Clean up AI debug statements
cm scrub run --stats-only         # Analyze target directories

# Docker builds
cm ddr build --target x86_64-unknown-linux-gnu --jobs 8

# Tools
cm tool list                      # List available tools
cm tool run bloat-check --binary target/release/app

# Debug
cm debug                          # Enable debug mode
```

### Notes

- **Pro Features**: WTF (CargoMate AI) commands require a Pro license
- **File Extensions**: Version files now use `.v` instead of `.version`
- **Shell Integration**: Use `cm install` to set up shell integration
- **Configuration**: Local config is stored in `.cg` file
- **History**: All command history is stored in `~/.shipwreck/`
- **Direct Questions**: You can ask WTF questions directly without the `ask` subcommand
- **Custom Optimizations**: Use `cm optimize custom` for fine-tuned build optimizations
- **License Management**: Use `cm register` to activate Pro features
- **Error Handling**: The `cm test` command helps test error handling scenarios
- **Dependency Analysis**: Use `cm deps` to find unused or missing dependencies
- **Code Generation**: Use `cm liberate` to auto-generate lib.rs files
- **Project Structure**: Use `cm tree` to generate beautiful directory trees
- **Code Quality**: Use `cm stub` to find TODOs and unimplemented code
- **Binary Testing**: Use `cm bin` to systematically test any binary
- **Probe Management**: Use `cm probe` for intelligent test suite management
- **Code Cleaning**: Use `cm strip` and `cm sweep` to clean up code
- **Docker Builds**: Use `cm ddr` for parallel Docker-based builds
- **Tool System**: Use `cm tool` to access helper tools and utilities
