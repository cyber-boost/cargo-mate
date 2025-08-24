# Cargo Mate Examples

This directory contains example code demonstrating how to use Cargo Mate in real development workflows.

## 📁 Examples

### [`basic_workflow.rs`](./basic_workflow.rs)
Demonstrates a simple development workflow using Cargo Mate:
- Saving project states with anchors
- Recording and replaying development workflows
- Basic performance monitoring

**Run this example:**
```bash
# This example shows what cargo-mate commands look like
# The actual functionality is in the cargo-mate binary
rustc --example basic_workflow && ./basic_workflow
```

### [`advanced_features.rs`](./advanced_features.rs)
Showcases advanced Cargo Mate capabilities:
- Complex multi-stage workflows
- State management and auto-save features
- Performance optimization and monitoring
- Workflow recording and publishing
- Task checklists and logging

## 🏗️ Architecture Note

These examples demonstrate the **published crate** functionality. The real Cargo Mate implementation contains 3,000+ lines of Rust code with advanced features like:

- AI-powered development assistance (`cm wtf`)
- Comprehensive error analysis and suggestions
- Automated version management
- Cross-platform shell integration
- Protected binary distribution system

## 🚀 Getting Started

1. Install Cargo Mate:
   ```bash
   cargo install cargo-mate
   ```

2. Try the examples:
   ```bash
   cm --help              # See all available commands
   cm journey record test # Record your first workflow
   cm anchor save state   # Save current project state
   ```

## 📚 Documentation

For complete documentation and more examples, visit:
- [Cargo Mate Website](https://cargo.do)
- [GitHub Repository](https://github.com/cyber-boost/cargo-mate)

## 🤝 Contributing

These examples are part of the published crate. For contributing to the actual Cargo Mate implementation, see the main repository's contribution guidelines.
