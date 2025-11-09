## Checklist Commands

**Source**: `cargo-mate/captain/src/cmd/checklist.rs` and `cargo-mate/captain/src/cmd/smune.rs:ChecklistAction`

### `cm checklist show` or `cm checklist list`
**Description**: Show current checklist items

**Source**: `checklist.rs:handle_checklist_internal()` - `ChecklistAction::Show` or `ChecklistAction::List` case

**Usage**:
```bash
cm checklist show
cm checklist list
```

**Implementation** (verified in `checklist.rs:25-44`):
- Loads checklist from `~/.shipwreck/checklists/items.json`
- Prints "📋 Checklist is empty" with help message if empty
- Otherwise prints "📋 Current Checklist:" header with cyan separator
- Shows each item with: ID, checkbox (☐ or ☑️), text, status (✅ or ❌)
- Prints progress summary: "📊 Progress: X/Y items completed"

**Output**:
```
📋 Current Checklist:
════════════════════════════════════════════════════════════
1. ☐ Fix memory leak in parser ❌
2. ☑️ Add async support ✅
3. ☐ Update documentation ❌

📊 Progress: 1/3 items completed
```

---

### `cm checklist add <item>`
**Description**: Add a new item to the checklist

**Source**: `checklist.rs:handle_checklist_internal()` - `ChecklistAction::Add` case

**Usage**:
```bash
cm checklist add "Fix memory leak in parser"
cm checklist add "Add async support"
```

**Implementation** (verified in `checklist.rs:45-56`):
- Loads existing checklist items
- Generates next ID (max existing ID + 1, or 1 if empty)
- Creates new `ChecklistItem` with:
  - `id`: Auto-incremented
  - `text`: Provided item text
  - `done`: false
  - `created_at`: Current UTC timestamp in RFC3339 format
- Saves to `~/.shipwreck/checklists/items.json` as pretty JSON
- Prints success message with item ID and next step hint

**Output**:
```
✅ Added item #1: Fix memory leak in parser
💡 Mark as done with: cm checklist done 1
```

---

### `cm checklist done <items>`
**Description**: Mark checklist items as completed

**Source**: `checklist.rs:handle_checklist_internal()` - `ChecklistAction::Done` case

**Usage**:
```bash
cm checklist done 1
cm checklist done 1,2,3
cm checklist done "1, 2, 3"
```

**Implementation** (verified in `checklist.rs:57-73`):
- Parses comma-separated item IDs from string
- Filters out invalid IDs (non-numeric)
- Marks matching items as done (only if not already done)
- Saves updated checklist to JSON file
- Prints success message with count of marked items
- If no items marked: prints error message

**Output**:
```
✅ Marked 2 item(s) as completed: 1,2
```

---

### `cm checklist clear [target]`
**Description**: Clear checklist items (default: "done")

**Source**: `checklist.rs:handle_checklist_internal()` - `ChecklistAction::Clear` case

**Usage**:
```bash
cm checklist clear        # Remove completed items (default)
cm checklist clear done   # Remove completed items
cm checklist clear all    # Remove all items
```

**Implementation** (verified in `checklist.rs:74-95`):
- Default target is "done" (from `smune.rs:ChecklistAction::Clear` definition)
- `"all"`: Clears all items from checklist
- `"done"`: Removes only completed items (retains incomplete items)
- Invalid target: Prints error message
- Saves updated checklist to JSON file

**Output**:
```
🗑️  Removed completed items from checklist
```

or

```
🗑️  Cleared all checklist items
```

---

## Checklist Storage

**Location**: `~/.shipwreck/checklists/items.json`  
**Format**: JSON array of ChecklistItem objects

**ChecklistItem Structure** (verified in `checklist.rs:97-102`):
```json
{
  "id": 1,
  "text": "Fix memory leak in parser",
  "done": false,
  "created_at": "2024-01-20T14:30:00Z"
}
```

## Implementation Details

**File Management** (verified in `checklist.rs:6-24`):
- Creates `~/.shipwreck/checklists/` directory if it doesn't exist
- Loads items from `items.json` file
- Handles empty file gracefully (treats as empty array)
- Handles JSON parse errors gracefully (treats as empty array)
- Saves items as pretty-printed JSON

**ID Generation**:
- Auto-increments from maximum existing ID
- Starts at 1 if checklist is empty
- IDs are unique per checklist

**Item Status**:
- `done: false` - Item not completed (shown as ☐ ❌)
- `done: true` - Item completed (shown as ☑️ ✅)

## All Checklist Actions

Verified from `smune.rs:ChecklistAction` enum:
- `Show` - Display checklist (same as List)
- `List` - Display checklist (same as Show)
- `Add { item: String }` - Add new item
- `Done { items: String }` - Mark items as done (comma-separated IDs)
- `Clear { target: String }` - Clear items (default: "done")

---

