## Captain's Log Commands

**Source**: `cargo-mate/captain/src/cmd/log.rs` and `cargo-mate/captain/src/cmd/smune.rs:LogAction`

### `cm log add <message> [--tags <tags>...]`
**Description**: Add an entry to the captain's log

**Source**: `log.rs:handle_log()` - `LogAction::Add` case

**Usage**:
```bash
cm log add "Switched to async runtime"
cm log add "Fixed memory leak in parser" --tags bug performance
```

**Implementation** (verified in `log.rs:6-9`):
- Creates `CaptainLog` instance
- Calls `log.log(&message, tags)` with message and tags vector
- Tags parameter is `Vec<String>` from command line

---

### `cm log search <query>`
**Description**: Search through log entries

**Source**: `log.rs:handle_log()` - `LogAction::Search` case

**Usage**:
```bash
cm log search "memory"
cm log search "performance"
```

**Implementation** (verified in `log.rs:10-21`):
- Calls `log.search(&query)` method
- Prints "No matching log entries found" if empty
- Otherwise prints count and list of matching entries
- Each entry shows timestamp (YYYY-MM-DD HH:MM:SS format) and message

---

### `cm log timeline [days]`
**Description**: Show log timeline for specified days (default: 7)

**Source**: `log.rs:handle_log()` - `LogAction::Timeline` case

**Usage**:
```bash
cm log timeline        # Last 7 days
cm log timeline 30     # Last 30 days
```

**Implementation** (verified in `log.rs:22-25`):
- Converts days parameter to appropriate type
- Calls `log.show_timeline(days)` method
- Default value is 7 days (from `smune.rs:LogAction::Timeline` definition)

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

**Source**: `log.rs:handle_log()` - `LogAction::Export` case

**Usage**:
```bash
cm log export ./project-log.md --format markdown
cm log export ./log.json --format json
cm log export ./report.html --format html
```

**Implementation** (verified in `log.rs:26-33`):
- Maps format string to `ExportFormat` enum:
  - `"json"` → `ExportFormat::Json`
  - `"html"` → `ExportFormat::Html`
  - Default (anything else) → `ExportFormat::Markdown`
- Calls `log.export(&path, fmt)` method
- Default format is "markdown" (from `smune.rs:LogAction::Export` definition)

---

### `cm log analyze`
**Description**: Analyze log patterns and statistics

**Source**: `log.rs:handle_log()` - `LogAction::Analyze` case

**Usage**:
```bash
cm log analyze
```

**Implementation** (verified in `log.rs:34-37`):
- Calls `log.analyze()` method which returns analysis object
- Calls `analysis.display()` to show results

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

### `cm log track <command>`
**Description**: Track a command execution with enhanced logging

**Source**: `log.rs:handle_log()` - `LogAction::Track` case

**Usage**:
```bash
cm log track "cargo build --release"
cm log track "cargo test"
```

**Implementation** (verified in `log.rs:38-47`):
- Prints tracking start message with command name in cyan
- Generates session ID: `{command}_{timestamp}` (command with spaces replaced by underscores, plus hex timestamp)
- Calls `run_tracked_command(&command, &session_id)` from utils module
- Prints success or error message based on result

**Output**:
```
🔍 Starting enhanced tracking for: cargo build --release
✅ Command tracked successfully
```

---

## Log Storage

**Location**: `~/.shipwreck/logs/` (handled by CaptainLog module)  
**Format**: JSON files with log entries

## Implementation Module

- **CaptainLog**: `cargo-mate/captain/src/captain/captain_log.rs` - Log management functionality
- **ExportFormat**: Enum for export formats (Json, Html, Markdown)

---