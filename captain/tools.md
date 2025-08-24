# 🚀 **Cargo Mate Performance Tools - Complete Command Reference**

## **Overview**

Cargo Mate now includes three powerful performance analysis tools to help you optimize your Rust applications:

- **`bloat-check`** - Binary size analysis and optimization
- **`cache-analyzer`** - CPU cache usage and performance monitoring
- **`async-lint`** - Async programming pattern detection and improvement

---

## **🎯 Performance Tools Quick Start**

### **Get Help**
```bash
cm tool list                    # See all available tools
cm tool help bloat-check       # Help for bloat-check
cm tool help cache-analyzer    # Help for cache-analyzer
cm tool help async-lint        # Help for async-lint
```

### **Quick Performance Check**
```bash
# Check binary size
cm tool bloat-check --binary target/release/myapp

# Analyze cache usage
cm tool cache-analyzer --target target/release/myapp --functions process_data,handle_request

# Lint async code
cm tool async-lint --input src/ --blocking --await --deadlock
```

---

## **📊 1. bloat-check - Binary Size Analysis**

### **What it does:**
Analyzes your compiled binary size, tracks changes between builds, identifies bloat sources, and suggests optimizations.

### **Perfect for:**
- Reducing application size
- Tracking size regressions
- Optimizing for embedded systems
- Understanding what's taking up space

### **Basic Usage:**
```bash
# Analyze current binary
cm tool bloat-check --binary target/release/myapp

# Compare with previous version
cm tool bloat-check --binary target/release/myapp --baseline previous-build/myapp

# Generate optimization report
cm tool bloat-check --binary target/release/myapp --optimize --report
```

### **Command Options:**

| Flag | Description | Example |
|------|-------------|---------|
| `--binary` | Path to binary to analyze | `--binary target/release/app` |
| `--baseline` | Baseline binary for comparison | `--baseline old-build/app` |
| `--threshold` | Size change threshold (%) | `--threshold 5.0` |
| `--symbols` | Show largest symbols | `--symbols` |
| `--debug-compare` | Compare debug vs release | `--debug-compare` |
| `--optimize` | Generate optimization suggestions | `--optimize` |
| `--report` | Generate detailed size report | `--report` |

### **Example Output:**
```
📊 Binary Size Analysis Report
═══════════════════════════════════════════════

Binary: target/release/myapp
Size: 2.4 MB
Symbols: 1,247

📈 Size Changes:
• Total size: +156 KB (+6.9%)
• Text section: +89 KB (+12.3%) ⚠️
• Data section: +45 KB (+3.2%)
• BSS section: +22 KB (+15.1%) ⚠️

🔍 Largest Symbols:
1. function_a (45.2 KB) - Consider inlining or splitting
2. static_data_b (32.8 KB) - Large static data structure

💡 Optimization Suggestions:
• Use #[inline] for frequently called small functions
• Replace large static arrays with dynamic allocation
```

### **Pro Tips:**
- Run after each major feature addition
- Use `--threshold 1.0` for strict monitoring
- Combine `--symbols --optimize` for detailed analysis
- Compare debug vs release to understand build impact

---

## **🔍 2. cache-analyzer - CPU Cache Analysis**

### **What it does:**
Monitors CPU cache usage, detects cache-unfriendly patterns, analyzes data structures, and suggests cache optimizations.

### **Perfect for:**
- Performance-critical applications
- High-throughput systems
- Memory-bound workloads
- Game development

### **Basic Usage:**
```bash
# Analyze specific functions
cm tool cache-analyzer --target target/release/myapp --functions process_data,handle_request

# Analyze data structures in source
cm tool cache-analyzer --target src/main.rs --data-structures --false-sharing

# Use Linux perf for profiling
cm tool cache-analyzer --target target/release/myapp --perf --threshold 10.0
```

### **Command Options:**

