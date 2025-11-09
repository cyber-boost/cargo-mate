## Treasure Map Commands

**Source**: `cargo-mate/captain/src/cmd/map.rs` and `cargo-mate/captain/src/cmd/smune.rs:MapAction`

### `cm map show`
**Description**: Display dependency tree visualization

**Source**: `map.rs:handle_map_internal()` - `MapAction::Show` case

**Usage**:
```bash
cm map show
```

**Implementation** (verified in `map.rs:6-8`):
- Creates `TreasureMap` instance
- Calls `map.show_map()` method (doesn't return Result, just displays)
- Uses `captain::treasure_map::TreasureMap` module

**Output**:
```
🗺️  Treasure Map - Dependency Visualization

📦 my-project v0.1.0
├── 📚 serde v1.0.195
│   └── 📚 serde_derive v1.0.195
├── 📚 tokio v1.35.1
│   ├── 📚 mio v0.8.10
│   └── 📚 bytes v1.5.0
└── 📚 reqwest v0.11.23
```

---

### `cm map analyze`
**Description**: Analyze dependencies for issues

**Source**: `map.rs:handle_map_internal()` - `MapAction::Analyze` case

**Usage**:
```bash
cm map analyze
```

**Implementation** (verified in `map.rs:9-12`):
- Creates `TreasureMap` instance
- Calls `map.analyze()` method which returns analysis object
- Calls `analysis.display()` to show results

**Output**:
```
=== Dependency Analysis ===
📊 Total dependencies: 42
   Direct: 8
   Dev: 5
   Max depth: 4
💾 Total size: 15.3 MB

⚠️  2 duplicate dependencies found:
   rand has versions: 0.7.3, 0.8.5

📦 Largest dependencies:
   tokio v1.35.1 - 2.1 MB
   reqwest v0.11.23 - 1.8 MB
```

---

### `cm map export <path>`
**Description**: Export dependency graph as DOT file

**Source**: `map.rs:handle_map_internal()` - `MapAction::Export` case

**Usage**:
```bash
cm map export ./deps.dot
dot -Tpng deps.dot -o deps.png  # Generate image with graphviz
```

**Implementation** (verified in `map.rs:13-15`):
- Creates `TreasureMap` instance
- Calls `map.export_dot(&path)` method
- Exports dependency graph in DOT format for graphviz

---

### `cm map path <from> <to>`
**Description**: Find dependency path between two crates

**Source**: `map.rs:handle_map_internal()` - `MapAction::Path` case

**Usage**:
```bash
cm map path serde tokio
```

**Implementation** (verified in `map.rs:16-26`):
- Creates `TreasureMap` instance
- Calls `map.find_path(&from, &to)` method
- If path found: prints numbered list with crate names in cyan
- If no path: prints "No path found between {from} and {to}"

**Output**:
```
📍 Path from serde to tokio:
  1. serde
  2. serde_json
  3. tower
  4. tokio
```

---

## Implementation Module

- **TreasureMap**: `cargo-mate/captain/src/captain/treasure_map.rs` - Dependency tree visualization

## All Map Actions

Verified from `smune.rs:MapAction` enum:
- `Show` - Display dependency tree
- `Analyze` - Analyze dependencies for issues
- `Export { path: PathBuf }` - Export as DOT file
- `Path { from: String, to: String }` - Find dependency path

---