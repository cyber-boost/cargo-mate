## Shell Integration

### `cm install`
**Description**: Install shell integration (bash, zsh, fish)

**Usage**:
```bash
cm install
```

**What it does**:
1. Detects your shell
2. Backs up your RC file
3. Adds cargo function override
4. Creates aliases (cm, cg)
5. Installs completions
6. Sets up auto-config loading

**After installation**:
```bash
source ~/.bashrc  # or ~/.zshrc
# OR use the new command:
cm activate
```

**Result**:
- `cargo` commands routed through cm
- `cm` available directly
- `cg` as quick alias
- Tab completion enabled
- `.cg` files auto-loaded in directories

---