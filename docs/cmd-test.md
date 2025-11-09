## Test Commands

**Source**: `cargo-mate/captain/src/cmd/test.rs` and `cargo-mate/captain/src/main.rs:execute_command()`

### `cm test`
**Description**: Test command for error handling - generates a test error to verify error logging functionality

**Source**: `test.rs:handle_test()` function

**Usage**:
```bash
cm test
```

**What it does** (verified in `test.rs:3-19`):
- Prints test command start message
- Creates `~/.shipwreck/errors/` directory if it doesn't exist
- Generates a deliberate test error message with:
  - Test error prefix: "🧪 Test Error: This is a deliberate test error from the test command"
  - Current UTC timestamp in RFC3339 format
  - Command name: "cm test"
  - Error description: "Test error - demonstrating error logging functionality"
- Writes error to `~/.shipwreck/errors/latest.txt`
- Prints file path, success message, and hint to view errors

**Output**:
```
🧪 Running test command that will generate and log an error...
📝 Error logged to: /home/user/.shipwreck/errors/latest.txt
✅ Test error successfully logged!
💡 Now run 'cm view errors' to see this error
```

**Error Log Format**:
The test error is logged with the following format:
```
🧪 Test Error: This is a deliberate test error from the test command
Time: 2024-01-20T14:30:00Z
Command: cm test
Error: Test error - demonstrating error logging functionality
```

**Use Cases**:
1. **Testing error system**: Verify that error logging works correctly
2. **Development**: Test error viewing and display functionality
3. **Debugging**: Check if error directory structure is set up properly
4. **Documentation**: Demonstrate error logging to users

**Related Commands**:
- `cm view errors` - View logged errors
- `cm view latest` - Quick view of latest issues

**Examples**:
```bash
# Generate test error
cm test

# View the logged error
cm view errors

# Quick view of latest
cm view latest
```

**Note**: This command is primarily for testing and development purposes. It intentionally generates an error to verify the error logging system is working correctly.

---

