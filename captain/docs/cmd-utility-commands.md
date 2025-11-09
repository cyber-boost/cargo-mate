## Utility Commands

### `cm checklist`
**Description**: Show current checklist (see `cmd-checklist.md` for full documentation)

**Usage**:
```bash
cm checklist show
cm checklist add "Fix bug"
cm checklist done 1
```

**Note**: See `cmd-checklist.md` for complete checklist command documentation.

---

### `cm history <kind> [limit]`
**Description**: Show build history

**Source**: `cargo-mate/captain/src/main.rs:execute_command()` - `Commands::History` case  
**Implementation**: `cargo-mate/captain/src/history.rs:show_history()`

**Usage**:
```bash
cm history summary 20    # Last 20 builds summary (default)
cm history errors 10     # Last 10 errors
cm history warnings 10   # Last 10 warnings
```

**Implementation** (verified in `main.rs:238-242` and `history.rs:49-63`):
- Default `kind`: "summary" (from `smune.rs:Commands::History` definition)
- Default `limit`: 50 (from `smune.rs:Commands::History` definition)
- Converts limit to usize and passes to `history::show_history()`
- Parses arguments and calls appropriate display function

**Available Kinds** (verified in `history.rs:parse_history_args()`):
- `summary` - Build summary with success/failure counts
- `errors` - List of errors from build history
- `warnings` - List of warnings from build history

**Storage**: `~/.shipwreck/history/` (handled by history module)

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

**Source**: `cargo-mate/captain/src/main.rs:execute_command()` - `Commands::Exec` case

**Usage**:
```bash
cm exec build --release
cm exec test --all
cm exec clippy
```

**Implementation** (verified in `main.rs:285-290`):
- **No license check** - Exec is called very frequently and license check would cause delays (15-30s on Mac)
- Converts cargo_args Vec<String> to Vec<&str>
- Calls `display::run_cargo_with_display(&args_refs)` for enhanced display
- Uses enhanced display system with progress bars and live updates

**Features**:
- Enhanced display with progress bars
- Real-time error/warning tracking
- Automatic result storage
- No license enforcement (for performance)

**Note**: Any unrecognized cm command automatically falls through to cargo (handled by default command handler)

---