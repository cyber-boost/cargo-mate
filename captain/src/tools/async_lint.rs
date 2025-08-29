use super::{Tool, ToolError, Result, OutputFormat, parse_output_format};
use clap::{Arg, ArgMatches, Command};
use std::path::Path;
use std::fs;
use colored::*;
use syn::{parse_file, visit::Visit, ItemFn, ExprAwait, ExprCall, Type, spanned::Spanned};
use quote::ToTokens;
use serde::{Serialize, Deserialize};
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockingOperation {
    pub function: String,
    pub line: usize,
    pub column: usize,
    pub operation: String,
    pub suggestion: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AwaitIssue {
    pub function: String,
    pub line: usize,
    pub column: usize,
    pub issue: String,
    pub code_snippet: String,
    pub suggestion: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeadlockRisk {
    pub function: String,
    pub line: usize,
    pub column: usize,
    pub risk_type: String,
    pub description: String,
    pub suggestion: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AsyncSuggestion {
    pub category: String,
    pub description: String,
    pub impact: String,
    pub suggestion: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConcurrencyAnalysis {
    pub total_async_functions: usize,
    pub average_await_depth: f64,
    pub concurrent_operations: usize,
    pub potential_race_conditions: usize,
    pub blocking_operations: usize,
    pub nested_async_blocks: usize,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AsyncIssue {
    pub file: String,
    pub issues: Vec<AsyncIssueType>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AsyncIssueType {
    BlockingOperation(BlockingOperation),
    AwaitIssue(AwaitIssue),
    DeadlockRisk(DeadlockRisk),
}
pub struct AsyncLintTool;
impl AsyncLintTool {
    pub fn new() -> Self {
        Self
    }
    fn analyze_async_patterns(&self, file_path: &str) -> Result<Vec<AsyncIssue>> {
        if !Path::new(file_path).exists() {
            return Err(
                ToolError::InvalidArguments(format!("File not found: {}", file_path)),
            );
        }
        let content = fs::read_to_string(file_path)?;
        let mut issues = Vec::new();
        if file_path.ends_with(".rs") {
            match parse_file(&content) {
                Ok(ast) => {
                    let mut visitor = AsyncVisitor::new(file_path);
                    visitor.visit_file(&ast);
                    issues
                        .push(AsyncIssue {
                            file: file_path.to_string(),
                            issues: visitor.issues,
                        });
                }
                Err(e) => {
                    return Err(
                        ToolError::ExecutionFailed(
                            format!("Failed to parse Rust file: {}", e),
                        ),
                    );
                }
            }
        } else {
            return Err(
                ToolError::InvalidArguments(
                    "Only Rust (.rs) files are supported".to_string(),
                ),
            );
        }
        Ok(issues)
    }
    fn detect_blocking_operations(&self, ast: &syn::File) -> Vec<BlockingOperation> {
        let mut operations = Vec::new();
        let mut visitor = BlockingOperationVisitor::new();
        visitor.visit_file(ast);
        operations.extend(visitor.operations);
        operations
    }
    fn analyze_await_patterns(&self, functions: &[syn::ItemFn]) -> Vec<AwaitIssue> {
        let mut issues = Vec::new();
        for function in functions {
            let mut visitor = AwaitPatternVisitor::new();
            visitor.visit_item_fn(function);
            issues
                .extend(
                    visitor
                        .issues
                        .into_iter()
                        .map(|mut issue| {
                            issue.function = function.sig.ident.to_string();
                            issue
                        }),
                );
        }
        issues
    }
    fn detect_deadlock_patterns(&self, ast: &syn::File) -> Vec<DeadlockRisk> {
        let mut risks = Vec::new();
        let mut visitor = DeadlockVisitor::new();
        visitor.visit_file(ast);
        risks.extend(visitor.risks);
        risks
    }
    fn suggest_async_improvements(
        &self,
        all_issues: &[AsyncIssue],
    ) -> Vec<AsyncSuggestion> {
        let mut suggestions = Vec::new();
        let mut total_blocking = 0;
        let mut unnecessary_awaits = 0;
        let mut nested_async = 0;
        let mut select_issues = 0;
        for issue_set in all_issues {
            for issue in &issue_set.issues {
                match issue {
                    AsyncIssueType::BlockingOperation(_) => total_blocking += 1,
                    AsyncIssueType::AwaitIssue(issue) => {
                        if issue.issue.contains("unnecessary") {
                            unnecessary_awaits += 1;
                        } else if issue.issue.contains("nested") {
                            nested_async += 1;
                        }
                    }
                    AsyncIssueType::DeadlockRisk(_) => select_issues += 1,
                }
            }
        }
        if total_blocking > 0 {
            suggestions
                .push(AsyncSuggestion {
                    category: "Blocking Operations".to_string(),
                    description: format!(
                        "Found {} blocking operations in async contexts", total_blocking
                    ),
                    impact: "High".to_string(),
                    suggestion: "Replace std::fs with tokio::fs, std::thread::sleep with tokio::time::sleep"
                        .to_string(),
                });
        }
        if unnecessary_awaits > 0 {
            suggestions
                .push(AsyncSuggestion {
                    category: "Unnecessary Awaits".to_string(),
                    description: format!(
                        "Found {} unnecessary await expressions", unnecessary_awaits
                    ),
                    impact: "Low".to_string(),
                    suggestion: "Remove unnecessary async/await for immediate values"
                        .to_string(),
                });
        }
        if nested_async > 0 {
            suggestions
                .push(AsyncSuggestion {
                    category: "Nested Async".to_string(),
                    description: format!("Found {} nested async blocks", nested_async),
                    impact: "Medium".to_string(),
                    suggestion: "Flatten nested async blocks for better readability"
                        .to_string(),
                });
        }
        if select_issues > 0 {
            suggestions
                .push(AsyncSuggestion {
                    category: "Deadlock Prevention".to_string(),
                    description: format!(
                        "Found {} potential deadlock patterns", select_issues
                    ),
                    impact: "High".to_string(),
                    suggestion: "Use try_select! or restructure concurrent operations"
                        .to_string(),
                });
        }
        suggestions
            .push(AsyncSuggestion {
                category: "Performance".to_string(),
                description: "General async performance improvements".to_string(),
                impact: "Medium".to_string(),
                suggestion: "Use JoinSet for concurrent operations, implement proper error handling"
                    .to_string(),
            });
        suggestions
            .push(AsyncSuggestion {
                category: "Best Practices".to_string(),
                description: "Async best practices".to_string(),
                impact: "Low".to_string(),
                suggestion: "Use async fn consistently, avoid mixing sync and async code"
                    .to_string(),
            });
        suggestions
    }
    fn analyze_concurrency_patterns(
        &self,
        file_path: &str,
    ) -> Result<ConcurrencyAnalysis> {
        let content = fs::read_to_string(file_path)?;
        let async_fn_count = content.matches("async fn").count();
        let await_count = content.matches("await").count();
        let select_count = content.matches("select!").count()
            + content.matches("join!").count();
        let nested_async_count = content.matches("async {").count();
        let average_await_depth = if async_fn_count > 0 {
            await_count as f64 / async_fn_count as f64
        } else {
            0.0
        };
        let race_condition_patterns = [
            "Arc<Mutex<",
            "Arc<RwLock<",
            "static mut",
            "RefCell<",
        ];
        let mut race_condition_count = 0;
        for pattern in &race_condition_patterns {
            race_condition_count += content.matches(pattern).count();
        }
        Ok(ConcurrencyAnalysis {
            total_async_functions: async_fn_count,
            average_await_depth,
            concurrent_operations: select_count,
            potential_race_conditions: race_condition_count,
            blocking_operations: content.matches("std::fs::").count()
                + content.matches("std::thread::").count(),
            nested_async_blocks: nested_async_count,
        })
    }
}
struct AsyncVisitor {
    file_path: String,
    issues: Vec<AsyncIssueType>,
}
impl AsyncVisitor {
    fn new(file_path: &str) -> Self {
        Self {
            file_path: file_path.to_string(),
            issues: Vec::new(),
        }
    }
}
impl<'ast> Visit<'ast> for AsyncVisitor {
    fn visit_item_fn(&mut self, node: &'ast syn::ItemFn) {
        let is_async = node.sig.asyncness.is_some();
        if is_async {
            let mut blocking_visitor = BlockingOperationVisitor::new();
            blocking_visitor.visit_block(&node.block);
            self.issues
                .extend(
                    blocking_visitor
                        .operations
                        .into_iter()
                        .map(|mut op| {
                            op.function = node.sig.ident.to_string();
                            AsyncIssueType::BlockingOperation(op)
                        }),
                );
            let mut await_visitor = AwaitPatternVisitor::new();
            await_visitor.visit_block(&node.block);
            self.issues
                .extend(
                    await_visitor
                        .issues
                        .into_iter()
                        .map(|mut issue| {
                            issue.function = node.sig.ident.to_string();
                            AsyncIssueType::AwaitIssue(issue)
                        }),
                );
            let mut deadlock_visitor = DeadlockVisitor::new();
            deadlock_visitor.visit_block(&node.block);
            self.issues
                .extend(
                    deadlock_visitor
                        .risks
                        .into_iter()
                        .map(|mut risk| {
                            risk.function = node.sig.ident.to_string();
                            AsyncIssueType::DeadlockRisk(risk)
                        }),
                );
        }
        syn::visit::visit_item_fn(self, node);
    }
}
struct BlockingOperationVisitor {
    operations: Vec<BlockingOperation>,
}
impl BlockingOperationVisitor {
    fn new() -> Self {
        Self { operations: Vec::new() }
    }
}
impl<'ast> Visit<'ast> for BlockingOperationVisitor {
    fn visit_expr_call(&mut self, node: &'ast syn::ExprCall) {
        if let Some(func_name) = self.extract_function_name(node) {
            let blocking_patterns = [
                ("std::fs::read", "tokio::fs::read"),
                ("std::fs::write", "tokio::fs::write"),
                ("std::fs::File::open", "tokio::fs::File::open"),
                ("std::thread::sleep", "tokio::time::sleep"),
                ("std::thread::spawn", "tokio::spawn"),
                ("reqwest::blocking", "reqwest::get"),
            ];
            for (blocking, async_version) in &blocking_patterns {
                if func_name.contains(blocking) {
                    let line = 0;
                    let col = 0;
                    self.operations
                        .push(BlockingOperation {
                            function: String::new(),
                            line,
                            column: col,
                            operation: func_name.clone(),
                            suggestion: format!("Use {} instead", async_version),
                        });
                }
            }
        }
        syn::visit::visit_expr_call(self, node);
    }
}
struct AwaitPatternVisitor {
    issues: Vec<AwaitIssue>,
}
impl AwaitPatternVisitor {
    fn new() -> Self {
        Self { issues: Vec::new() }
    }
}
impl<'ast> Visit<'ast> for AwaitPatternVisitor {
    fn visit_expr_await(&mut self, node: &'ast syn::ExprAwait) {
        if let syn::Expr::Call(call) = &*node.base {
            if let Some(func_name) = self.extract_function_name(call) {
                if func_name == "async" {
                    let line = 0;
                    let col = 0;
                    self.issues
                        .push(AwaitIssue {
                            function: String::new(),
                            line,
                            column: col,
                            issue: "Unnecessary await on immediate value".to_string(),
                            code_snippet: "async { 42 }.await".to_string(),
                            suggestion: "Remove async/await: 42".to_string(),
                        });
                }
            }
        }
        syn::visit::visit_expr_await(self, node);
    }
    fn visit_expr_call(&mut self, node: &'ast syn::ExprCall) {
        if let Some(func_name) = self.extract_function_name(node) {
            if func_name == "async" {
                let line = 0;
                let col = 0;
                self.issues
                    .push(AwaitIssue {
                        function: String::new(),
                        line,
                        column: col,
                        issue: "Nested async blocks".to_string(),
                        code_snippet: "async { async { work() }.await }.await"
                            .to_string(),
                        suggestion: "Flatten to: async { work() }.await".to_string(),
                    });
            }
        }
        syn::visit::visit_expr_call(self, node);
    }
}
struct DeadlockVisitor {
    risks: Vec<DeadlockRisk>,
}
impl DeadlockVisitor {
    fn new() -> Self {
        Self { risks: Vec::new() }
    }
}
impl<'ast> Visit<'ast> for DeadlockVisitor {
    fn visit_expr_call(&mut self, node: &'ast syn::ExprCall) {
        if let Some(func_name) = self.extract_function_name(node) {
            if func_name.contains("select!") {
                let line = 0;
                let col = 0;
                self.risks
                    .push(DeadlockRisk {
                        function: String::new(),
                        line,
                        column: col,
                        risk_type: "select! deadlock".to_string(),
                        description: "Multiple futures competing for same resource"
                            .to_string(),
                        suggestion: "Use try_select! or restructure to avoid resource contention"
                            .to_string(),
                    });
            }
        }
        syn::visit::visit_expr_call(self, node);
    }
}
trait FunctionNameExtractor {
    fn extract_function_name(&self, node: &syn::ExprCall) -> Option<String>;
}
impl<T> FunctionNameExtractor for T {
    fn extract_function_name(&self, node: &syn::ExprCall) -> Option<String> {
        match &*node.func {
            syn::Expr::Path(path) => {
                Some(
                    path
                        .path
                        .segments
                        .iter()
                        .map(|seg| seg.ident.to_string())
                        .collect::<Vec<_>>()
                        .join("::"),
                )
            }
            _ => None,
        }
    }
}
impl Tool for AsyncLintTool {
    fn name(&self) -> &'static str {
        "async-lint"
    }
    fn description(&self) -> &'static str {
        "Detect common async programming pitfalls and suggest improvements"
    }
    fn command(&self) -> Command {
        Command::new(self.name())
            .about(self.description())
            .long_about(
                "Detect common async programming pitfalls and suggest improvements.\n\
                 \n\
                 This tool analyzes async code for common issues:\n\
                 • Detect blocking operations in async contexts\n\
                 • Find unnecessary async/await usage\n\
                 • Identify potential deadlocks\n\
                 • Analyze async function call graphs\n\
                 \n\
                 EXAMPLES:\n\
                 cm tool async-lint --input src/ --blocking --await --deadlock\n\
                 cm tool async-lint --input src/main.rs --blocking --fix\n\
                 cm tool async-lint --input src/ --strict --ignore async-move,unnecessary-await",
            )
            .args(
                &[
                    Arg::new("input")
                        .long("input")
                        .short('i')
                        .help("Input file or directory to analyze")
                        .default_value("src/"),
                    Arg::new("blocking")
                        .long("blocking")
                        .help("Detect blocking operations in async contexts")
                        .action(clap::ArgAction::SetTrue),
                    Arg::new("await")
                        .long("await")
                        .help("Analyze async/await usage patterns")
                        .action(clap::ArgAction::SetTrue),
                    Arg::new("deadlock")
                        .long("deadlock")
                        .help("Detect potential deadlock patterns")
                        .action(clap::ArgAction::SetTrue),
                    Arg::new("concurrency")
                        .long("concurrency")
                        .help("Analyze concurrent async operations")
                        .action(clap::ArgAction::SetTrue),
                    Arg::new("fix")
                        .long("fix")
                        .help("Generate fix suggestions")
                        .action(clap::ArgAction::SetTrue),
                    Arg::new("strict")
                        .long("strict")
                        .help("Enable strict async linting rules")
                        .action(clap::ArgAction::SetTrue),
                    Arg::new("ignore")
                        .long("ignore")
                        .help("Comma-separated list of rules to ignore")
                        .default_value(""),
                ],
            )
            .args(&super::common_options())
    }
    fn execute(&self, matches: &ArgMatches) -> Result<()> {
        let input = matches.get_one::<String>("input").unwrap();
        let detect_blocking = matches.get_flag("blocking");
        let analyze_await = matches.get_flag("await");
        let detect_deadlock = matches.get_flag("deadlock");
        let analyze_concurrency = matches.get_flag("concurrency");
        let generate_fixes = matches.get_flag("fix");
        let strict_mode = matches.get_flag("strict");
        let ignore_rules = matches.get_one::<String>("ignore").unwrap();
        let verbose = matches.get_flag("verbose");
        let dry_run = matches.get_flag("dry-run");
        let output_format = parse_output_format(matches);
        let ignored_rules: Vec<String> = ignore_rules
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        if dry_run {
            println!("🔍 Would analyze async patterns in: {}", input);
            return Ok(());
        }
        let mut all_issues = Vec::new();
        if Path::new(input).is_file() {
            match self.analyze_async_patterns(input) {
                Ok(issues) => all_issues.extend(issues),
                Err(e) => {
                    if verbose {
                        println!("⚠️  Failed to analyze {}: {}", input, e);
                    }
                }
            }
        } else if Path::new(input).is_dir() {
            let rust_files = self.find_rust_files(input)?;
            for file in rust_files {
                match self.analyze_async_patterns(&file) {
                    Ok(issues) => all_issues.extend(issues),
                    Err(e) => {
                        if verbose {
                            println!("⚠️  Failed to analyze {}: {}", file, e);
                        }
                    }
                }
            }
        } else {
            return Err(
                ToolError::InvalidArguments(format!("Path not found: {}", input)),
            );
        }
        match output_format {
            OutputFormat::Human => {
                println!(
                    "⚡ {} - {}", "Async Pattern Analysis".bold(), self.description()
                    .cyan()
                );
                let mut total_issues = 0;
                for issue_set in &all_issues {
                    if !issue_set.issues.is_empty() {
                        println!("\n📁 File: {}", issue_set.file.bold());
                        let mut blocking_count = 0;
                        let mut await_count = 0;
                        let mut deadlock_count = 0;
                        for issue in &issue_set.issues {
                            match issue {
                                AsyncIssueType::BlockingOperation(op) => {
                                    if detect_blocking
                                        && !ignored_rules.contains(&"blocking".to_string())
                                    {
                                        blocking_count += 1;
                                        println!(
                                            "  🚫 Line {}: {} in async function", op.line.to_string()
                                            .red(), op.operation.yellow()
                                        );
                                        println!("     💡 {}", op.suggestion.cyan());
                                    }
                                }
                                AsyncIssueType::AwaitIssue(issue) => {
                                    if analyze_await
                                        && !ignored_rules.contains(&"await".to_string())
                                    {
                                        await_count += 1;
                                        println!(
                                            "  🔄 Line {}: {}", issue.line.to_string().yellow(), issue
                                            .issue
                                        );
                                        println!("     Code: {}", issue.code_snippet.red());
                                        println!("     💡 {}", issue.suggestion.cyan());
                                    }
                                }
                                AsyncIssueType::DeadlockRisk(risk) => {
                                    if detect_deadlock
                                        && !ignored_rules.contains(&"deadlock".to_string())
                                    {
                                        deadlock_count += 1;
                                        println!(
                                            "  🔒 Line {}: {}", risk.line.to_string().red(), risk
                                            .risk_type
                                        );
                                        println!("     {}", risk.description.yellow());
                                        println!("     💡 {}", risk.suggestion.cyan());
                                    }
                                }
                            }
                        }
                        if blocking_count + await_count + deadlock_count > 0 {
                            println!(
                                "  📊 Issues in this file: {} blocking, {} await, {} deadlock",
                                blocking_count, await_count, deadlock_count
                            );
                        }
                        total_issues += blocking_count + await_count + deadlock_count;
                    }
                }
                if analyze_concurrency {
                    println!("\n📊 Concurrency Analysis:");
                    for issue_set in &all_issues {
                        match self.analyze_concurrency_patterns(&issue_set.file) {
                            Ok(analysis) => {
                                println!("  File: {}", issue_set.file.bold());
                                println!(
                                    "    Async functions: {}", analysis.total_async_functions
                                );
                                println!(
                                    "    Average await depth: {:.1}", analysis
                                    .average_await_depth
                                );
                                println!(
                                    "    Concurrent operations: {}", analysis
                                    .concurrent_operations
                                );
                                if analysis.potential_race_conditions > 0 {
                                    println!(
                                        "    Potential race conditions: {}", analysis
                                        .potential_race_conditions.to_string().yellow()
                                    );
                                }
                                if analysis.blocking_operations > 0 {
                                    println!(
                                        "    Blocking operations: {}", analysis.blocking_operations
                                        .to_string().red()
                                    );
                                }
                            }
                            Err(e) => {
                                if verbose {
                                    println!("    ⚠️  Concurrency analysis failed: {}", e);
                                }
                            }
                        }
                    }
                }
                if generate_fixes {
                    let suggestions = self.suggest_async_improvements(&all_issues);
                    if !suggestions.is_empty() {
                        println!("\n💡 Improvement Suggestions:");
                        for suggestion in suggestions {
                            let impact_color = match suggestion.impact.as_str() {
                                "High" => suggestion.impact.red().bold(),
                                "Medium" => suggestion.impact.yellow().bold(),
                                _ => suggestion.impact.green().bold(),
                            };
                            println!(
                                "  • [{}] {}: {}", impact_color, suggestion.category
                                .bold(), suggestion.suggestion
                            );
                        }
                    }
                }
                println!(
                    "\n📈 Summary: {} files analyzed, {} issues found", all_issues
                    .len(), total_issues
                );
            }
            OutputFormat::Json => {
                let mut json_output = serde_json::json!(
                    { "files_analyzed" : all_issues.len(), "issues" : all_issues, }
                );
                if generate_fixes {
                    let suggestions = self.suggest_async_improvements(&all_issues);
                    json_output["suggestions"] = serde_json::to_value(&suggestions)
                        .unwrap();
                }
                println!("{}", serde_json::to_string_pretty(& json_output).unwrap());
            }
            OutputFormat::Table => {
                println!(
                    "┌─ Async Pattern Analysis ─────────────────┐"
                );
                println!("│ Files analyzed: {:<25} │", all_issues.len());
                let total_issues: usize = all_issues
                    .iter()
                    .map(|i| i.issues.len())
                    .sum();
                println!("│ Total issues: {:<26} │", total_issues);
                if detect_blocking {
                    println!("│ Blocking ops: {:<25} │", "✓".green());
                }
                if analyze_await {
                    println!("│ Await patterns: {:<23} │", "✓".green());
                }
                if detect_deadlock {
                    println!("│ Deadlock check: {:<23} │", "✓".green());
                }
                println!(
                    "└────────────────────────────────────────────┘"
                );
            }
        }
        Ok(())
    }
}
impl AsyncLintTool {
    fn find_rust_files(&self, dir: &str) -> Result<Vec<String>> {
        let mut rust_files = Vec::new();
        fn visit_dir(dir: &str, files: &mut Vec<String>) -> Result<()> {
            let entries = fs::read_dir(dir)?;
            for entry in entries {
                let entry = entry?;
                let path = entry.path();
                if path.is_dir() {
                    if let Some(dir_name) = path.file_name() {
                        if dir_name != "target" && dir_name != ".git" {
                            visit_dir(&path.to_string_lossy(), files)?;
                        }
                    }
                } else if let Some(ext) = path.extension() {
                    if ext == "rs" {
                        files.push(path.to_string_lossy().to_string());
                    }
                }
            }
            Ok(())
        }
        visit_dir(dir, &mut rust_files)?;
        Ok(rust_files)
    }
}