| Flag | Description | Example |
|------|-------------|---------|
| `--target` | Binary or source file to analyze | `--target target/release/app` |
| `--functions` | Functions to analyze (comma-separated) | `--functions func1,func2` |
| `--perf` | Use Linux perf for profiling | `--perf` |
| `--cachegrind` | Use cachegrind for analysis | `--cachegrind` |
| `--data-structures` | Analyze struct layouts | `--data-structures` |
| `--false-sharing` | Detect false sharing issues | `--false-sharing` |
| `--prefetch` | Analyze prefetching efficiency | `--prefetch` |
| `--threshold` | Cache miss rate threshold (%) | `--threshold 5.0` |

### **Example Output:**
```
🔍 CPU Cache Analysis Report
═══════════════════════════════════════════════

Function: process_large_dataset
Cache Miss Rate: 12.3% (High ⚠️)
L1 Cache Hits: 87.7%
L2 Cache Hits: 95.2%

🚨 Issues Detected:
1. High cache misses in loop at line 142
   - Array access pattern causes cache thrashing
   - Consider changing iteration order

📊 Data Structure Analysis:
Struct: LargeDataStructure (size: 128 bytes)
• Cache line alignment: Poor (spans 2 cache lines)
• Hot fields: data[0], metadata.status
• Cold fields: debug_info, timestamps

💡 Optimization Suggestions:
• Reorder struct fields by access frequency
• Use cache-aligned allocations for hot data
```

### **Pro Tips:**
- Focus on hot functions first
- Use `--data-structures` on your main structs
- Look for cache miss rates > 5%
- Consider SOA (Struct of Arrays) for cache optimization

---

## **⚡ 3. async-lint - Async Pattern Detection**

### **What it does:**
Detects common async programming pitfalls, finds blocking operations in async contexts, and suggests improvements.

### **Perfect for:**
- Async web servers
- Concurrent applications
- Network services
- Real-time systems

### **Basic Usage:**
```bash
# Analyze entire source directory
cm tool async-lint --input src/ --blocking --await --deadlock

# Focus on specific issues
cm tool async-lint --input src/main.rs --blocking --fix

# Strict analysis with custom rules
cm tool async-lint --input src/ --strict --ignore async-move,unnecessary-await
```

### **Command Options:**

| Flag | Description | Example |
|------|-------------|---------|
| `--input` | Source file or directory | `--input src/` |
| `--blocking` | Detect blocking operations | `--blocking` |
| `--await` | Analyze await patterns | `--await` |
| `--deadlock` | Detect deadlock risks | `--deadlock` |
| `--concurrency` | Analyze concurrent operations | `--concurrency` |
| `--fix` | Generate fix suggestions | `--fix` |
| `--strict` | Enable strict linting | `--strict` |
| `--ignore` | Rules to ignore (comma-separated) | `--ignore rule1,rule2` |

### **Example Output:**
```
⚡ Async Pattern Analysis Report
═══════════════════════════════════════════════

File: src/handlers.rs
Issues Found: 5

🚨 Blocking Operations in Async Context:
1. Line 45: std::fs::read() in async function
   • Use tokio::fs::read() instead
   • Impact: Blocks async runtime

🔄 Async/Await Issues:
2. Line 123: Unnecessary await on immediate value
   • Code: let x = async { 42 }.await;
   • Suggestion: let x = 42;

🔒 Deadlock Risks:
3. Line 201: Potential deadlock in select! macro
   • Multiple futures competing for same resource
   • Suggestion: Use try_select! or restructure

💡 Improvement Suggestions:
• Replace std::fs with tokio::fs for async file operations
• Use tokio::time::sleep for async delays
```

### **Pro Tips:**
- Run on all async code
- Use `--fix` to get actionable suggestions
- Pay special attention to blocking operations
- Use `--strict` for production code

---

## **🎨 Output Formats**

All tools support multiple output formats:

### **Human Readable (Default)**
```bash
cm tool bloat-check --binary myapp
```

### **JSON Output**
```bash
cm tool bloat-check --binary myapp --output json
```

### **Table Format**
```bash
cm tool bloat-check --binary myapp --output table
```

### **Verbose Mode**
```bash
cm tool bloat-check --binary myapp --verbose
```

---

## **🔧 Common Workflows**

