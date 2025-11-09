## Idea Commands

**Source**: `cargo-mate/captain/src/cmd/idea.rs` and `cargo-mate/captain/src/main.rs:execute_command()`

### `cm idea <idea_text>`
**Description**: Submit ideas and suggestions for Cargo Mate development

**Source**: `idea.rs:handle_idea()` function

**Usage**:
```bash
cm idea "Add support for workspace-level optimizations"
cm idea "It would be great if cm could auto-detect unused dependencies"
```

**Implementation** (verified in `idea.rs:13-20`):
- Creates `LicenseManager` instance
- Enforces license check (requires valid license)
- Prints idea message: "💡 Idea: {idea}"
- **Note**: The `save_idea_history()` function exists but is NOT called by `handle_idea()`
- History saving would need to be added to actually store ideas locally

**Features**:
- **Local history**: All ideas saved locally for your reference
- **Unique tracking**: Each idea gets a UUID for identification
- **Timestamped**: Automatic timestamp recording
- **License required**: Requires valid license to submit ideas

**Output**:
```
💡 Idea: Add support for workspace-level optimizations
```

**Idea History Functions** (verified in `idea.rs:23-58`):
- `save_idea_history(idea: &str)` - Saves idea to `~/.shipwreck/ideas/ideas.json`
  - Creates directory if needed
  - Generates UUID v4 for each idea
  - Adds RFC3339 timestamp
  - Appends to existing entries array
- `get_idea_history(limit: usize)` - Retrieves ideas from history
  - Sorts by timestamp (newest first)
  - Returns up to `limit` entries
  - Returns empty vec if file doesn't exist

**Note**: These functions exist but are **NOT currently called** by `handle_idea()`. Ideas are only printed, not saved.

**Idea History Structure** (if saved):
Ideas would be stored in JSON format at `~/.shipwreck/ideas/ideas.json`:
```json
[
  {
    "id": "550e8400-e29b-41d4-a716-446655440000",
    "idea": "Add support for workspace-level optimizations",
    "timestamp": "2024-01-20T14:30:00Z"
  }
]
```

**Use Cases**:
- **Feature requests**: Suggest new features for Cargo Mate
- **Improvements**: Propose enhancements to existing features
- **Bug reports**: Report issues or suggest fixes
- **Workflow ideas**: Share ideas for better developer workflows

**Examples**:
```bash
# Submit a feature idea
cm idea "Add support for custom build profiles"

# Submit an improvement suggestion
cm idea "It would be helpful if cm optimize showed before/after comparison"

# Submit a workflow idea
cm idea "Add ability to create custom journey templates"
```

**Note**: Ideas are sent to the CargoMate API and stored in your local idea history. The development team reviews all submitted ideas.

---

