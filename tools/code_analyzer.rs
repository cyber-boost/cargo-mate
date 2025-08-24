use crate::tools::{Tool, Result, ToolError, common_options, parse_output_format, OutputFormat};
use clap::{Arg, ArgMatches, Command};
use colored::*;
use std::collections::HashMap;
use std::path::Path;
use std::fs;
use regex::Regex;
use walkdir::WalkDir;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone)]
pub struct CodeAnalyzer;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionMetrics {
    pub name: String,
    pub file: String,
    pub lines: usize,
    pub complexity: f64,
    pub parameters: usize,
    pub is_public: bool,
    pub is_async: bool,
    pub return_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeAnalysis {
    pub total_functions: usize,
    pub public_functions: usize,
    pub async_functions: usize,
    pub average_complexity: f64,
    pub total_lines: usize,
    pub largest_function: Option<FunctionMetrics>,
    pub functions: Vec<FunctionMetrics>,
}

impl CodeAnalyzer {
    pub fn new() -> Self {
        Self
    }

    fn parse_rust_functions(&self, file_path: &str) -> Result<Vec<FunctionMetrics>> {
        let content = fs::read_to_string(file_path).map_err(|e| {
            ToolError::InvalidArguments(format!("Failed to read {}: {}", file_path, e))
        })?;

        let mut functions = Vec::new();

        // Simple function regex
        let function_regex = Regex::new(r"(?s)(?:pub\s+)?(?:async\s+)?(?:unsafe\s+)?fn\s+(\w+)\s*\(([^)]*)\)\s*(?:->\s*([^;{]*?))?\s*\{([^}]*)\}").unwrap();

        for captures in function_regex.captures_iter(&content) {
            let function_name = captures[1].to_string();
            let parameters = captures.get(2).map_or("", |m| m.as_str());
            let return_type = captures.get(3).map_or("()", |m| m.as_str().trim());
            let function_body = captures.get(4).map_or("", |m| m.as_str());

            let metrics = FunctionMetrics {
                name: function_name.clone(),
                file: file_path.to_string(),
                lines: function_body.lines().count(),
                complexity: self.calculate_complexity(function_body),
                parameters: self.count_parameters(parameters),
                is_public: function_name.starts_with("pub") || content.contains(&format!("pub fn {}", function_name)),
                is_async: content.contains("async fn") && content.contains(&function_name),
                return_type: return_type.to_string(),
            };

            functions.push(metrics);
        }

        Ok(functions)
    }

    fn calculate_complexity(&self, code: &str) -> f64 {
        let lines = code.lines().count() as f64;
        let branches = code.matches("if").count() as f64 + code.matches("match").count() as f64;
        let loops = code.matches("for").count() + code.matches("while").count() + code.matches("loop").count();

        // Simple complexity formula
        lines * 0.1 + branches * 0.5 + loops as f64 * 0.8
    }

    fn count_parameters(&self, params: &str) -> usize {
        if params.trim().is_empty() {
            return 0;
        }
        params.split(',').filter(|p| !p.trim().is_empty()).count()
    }

    fn analyze_codebase(&self, path: &str) -> Result<CodeAnalysis> {
        let mut all_functions = Vec::new();

        if Path::new(path).is_file() {
            if path.ends_with(".rs") {
                all_functions.extend(self.parse_rust_functions(path)?);
            }
        } else {
            for entry in WalkDir::new(path)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.path().extension().map_or(false, |ext| ext == "rs"))
            {
                all_functions.extend(self.parse_rust_functions(&entry.path().to_string_lossy())?);
            }
        }

        let total_functions = all_functions.len();
        let public_functions = all_functions.iter().filter(|f| f.is_public).count();
        let async_functions = all_functions.iter().filter(|f| f.is_async).count();
        let total_lines = all_functions.iter().map(|f| f.lines).sum::<usize>();
        let average_complexity = if total_functions > 0 {
            all_functions.iter().map(|f| f.complexity).sum::<f64>() / total_functions as f64
        } else {
            0.0
        };