### **Performance Investigation Workflow:**
```bash
# 1. Check binary size first
cm tool bloat-check --binary target/release/app --optimize

# 2. If size is okay, check cache usage
cm tool cache-analyzer --target target/release/app --perf --functions hot_function

# 3. If async, check for patterns
cm tool async-lint --input src/ --blocking --deadlock --fix
```

### **CI/CD Integration:**
```bash
# In your CI pipeline
cm tool bloat-check --binary target/release/app --threshold 2.0
cm tool async-lint --input src/ --strict --blocking
```

### **Development Workflow:**
```bash
# After major changes
cm tool bloat-check --binary target/release/app --baseline main-build/app
cm tool cache-analyzer --target src/ --data-structures --false-sharing
```

---

## **⚠️ Requirements & Dependencies**

### **System Requirements:**
- **Linux**: `perf` tool for cache analysis (`apt install linux-tools-common`)
- **macOS**: Limited cache analysis (perf not available)
- **Windows**: Limited functionality (external tools may not work)

### **Rust Dependencies:**
All tools use existing dependencies:
- `syn` - For AST parsing
- `quote` - For code generation
- `clap` - For CLI parsing
- `colored` - For colored output
- `serde_json` - For JSON output

### **External Tools:**
- `size` - For binary section analysis
- `nm` - For symbol analysis
- `perf` - For cache profiling (optional)

---

## **🚀 Advanced Usage Examples**

### **Comprehensive Performance Audit:**
```bash
# Run all performance tools
cm tool bloat-check --binary target/release/app --symbols --optimize --report
cm tool cache-analyzer --target target/release/app --perf --data-structures --prefetch
cm tool async-lint --input src/ --blocking --await --deadlock --concurrency --fix
```

### **Monitoring Performance Regression:**
```bash
# Before changes
cp target/release/app baseline-app
cm tool bloat-check --binary target/release/app --report > baseline-report.txt

# After changes
cm tool bloat-check --binary target/release/app --baseline baseline-app --threshold 1.0
```

### **Async Code Review:**
```bash
# Review async patterns before merge
cm tool async-lint --input src/ --strict --blocking --deadlock --concurrency --output json | jq '.issues'
```

### **Cache Optimization:**
```bash
# Find cache issues in hot path
cm tool cache-analyzer --target target/release/app --functions hot_function1,hot_function2 --perf --threshold 2.0
```

---

## **📈 Performance Impact**

### **Expected Improvements:**
- **Binary Size**: 10-30% reduction with optimizations
- **Cache Performance**: 20-50% improvement in cache hit rates
- **Async Performance**: 2-10x throughput improvements

### **Typical Use Cases:**
- **Web Services**: 15-25% performance improvement
- **Data Processing**: 30-40% cache efficiency gains
- **Embedded Systems**: 20-35% binary size reduction
- **Games**: 25-45% cache-related performance boosts

---

## **🔍 Troubleshooting**

### **Common Issues:**

**"Tool not available" errors:**
```bash
# Install required tools
sudo apt install linux-tools-common  # Linux
# macOS: Limited functionality expected
```

**"Binary not found" errors:**
```bash
# Build your project first
cargo build --release
cm tool bloat-check --binary target/release/your-app
```

**"Failed to parse Rust file" errors:**
```bash
# Check syntax
cargo check
# Ensure file is valid Rust code
```

**"Permission denied" errors:**
```bash
# Make binary executable
chmod +x target/release/your-app
```

### **Performance Tips:**
- Run tools on release builds for accurate results
- Use `--verbose` for detailed debugging information
- Focus on one tool at a time for deep analysis
- Save baseline measurements for comparison

---

## **🎯 Success Metrics**

### **Good Results:**
- Binary size changes < 5%
- Cache miss rates < 5%
- Zero blocking operations in async code
- No false sharing issues in hot structs

### **Warning Signs:**
- Binary size increases > 10%
- Cache miss rates > 10%
- Multiple blocking operations
- Complex deadlock patterns

### **Optimization Targets:**
- Reduce binary size by 15-25%
- Improve cache hit rates by 20-40%
- Eliminate blocking operations entirely
- Simplify async control flow

---

## **🚀 Getting Started Checklist**

