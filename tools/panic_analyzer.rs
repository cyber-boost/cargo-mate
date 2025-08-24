use super::{Tool, Result, ToolError, common_options, parse_output_format, OutputFormat};
use clap::{Arg, ArgMatches, Command};
use colored::*;
use std::path::Path;
use std::fs;
use std::collections::HashMap;
use regex::Regex;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct PanicAnalyzerTool;

#[derive(Debug, Clone, Deserialize, Serialize)]
struct PanicInfo {
    message: String,
    location: String,
    context: Vec<String>,
    suggestions: Vec<String>,
    timestamp: String,
    frequency: usize,
}

#[derive(Debug, Deserialize, Serialize)]
struct PanicReport {
    total_panics: usize,
    unique_patterns: usize,
    most_common: Vec<PanicInfo>,
    recent_panics: Vec<PanicInfo>,
}

impl PanicAnalyzerTool {
    pub fn new() -> Self {
        Self
    }

    fn parse_panic_message(&self, line: &str) -> Option<String> {
        let panic_patterns = [
            r"thread '.*' panicked at '(.*?)'",
            r"panic!(.*)",
            r"unreachable!(.*)",
            r"todo!(.*)",
            r"unimplemented!(.*)",
        ];

        for pattern in &panic_patterns {
            if let Ok(regex) = Regex::new(pattern) {
                if let Some(captures) = regex.captures(line) {
                    if let Some(message) = captures.get(1) {
                        return Some(message.as_str().to_string());
                    }
                }
            }
        }

        None
    }

    fn parse_location(&self, line: &str) -> Option<String> {
        let location_patterns = [
            r"at (.+:\d+)",
            r"at (.+:\d+:\d+)",
        ];

        for pattern in &location_patterns {
            if let Ok(regex) = Regex::new(pattern) {
                if let Some(captures) = regex.captures(line) {
                    if let Some(location) = captures.get(1) {
                        return Some(location.as_str().to_string());
                    }
                }
            }
        }

        None
    }

    fn extract_source_context(&self, location: &str, context_lines: usize) -> Result<Vec<String>> {
        let parts: Vec<&str> = location.split(':').collect();
        if parts.len() < 2 {
            return Ok(vec!["Could not parse location".to_string()]);
        }

        let file_path = parts[0];
        let line_num: usize = parts[1].parse().unwrap_or(1);

        let path = Path::new(file_path);
        if !path.exists() {
            return Ok(vec![format!("File not found: {}", file_path)]);
        }

        let content = fs::read_to_string(path)
            .map_err(|e| ToolError::ExecutionFailed(format!("Cannot read {}: {}", file_path, e)))?;

        let lines: Vec<&str> = content.lines().collect();
        let start = line_num.saturating_sub(context_lines + 1);
        let end = (line_num + context_lines).min(lines.len());

        let mut context = Vec::new();
        for (i, line) in lines.iter().enumerate().skip(start).take(end - start) {
            let marker = if i + 1 == line_num { ">>> " } else { "    " };
            context.push(format!("{}{}: {}", marker, i + 1, line));
        }

        Ok(context)
    }

    fn generate_suggestions(&self, panic_message: &str) -> Vec<String> {
        let mut suggestions = Vec::new();

        let message_lower = panic_message.to_lowercase();

        if message_lower.contains("index out of bounds") {
            suggestions.push("Check array/vector bounds before accessing elements".to_string());
            suggestions.push("Use .get(index) instead of [index] for safe access".to_string());
            suggestions.push("Add bounds checking with if index < len".to_string());
        }

        if message_lower.contains("called `option::unwrap()`") {
            suggestions.push("Use .unwrap_or(default_value) for safe unwrapping".to_string());
            suggestions.push("Use .unwrap_or_else(|| default_fn()) for computed defaults".to_string());
            suggestions.push("Use if let Some(value) = option pattern".to_string());
        }

        if message_lower.contains("called `result::unwrap()`") {
            suggestions.push("Use .unwrap_or(default_value) for safe error handling".to_string());
            suggestions.push("Use ? operator in functions that return Result".to_string());
            suggestions.push("Use match or if let for proper error handling".to_string());
        }

        if message_lower.contains("borrow checker") {
            suggestions.push("Check for multiple mutable borrows of the same value".to_string());
            suggestions.push("Use references with different lifetimes".to_string());
            suggestions.push("Consider cloning the value if appropriate".to_string());
        }

        if message_lower.contains("cannot move out") {
            suggestions.push("Use references (&) instead of moving values".to_string());
            suggestions.push("Implement Copy trait for simple types".to_string());
            suggestions.push("Use .clone() if the type implements Clone".to_string());
        }

        if message_lower.contains("overflow") {
            suggestions.push("Use checked operations: checked_add, checked_sub, etc.".to_string());
            suggestions.push("Add bounds checking before arithmetic operations".to_string());
            suggestions.push("Use saturating operations for safe overflow handling".to_string());
        }

        if suggestions.is_empty() {
            suggestions.push("Review the panic location and ensure proper error handling".to_string());
            suggestions.push("Consider using Result<T, E> instead of panicking".to_string());
            suggestions.push("Add debug logging before the panic location".to_string());
        }

        suggestions
    }