        let largest_function = all_functions.iter()
            .max_by_key(|f| f.lines)
            .cloned();

        Ok(CodeAnalysis {
            total_functions,
            public_functions,
            async_functions,
            average_complexity,
            total_lines,
            largest_function,
            functions: all_functions,
        })
    }

    fn display_analysis(&self, analysis: &CodeAnalysis, verbose: bool) {
        println!("\n📊 {} - Code Analysis Report", "CargoMate CodeAnalyzer".bold().blue());
        println!("{}", "═".repeat(50).blue());

        println!("\n📈 Summary:");
        println!("  • Total Functions: {}", analysis.total_functions);
        println!("  • Public API: {}", analysis.public_functions);
        println!("  • Async Functions: {}", analysis.async_functions);
        println!("  • Total Lines: {}", analysis.total_lines);
        println!("  • Average Complexity: {:.2}", analysis.average_complexity);

        if let Some(ref largest) = analysis.largest_function {
            println!("  • Largest Function: {} ({} lines)", largest.name, largest.lines);
        }

        if verbose && !analysis.functions.is_empty() {
            println!("\n🔍 Function Details:");

            for function in &analysis.functions {
                let complexity_color = if function.complexity > 10.0 {
                    format!("{:.2}", function.complexity).red()
                } else if function.complexity > 5.0 {
                    format!("{:.2}", function.complexity).yellow()
                } else {
                    format!("{:.2}", function.complexity).green()
                };

                println!("  • {}::{} ({}, {} lines, complexity: {})",
                    function.file.split('/').last().unwrap_or(&function.file),
                    function.name.cyan(),
                    if function.is_public { "public".green() } else { "private".dimmed() },
                    function.lines,
                    complexity_color);
            }
        }

        println!("\n✅ Analysis complete!");
    }
}

impl Tool for CodeAnalyzer {
    fn name(&self) -> &'static str {
        "code-analyzer"
    }

    fn description(&self) -> &'static str {
        "Analyze Rust code metrics and complexity"
    }

    fn command(&self) -> Command {
        Command::new(self.name())
            .about(self.description())
            .long_about("Analyzes Rust code to provide metrics on function complexity, API surface, and code quality indicators.")
            .args(&[
                Arg::new("path")
                    .long("path")
                    .short('p')
                    .help("Path to Rust project to analyze")
                    .default_value("."),
                Arg::new("output-format")
                    .long("output-format")
                    .short('f')
                    .help("Output format (human, json, table)")
                    .default_value("human"),
            ])
            .args(&common_options())
    }

    fn execute(&self, matches: &ArgMatches) -> Result<()> {
        let path = matches.get_one::<String>("path").unwrap();
        let output_format = matches.get_one::<String>("output-format").unwrap();
        let verbose = matches.get_flag("verbose");

        println!("📊 {} - Analyzing Code Metrics", "CargoMate CodeAnalyzer".bold().blue());

        // Validate path exists
        if !Path::new(path).exists() {
            return Err(ToolError::InvalidArguments(format!("Path {} does not exist", path)));
        }

        // Analyze codebase
        let analysis = self.analyze_codebase(path)?;

        if analysis.total_functions == 0 {
            println!("{}", "No Rust functions found to analyze".yellow());
            return Ok(());
        }

        // Display results
        match output_format.as_str() {
            "json" => {
                let json = serde_json::to_string_pretty(&analysis)?;
                println!("{}", json);
            }
            "table" => {
                println!("{:<30} {:<10} {:<12} {:<8} {:<8}",
                    "Function", "Lines", "Complexity", "Params", "Public");
                println!("{}", "─".repeat(70));
                for function in &analysis.functions {
                    println!("{:<30} {:<10} {:<12.2} {:<8} {:<8}",
                        format!("{}::{}", function.file.split('/').last().unwrap_or(""), function.name),
                        function.lines,
                        function.complexity,
                        function.parameters,
                        if function.is_public { "Yes" } else { "No" });
                }
            }
            _ => {
                self.display_analysis(&analysis, verbose);
            }
        }

        Ok(())
    }
}

impl Default for CodeAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}