- [ ] Install Cargo Mate
- [ ] Build your project with `cargo build --release`
- [ ] Run `cm tool bloat-check --binary target/release/app`
- [ ] Address any high-impact issues found
- [ ] Run `cm tool cache-analyzer` on hot functions
- [ ] Run `cm tool async-lint` on async code
- [ ] Set up CI integration for ongoing monitoring

---

## **📚 Additional Resources**

### **Learning Resources:**
- [Rust Performance Book](https://nnethercote.github.io/perf-book/)
- [Tokio Async Patterns](https://tokio.rs/tokio/tutorial)
- [Cache Optimization Guide](https://www.agner.org/optimize/)

### **Related Tools:**
- `cargo flamegraph` - Flame graph generation
- `cargo audit` - Security vulnerability scanning
- `cargo expand` - Macro expansion
- `cargo bloat` - Alternative binary analysis

---

**💡 Pro Tip:** Start with `bloat-check` for quick wins, then move to `cache-analyzer` for performance-critical code, and use `async-lint` to ensure your async code is optimal.

**🎉 Happy optimizing!** These tools will help you write faster, smaller, and more efficient Rust applications.

---

# 🔍 **Cargo Mate Debugging & Diagnostics Tools - Complete Command Reference**

## **Overview**

Cargo Mate now includes three powerful debugging and diagnostics tools to help you understand and troubleshoot complex Rust code:

- **`macro-expand`** - Better macro expansion viewer with step-by-step expansion
- **`lifetime-visualizer`** - Lifetime relationship visualizer with interactive exploration
- **`compile-time-tracker`** - Compilation bottleneck tracker with optimization suggestions

---

## **🎯 Debugging Tools Quick Start**

### **Get Help**
```bash
cm tool list                    # See all available tools
cm tool help macro-expand       # Help for macro-expand
cm tool help lifetime-visualizer # Help for lifetime-visualizer
cm tool help compile-time-tracker # Help for compile-time-tracker
```

### **Quick Debugging Check**
```bash
# Understand macro expansion
cm tool macro-expand --input src/lib.rs --step-by-step

# Visualize lifetime relationships
cm tool lifetime-visualizer --input src/ --issues --suggest

# Track compilation performance
cm tool compile-time-tracker --clean-build --bottlenecks
```

---

## **🔍 4. macro-expand - Macro Expansion Viewer**

### **What it does:**
Provides a comprehensive macro expansion viewer with step-by-step expansion, syntax highlighting, and interactive exploration of procedural and declarative macros.

### **Perfect for:**
- Understanding complex macros
- Debugging macro-generated code
- Learning how macros work internally
- Troubleshooting macro compilation errors

### **Basic Usage:**
```bash
# Expand all macros in a file
cm tool macro-expand --input src/lib.rs --step-by-step

# Expand specific macro with highlighting
cm tool macro-expand --input src/main.rs --macro my_macro --highlight

# Compare original vs expanded with diff
cm tool macro-expand --input src/lib.rs --diff --validate

# Generate HTML report
cm tool macro-expand --input src/lib.rs --format html --output macro-report.html
```

### **Command Options:**

| Flag | Description | Example |
|------|-------------|---------|
| `--input` | Input Rust file to analyze | `--input src/lib.rs` |
| `--macro` | Specific macro to expand | `--macro vec!` |
| `--step-by-step` | Show step-by-step expansion | `--step-by-step` |
| `--highlight` | Highlight expanded code | `--highlight` |
| `--diff` | Show diff between original and expanded | `--diff` |
| `--validate` | Validate that expanded code compiles | `--validate` |
| `--interactive` | Interactive macro exploration | `--interactive` |
| `--output` | Output file for expanded code | `--output expanded.rs` |
| `--format` | Output format: rust, html, json | `--format html` |

### **Example Output:**
```
🔍 Macro Expansion Analysis
═══════════════════════════════════════════════

File: src/lib.rs
Macros Found: 5

📊 Expansion Summary:
• Declarative macros: 2
• Procedural macros: 3
• Total expansion size: +1,247 lines

🔬 Macro: my_vec![1, 2, 3]
═══════════════════════════════════════════════

Step 1 - Initial:
```
my_vec![1, 2, 3]
```

Step 2 - Pattern Match:
```
{
    let mut v = Vec::new();
    v.push(1);
    v.push(2);
    v.push(3);
    v
}
```

✅ Validation: Expanded code compiles successfully

📈 Expansion Metrics:
• Original size: 1 line
• Expanded size: 5 lines
• Expansion ratio: 5.0x
• Dependencies: std::vec::Vec
```

### **Pro Tips:**
- Use `--step-by-step` to understand complex macro transformations
- Combine `--diff --validate` to catch expansion issues
- Use `--format html` for sharing macro expansions with others
- Focus on procedural macros first as they're more complex

---

## **🔗 5. lifetime-visualizer - Lifetime Relationship Visualizer**

### **What it does:**
Visualizes lifetime relationships in Rust code, building lifetime dependency graphs and detecting potential lifetime issues with interactive exploration.

### **Perfect for:**
- Understanding complex lifetime relationships
- Debugging lifetime compilation errors
- Learning lifetime patterns
- Code review for lifetime correctness

### **Basic Usage:**
```bash
# Analyze lifetimes in a file
cm tool lifetime-visualizer --input src/lib.rs --issues --suggest

# Visualize lifetime relationships
cm tool lifetime-visualizer --input src/main.rs --visualize --format mermaid

# Check specific function
cm tool lifetime-visualizer --input src/lib.rs --function process_data --borrow-check

# Generate comprehensive report
cm tool lifetime-visualizer --input src/ --issues --suggest --visualize --output lifetime-report.md
```

### **Command Options:**

| Flag | Description | Example |
|------|-------------|---------|
| `--input` | Input file or directory | `--input src/` |
| `--function` | Specific function to analyze | `--function process_data` |
| `--visualize` | Generate lifetime visualization | `--visualize` |
| `--issues` | Detect lifetime issues | `--issues` |
| `--suggest` | Generate improvement suggestions | `--suggest` |
| `--format` | Visualization format | `--format mermaid` |
| `--borrow-check` | Analyze borrowing patterns | `--borrow-check` |
| `--interactive` | Interactive exploration | `--interactive` |
| `--output` | Output file for visualization | `--output lifetimes.md` |

### **Example Output:**
```
🔗 Lifetime Analysis Report
═══════════════════════════════════════════════

File: src/lib.rs
Functions Analyzed: 12
Lifetimes Detected: 8

📊 Lifetime Summary:
• Explicit lifetimes: 5
• Lifetime constraints: 3
• Complex relationships: 2

⚠️  Lifetime Issues Detected:
1. Function: process_data<'a, 'b>(data: &'a mut Vec<T>, config: &'b Config)
   • Issue: Multiple lifetimes without explicit constraints
   • Risk: Use-after-free if config outlives data

💡 Improvement Suggestions:
1. Add lifetime constraints: process_data<'a, 'b: 'a>
2. Use HRTB for cache_get: for<'k> fn cache_get<'k>(key: &'k Key)
```

### **Pro Tips:**
- Use `--visualize --format mermaid` for GitHub-compatible diagrams
- Focus on functions with `--function` for deep analysis
- Use `--borrow-check` to understand borrowing patterns
- Review suggestions carefully as they may require significant refactoring

---

## **⏱️ 6. compile-time-tracker - Compilation Bottleneck Tracker**

### **What it does:**
Tracks and analyzes compilation bottlenecks, identifies slow-to-compile crates, and generates optimization suggestions with parallel compilation analysis.

### **Perfect for:**
- Reducing build times
- Identifying compilation bottlenecks
- Optimizing CI/CD pipelines
- Improving developer productivity

### **Basic Usage:**
```bash
# Run compilation timing analysis
cm tool compile-time-tracker --clean-build --bottlenecks

# Test incremental compilation
cm tool compile-time-tracker --incremental --verbose-timing

# Analyze parallel compilation
cm tool compile-time-tracker --parallel --jobs 8

# Generate optimization report
cm tool compile-time-tracker --optimize --threshold 5.0
```

### **Command Options:**

| Flag | Description | Example |
|------|-------------|---------|
| `--manifest` | Path to Cargo.toml | `--manifest Cargo.toml` |
| `--clean-build` | Run clean build for baseline | `--clean-build` |
| `--incremental` | Test incremental compilation | `--incremental` |
| `--bottlenecks` | Identify compilation bottlenecks | `--bottlenecks` |
| `--parallel` | Analyze parallel opportunities | `--parallel` |
| `--optimize` | Generate optimization suggestions | `--optimize` |
| `--threshold` | Bottleneck threshold in seconds | `--threshold 10.0` |
| `--jobs` | Number of parallel jobs | `--jobs 4` |
| `--verbose-timing` | Show detailed timing | `--verbose-timing` |

### **Example Output:**
```
⏱️  Compilation Time Analysis Report
═══════════════════════════════════════════════

Project: my-rust-app
Total Build Time: 45.2 seconds
Peak Memory Usage: 2.1 GB

📊 Build Performance:
• Clean build: 45.2s
• Incremental build: 8.7s
• Speedup: 5.2x
• CPU utilization: 78%

🐌 Bottlenecks Identified:
1. serde_derive (12.8s) - 28.3% of total time
   • Issue: Heavy procedural macro usage
   • Impact: Blocks parallel compilation

🔄 Parallelization Analysis:
• Current jobs: 4
• Optimal jobs: 6-8
• Recommendation: Increase --jobs to 6 for 1.5x speedup

💡 Optimization Suggestions:
1. Reduce procedural macro usage in serde_derive
2. Enable pipelined compilation: RUSTC_WRAPPER=sccache
3. Split large crates into smaller units
```

### **Pro Tips:**
- Run `--clean-build` first to establish baseline
- Use `--incremental` to measure incremental compilation effectiveness
- Focus on bottlenecks taking >5% of build time
- Consider `--jobs` settings for your specific hardware

---

## **🔧 Common Debugging Workflows**

### **Macro Debugging Workflow:**
```bash
# 1. Find all macros in your code
cm tool macro-expand --input src/ --step-by-step

# 2. Focus on specific problematic macro
cm tool macro-expand --input src/lib.rs --macro my_macro --diff --validate

# 3. Generate HTML report for sharing
cm tool macro-expand --input src/lib.rs --format html --output macro-debug.html
```

### **Lifetime Debugging Workflow:**
```bash
# 1. Quick lifetime check
cm tool lifetime-visualizer --input src/ --issues

# 2. Deep dive into specific function
cm tool lifetime-visualizer --input src/lib.rs --function complex_fn --borrow-check --suggest

# 3. Visualize for documentation
cm tool lifetime-visualizer --input src/ --visualize --format mermaid --output lifetimes.md
```

### **Build Performance Workflow:**
```bash
# 1. Establish baseline
cm tool compile-time-tracker --clean-build --verbose-timing

# 2. Find bottlenecks
cm tool compile-time-tracker --bottlenecks --threshold 5.0

# 3. Optimize build settings
cm tool compile-time-tracker --parallel --jobs 8 --optimize
```

---

## **🎨 Output Formats & Integration**

### **Multiple Output Formats:**
All debugging tools support:
- **Human Readable** (default) - Colored terminal output
- **JSON** - Machine-readable structured data
- **Table** - Compact tabular format
- **HTML/Mermaid** (lifetime) - Visual documentation

### **CI/CD Integration Examples:**
```bash
# In GitHub Actions
- name: Analyze Lifetimes
  run: cm tool lifetime-visualizer --input src/ --issues --output json | jq '.issues'

# Build performance monitoring
- name: Track Compilation
  run: cm tool compile-time-tracker --clean-build --bottlenecks --threshold 10.0

# Macro validation
- name: Validate Macros
  run: cm tool macro-expand --input src/ --validate
```

---

## **⚠️ Requirements & Limitations**

### **System Requirements:**
- **Linux**: Full functionality with `perf` for compilation tracking
- **macOS**: Limited compilation profiling
- **Windows**: Basic functionality, some external tools may not work

### **Dependencies:**
- `syn` - For AST parsing and analysis
- `quote` - For code generation
- `cargo` - For compilation tracking
- `perf` (optional) - For detailed profiling

### **Performance Impact:**
- **Macro analysis**: Fast, typically <1 second
- **Lifetime analysis**: Fast, O(n) where n = AST nodes
- **Compilation tracking**: Requires full build, time = build time

---

## **🚀 Advanced Usage Examples**

### **Comprehensive Code Review:**
```bash
# Analyze all aspects of a Rust project
cm tool macro-expand --input src/ --step-by-step --validate
cm tool lifetime-visualizer --input src/ --issues --suggest --borrow-check
cm tool compile-time-tracker --clean-build --bottlenecks --parallel --optimize
```

### **Documentation Generation:**
```bash
# Generate lifetime diagrams for docs
cm tool lifetime-visualizer --input src/ --visualize --format mermaid --output docs/lifetimes.md

# Create macro expansion examples
cm tool macro-expand --input examples/ --format html --output docs/macros.html
```

### **Performance Monitoring:**
```bash
# Set up build performance monitoring
cm tool compile-time-tracker --clean-build --output baseline.json

# Compare with optimized build
RUSTC_WRAPPER=sccache cm tool compile-time-tracker --clean-build --output optimized.json
```

### **Interactive Debugging:**
```bash
# Interactive macro exploration
cm tool macro-expand --input src/complex_macro.rs --interactive

# Interactive lifetime analysis
cm tool lifetime-visualizer --input src/ --interactive
```

---

## **📈 Success Metrics**

### **Macro Analysis:**
- ✅ All macros expand without syntax errors
- ✅ Expansion ratios are reasonable (<10x)
- ✅ No unresolved names in expanded code
- ✅ Dependencies are properly tracked

### **Lifetime Analysis:**
- ✅ No high-severity lifetime issues
- ✅ Clear lifetime relationships in complex functions
- ✅ Proper HRTB usage where applicable
- ✅ No unnecessary explicit lifetimes

### **Compilation Tracking:**
- ✅ Build time < 2 minutes for typical projects
- ✅ No single crate takes >10% of build time
- ✅ CPU utilization >60% during builds
- ✅ Incremental builds are significantly faster

---

## **🔍 Troubleshooting**

### **Common Issues:**

**"Failed to parse Rust file"**
```bash
# Check syntax first
cargo check

# Ensure file is valid Rust
rustfmt src/your_file.rs
```

**"No timing data available"**
```bash
# For compilation tracking
cargo clean  # Start fresh
cm tool compile-time-tracker --clean-build

# Install perf for detailed profiling
sudo apt install linux-tools-common
```

**"Macro definition not found"**
```bash
# Check macro_rules! definition exists
grep -n "macro_rules!" src/your_file.rs

# Or the macro might be in dependencies
cargo doc --open  # Check generated docs
```

**"Permission denied"**
```bash
# For output files
mkdir -p output_dir
cm tool macro-expand --input src/ --output output_dir/result.html
```

### **Performance Tips:**
- Use specific file paths rather than directories for faster analysis
- Enable `--verbose` for detailed debugging information
- Save baseline measurements for comparison
- Run tools on debug builds first, then optimize

---

## **🎯 Best Practices**

### **Macro Development:**
- Use `--validate` during macro development
- Test expansions with `--diff` to catch issues
- Document complex macros with `--format html`
- Use `--step-by-step` for peer review

### **Lifetime Management:**
- Run lifetime analysis before complex refactoring
- Use `--suggest` to learn lifetime patterns
- Generate visualizations for documentation
- Focus on functions with multiple explicit lifetimes

### **Build Optimization:**
- Establish build time baselines
- Monitor for regression with `--clean-build`
- Use `--bottlenecks` to prioritize optimizations
- Test different `--jobs` values for your hardware

---

**💡 Pro Tip:** Start with `macro-expand` for quick wins in understanding code, then use `lifetime-visualizer` for correctness, and `compile-time-tracker` for productivity.

**🐛 Happy debugging!** These tools will help you write cleaner, faster, and more maintainable Rust applications.
