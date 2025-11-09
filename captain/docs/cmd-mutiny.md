## Mutiny Mode Commands

**Source**: `cargo-mate/captain/src/cmd/mutiny.rs` and `cargo-mate/captain/src/cmd/smune.rs:MutinyAction`

### `cm mutiny activate <reason>`
**Description**: Activate mutiny mode to override cargo restrictions

**Source**: `mutiny.rs:handle_mutiny_internal()` - `MutinyAction::Activate` case

**Usage**:
```bash
cm mutiny activate "Quick prototype testing"
```

**Implementation** (verified in `mutiny.rs:5-7`):
- Creates `MutinyMode` instance
- Calls `mutiny.activate(&reason)` with provided reason string
- Uses `mutiny::MutinyMode` module for state management

---

### `cm mutiny deactivate`
**Description**: Deactivate mutiny mode

**Source**: `mutiny.rs:handle_mutiny_internal()` - `MutinyAction::Deactivate` case

**Usage**:
```bash
cm mutiny deactivate
```

**Implementation** (verified in `mutiny.rs:8-10`):
- Creates `MutinyMode` instance
- Calls `mutiny.deactivate()` method

---

### `cm mutiny allow-warnings`
**Description**: Allow warnings for 1 hour

**Source**: `mutiny.rs:handle_mutiny_internal()` - `MutinyAction::AllowWarnings` case

**Usage**:
```bash
cm mutiny allow-warnings
```

**Implementation** (verified in `mutiny.rs:11-13`):
- Creates `MutinyMode` instance
- Calls `mutiny.allow_warnings()` method
- Duration and behavior handled by MutinyMode module

---

### `cm mutiny skip-tests`
**Description**: Skip tests when building

**Source**: `mutiny.rs:handle_mutiny_internal()` - `MutinyAction::SkipTests` case

**Usage**:
```bash
cm mutiny skip-tests
```

**Implementation** (verified in `mutiny.rs:14-16`):
- Creates `MutinyMode` instance
- Calls `mutiny.skip_tests()` method

---

### `cm mutiny force`
**Description**: Force build ignoring dirty state and lockfile

**Source**: `mutiny.rs:handle_mutiny_internal()` - `MutinyAction::Force` case

**Usage**:
```bash
cm mutiny force
```

**Implementation** (verified in `mutiny.rs:17-19`):
- Creates `MutinyMode` instance
- Calls `mutiny.force_build()` method

---

### `cm mutiny yolo`
**Description**: DANGEROUS - Disable ALL checks for 30 minutes

**Source**: `mutiny.rs:handle_mutiny_internal()` - `MutinyAction::Yolo` case

**Usage**:
```bash
cm mutiny yolo
```

**Implementation** (verified in `mutiny.rs:20-22`):
- Creates `MutinyMode` instance
- Calls `mutiny.yolo_mode()` method
- Duration and disabled checks handled by MutinyMode module

**⚠️ WARNING**: This disables:
- All lints
- All tests
- Format checking
- Security audits
- Lockfile checking

---

### `cm mutiny status`
**Description**: Show current mutiny mode status

**Source**: `mutiny.rs:handle_mutiny_internal()` - `MutinyAction::Status` case

**Usage**:
```bash
cm mutiny status
```

**Output**:
```
=== Mutiny Mode Status ===
Status: 🏴‍☠️ ACTIVE

📋 Active Overrides:
   allow_warnings - Temporarily allowing warnings
      Expires in: 45 minutes

🚩 Forced Flags:
   --cap-lints=warn

⏭️  Skipped Checks:
   test
```

**Implementation** (verified in `mutiny.rs:23-25`):
- Creates `MutinyMode` instance
- Calls `mutiny.status()` method (doesn't return Result, just displays)

---

## Mutiny Mode Storage

**Location**: `~/.shipwreck/mutiny.toml` (handled by MutinyMode module)  
**Format**: TOML configuration file

## Implementation Module

- **MutinyMode**: `cargo-mate/captain/src/mutiny.rs` - Mutiny state management

## All Mutiny Actions

Verified from `smune.rs:MutinyAction` enum:
- `Activate { reason: String }`
- `Deactivate`
- `AllowWarnings`
- `SkipTests`
- `Force`
- `Yolo`
- `Status`

---