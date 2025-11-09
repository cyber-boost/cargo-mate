# 🌳 Tree - Beautiful Directory Tree Generator

**Source**: `cargo-mate/captain/src/cmd/tree.rs` and `cargo-mate/captain/src/cmd/smune.rs:TreeAction`

Tree is a powerful command that generates beautiful, markdown-formatted directory trees perfect for README files and project documentation.

## Overview

**Main Handler**: `tree.rs:handle_tree()` (verified in `tree.rs:428-500`)

Tree recursively scans your project directory and creates a beautifully formatted markdown file showing the complete directory structure. It includes options for file sizes, line counts, modification dates, and multiple styling options.

## Usage

### Basic Usage

```bash
# Generate tree for current directory
cm tree

# Generate tree for specific directory
cm tree -t ./src

# Specify output file
cm tree -o PROJECT_STRUCTURE.md

# Full example with options
cm tree -t ./src -o structure.md --file-size --line-count --dates
```

### Command Options

- `-t, --target <PATH>`: Target directory to scan (default: `.`)
- `-o, --out <PATH>`: Output file path (default: `cm-tree-[timestamp].md`)
- `--no-folders`: Exclude directories from the tree
- `--no-files`: Exclude files from the tree
- `--folder-size`: Include folder sizes (calculated from contained files)
- `--file-size`: Include file sizes
- `--line-count`: Count lines in files (slower but informative)
- `--dates`: Include modification dates
- `--style <STYLE>`: Choose output style (default: `readme`)
  - `basic`: Simple, minimal tree
  - `readme`: README-friendly format with emojis
  - `cm`: Cargo Mate branded style
  - `hard`: Detailed, information-dense format
  - `easy`: Simple, easy-to-read format
- `--yolo`: Activate YOLO mode 🎉 (adds fun message)

### Subcommands

**Source**: `tree.rs:handle_tree()` matches on `TreeAction` enum (verified in `tree.rs:444-453`)

#### History

**Source**: `tree.rs:handle_tree_history()` (verified in `tree.rs:534-566`)

View all previously generated trees:

```bash
cm tree history
```

**Implementation**:
- Calls `list_tree_history()` to get all trees from `~/.shipwreck/trees/`
- Displays each tree with filename and path
- Extracts timestamp from filename format `cm-tree-{timestamp}.md`

Lists all trees saved in `~/.shipwreck/trees/` with timestamps.

#### Show

**Source**: `tree.rs:handle_tree_show()` (verified in `tree.rs:568-592`)

Display a specific tree from history:

```bash
cm tree show cm-tree-20241201_143022
cm tree show 20241201  # Partial match works too
```

**Implementation**:
- Searches history for trees matching the name (partial match supported)
- Reads and displays the full tree content
- Falls back to first tree if exact match not found

#### Find

**Source**: `tree.rs:handle_tree_find()` (verified in `tree.rs:593-620`)

Search through tree history for specific content:

```bash
cm tree find "src"
cm tree find "config"
```

**Implementation**:
- Reads all tree files from history
- Performs case-insensitive search for query string
- Displays matching trees with filenames and paths

Searches the content of all saved trees for the query string.

## Examples

### Example 1: Basic README Tree

```bash
cm tree -t . -o PROJECT_STRUCTURE.md --style readme
```

Generates a README-friendly tree with emojis and nice formatting.

### Example 2: Detailed Analysis

```bash
cm tree --file-size --line-count --dates --folder-size
```

Creates a comprehensive tree with all metadata enabled.

### Example 3: Files Only

```bash
cm tree --no-folders --file-size --line-count
```

Shows only files with their sizes and line counts, perfect for code analysis.

### Example 4: YOLO Mode

```bash
cm tree --yolo --style hard
```

Generates a tree in HARD MODE with YOLO activation message! 🎉

## Output Format

The generated markdown file includes:

1. **Header**: Style-specific header with project information
2. **Tree Structure**: Beautiful ASCII tree with proper indentation
3. **Metadata**: Optional file/folder sizes, line counts, and dates
4. **Summary**: Statistics about directories, files, total size, and lines
5. **Footer**: Generation timestamp

### Example Output

```markdown
# 📁 Project Structure

This document shows the directory structure of the project.

```
project/
├── src/
│   ├── main.rs  // 2.5 KB, 150 lines, modified: 2024-12-01
│   ├── lib.rs   // 1.2 KB, 80 lines, modified: 2024-12-01
│   └── utils/
│       └── helper.rs  // 0.8 KB, 45 lines, modified: 2024-11-30
├── Cargo.toml  // 0.5 KB, 25 lines, modified: 2024-12-01
└── README.md   // 3.2 KB, 120 lines, modified: 2024-12-01
```

## Summary

- **Directories**: 2
- **Files**: 5
- **Total Size**: 8.2 KB
- **Total Lines**: 420

---
*Generated on 2024-12-01 14:30:22 by Cargo Mate*
```

## Style Options

### Basic
Simple, minimal tree without extra formatting:
```markdown
# Directory Tree
```

### Readme (Default)
README-friendly format with emojis and helpful text:
```markdown
# 📁 Project Structure

This document shows the directory structure of the project.
```

### CM (Cargo Mate)
Cargo Mate branded style:
```markdown
# 🚢 Cargo Mate Project Tree

Generated by Cargo Mate `tree` command.
```

### Hard
Information-dense format with warning:
```markdown
# ⚡ HARD MODE: Project Structure

**Warning**: This tree contains detailed information. Handle with care.
```

### Easy
Simple, easy-to-read format:
```markdown
# 🌟 Easy Mode: Project Structure

A simple, easy-to-read directory tree.
```

## Tree History

All generated trees are automatically saved to `~/.shipwreck/trees/` for future reference. This allows you to:

- Track project structure changes over time
- Compare different versions
- Search through historical trees
- Quickly access previous trees

### History Management

```bash
# List all trees
cm tree history

# Show a specific tree
cm tree show <name>

# Search trees
cm tree find <query>
```

## Use Cases

### 1. README Documentation
Perfect for adding a project structure section to your README:

```bash
cm tree -t . -o PROJECT_STRUCTURE.md --style readme
# Then copy the content to your README.md
```

### 2. Project Analysis
Analyze project size and complexity:

```bash
cm tree --file-size --line-count --folder-size
```

### 3. Code Review
Generate a tree before code review to understand structure:

```bash
cm tree -t ./src --line-count --dates
```

### 4. Documentation
Include in project documentation:

```bash
cm tree -t . -o docs/structure.md --style cm
```

## Performance

- **Fast**: Basic tree generation is very fast
- **Line Counting**: Adding `--line-count` will slow down generation as it reads each file
- **Folder Sizes**: `--folder-size` requires scanning all files in each directory
- **Large Projects**: Works efficiently even on large projects with thousands of files

## Tips

1. **For READMEs**: Use `--style readme` for best results
2. **For Analysis**: Enable `--file-size` and `--line-count` together
3. **Quick Preview**: Generate without options first, then add metadata if needed
4. **History**: Check `cm tree history` to see all your generated trees
5. **Search**: Use `cm tree find` to locate trees containing specific paths or files

## Integration

Tree is integrated with Cargo Mate's license system and requires a valid license to use. All generated trees are automatically saved to the history directory for easy access later.

## See Also

- `cm liberate` - Generate lib.rs from project files
- `cm map` - Visualize dependency maps
- `cm deps` - Analyze dependencies

---

**Note**: Tree is perfect for keeping your README files up-to-date with your project structure. Generate it regularly to maintain accurate documentation!

