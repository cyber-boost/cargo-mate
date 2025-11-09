## Version Management Commands

The version management system provides automatic version incrementing on cargo operations while maintaining full user control over versioning policies. This feature implements a comprehensive auto-versioning system that automatically increments version numbers on project operations (check, build, etc.) while maintaining synchronization with Cargo.toml.

### Key Features
- **Automatic Incrementing**: Version increments automatically on cargo operations
- **Policy-Based**: Multiple increment policies (Patch, Minor, Major, Custom)
- **Cargo.toml Sync**: Automatically synchronizes with Cargo.toml version field
- **Flexible Configuration**: Enable/disable per project with easy configuration
- **Semantic Versioning**: Follows semantic versioning principles

### `cm version init [version]`
**Description**: Initialize auto-versioning for the project

**Usage**:
```bash
cm version init              # Use default 1.0.0
cm version init 2.0.0        # Start with specific version
```

**What it does**:
- Creates `.v` file in project root with version configuration
- Sets up auto-incrementing on cargo operations
- Configures versioning policies (Patch, Minor, Major)
- Integrates with Cargo.toml version field
- Sets up pre and post-operation hooks for automatic version management

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
- `patch`: Increment patch version (1.0.0 -> 1.0.1) - Default, for bug fixes
- `minor`: Increment minor version (1.0.0 -> 1.1.0) - For new features
- `major`: Increment major version (1.0.0 -> 2.0.0) - For breaking changes

---

## Integration with Cargo Operations

The version system automatically integrates with cargo operations:

### Automatic Increment Triggers
- `cm check` - Auto-increments before compilation
- `cm build` - Auto-increments before building
- `cm exec <args>` - Auto-increments before cargo execution
- `cm test` - Auto-increments before testing

### Hook System
- **Pre-operation Hook**: Runs before cargo operations to increment version
- **Post-operation Hook**: Shows version info after operations complete

### Configuration File Format

The `.v` file stores version configuration in TOML format:

```toml
auto_increment = true
version_file = ".v"
current_version = "2.1.1"
increment_policy = "Patch"
version_format = "Semantic"
```

## Best Practices

### Initial Setup
1. **Start Fresh**: Run `cm version init` at project start
2. **Choose Policy**: Select appropriate increment policy for your workflow
3. **Version Alignment**: Ensure initial version matches your Cargo.toml

### Daily Usage
- **Automatic**: Version increments automatically on every cargo operation
- **Manual Control**: Use `cm version increment` for specific increments
- **Configuration**: Easy enable/disable and policy changes via `cm version config`
- **Visibility**: Current version displayed after operations

### Workflow Integration
```bash
# Initialize versioning
cm version init 1.0.0

# Development workflow (auto-increments)
cm check    # 1.0.0 -> 1.0.1
cm build    # 1.0.1 -> 1.0.2

# Manual version bump for releases
cm version increment minor  # 1.0.2 -> 1.1.0

# Disable auto-increment for testing
cm version config disable
cm check  # No increment
cm version config enable  # Re-enable
```

## Troubleshooting

### Version Not Auto-Incrementing
```bash
# Check if auto-increment is enabled
cm version config show

# Enable if disabled
cm version config enable

# Verify version file exists
cat .v
```

### Cargo.toml Out of Sync
```bash
# Check current version
cm version info

# Manually sync if needed
cm version set $(cm version info | grep "Current Version" | awk '{print $3}')
```

### Version File Missing
```bash
# Re-initialize versioning
cm version init

# Or restore from Cargo.toml
cm version set $(grep "^version" Cargo.toml | cut -d'"' -f2)
```

## Technical Details

### File Structure
- **`.v`**: Project version configuration file (TOML format)
- **`Cargo.toml`**: Auto-synchronized version field
- **`~/.shipwreck/`**: Global version configuration (if configured)

### Error Handling
- Graceful fallback if version operations fail
- Non-blocking integration with cargo operations
- Comprehensive error messages and logging
- Version operations don't block cargo execution

### Performance
- Minimal overhead on cargo operations
- Fast version file parsing and updates
- Efficient Cargo.toml synchronization

## Future Enhancements

Planned features for future versions:
- **Version History Tracking**: Git integration for version commits
- **Change Log Generation**: Automatic changelog from version increments
- **Rollback Capabilities**: Restore previous versions
- **Advanced Policies**: Date-based versioning, build number integration
- **Team Collaboration**: Version conflict resolution, branch-specific versioning
- **CI/CD Integration**: Automated version management in pipelines

---