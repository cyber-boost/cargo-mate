## Config Commands

**Source**: `cargo-mate/captain/src/cmd/config.rs` and `cargo-mate/captain/src/captain/config.rs`

### `cm config init`
**Description**: Initialize local project config

**Usage**:
```bash
cm config init
```

**Source**: `config.rs:handle_config_internal()` - `ConfigAction::Init` case

**What it does** (verified in `config.rs:ConfigManager::init_local()`):
- Creates `.cg` file in project root
- Initializes with default configuration structure
- Uses TOML format for configuration

---

### `cm config set <key> <value> [--local]`
**Description**: Set a configuration value

**Usage**:
```bash
cm config set project.name "my-app" --local
cm config set build.default_profile release
cm config set auto_fix.format_on_save true
```

**Available Keys** (verified in `config.rs:get_from_config()`):
- `project.name` - Project name (String)
- `project.default_journey` - Default journey name (String)
- `project.theme` - UI theme (String, default: "nautical")
- `project.auto_checklist` - Auto-generate checklists (bool)
- `project.track_performance` - Track build performance (bool)
- `auto_fix.format_on_save` - Format on save (bool)
- `auto_fix.clippy_on_build` - Run clippy on build (bool)
- `auto_fix.auto_deps_update` - Auto-update dependencies (bool)
- `auto_fix.fix_warnings` - Auto-fix warnings (bool)
- `auto_fix.suggest_fixes` - Suggest fixes (bool)
- `build.default_profile` - Default build profile (String)
- `build.parallel_jobs` - Number of parallel jobs (usize, optional)
- `build.incremental` - Enable incremental compilation (bool)
- `build.cache_artifacts` - Cache build artifacts (bool)
- `shortcuts.<name>` - Custom command shortcuts (String)

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

**Hook Types** (verified in `config.rs:HookSettings` struct):
- `pre_build`: Run before cargo commands (Vec<String>)
- `post_build`: Run after successful builds (Vec<String>)
- `on_error`: Run when build fails (Vec<String>)
- `on_success`: Run when build succeeds (Vec<String>)

---

### `cm config reset`
**Description**: Reset configuration to defaults

**Usage**:
```bash
cm config reset
```

**Source**: `config.rs:handle_config_internal()` - `ConfigAction::Reset` case

**What it does**:
- Resets configuration to default values
- Can reset global or local config (based on context)

---

## Configuration File Structure

**Global Config**: `~/.shipwreck/config.toml`  
**Local Config**: `.cg` (project root)

**Configuration Structure** (verified in `config.rs:ProjectConfig`):
```toml
[project]
name = "my-project"
default_journey = "dev-cycle"
theme = "nautical"
auto_checklist = true
track_performance = true

[shortcuts]
b = "build --release"
t = "test --all"

[auto_fix]
format_on_save = true
clippy_on_build = false
auto_deps_update = false
fix_warnings = false
suggest_fixes = true

[build]
default_profile = "dev"
parallel_jobs = 8
incremental = true
cache_artifacts = true

[hooks]
pre_build = ["cargo fmt --check"]
post_build = ["cargo test --quiet"]
on_error = ["cm checklist"]
on_success = ["echo 'Build successful!'"]

[version_control]
auto_git_commit = false
auto_anchor_git = false
```

## Implementation Details

**Config Resolution** (verified in `config.rs:get()`):
1. Checks local config (`.cg`) first
2. Falls back to global config (`~/.shipwreck/config.toml`)
3. Returns `None` if key not found in either

**Config Storage**:
- Global config: `~/.shipwreck/config.toml`
- Local config: `.cg` in project root
- Both use TOML format
- Local config takes precedence over global

**Default Values** (verified in `config.rs:ProjectConfig::default()`):
- Theme: "nautical"
- Auto-checklist: false
- Track performance: false
- Format on save: false
- Clippy on build: false
- Default profile: "dev"
- Incremental: true

---