## User Commands

**Source**: `cargo-mate/captain/src/main.rs:execute_command()` - `Commands::User` case

### `cm user`
**Description**: Display user information, license status, and CargoMate Pro features availability

**Usage**:
```bash
cm user
```

**Implementation** (verified in `main.rs:277-280`):
- Currently prints: "👤 User management requires captain binary"
- Returns `Ok(())` immediately
- **Note**: This is a stub implementation - full functionality requires captain binary
- License check is performed before this command (verified in `main.rs:license_manager.enforce_license("user")`)

**What it currently does**:
- Prints message indicating captain binary is required
- No actual user information display (stub implementation)

**Output**:
```
👤 User Information:
══════════════════════════════════════════════════
📧 Email: user@example.com
📋 License Status: Active
📊 License Type: Pro
⏰ Expires: 2024-12-31
🎯 Pro Features: Enabled

✅ Available Features:
   - WTF (CargoMate AI)
   - Advanced analytics
   - Priority support
══════════════════════════════════════════════════
```

**Features**:
- **User details**: Shows registered user information
- **License status**: Displays current license state (Active/Inactive/Trial)
- **Feature access**: Lists available Pro features
- **License info**: Shows license type, expiration, and remaining commands

**Use Cases**:
1. **Status check**: Verify your account and license status
2. **Feature access**: See which Pro features are available
3. **Troubleshooting**: Check if license is properly configured
4. **Account info**: View your user account details

**Examples**:
```bash
# View user information
cm user
```

**Related Commands**:
- `cm register` - Register or check license status
- `cm wtf` - Access Pro AI features (requires active license)

**Note**: This command requires a valid license. It provides comprehensive information about your Cargo Mate account and available features.

---

