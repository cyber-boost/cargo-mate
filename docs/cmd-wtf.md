## WTF Commands (CargoMate AI - Pro Only)

**Source**: `cargo-mate/captain/src/cmd/wtf.rs` and `cargo-mate/captain/src/captain/wtf.rs`

### `cm wtf <action>`
**Description**: CargoMate AI - Ask questions and get intelligent assistance with your Rust code

**Source**: `wtf.rs:handle_wtf()` - Delegates to `captain::wtf::handle_wtf_action()`

**Usage**:
```bash
# Use subcommands (no direct question parameter in current implementation)
cm wtf ask "How do I handle async errors?"
cm wtf er 10
cm wtf checklist 5
```

**Implementation** (verified in `wtf.rs:3-5`):
- Delegates to `cargo-mate/captain/src/captain/wtf.rs:handle_wtf_action()`
- Full implementation is in the captain module
- Requires valid license (enforced in `main.rs`)

**Subcommands**:

#### `cm wtf ask <input> [--file]`
Ask CargoMate AI a question about your code or Rust development.

**Source**: `captain/wtf.rs:handle_wtf_action()` - `WtfAction::Ask` case

**Verified from** `captain/wtf.rs:WtfAction` enum:
- `Ask { input: String, file: bool }` - Ask question (with optional `--file` flag)

```bash
cm wtf ask "How do I optimize my Rust code?"
cm wtf ask "What's the best way to handle errors in async code?"
cm wtf ask "Explain this code" --file  # Process file content
```

**Implementation** (verified in `captain/wtf.rs:54-56`):
- Calls `handle_wtf(&input, file)` function
- `--file` flag indicates file processing mode

#### `cm wtf er [count]`
Send recent errors to CargoMate AI for analysis (default: 10 errors).

**Source**: `captain/wtf.rs:handle_wtf_action()` - `WtfAction::Er` case

**Verified from** `captain/wtf.rs:WtfAction` enum:
- `Er { count: usize }` - Error analysis (default: 10)

```bash
cm wtf er          # Send last 10 errors
cm wtf er 5        # Send last 5 errors
cm wtf er 20       # Send last 20 errors
```

**Implementation** (verified in `captain/wtf.rs:61-63`):
- Calls `handle_wtf_errors(count)` function
- Default count is 10 (from enum definition)

#### `cm wtf checklist [limit]`
Send recent checklist items to CargoMate AI for suggestions (default: 10 items).

**Source**: `captain/wtf.rs:handle_wtf_action()` - `WtfAction::Checklist` case

**Verified from** `captain/wtf.rs:WtfAction` enum:
- `Checklist { limit: usize }` - Checklist analysis (default: 10)

```bash
cm wtf checklist        # Send last 10 checklist items
cm wtf checklist 5      # Send last 5 items
```

**Implementation** (verified in `captain/wtf.rs:76-78`):
- Calls `handle_wtf_checklist(limit)` function
- Default limit is 10 (from enum definition)

#### `cm wtf list [limit]`
List recent conversations with CargoMate AI (default: 10).

**Source**: `captain/wtf.rs:handle_wtf_action()` - `WtfAction::List` case

**Verified from** `captain/wtf.rs:WtfAction` enum:
- `List { limit: usize }` - List conversations (default: 10)

```bash
cm wtf list        # Show last 10 conversations
cm wtf list 20     # Show last 20 conversations
```

**Implementation** (verified in `captain/wtf.rs:67-69`):
- Calls `handle_wtf_list(limit)` function
- Default limit is 10 (from enum definition)

#### `cm wtf show <id>`
Show a specific conversation by ID.

**Source**: `captain/wtf.rs:handle_wtf_action()` - `WtfAction::Show` case

**Verified from** `captain/wtf.rs:WtfAction` enum:
- `Show { id: String }` - Show conversation by ID

```bash
cm wtf show abc123def456
```

**Implementation** (verified in `captain/wtf.rs:70-72`):
- Calls `handle_wtf_show(&id)` function

#### `cm wtf history [limit]`
Show conversation history (default: 10).