    fn analyze_log_file(&self, log_path: &str) -> Result<Vec<PanicInfo>> {
        let path = Path::new(log_path);
        if !path.exists() {
            return Err(ToolError::ExecutionFailed(format!("Log file not found: {}", log_path)));
        }

        let content = fs::read_to_string(path)
            .map_err(|e| ToolError::ExecutionFailed(format!("Cannot read log file: {}", e)))?;

        let mut panics = Vec::new();
        let lines: Vec<&str> = content.lines().collect();

        for (i, line) in lines.iter().enumerate() {
            if let Some(message) = self.parse_panic_message(line) {
                let location = if let Some(loc) = self.parse_location(line) {
                    loc
                } else {
                    // Try to find location in nearby lines
                    let mut location = "Unknown location".to_string();
                    for j in (i + 1)..lines.len().min(i + 10) {
                        if let Some(loc) = self.parse_location(lines[j]) {
                            location = loc;
                            break;
                        }
                    }
                    location
                };

                let context = if location != "Unknown location" {
                    self.extract_source_context(&location, 3).unwrap_or_default()
                } else {
                    vec!["Could not extract source context".to_string()]
                };

                let suggestions = self.generate_suggestions(&message);

                panics.push(PanicInfo {
                    message,
                    location,
                    context,
                    suggestions,
                    timestamp: chrono::Utc::now().to_rfc3339(),
                    frequency: 1,
                });
            }
        }

        Ok(panics)
    }

    fn analyze_recent_panics(&self, count: usize) -> Result<Vec<PanicInfo>> {
        // Look for common panic log locations
        let log_paths = [
            ".cargo-mate/panics.log",
            "target/debug/panic.log",
            "/tmp/cargo-mate-panics.log",
        ];

        let mut all_panics = Vec::new();

        for log_path in &log_paths {
            if let Ok(panics) = self.analyze_log_file(log_path) {
                all_panics.extend(panics);
            }
        }

        // Sort by recency and take the most recent ones
        all_panics.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        all_panics.truncate(count);

        Ok(all_panics)
    }

    fn group_similar_panics(&self, panics: &[PanicInfo]) -> HashMap<String, Vec<PanicInfo>> {
        let mut groups: HashMap<String, Vec<PanicInfo>> = HashMap::new();

        for panic in panics {
            let key = panic.message.to_lowercase();
            groups.entry(key).or_insert_with(Vec::new).push(panic.clone());
        }

        groups
    }

    fn generate_report(&self, panics: &[PanicInfo], format: OutputFormat, verbose: bool) -> Result<()> {
        match format {
            OutputFormat::Json => {
                let grouped = self.group_similar_panics(panics);
                let report = PanicReport {
                    total_panics: panics.len(),
                    unique_patterns: grouped.len(),
                    most_common: panics.to_vec(),
                    recent_panics: panics.to_vec(),
                };
                println!("{}", serde_json::to_string_pretty(&report).unwrap());
            }
            OutputFormat::Table => {
                println!("{:<50} {:<30} {:<15}", "Panic Message", "Location", "Suggestions");
                println!("{}", "─".repeat(100));

                for panic in panics {
                    let message = panic.message.chars().take(47).collect::<String>();
                    let location = panic.location.chars().take(27).collect::<String>();
                    let suggestion_count = panic.suggestions.len().to_string();

                    println!("{:<50} {:<30} {:<15}",
                            message,
                            location,
                            format!("{} suggestions", suggestion_count));
                }
            }
            OutputFormat::Human => {
                println!("{}", "🚨 Panic Analysis Report".bold().red());
                println!("{}", "═".repeat(50).red());

                if panics.is_empty() {
                    println!("✅ No panics found in recent logs");
                    return Ok(());
                }

                println!("📊 Found {} panic(s)", panics.len());

                let grouped = self.group_similar_panics(panics);
                println!("🔍 Unique patterns: {}", grouped.len());

                for (i, panic) in panics.iter().enumerate() {
                    println!("\n{}. {}", i + 1, panic.message.red().bold());
                    println!("   📍 Location: {}", panic.location.cyan());

                    if verbose {
                        println!("   📝 Context:");
                        for line in &panic.context {
                            println!("      {}", line);
                        }

                        if !panic.suggestions.is_empty() {
                            println!("   💡 Suggestions:");
                            for suggestion in &panic.suggestions {
                                println!("      • {}", suggestion.yellow());
                            }
                        }
                    }
                }

                if !verbose && !panics.is_empty() {
                    println!("\n💡 Use --verbose to see source context and fix suggestions");
                }
            }
        }
        Ok(())
    }
}

