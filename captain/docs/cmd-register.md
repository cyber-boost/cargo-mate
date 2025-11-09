## Register Commands

**Source**: `cargo-mate/captain/src/cmd/register.rs` and `cargo-mate/captain/src/main.rs:execute_command()`

### `cm register [<license_key>] [--status] [--remaining]`
**Description**: Register Cargo Mate with a license key or check license status

**Source**: `main.rs:execute_command()` - `Commands::Register` case

**Usage**:
```bash
# Register with license key
cm register CM-12345-67890-ABCDE

# Check current license status
cm register --status

# Show remaining commands count
cm register --remaining

# Show registration help
cm register
```

**Implementation** (verified in `main.rs:243-252`):
- **Note**: The actual registration logic is in `main.rs`, not `register.rs`
- In `main.rs`: Sets config values via `ConfigManager::set()`:
  - If `license_key` provided: sets `"license.key"` to key value (local config)
  - If `--status` flag: sets `"license.status"` to "true" (local config)
  - If `--remaining` flag: sets `"license.remaining"` to "true" (local config)
- The `register.rs:handle_register()` function is a stub that just prints registration info
- Actual license enforcement happens via `LicenseManager::enforce_license("register")`

**What it does**:
- Stores license key in local config (`license.key`)
- Sets license status flag (`license.status`)
- Sets remaining commands flag (`license.remaining`)
- License validation handled by `LicenseManager` module

**Options**:
- `<license_key>`: License key to register (format: `CM-XXXXX-XXXXX-XXXXX`)
- `--status`: Check current license status without registering
- `--remaining`: Show only remaining commands count

**License Key Format**:
License keys follow the format: `CM-XXXXX-XXXXX-XXXXX`
- Example: `CM-12345-67890-ABCDE`
- Each segment is 5 characters (alphanumeric)

**Output Examples**:

**Registration**:
```
📝 Registration: license_key=Some("CM-12345-67890-ABCDE"), status=false, remaining=false
```

**Status Check**:
```
📋 License Status: Active
```

**Remaining Commands**:
```
📊 Remaining Commands: 150
```

**Configuration Storage** (verified in `main.rs:243-252`):
License information is stored in **local config** (`.cg` file):
- License key: `license.key` (set when `<license_key>` provided)
- License status: `license.status` (set when `--status` flag used)
- Remaining commands: `license.remaining` (set when `--remaining` flag used)
- All values stored with `local: true` flag (project-specific config)

**Use Cases**:
1. **Activation**: Register your license key to activate Pro features
2. **Status check**: Verify your license is active
3. **Trial tracking**: Monitor remaining commands on trial licenses
4. **Troubleshooting**: Check license configuration

**Examples**:
```bash
# Register with license key
cm register CM-12345-67890-ABCDE

# Check if license is active
cm register --status

# See how many commands left
cm register --remaining

# Get help
cm register
```

**Related Commands**:
- `cm user` - Display user information and license status
- `cm wtf` - Access Pro features (requires active license)

**Note**: License registration is required to access Pro features like WTF (CargoMate AI). Free features are available without registration.

---

