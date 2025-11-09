## Journey Commands

**Source**: `cargo-mate/captain/src/cmd/journey.rs` and `cargo-mate/captain/src/cmd/smune.rs:JourneyAction`

### `cm journey record <name>`
**Description**: Start recording a journey (sequence of commands)

**Source**: `journey.rs:handle_journey()` - `JourneyAction::Record` case

**Usage**:
```bash
cm journey record deploy-flow
# Execute your commands...
# Press Ctrl+D to stop recording
```

**Implementation** (verified in `journey.rs:5-12`):
- Creates `JourneyRecorder` instance
- Starts recording with provided name
- Polls recording status every 100ms
- Stops recording with message "User recorded journey"
- Uses `journey::JourneyRecorder` module for actual recording logic

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

**Source**: `journey.rs:handle_journey()` - `JourneyAction::Play` case

**Usage**:
```bash
cm journey play deploy-flow
cm journey play deploy-flow --dry-run  # Preview without executing
```

**Implementation** (verified in `journey.rs:13-17`):
- Creates `JourneyPlayer` with `dry_run` flag and `interactive=true`
- Loads journey by name
- Plays the loaded journey
- Uses `journey::JourneyPlayer` module for playback logic

---

### `cm journey list`
**Description**: List all available journeys

**Source**: `journey.rs:handle_journey()` - `JourneyAction::List` case

**Usage**:
```bash
cm journey list
```

**Implementation** (verified in `journey.rs:18-26`):
- Calls `journey::list_journeys()` function
- Prints "No journeys found" if empty
- Otherwise prints "📚 Available journeys:" header
- Lists each journey name in cyan color

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

**Source**: `journey.rs:handle_journey()` - `JourneyAction::Export` case

**Usage**:
```bash
cm journey export deploy-flow ./deploy.journey.json
```

**Implementation** (verified in `journey.rs:27-29`):
- Calls `journey::export_journey(&name, &output)` function
- Exports journey to specified file path

---

### `cm journey import <path>`
**Description**: Import a journey from a file

**Source**: `journey.rs:handle_journey()` - `JourneyAction::Import` case

**Usage**:
```bash
cm journey import ./shared-journey.json
```

**Implementation** (verified in `journey.rs:30-32`):
- Calls `journey::import_journey(&path)` function
- Imports journey from specified file path

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

**Source**: `journey.rs:handle_journey()` - `JourneyAction::Publish` case

**Implementation** (verified in `journey.rs:33-35`):
- Calls `journey::JourneyMarketplace::publish(&name, tags)` function
- Tags parameter is `Vec<String>` from command line
- Uses JourneyMarketplace module for GitHub Gist integration

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

**Source**: `journey.rs:handle_journey()` - `JourneyAction::Download` case

**Usage**:
```bash
cm journey download abc123def456
```

**Implementation** (verified in `journey.rs:36-38`):
- Calls `journey::JourneyMarketplace::download(&gist_id)` function
- Downloads journey from GitHub Gist by ID
- Uses JourneyMarketplace module for GitHub Gist integration

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

**Source**: `journey.rs:handle_journey()` - `JourneyAction::Search` case

**Usage**:
```bash
cm journey search "rust build"
cm journey search testing
```

**Implementation** (verified in `journey.rs:39-41`):
- Calls `journey::JourneyMarketplace::search(&query)` function
- Searches GitHub Gists for matching journeys
- Uses JourneyMarketplace module for search functionality

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

**Source**: `journey.rs:handle_journey()` - `JourneyAction::Published` case

**Usage**:
```bash
cm journey published
```

**Implementation** (verified in `journey.rs:42-51`):
- Calls `journey::JourneyMarketplace::list_published()` function
- Prints "No published journeys found" if empty
- Otherwise prints "📤 Your published journeys:" header
- Lists each published journey name in cyan color

**Output**:
```
📤 Your published journeys:
  • build-flow
  • test-suite
  • release-process
```

**Storage**: Published journeys stored in `~/.shipwreck/journeys/.published.json` (handled by JourneyMarketplace module)

---

## Journey Storage

**Location**: `~/.shipwreck/journeys/`  
**Format**: JSON files (one per journey)

## Implementation Modules

- **JourneyRecorder**: `cargo-mate/captain/src/journey.rs` - Recording functionality
- **JourneyPlayer**: `cargo-mate/captain/src/journey.rs` - Playback functionality
- **JourneyMarketplace**: `cargo-mate/captain/src/journey.rs` - GitHub Gist integration

---