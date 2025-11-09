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

**Source**: `cargo-mate/captain/src/main.rs` - `handle_default_command()` function

---

### `cm init`
**Description**: Initialize cargo-mate and automatically set up shell integration

**Source**: `cargo-mate/captain/src/cmd/init.rs` - `handle_init()` function

**Usage**:
```bash
cm init
```

**What it does** (verified in `init.rs:115-197`):
- Creates local `.cg` config file via `ConfigManager::init_local()`
- Detects shell type (bash, zsh, fish) via `ShellIntegration::detect_shell()`
- Automatically installs shell integration via `ShellIntegration::add_shell_integration()`
- Checks if integration already exists before adding
- Creates `~/.shipwreck/` directory structure if it doesn't exist
- Sets up error, warnings, checklists, history, wtf_history, and idea_history directories

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
- All `cargo` commands will be intercepted by cm (via shell function)
- You can use `cm` directly for cargo-mate commands
- `cg` is available as a shortcut (if added by shell integration)

**Directory Structure Created** (verified in `init.rs:ensure_initialized()`):
```
~/.shipwreck/
├── errors/
├── warnings/
├── checklists/
├── history/
├── wtf_history/
├── idea_history/
└── config.toml (if doesn't exist)
```

**Shell Integration Details** (verified in `init.rs:init_cargo_mate()`):
- Detects shell automatically (bash, zsh, fish)
- Finds appropriate RC file (`.bashrc`, `.zshrc`, `config.fish`)
- Checks for existing integration before adding
- Creates backup if needed (handled by ShellIntegration module)

**First Run Behavior** (verified in `init.rs:ensure_initialized()`):
- Automatically creates `~/.shipwreck/` directory structure on first run
- Attempts automatic shell integration installation
- Falls back gracefully if auto-setup fails

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

**Source**: `cargo-mate/captain/src/cmd/activate.rs` - `handle_activate()` function

**What it does** (verified in `activate.rs:5-119`):
- Detects current shell type
- Sources your shell configuration file (`.bashrc`, `.zshrc`, `config.fish`, etc.)
- Activates cargo-mate integration without restarting terminal
- Checks if integration exists before attempting to activate
- Provides clear feedback about activation status
- Handles errors gracefully with helpful messages

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