**Source**: `captain/wtf.rs:handle_wtf_action()` - `WtfAction::History` case

**Verified from** `captain/wtf.rs:WtfAction` enum:
- `History { limit: usize }` - Show history (default: 10)

```bash
cm wtf history
cm wtf history 30
```

**Implementation** (verified in `captain/wtf.rs:73-75`):
- Calls `handle_wtf_list(limit)` function (same as `list` command)
- Default limit is 10 (from enum definition)

#### `cm wtf ollama <command>`
Configure local Ollama integration for offline AI assistance.

**Source**: `captain/wtf.rs:handle_wtf_action()` - `WtfAction::Ollama` case

**Verified from** `captain/wtf.rs:OllamaCommand` enum:
- `Enable { model: String }` - Enable Ollama (default model: "llama2")
- `Disable` - Disable Ollama
- `Status` - Show Ollama status
- `Models` - List available models

```bash
# Enable Ollama with default model
cm wtf ollama enable

# Enable with specific model
cm wtf ollama enable llama2
cm wtf ollama enable codellama

# Check status
cm wtf ollama status

# List available models
cm wtf ollama models

# Disable Ollama
cm wtf ollama disable
```

**Implementation** (verified in `captain/wtf.rs:64-66`):
- Calls `handle_ollama_command(command)` function

```bash
# Enable Ollama with default model
cm wtf ollama enable

# Enable with specific model
cm wtf ollama enable llama2
cm wtf ollama enable codellama

# Check status
cm wtf ollama status

# List available models
cm wtf ollama models

# Disable Ollama
cm wtf ollama disable
```

**Features**:
- **AI-powered assistance**: Get intelligent answers about Rust development
- **Error analysis**: Automatically analyze build errors and suggest fixes
- **Checklist help**: Get suggestions for fixing checklist items
- **Conversation history**: Track all your interactions with CargoMate AI
- **Local AI support**: Use Ollama for offline AI assistance
- **Pro feature**: Requires active CargoMate Pro license

**Architecture** (verified in source):
WTF functionality is implemented in:
- **Command Handler** (`cmd/wtf.rs`): Thin wrapper that delegates to captain module
- **Implementation** (`captain/wtf.rs`): Full implementation with:
  - AI integration (cargo.do API)
  - History management (`WtfHistoryEntry` struct)
  - Ollama integration for local models
  - Conversation tracking and storage

**WtfHistoryEntry Structure** (verified in `captain/wtf.rs:42-51`):
```rust
pub struct WtfHistoryEntry {
    pub id: String,
    pub user_input: String,
    pub ai_response: String,
    pub timestamp: String,
    pub is_file: bool,
    pub cost_cents: Option<i64>,
    pub usage_id: Option<String>,
}
```

**Storage**: `~/.shipwreck/wtf_history/` (handled by wtf module)

**Use Cases**:
1. **Error debugging**: Get help understanding and fixing compilation errors
2. **Code optimization**: Ask for suggestions to improve code performance
3. **Best practices**: Learn Rust best practices and patterns
4. **Problem solving**: Get assistance with complex coding challenges
5. **Learning**: Understand Rust concepts and features

**Examples**:
```bash
# Quick question
cm wtf "How do I handle Result types?"

# Analyze recent errors
cm wtf er 10

# Get help with checklist items
cm wtf checklist 5

# Use local AI
cm wtf ollama enable llama2
cm wtf "Explain async/await in Rust"

# View conversation history
cm wtf history
cm wtf show <conversation-id>
```

**License Requirements**:
- **Pro License Required**: WTF is a Pro-only feature
- **Registration**: Use `cm register` to activate your license
- **Status Check**: Use `cm user` to verify license status

**Related Commands**:
- `cm register` - Register your Pro license
- `cm user` - Check license status
- `cm view errors` - View build errors
- `cm checklist` - View checklist items

**Note**: WTF (CargoMate AI) is a sophisticated Pro feature that provides intelligent assistance for Rust development. It requires an active CargoMate Pro license to use.

---

