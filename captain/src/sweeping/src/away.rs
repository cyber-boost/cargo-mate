use anyhow::{Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use colored::*;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use syn::{visit_mut::VisitMut, Expr, ExprMacro, Stmt};
use walkdir::WalkDir;
pub use crate::*;
#[derive(Parser, Debug)]
#[command(name = "sweep")]
#[command(
    about = "🧹 Sweep away println! and eprintln! debug statements from Rust code"
)]
#[command(
    long_about = "Intelligently clean debug print statements that AI assistants love to spam everywhere"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
    #[arg(short, long, global = true)]
    verbose: bool,
    #[arg(short = 'c', long, global = true, default_value = ".sweep.toml")]
    config: PathBuf,
}
#[derive(Subcommand, Debug)]
pub enum Commands {
    Scan {
        #[arg(default_value = ".")]
        path: PathBuf,
        #[arg(long)]
        include_tests: bool,
        #[arg(long)]
        include_examples: bool,
        #[arg(long)]
        export: Option<PathBuf>,
    },
    #[command(visible_alias = "clean")]
    Sweep {
        #[arg(default_value = ".")]
        path: PathBuf,
        #[arg(short = 'n', long)]
        dry_run: bool,
        #[arg(short, long)]
        interactive: bool,
        #[arg(short, long)]
        prompt: bool,
        #[arg(long)]
        keep_main: bool,
        #[arg(long)]
        keep_tests: bool,
        #[arg(long)]
        keep_examples: bool,
        #[arg(short, long)]
        backup: bool,
        #[arg(short = 'y', long)]
        yes: bool,
        #[arg(long)]
        pristine: bool,
        #[arg(long)]
        format: bool,
        #[arg(long)]
        organize_imports: bool,
        #[arg(long)]
        add_docs: bool,
        #[arg(long)]
        fix_clippy: bool,
    },
    Convert {
        #[arg(default_value = ".")]
        path: PathBuf,
        #[arg(long, default_value = "debug")]
        println_level: LogLevel,
        #[arg(long, default_value = "error")]
        eprintln_level: LogLevel,
        #[arg(short = 'n', long)]
        dry_run: bool,
        #[arg(long)]
        add_dependency: bool,
    },
    Analyze {
        #[arg(default_value = ".")]
        path: PathBuf,
        #[arg(long, default_value = "10")]
        top: usize,
    },
    Init { #[arg(short, long)] force: bool },
}
#[derive(Debug, Clone, ValueEnum)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}
impl LogLevel {
    fn as_str(&self) -> &str {
        match self {
            LogLevel::Trace => "trace",
            LogLevel::Debug => "debug",
            LogLevel::Info => "info",
            LogLevel::Warn => "warn",
            LogLevel::Error => "error",
        }
    }
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PrintStatement {
    pub file: PathBuf,
    pub line: usize,
    pub column: usize,
    pub kind: PrintKind,
    pub content: String,
    pub context: PrintContext,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum PrintKind {
    Println,
    Eprintln,
    Print,
    Eprint,
    DbgMacro,
}
impl PrintKind {
    fn as_str(&self) -> &str {
        match self {
            PrintKind::Println => "println!",
            PrintKind::Eprintln => "eprintln!",
            PrintKind::Print => "print!",
            PrintKind::Eprint => "eprint!",
            PrintKind::DbgMacro => "dbg!",
        }
    }
    fn color_str(&self) -> ColoredString {
        match self {
            PrintKind::Println | PrintKind::Print => self.as_str().blue(),
            PrintKind::Eprintln | PrintKind::Eprint => self.as_str().red(),
            PrintKind::DbgMacro => self.as_str().yellow(),
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
enum PrintContext {
    MainFunction,
    TestFunction,
    RegularFunction(String),
    ImplBlock,
    Module,
    Unknown,
}
#[derive(Debug, Serialize, Deserialize)]
struct SweepConfig {
    keep_patterns: Vec<String>,
    remove_patterns: Vec<String>,
    skip_files: Vec<String>,
    skip_dirs: Vec<String>,
    keep_in_main: bool,
    keep_in_tests: bool,
    keep_in_examples: bool,
    default_println_level: String,
    default_eprintln_level: String,
    remembered_patterns: HashMap<String, PatternDecision>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
struct PatternDecision {
    pattern: String,
    action: DecisionAction,
    created_at: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
enum DecisionAction {
    AlwaysRemove,
    AlwaysKeep,
    AskEachTime,
}
impl Default for SweepConfig {
    fn default() -> Self {
        Self {
            keep_patterns: vec![
                "Version:".to_string(), "Usage:".to_string(), "Help:".to_string(),
                "Error:".to_string(), "Welcome".to_string(), "Success".to_string(),
            ],
            remove_patterns: vec![
                "DEBUG:".to_string(), "TODO:".to_string(), "FIXME:".to_string(), "XXX:"
                .to_string(), "TEMP:".to_string(), "HERE".to_string(), "got here"
                .to_string(), "reached".to_string(), "entering".to_string(), "exiting"
                .to_string(),
            ],
            skip_files: vec![],
            skip_dirs: vec![
                "target".to_string(), ".git".to_string(), "vendor".to_string()
            ],
            keep_in_main: true,
            keep_in_tests: true,
            keep_in_examples: true,
            default_println_level: "debug".to_string(),
            default_eprintln_level: "error".to_string(),
            remembered_patterns: HashMap::new(),
        }
    }
}
pub struct Sweeper {
    config: SweepConfig,
    multi_progress: MultiProgress,
    pattern_cache: HashMap<String, DecisionAction>,
}
impl Sweeper {
    pub fn new() -> Self {
        Self {
            config: SweepConfig::default(),
            multi_progress: MultiProgress::new(),
            pattern_cache: HashMap::new(),
        }
    }
    pub fn load_config(&mut self, path: &Path) -> Result<()> {
        if path.exists() {
            let content = fs::read_to_string(path)?;
            #[cfg(feature = "pristine")]
            {
                self.config = toml::from_str(&content)?;
            }
            for (_, decision) in &self.config.remembered_patterns {
                self.pattern_cache
                    .insert(decision.pattern.clone(), decision.action.clone());
            }
        }
        Ok(())
    }
    fn save_config(&self, path: &Path) -> Result<()> {
        #[cfg(feature = "pristine")]
        {
            let toml = toml::to_string_pretty(&self.config)?;
            fs::write(path, toml)?;
        }
        Ok(())
    }
    pub fn scan_directory(
        &self,
        path: &Path,
        include_tests: bool,
        include_examples: bool,
    ) -> Result<Vec<PrintStatement>> {
        let mut statements = Vec::new();
        let pb = self.multi_progress.add(ProgressBar::new_spinner());
        pb.set_style(
            ProgressStyle::default_spinner().template("{spinner:.green} {msg}").unwrap(),
        );
        pb.set_message("🔍 Sweeping for print statements...");
        let mut file_count = 0;
        for entry in WalkDir::new(path)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map_or(false, |ext| ext == "rs"))
        {
            let file_path = entry.path();
            if self.should_skip_file(file_path, include_tests, include_examples) {
                continue;
            }
            file_count += 1;
            pb.set_message(
                format!(
                    "🔍 Scanning: {} (found {} so far)", file_path.file_name()
                    .unwrap_or_default().to_string_lossy(), statements.len()
                ),
            );
            let content = fs::read_to_string(file_path)?;
            let file_statements = self.find_print_statements(file_path, &content)?;
            statements.extend(file_statements);
        }
        pb.finish_with_message(
            format!(
                "✅ Found {} print statements in {} files", statements.len().to_string()
                .yellow(), file_count.to_string().cyan()
            ),
        );
        Ok(statements)
    }
    fn should_skip_file(
        &self,
        path: &Path,
        include_tests: bool,
        include_examples: bool,
    ) -> bool {
        let path_str = path.to_string_lossy();
        for skip_dir in &self.config.skip_dirs {
            if path_str.contains(skip_dir) {
                return true;
            }
        }
        if !include_tests
            && (path_str.contains("/tests/") || path_str.ends_with("_test.rs")
                || (path_str.contains("mod") && path_str.contains("test")))
        {
            return true;
        }
        if !include_examples && path_str.contains("/examples/") {
            return true;
        }
        false
    }
    fn find_print_statements(
        &self,
        file_path: &Path,
        content: &str,
    ) -> Result<Vec<PrintStatement>> {
        let mut statements = Vec::new();
        if let Ok(mut syntax_tree) = syn::parse_file(content) {
            let mut visitor = PrintStatementVisitor::new(file_path, content);
            visitor.visit_file_mut(&mut syntax_tree);
            statements = visitor.statements;
        } else {
            statements = self.find_print_statements_regex(file_path, content)?;
        }
        Ok(statements)
    }
    fn find_print_statements_regex(
        &self,
        file_path: &Path,
        content: &str,
    ) -> Result<Vec<PrintStatement>> {
        let mut statements = Vec::new();
        let patterns = vec![
            (r"println!\s*\([^)]*\)", PrintKind::Println), (r"eprintln!\s*\([^)]*\)",
            PrintKind::Eprintln), (r"print!\s*\([^)]*\)", PrintKind::Print),
            (r"eprint!\s*\([^)]*\)", PrintKind::Eprint), (r"dbg!\s*\([^)]*\)",
            PrintKind::DbgMacro),
        ];
        for (pattern_str, kind) in patterns {
            let re = Regex::new(pattern_str)?;
            for mat in re.find_iter(content) {
                let line_num = content[..mat.start()].lines().count() + 1;
                let line_start = content[..mat.start()].rfind('\n').map_or(0, |i| i + 1);
                let column = mat.start() - line_start + 1;
                statements
                    .push(PrintStatement {
                        file: file_path.to_path_buf(),
                        line: line_num,
                        column,
                        kind: kind.clone(),
                        content: mat.as_str().to_string(),
                        context: self.determine_context(content, mat.start()),
                    });
            }
        }
        Ok(statements)
    }
    fn determine_context(&self, content: &str, position: usize) -> PrintContext {
        let before = &content[..position];
        if before.contains("#[test]") || before.contains("#[cfg(test)]") {
            return PrintContext::TestFunction;
        }
        if let Some(main_pos) = before.rfind("fn main") {
            if !before[main_pos..].contains('}') {
                return PrintContext::MainFunction;
            }
        }
        if let Some(fn_pos) = before.rfind("fn ") {
            let after_fn = &before[fn_pos + 3..];
            if let Some(name_end) = after_fn.find(|c: char| c == '(' || c == '<') {
                let fn_name = after_fn[..name_end].trim();
                return PrintContext::RegularFunction(fn_name.to_string());
            }
        }
        PrintContext::Unknown
    }
    pub fn sweep_files(
        &mut self,
        statements: Vec<PrintStatement>,
        options: &SweepOptions,
        config_path: &Path,
    ) -> Result<SweepStats> {
        let mut stats = SweepStats::default();
        let mut files_to_process: HashMap<PathBuf, Vec<PrintStatement>> = HashMap::new();
        for stmt in statements {
            if self.should_keep_statement(&stmt, options) {
                stats.kept += 1;
                continue;
            }
            files_to_process.entry(stmt.file.clone()).or_default().push(stmt);
        }
        if files_to_process.is_empty() {
            println!("{}", "✨ Already clean! No statements to sweep.".green());
            return Ok(stats);
        }
        let pb = self
            .multi_progress
            .add(ProgressBar::new(files_to_process.len() as u64));
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{bar:40.cyan/blue}] {pos}/{len} {msg}")
                .unwrap(),
        );
        for (file_path, file_statements) in files_to_process {
            pb.set_message(
                format!(
                    "🧹 Sweeping: {}", file_path.file_name().unwrap_or_default()
                    .to_string_lossy()
                ),
            );
            let mut statements_to_remove = Vec::new();
            for stmt in file_statements {
                let should_remove = if options.yes {
                    true
                } else if options.interactive || options.prompt {
                    self.handle_interactive_decision(&stmt, options, config_path)?
                } else {
                    true
                };
                if should_remove {
                    statements_to_remove.push(stmt);
                    stats.removed += 1;
                } else {
                    stats.kept += 1;
                }
            }
            if !options.dry_run && !statements_to_remove.is_empty() {
                if options.backup {
                    let backup_path = file_path.with_extension("rs.bak");
                    fs::copy(&file_path, backup_path)?;
                }
                let content = fs::read_to_string(&file_path)?;
                let cleaned_content = self
                    .remove_statements_from_content(&content, &statements_to_remove)?;
                fs::write(&file_path, cleaned_content)?;
                stats.files_modified += 1;
            }
            pb.inc(1);
        }
        pb.finish_with_message("✨ Sweep complete!");
        Ok(stats)
    }
    fn handle_interactive_decision(
        &mut self,
        stmt: &PrintStatement,
        options: &SweepOptions,
        config_path: &Path,
    ) -> Result<bool> {
        let pattern = self.extract_pattern(&stmt.content);
        if let Some(decision) = self.pattern_cache.get(&pattern) {
            match decision {
                DecisionAction::AlwaysRemove => return Ok(true),
                DecisionAction::AlwaysKeep => return Ok(false),
                DecisionAction::AskEachTime => {}
            }
        }
        println!("\n{} {}", "📍".cyan(), stmt.file.display().to_string().dimmed());
        self.display_statement(stmt);
        if options.prompt {
            println!("\nPattern detected: {}", pattern.yellow());
            println!("What should I do with statements containing this pattern?");
            println!("  [r] Remove this one");
            println!("  [k] Keep this one");
            println!("  [R] Always remove '{}'", pattern);
            println!("  [K] Always keep '{}'", pattern);
            println!("  [s] Skip file");
            loop {
                print!("Choice: ");
                io::stdout().flush()?;
                let mut input = String::new();
                io::stdin().read_line(&mut input)?;
                let choice = input.trim().to_lowercase();
                match choice.as_str() {
                    "r" => return Ok(true),
                    "k" => return Ok(false),
                    "R" | "rr" => {
                        self.remember_pattern(
                            pattern.clone(),
                            DecisionAction::AlwaysRemove,
                            config_path,
                        )?;
                        return Ok(true);
                    }
                    "K" | "kk" => {
                        self.remember_pattern(
                            pattern.clone(),
                            DecisionAction::AlwaysKeep,
                            config_path,
                        )?;
                        return Ok(false);
                    }
                    "s" => return Ok(false),
                    _ => println!("Invalid choice. Please try again."),
                }
            }
        } else {
            self.confirm("Remove this statement?")
        }
    }
    fn extract_pattern(&self, content: &str) -> String {
        let content = content.trim();
        for pattern in &[
            "HERE",
            "DEBUG:",
            "TODO:",
            "FIXME:",
            "Error:",
            "Warning:",
            "Info:",
        ] {
            if content.contains(pattern) {
                return pattern.to_string();
            }
        }
        if let Some(start) = content.find('"') {
            if let Some(end) = content[start + 1..].find('"') {
                let inner = &content[start + 1..start + 1 + end];
                if inner.len() <= 30 {
                    return inner.to_string();
                } else {
                    let words: Vec<&str> = inner.split_whitespace().take(3).collect();
                    return words.join(" ");
                }
            }
        }
        if content.starts_with("println!") {
            return "println!".to_string();
        } else if content.starts_with("eprintln!") {
            return "eprintln!".to_string();
        }
        "generic".to_string()
    }
    fn remember_pattern(
        &mut self,
        pattern: String,
        action: DecisionAction,
        config_path: &Path,
    ) -> Result<()> {
        println!(
            "  {} Remembering: always {} patterns with '{}'", "💾".green(), match
            action { DecisionAction::AlwaysRemove => "remove", DecisionAction::AlwaysKeep
            => "keep", DecisionAction::AskEachTime => "ask about", }, pattern.yellow()
        );
        self.pattern_cache.insert(pattern.clone(), action.clone());
        let decision = PatternDecision {
            pattern: pattern.clone(),
            action,
            created_at: chrono::Utc::now().to_rfc3339(),
        };
        self.config.remembered_patterns.insert(pattern, decision);
        self.save_config(config_path)?;
        Ok(())
    }
    fn should_keep_statement(
        &self,
        stmt: &PrintStatement,
        options: &SweepOptions,
    ) -> bool {
        match &stmt.context {
            PrintContext::MainFunction if options.keep_main => return true,
            PrintContext::TestFunction if options.keep_tests => return true,
            _ => {}
        }
        if options.keep_examples && stmt.file.to_string_lossy().contains("/examples/") {
            return true;
        }
        for pattern in &self.config.keep_patterns {
            if stmt.content.contains(pattern) {
                return true;
            }
        }
        false
    }
    fn remove_statements_from_content(
        &self,
        content: &str,
        statements: &[PrintStatement],
    ) -> Result<String> {
        let mut result = content.to_string();
        let mut sorted_statements = statements.to_vec();
        sorted_statements
            .sort_by(|a, b| b.line.cmp(&a.line).then(b.column.cmp(&a.column)));
        for stmt in sorted_statements {
            let lines: Vec<&str> = result.lines().collect();
            if stmt.line > 0 && stmt.line <= lines.len() {
                let line_idx = stmt.line - 1;
                let line = lines[line_idx];
                let trimmed = line.trim();
                if trimmed == stmt.content.trim()
                    || trimmed.ends_with(&format!("{};", stmt.content.trim()))
                {
                    let mut new_lines = lines.to_vec();
                    new_lines.remove(line_idx);
                    result = new_lines.join("\n");
                } else {
                    result = result.replace(&stmt.content, "");
                }
            }
        }
        Ok(result)
    }
    fn convert_to_log(
        &self,
        statements: Vec<PrintStatement>,
        options: &ConvertOptions,
    ) -> Result<ConvertStats> {
        let mut stats = ConvertStats::default();
        let mut files_to_process: HashMap<PathBuf, Vec<PrintStatement>> = HashMap::new();
        for stmt in statements {
            files_to_process.entry(stmt.file.clone()).or_default().push(stmt);
        }
        let pb = self
            .multi_progress
            .add(ProgressBar::new(files_to_process.len() as u64));
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{bar:40.cyan/blue}] {pos}/{len} {msg}")
                .unwrap(),
        );
        for (file_path, file_statements) in files_to_process {
            pb.set_message(
                format!(
                    "🔄 Converting: {}", file_path.file_name().unwrap_or_default()
                    .to_string_lossy()
                ),
            );
            if !options.dry_run {
                let content = fs::read_to_string(&file_path)?;
                let converted_content = self
                    .convert_statements_in_content(&content, &file_statements, options)?;
                let final_content = if !content.contains("use log::") {
                    format!("use log::*;\n\n{}", converted_content)
                } else {
                    converted_content
                };
                fs::write(&file_path, final_content)?;
            }
            stats.converted += file_statements.len();
            stats.files_modified += 1;
            pb.inc(1);
        }
        pb.finish_with_message("✅ Conversion complete!");
        if options.add_dependency {
            self.add_log_dependency()?;
        }
        Ok(stats)
    }
    fn convert_statements_in_content(
        &self,
        content: &str,
        statements: &[PrintStatement],
        options: &ConvertOptions,
    ) -> Result<String> {
        let mut result = content.to_string();
        for stmt in statements {
            let log_level = match stmt.kind {
                PrintKind::Println | PrintKind::Print => &options.println_level,
                PrintKind::Eprintln | PrintKind::Eprint => &options.eprintln_level,
                PrintKind::DbgMacro => &LogLevel::Debug,
            };
            let log_macro = format!("{}!", log_level.as_str());
            let inner = stmt
                .content
                .trim_start_matches(stmt.kind.as_str())
                .trim_start_matches('!')
                .trim_start_matches('(')
                .trim_end_matches(')')
                .trim_end_matches(';');
            let replacement = format!("{log_macro}({inner})");
            result = result.replace(&stmt.content.trim_end_matches(';'), &replacement);
        }
        Ok(result)
    }
    fn add_log_dependency(&self) -> Result<()> {
        let cargo_toml_path = PathBuf::from("Cargo.toml");
        if !cargo_toml_path.exists() {
            return Err(anyhow::anyhow!("Cargo.toml not found"));
        }
        let content = fs::read_to_string(&cargo_toml_path)?;
        if !content.contains("log =") && !content.contains("log=") {
            println!("{}", "📦 Adding log dependency to Cargo.toml...".yellow());
            let mut lines: Vec<String> = content
                .lines()
                .map(|s| s.to_string())
                .collect();
            let mut in_deps = false;
            let mut inserted = false;
            for i in 0..lines.len() {
                if lines[i].starts_with("[dependencies]") {
                    in_deps = true;
                } else if in_deps && !inserted
                    && (lines[i].starts_with('[') || lines[i].trim().is_empty())
                {
                    lines.insert(i, "log = \"0.4\"".to_string());
                    inserted = true;
                    break;
                }
            }
            if !inserted {
                lines.push("log = \"0.4\"".to_string());
            }
            fs::write(&cargo_toml_path, lines.join("\n"))?;
            println!("{}", "✅ Added log = \"0.4\" to dependencies".green());
        }
        Ok(())
    }
    pub fn analyze_patterns(
        &self,
        statements: &[PrintStatement],
        top_n: usize,
    ) -> AnalysisReport {
        let mut report = AnalysisReport::default();
        let mut file_counts: HashMap<PathBuf, usize> = HashMap::new();
        let mut kind_counts: HashMap<PrintKind, usize> = HashMap::new();
        let mut context_counts: HashMap<String, usize> = HashMap::new();
        for stmt in statements {
            *file_counts.entry(stmt.file.clone()).or_default() += 1;
            *kind_counts.entry(stmt.kind.clone()).or_default() += 1;
            let context_str = match &stmt.context {
                PrintContext::MainFunction => "main()".to_string(),
                PrintContext::TestFunction => "test".to_string(),
                PrintContext::RegularFunction(name) => format!("fn {}", name),
                PrintContext::ImplBlock => "impl block".to_string(),
                PrintContext::Module => "module".to_string(),
                PrintContext::Unknown => "unknown".to_string(),
            };
            *context_counts.entry(context_str).or_default() += 1;
        }
        let mut file_vec: Vec<_> = file_counts.into_iter().collect();
        file_vec.sort_by(|a, b| b.1.cmp(&a.1));
        report.top_files = file_vec.into_iter().take(top_n).collect();
        report.kind_distribution = kind_counts;
        report.context_distribution = context_counts;
        report.total_statements = statements.len();
        let mut pattern_counts: HashMap<String, usize> = HashMap::new();
        for stmt in statements {
            if stmt.content.contains("DEBUG") {
                *pattern_counts.entry("DEBUG markers".to_string()).or_default() += 1;
            }
            if stmt.content.contains("TODO") || stmt.content.contains("FIXME") {
                *pattern_counts.entry("TODO/FIXME markers".to_string()).or_default()
                    += 1;
            }
            if stmt.content.contains("HERE") || stmt.content.contains("got here") {
                *pattern_counts.entry("HERE markers".to_string()).or_default() += 1;
            }
            if stmt.content.contains("Error") || stmt.content.contains("error") {
                *pattern_counts.entry("Error messages".to_string()).or_default() += 1;
            }
            if stmt.content.contains("{:?}") {
                *pattern_counts.entry("Debug formatting {:?}".to_string()).or_default()
                    += 1;
            }
        }
        report.common_patterns = pattern_counts;
        report
    }
    pub fn display_statement(&self, stmt: &PrintStatement) {
        println!(
            "  {} line {} col {} - {} in {:?}", stmt.kind.color_str(), stmt.line
            .to_string().dimmed(), stmt.column.to_string().dimmed(), stmt.content.trim()
            .yellow(), stmt.context
        );
    }
}
pub fn export_to_json(statements: &[PrintStatement], export_path: &Path) -> Result<()> {
    let json = serde_json::to_string_pretty(statements)?;
    fs::write(export_path, json)?;
    Ok(())
}
pub fn create_default_config(path: &Path) -> Result<()> {
    let config = SweepConfig::default();
    let toml = toml::to_string_pretty(&config)?;
    fs::write(path, toml)?;
    println!("{} Created default sweep config at {}", "✅".green(), path.display());
    Ok(())
}
pub fn convert_statement_in_file(
    file_path: &Path,
    old_content: &str,
    new_content: &str,
) -> Result<()> {
    fs::write(file_path, new_content)?;
    Ok(())
}
impl Sweeper {
    pub fn display_report(&self, report: &AnalysisReport) {
        println!("\n{}", "📊 Sweep Analysis Report".bold().cyan());
        println!("{}", "═".repeat(60).cyan());
        println!(
            "\n{} {}", "Total Statements:".bold(), report.total_statements.to_string()
            .yellow()
        );
        println!("\n{}", "Distribution by Type:".bold());
        for (kind, count) in &report.kind_distribution {
            println!("  {} {}", kind.color_str(), count.to_string().white());
        }
        println!("\n{}", "Distribution by Context:".bold());
        for (context, count) in &report.context_distribution {
            println!("  {}: {}", context.green(), count.to_string().white());
        }
        println!("\n{}", "Top Files:".bold());
        for (file, count) in &report.top_files {
            let file_str = file.to_string_lossy();
            let display_path = if file_str.len() > 50 {
                format!("...{}", & file_str[file_str.len() - 47..])
            } else {
                file_str.to_string()
            };
            println!(
                "  {} - {} statements", display_path.blue(), count.to_string().yellow()
            );
        }
        if !report.common_patterns.is_empty() {
            println!("\n{}", "Common Patterns:".bold());
            for (pattern, count) in &report.common_patterns {
                println!("  {}: {}", pattern.cyan(), count.to_string().white());
            }
        }
        println!("\n{}", "💡 Recommendations:".bold().green());
        if report.total_statements > 50 {
            println!("  ⚠️  High number of print statements detected");
            println!(
                "  💡 Consider using a logging framework (log, env_logger, tracing)"
            );
            println!("  🧹 Run 'sweep sweep -p' to clean with pattern memory");
        }
        if report.kind_distribution.get(&PrintKind::DbgMacro).unwrap_or(&0) > &10 {
            println!(
                "  ⚠️  Many dbg! macros found - these should not be in production code"
            );
        }
        let here_patterns = report.common_patterns.get("HERE markers").unwrap_or(&0);
        if here_patterns > &5 {
            println!("  🤖 Looks like AI assistants have been here!");
            println!("  🧹 Run 'sweep sweep -y' to quickly clean these up");
        }
        println!();
    }
    fn confirm(&self, prompt: &str) -> Result<bool> {
        print!("{} {} (y/N): ", "?".yellow(), prompt);
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        Ok(input.trim().to_lowercase().starts_with('y'))
    }
    fn init_config(&self, path: &Path, force: bool) -> Result<()> {
        if path.exists() && !force {
            println!(
                "{} Config file already exists at {}", "⚠️".yellow(), path.display()
            );
            println!("Use --force to overwrite");
            return Ok(());
        }
        let config = SweepConfig::default();
        #[cfg(feature = "pristine")]
        {
            let toml = toml::to_string_pretty(&config)?;
            fs::write(path, toml)?;
        }
        println!(
            "{} Created sweep config at {}", "✅".green(), path.display().to_string()
            .cyan()
        );
        println!("\n{}", "You can now customize:".bold());
        println!("  • Patterns to always keep or remove");
        println!("  • Directories and files to skip");
        println!("  • Default behaviors for main/test/example files");
        println!("\n{} {}", "Edit:".dimmed(), path.display().to_string().cyan());
        Ok(())
    }
}
struct PrintStatementVisitor<'a> {
    file_path: &'a Path,
    content: &'a str,
    statements: Vec<PrintStatement>,
    current_context: PrintContext,
}
impl<'a> PrintStatementVisitor<'a> {
    fn new(file_path: &'a Path, content: &'a str) -> Self {
        Self {
            file_path,
            content,
            statements: Vec::new(),
            current_context: PrintContext::Unknown,
        }
    }
}
impl<'ast> VisitMut for PrintStatementVisitor<'ast> {
    fn visit_expr_macro_mut(&mut self, mac: &mut ExprMacro) {
        let macro_name = mac
            .mac
            .path
            .segments
            .last()
            .map(|s| s.ident.to_string())
            .unwrap_or_default();
        let kind = match macro_name.as_str() {
            "println" => Some(PrintKind::Println),
            "eprintln" => Some(PrintKind::Eprintln),
            "print" => Some(PrintKind::Print),
            "eprint" => Some(PrintKind::Eprint),
            "dbg" => Some(PrintKind::DbgMacro),
            _ => None,
        };
        if let Some(kind) = kind {
            let line = 1;
            let column = 1;
            self.statements
                .push(PrintStatement {
                    file: self.file_path.to_path_buf(),
                    line,
                    column,
                    kind,
                    content: quote::quote!(# mac).to_string(),
                    context: self.current_context.clone(),
                });
        }
        syn::visit_mut::visit_expr_macro_mut(self, mac);
    }
    fn visit_item_fn_mut(&mut self, func: &mut syn::ItemFn) {
        let old_context = self.current_context.clone();
        self.current_context = if func.sig.ident == "main" {
            PrintContext::MainFunction
        } else if func.attrs.iter().any(|attr| attr.path().is_ident("test")) {
            PrintContext::TestFunction
        } else {
            PrintContext::RegularFunction(func.sig.ident.to_string())
        };
        syn::visit_mut::visit_item_fn_mut(self, func);
        self.current_context = old_context;
    }
}
pub struct SweepOptions {
    pub dry_run: bool,
    pub interactive: bool,
    pub prompt: bool,
    pub keep_main: bool,
    pub keep_tests: bool,
    pub keep_examples: bool,
    pub backup: bool,
    pub yes: bool,
}
struct ConvertOptions {
    println_level: LogLevel,
    eprintln_level: LogLevel,
    dry_run: bool,
    add_dependency: bool,
}
#[derive(Default)]
pub struct SweepStats {
    removed: usize,
    kept: usize,
    files_modified: usize,
}
#[derive(Default)]
struct ConvertStats {
    converted: usize,
    files_modified: usize,
}
#[derive(Default)]
pub struct AnalysisReport {
    total_statements: usize,
    top_files: Vec<(PathBuf, usize)>,
    kind_distribution: HashMap<PrintKind, usize>,
    context_distribution: HashMap<String, usize>,
    common_patterns: HashMap<String, usize>,
}
pub fn run_with_cli(cli: Cli) -> Result<()> {
    let mut sweeper = Sweeper::new();
    sweeper.load_config(&cli.config)?;
    match cli.command {
        Commands::Scan { path, include_tests, include_examples, export } => {
            let statements = sweeper
                .scan_directory(&path, include_tests, include_examples)?;
            if statements.is_empty() {
                println!("{}", "✨ Clean! No print statements found.".green());
                return Ok(());
            }
            println!("\n{}", "📋 Scan Results".bold().blue());
            println!("{}", "─".repeat(60).dimmed());
            for stmt in &statements {
                sweeper.display_statement(stmt);
            }
            println!("\n{}: {}", "Total".bold(), statements.len().to_string().yellow());
            if let Some(export_path) = export {
                #[cfg(feature = "pristine")]
                {
                    let json = serde_json::to_string_pretty(&statements)?;
                    fs::write(&export_path, json)?;
                }
                println!("\n✅ Exported to {}", export_path.display());
            }
        }
        Commands::Sweep {
            path,
            dry_run,
            interactive,
            prompt,
            keep_main,
            keep_tests,
            keep_examples,
            backup,
            yes,
            pristine,
            format,
            organize_imports,
            add_docs,
            fix_clippy,
        } => {
            let statements = sweeper.scan_directory(&path, false, false)?;
            if statements.is_empty() {
                println!(
                    "{}", "✨ Already swept clean! No print statements found.".green()
                );
                return Ok(());
            }
            println!(
                "\n{} Found {} statements to potentially sweep", "🧹".cyan(),
                statements.len().to_string().yellow()
            );
            let options = SweepOptions {
                dry_run,
                interactive,
                prompt,
                keep_main,
                keep_tests,
                keep_examples,
                backup,
                yes,
            };
            let stats = sweeper.sweep_files(statements, &options, &cli.config)?;
            println!("\n{}", "🧹 Sweep Summary".bold().green());
            println!("{}", "═".repeat(60).cyan());
            println!(
                "  {} Removed: {}", "🗑️".red(), stats.removed.to_string().red()
            );
            println!("  {} Kept: {}", "✅".green(), stats.kept.to_string().green());
            println!(
                "  {} Files modified: {}", "📝".yellow(), stats.files_modified
                .to_string().yellow()
            );
            if dry_run {
                println!(
                    "\n{}", "💡 This was a dry run. Remove -n to apply changes."
                    .yellow().italic()
                );
            }
        }
        Commands::Convert {
            path,
            println_level,
            eprintln_level,
            dry_run,
            add_dependency,
        } => {
            let statements = sweeper.scan_directory(&path, true, true)?;
            if statements.is_empty() {
                println!("{}", "✨ No print statements found to convert!".green());
                return Ok(());
            }
            let options = ConvertOptions {
                println_level,
                eprintln_level,
                dry_run,
                add_dependency,
            };
            let stats = sweeper.convert_to_log(statements, &options)?;
            println!("\n{}", "🔄 Conversion Summary".bold().green());
            println!("{}", "═".repeat(60).cyan());
            println!("  Converted: {} statements", stats.converted.to_string().green());
            println!("  Files modified: {}", stats.files_modified.to_string().yellow());
            if dry_run {
                println!(
                    "\n{}", "This was a dry run. Remove -n to apply changes.".yellow()
                );
            }
        }
        Commands::Analyze { path, top } => {
            let statements = sweeper.scan_directory(&path, true, true)?;
            if statements.is_empty() {
                println!(
                    "{}", "✨ Perfectly clean! No print statements found.".green()
                );
                return Ok(());
            }
            let report = sweeper.analyze_patterns(&statements, top);
            sweeper.display_report(&report);
        }
        Commands::Init { force } => {
            sweeper.init_config(&cli.config, force)?;
        }
    }
    Ok(())
}
fn main() -> Result<()> {
    let cli = Cli::parse();
    run_with_cli(cli)
}