impl Tool for PanicAnalyzerTool {
    fn name(&self) -> &'static str {
        "panic-analyzer"
    }

    fn description(&self) -> &'static str {
        "Parse panic messages and provide debugging context with fix suggestions"
    }

    fn command(&self) -> Command {
        Command::new(self.name())
            .about(self.description())
            .long_about("Analyze panic messages from logs, show source code context, and provide fix suggestions based on common patterns")
            .args(&[
                Arg::new("recent")
                    .long("recent")
                    .short('r')
                    .help("Analyze recent panics from log files")
                    .action(clap::ArgAction::SetTrue),
                Arg::new("count")
                    .long("count")
                    .short('c')
                    .help("Number of recent panics to analyze")
                    .default_value("10"),
                Arg::new("log-file")
                    .long("log-file")
                    .short('f')
                    .help("Specific log file to analyze"),
                Arg::new("pattern")
                    .long("pattern")
                    .short('p')
                    .help("Search for specific panic patterns"),
                Arg::new("context")
                    .long("context")
                    .short('x')
                    .help("Number of context lines around panic location")
                    .default_value("3"),
                Arg::new("suggest-fixes")
                    .long("suggest-fixes")
                    .help("Show fix suggestions for panics")
                    .action(clap::ArgAction::SetTrue),
                Arg::new("report")
                    .long("report")
                    .help("Generate panic analysis report")
                    .action(clap::ArgAction::SetTrue),
            ])
            .args(&common_options())
    }

    fn execute(&self, matches: &ArgMatches) -> Result<()> {
        let recent = matches.get_flag("recent");
        let count: usize = matches.get_one::<String>("count")
            .unwrap()
            .parse()
            .map_err(|_| ToolError::InvalidArguments("Invalid count value".to_string()))?;
        let log_file = matches.get_one::<String>("log-file");
        let pattern = matches.get_one::<String>("pattern");
        let context_lines: usize = matches.get_one::<String>("context")
            .unwrap()
            .parse()
            .map_err(|_| ToolError::InvalidArguments("Invalid context value".to_string()))?;
        let suggest_fixes = matches.get_flag("suggest-fixes");
        let report = matches.get_flag("report");
        let output_format = parse_output_format(matches);
        let verbose = matches.get_flag("verbose");

        println!("🚨 {} - Analyzing panic messages", "CargoMate PanicAnalyzer".bold().red());

        let panics = if let Some(log_path) = log_file {
            self.analyze_log_file(log_path)?
        } else if recent {
            self.analyze_recent_panics(count)?
        } else {
            // Default: check common locations
            self.analyze_recent_panics(count)?
        };

        // Filter by pattern if specified
        let filtered_panics: Vec<PanicInfo> = if let Some(pat) = pattern {
            panics.into_iter()
                .filter(|p| p.message.to_lowercase().contains(&pat.to_lowercase()))
                .collect()
        } else {
            panics
        };

        if filtered_panics.is_empty() {
            println!("✅ No panics found matching criteria");
            return Ok(());
        }

        if report {
            self.generate_report(&filtered_panics, output_format, verbose)?;
        } else {
            self.generate_report(&filtered_panics, output_format, verbose)?;
        }

        Ok(())
    }
}

impl Default for PanicAnalyzerTool {
    fn default() -> Self {
        Self::new()
    }
}
