## Anchor Commands

The anchor system provides powerful project state management, allowing you to save and restore complete project snapshots. This includes Git state, file contents, dependencies, and even auto-update capabilities for continuous state tracking.

### Key Features
- **Complete State Capture**: Saves Git commit, Cargo.lock, source files, and environment
- **SHA256 Verification**: Ensures file integrity with cryptographic hashing
- **Auto-Update Mode**: Continuous monitoring and automatic updates of saved anchors
- **Background Operation**: Non-blocking auto-update mode for seamless workflow
- **Easy Restoration**: One-command restoration to any saved state

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

## Storage and File Management

### Anchor Storage Location
- **Global Storage**: `~/.shipwreck/anchors/`
- **Per-Anchor Directory**: Each anchor gets its own directory
- **File Organization**: Organized by anchor name with timestamp metadata

### What Gets Saved

**Git State**:
- Current commit hash
- Branch information
- Working directory status

**Dependencies**:
- Complete `Cargo.lock` snapshot
- Dependency versions and checksums

**Source Files**:
- All source files with SHA256 verification
- File modification timestamps
- Directory structure

**Environment**:
- Environment variables relevant to build
- Build configuration state

**Metadata**:
- Anchor creation timestamp
- User-provided message
- Project metadata

## Best Practices

### Creating Meaningful Anchors

```bash
# Before major refactoring
cm anchor save before-refactor --message "Pre-refactoring state"

# Before dependency updates
cm anchor save pre-deps-update --message "Before updating dependencies"

# Release checkpoints
cm anchor save v1.0.0 --message "Release v1.0.0 checkpoint"
```

### Using Auto-Update Mode

**When to Use**:
- Long-running development sessions
- Continuous integration workflows
- Experimental feature branches
- Documentation updates

**Workflow**:
```bash
# Start auto-update at beginning of session
cm anchor auto my-feature-branch

# Work normally - changes are automatically saved
# ... edit files ...

# Stop when done
cm anchor stop my-feature-branch
```

### Restoration Workflow

```bash
# Check available anchors
cm anchor list

# See what changed
cm anchor diff before-refactor

# Restore if needed
cm anchor restore before-refactor
```

## Troubleshooting

### Anchor Not Found

```bash
# List all anchors
cm anchor list

# Check storage directory
ls -la ~/.shipwreck/anchors/

# Verify anchor name spelling
cm anchor show <name>
```

### Auto-Update Not Working

```bash
# Check if auto-update is running
ps aux | grep "anchor auto"

# Restart auto-update
cm anchor stop <name>
cm anchor auto <name> --foreground  # Test in foreground first
```

### Restoration Issues

**Git Conflicts**:
```bash
# Check git status before restore
git status

# Stash changes if needed
git stash
cm anchor restore <name>
```

**File Permission Issues**:
```bash
# Check file permissions
ls -la ~/.shipwreck/anchors/

# Fix permissions if needed
chmod -R 755 ~/.shipwreck/anchors/
```

### Storage Space

```bash
# Check anchor storage size
du -sh ~/.shipwreck/anchors/

# Remove old anchors
rm -rf ~/.shipwreck/anchors/old-anchor-name
```

## Technical Details

### File Monitoring

**Auto-Update Monitoring**:
- Monitors only relevant directories (src/, Cargo.toml, etc.)
- Efficient file watching with minimal overhead
- Real-time updates when files change
- Background daemon for non-blocking operation

### SHA256 Verification

**File Integrity**:
- All saved files are verified with SHA256 hashes
- Ensures file integrity during save and restore
- Detects file corruption or tampering
- Automatic verification on restore

### Git Integration

**State Capture**:
- Captures current Git commit hash
- Stores branch information
- Preserves working directory state
- Handles uncommitted changes

### Performance

**Save Operation**:
- Fast file copying with progress indication
- Parallel file processing where possible
- Efficient storage with compression

**Restore Operation**:
- Fast restoration from saved state
- Atomic operations where possible
- Rollback capability on failure

## Integration with Other Features

### With Journeys

```bash
# Record a journey that includes anchor operations
cm journey record feature-workflow
cm anchor save checkpoint-1
# ... work ...
cm anchor save checkpoint-2
```

### With Version Management

```bash
# Save anchor before version bump
cm anchor save pre-version-bump
cm version increment minor
cm anchor save post-version-bump
```

### With Optimize

```bash
# Save state before optimization
cm anchor save pre-optimize
cm optimize aggressive
# Test build
cm anchor restore pre-optimize  # If issues occur
```

## Advanced Usage

### Anchor Comparison

```bash
# Compare two anchors
cm anchor diff anchor-1
cm anchor diff anchor-2

# See detailed differences
cm anchor show anchor-1
cm anchor show anchor-2
```

### Batch Operations

```bash
# Save multiple checkpoints
for checkpoint in dev test prod; do
    cm anchor save $checkpoint --message "Checkpoint: $checkpoint"
done
```

### CI/CD Integration

```bash
# In CI pipeline
cm anchor save ci-checkpoint-$(date +%Y%m%d)
# Run tests
# Restore if tests fail
cm anchor restore ci-checkpoint-$(date +%Y%m%d)
```

## Limitations

### Known Limitations
- **Large Projects**: Very large projects may take longer to save/restore
- **Binary Files**: Large binary files increase storage requirements
- **Network Dependencies**: Network-dependent builds may not fully restore
- **External Services**: Cannot restore state of external services or databases

### Platform Considerations
- **File Permissions**: May need adjustment on different platforms
- **Path Separators**: Handled automatically across platforms
- **Symlinks**: Symlinks are preserved but may need manual handling

## Future Enhancements

Planned features:
- **Incremental Anchors**: Only save changed files
- **Compression**: Automatic compression of anchor storage
- **Remote Storage**: Backup anchors to remote storage
- **Anchor Sharing**: Share anchors between team members
- **Selective Restoration**: Restore only specific files or directories

---