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

### Usage: cm <COMMAND>

Commands:
  init              Initialize a new cargo-mate project
  install           Install cargo-mate system-wide
  activate          Activate cargo-mate for current project
  checklist         Show project checklist
  history [kind] [limit] Show command history (default: summary, 50)
  exec <cargo_args> Execute cargo command (fallback for standard cargo commands)
  test              Test command for error handling
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
