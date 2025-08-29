use super::{Tool, Result, ToolError, common_options, parse_output_format, OutputFormat};
use clap::{Arg, ArgMatches, Command};
use colored::*;
use std::path::Path;
use std::fs;
use std::collections::HashMap;
use regex::Regex;
use syn::{parse_file, Item, ItemMacro, Lit};
#[derive(Debug, Clone)]
pub struct SqlMacroCheckTool;
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct SqlAnalysisReport {
    files_analyzed: usize,
    sql_queries_found: usize,
    macros_analyzed: Vec<SqlMacroAnalysis>,
    security_issues: Vec<SecurityIssue>,
    performance_issues: Vec<PerformanceIssue>,
    syntax_errors: Vec<SyntaxError>,
    suggestions: Vec<String>,
    timestamp: String,
}
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct SqlMacroAnalysis {
    file_path: String,
    macro_name: String,
    sql_query: String,
    line_number: usize,
    parameters: Vec<String>,
    security_score: u8,
    performance_score: u8,
    issues: Vec<String>,
}
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct SecurityIssue {
    file_path: String,
    line_number: usize,
    issue_type: String,
    description: String,
    severity: String,
    sql_snippet: String,
    suggestion: String,
}
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct PerformanceIssue {
    file_path: String,
    line_number: usize,
    issue_type: String,
    description: String,
    sql_snippet: String,
    suggestion: String,
}
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct SyntaxError {
    file_path: String,
    line_number: usize,
    error_type: String,
    description: String,
    sql_snippet: String,
}
impl SqlMacroCheckTool {
    pub fn new() -> Self {
        Self
    }
    fn find_rust_files(&self, directory: &str) -> Result<Vec<String>> {
        let mut files = Vec::new();
        self.find_rust_files_recursive(directory, &mut files)?;
        Ok(files)
    }
    fn find_rust_files_recursive(
        &self,
        dir: &str,
        files: &mut Vec<String>,
    ) -> Result<()> {
        let path = Path::new(dir);
        if !path.exists() {
            return Ok(());
        }
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                let dir_name = path.file_name().unwrap_or_default().to_string_lossy();
                if !matches!(dir_name.as_ref(), "target" | ".git" | "node_modules") {
                    self.find_rust_files_recursive(&path.to_string_lossy(), files)?;
                }
            } else if let Some(ext) = path.extension() {
                if ext == "rs" {
                    files.push(path.to_string_lossy().to_string());
                }
            }
        }
        Ok(())
    }
    fn analyze_sql_macros(&self, file_path: &str) -> Result<Vec<SqlMacroAnalysis>> {
        let content = fs::read_to_string(file_path)?;
        let mut analyses = Vec::new();
        let macro_patterns = vec![
            r#"sql!s*\(\s*"([^"]+)""#, r#"query!s*\(\s*"([^"]+)""#,
            r#"sqlx::query!s*\(\s*"([^"]+)""#,
            r#"diesel::prelude::sql_query\s*\(\s*"([^"]+)""#,
            r#"sea_orm::Statement::from_string\s*\(\s*"([^"]+)""#,
        ];
        for (line_num, line) in content.lines().enumerate() {
            for pattern in &macro_patterns {
                if let Ok(regex) = Regex::new(pattern) {
                    if let Some(captures) = regex.captures(line) {
                        if let Some(sql_match) = captures.get(1) {
                            let sql_query = sql_match.as_str().to_string();
                            let macro_name = self.extract_macro_name(line);
                            let analysis = self
                                .analyze_sql_query(
                                    file_path.to_string(),
                                    macro_name,
                                    sql_query,
                                    line_num + 1,
                                );
                            analyses.push(analysis);
                        }
                    }
                }
            }
        }
        Ok(analyses)
    }
    fn extract_macro_name(&self, line: &str) -> String {
        if line.contains("sqlx::query!") {
            "sqlx::query!".to_string()
        } else if line.contains("diesel::") {
            "diesel::sql_query".to_string()
        } else if line.contains("sea_orm::") {
            "sea_orm::Statement".to_string()
        } else if line.contains("sql!") {
            "sql!".to_string()
        } else if line.contains("query!") {
            "query!".to_string()
        } else {
            "unknown_macro".to_string()
        }
    }
    fn analyze_sql_query(
        &self,
        file_path: String,
        macro_name: String,
        sql_query: String,
        line_number: usize,
    ) -> SqlMacroAnalysis {
        let mut issues = Vec::new();
        let mut security_score = 100;
        let mut performance_score = 100;
        let parameters = self.extract_parameters(&sql_query);
        if self.has_sql_injection_risks(&sql_query) {
            security_score -= 50;
            issues.push("Potential SQL injection vulnerability".to_string());
        }
        if self.has_unparameterized_queries(&sql_query, &parameters) {
            security_score -= 30;
            issues.push("Unparameterized query detected".to_string());
        }
        if self.uses_deprecated_features(&sql_query) {
            security_score -= 20;
            issues.push("Uses deprecated SQL features".to_string());
        }
        if self.has_select_star(&sql_query) {
            performance_score -= 20;
            issues.push("SELECT * detected - specify columns explicitly".to_string());
        }
        if self.has_missing_indexes(&sql_query) {
            performance_score -= 15;
            issues.push("Query may benefit from additional indexes".to_string());
        }
        if self.has_cartesian_product(&sql_query) {
            performance_score -= 25;
            issues.push("Potential Cartesian product in JOIN".to_string());
        }
        if self.has_inefficient_functions(&sql_query) {
            performance_score -= 10;
            issues.push("Uses potentially inefficient functions".to_string());
        }
        SqlMacroAnalysis {
            file_path,
            macro_name,
            sql_query,
            line_number,
            parameters,
            security_score,
            performance_score,
            issues,
        }
    }
    fn extract_parameters(&self, sql_query: &str) -> Vec<String> {
        let mut parameters = Vec::new();
        let param_patterns = vec![r"\$\d+", r"\?", r":\w+", r"#\{\w+\}", r"%s|%d|%f",];
        for pattern in param_patterns {
            if let Ok(regex) = Regex::new(pattern) {
                for captures in regex.captures_iter(sql_query) {
                    if let Some(param) = captures.get(0) {
                        parameters.push(param.as_str().to_string());
                    }
                }
            }
        }
        parameters
    }
    fn has_sql_injection_risks(&self, sql_query: &str) -> bool {
        sql_query.contains(" + ") || sql_query.contains(" || ")
            || sql_query.to_lowercase().contains("concat") || sql_query.contains("' + ")
            || sql_query.contains(" + '")
    }
    fn has_unparameterized_queries(
        &self,
        sql_query: &str,
        parameters: &[String],
    ) -> bool {
        let string_literals = Regex::new(r"'[^']*'").unwrap();
        let string_count = string_literals.captures_iter(sql_query).count();
        string_count > parameters.len() + 2
    }
    fn uses_deprecated_features(&self, sql_query: &str) -> bool {
        let deprecated_features = vec!["mysql_", "old_", "deprecated_", "type=",];
        deprecated_features
            .iter()
            .any(|feature| sql_query.to_lowercase().contains(feature))
    }
    fn has_select_star(&self, sql_query: &str) -> bool {
        Regex::new(r"\bSELECT\s+\*").unwrap().is_match(sql_query)
    }
    fn has_missing_indexes(&self, sql_query: &str) -> bool {
        let where_clause = Regex::new(r"WHERE\s+(.+?)(?:ORDER|GROUP|LIMIT|$)").unwrap();
        if let Some(captures) = where_clause.captures(sql_query) {
            if let Some(where_part) = captures.get(1) {
                let where_text = where_part.as_str();
                where_text.contains("LIKE '%") || where_text.contains("NOT IN")
                    || where_text.contains("OR") && !where_text.contains("AND")
            } else {
                false
            }
        } else {
            false
        }
    }
    fn has_cartesian_product(&self, sql_query: &str) -> bool {
        let join_without_on = Regex::new(r"JOIN\s+\w+\s*(?:WHERE|ORDER|GROUP|HAVING|$)")
            .unwrap();
        let multiple_from = Regex::new(r"FROM\s+\w+.*[,;]\s*\w+").unwrap();
        join_without_on.is_match(sql_query) || multiple_from.is_match(sql_query)
    }
    fn has_inefficient_functions(&self, sql_query: &str) -> bool {
        let inefficient_functions = vec![
            "count(*)", "distinct(", "substring(", "concat(", "coalesce(",
        ];
        inefficient_functions.iter().any(|func| sql_query.to_lowercase().contains(func))
    }
    fn check_sql_syntax(&self, sql_query: &str) -> Vec<SyntaxError> {
        let mut errors = Vec::new();
        let paren_count = sql_query.chars().filter(|&c| c == '(').count()
            - sql_query.chars().filter(|&c| c == ')').count();
        if paren_count != 0 {
            errors
                .push(SyntaxError {
                    file_path: "unknown".to_string(),
                    line_number: 0,
                    error_type: "unmatched_parentheses".to_string(),
                    description: format!(
                        "Unmatched parentheses: {} open, {} close", sql_query.chars()
                        .filter(|& c | c == '(').count(), sql_query.chars().filter(|& c |
                        c == ')').count()
                    ),
                    sql_snippet: sql_query.to_string(),
                });
        }
        if !sql_query.trim().ends_with(';')
            && !sql_query.to_lowercase().contains("select")
        {
            errors
                .push(SyntaxError {
                    file_path: "unknown".to_string(),
                    line_number: 0,
                    error_type: "missing_semicolon".to_string(),
                    description: "SQL statement should end with semicolon".to_string(),
                    sql_snippet: sql_query.to_string(),
                });
        }
        if sql_query.to_lowercase().contains("sele ct") {
            errors
                .push(SyntaxError {
                    file_path: "unknown".to_string(),
                    line_number: 0,
                    error_type: "typo_in_keyword".to_string(),
                    description: "Possible typo in SELECT keyword".to_string(),
                    sql_snippet: sql_query.to_string(),
                });
        }
        errors
    }
    fn generate_suggestions(&self, analyses: &[SqlMacroAnalysis]) -> Vec<String> {
        let mut suggestions = Vec::new();
        let has_security_issues = analyses.iter().any(|a| a.security_score < 100);
        let has_performance_issues = analyses.iter().any(|a| a.performance_score < 100);
        if has_security_issues {
            suggestions
                .push("Use parameterized queries to prevent SQL injection".to_string());
            suggestions.push("Avoid string concatenation in SQL queries".to_string());
            suggestions
                .push(
                    "Use prepared statements with proper parameter binding".to_string(),
                );
        }
        if has_performance_issues {
            suggestions
                .push(
                    "Specify columns explicitly instead of using SELECT *".to_string(),
                );
            suggestions
                .push(
                    "Add appropriate indexes for frequently queried columns".to_string(),
                );
            suggestions
                .push(
                    "Avoid Cartesian products by using proper JOIN conditions"
                        .to_string(),
                );
            suggestions
                .push("Consider query optimization and EXPLAIN plans".to_string());
        }
        suggestions.push("Use database migrations for schema changes".to_string());
        suggestions
            .push("Implement proper error handling for database operations".to_string());
        suggestions
            .push("Add database connection pooling for better performance".to_string());
        suggestions.push("Use transactions for multi-statement operations".to_string());
        suggestions
    }
    fn display_report(
        &self,
        report: &SqlAnalysisReport,
        output_format: OutputFormat,
        verbose: bool,
    ) {
        match output_format {
            OutputFormat::Human => {
                println!(
                    "\n🔍 {} - SQL Macro Analysis Report", "CargoMate SqlMacroCheck"
                    .bold().blue()
                );
                println!("{}", "═".repeat(60).blue());
                println!("\n📊 Summary:");
                println!("  • Files Analyzed: {}", report.files_analyzed);
                println!("  • SQL Queries Found: {}", report.sql_queries_found);
                println!("  • Security Issues: {}", report.security_issues.len());
                println!(
                    "  • Performance Issues: {}", report.performance_issues.len()
                );
                println!("  • Syntax Errors: {}", report.syntax_errors.len());
                if !report.macros_analyzed.is_empty() && verbose {
                    println!("\n🔧 SQL Macros Analyzed:");
                    for analysis in &report.macros_analyzed {
                        let security_icon = if analysis.security_score >= 80 {
                            "🛡️"
                        } else {
                            "⚠️"
                        };
                        let performance_icon = if analysis.performance_score >= 80 {
                            "🚀"
                        } else {
                            "🐌"
                        };
                        println!(
                            "  {} {} - {} (Security: {}, Performance: {})", analysis
                            .macro_name, analysis.file_path.split('/').last().unwrap_or(&
                            analysis.file_path), format!("{}:{}", analysis.line_number,
                            analysis.sql_query.chars().take(50).collect::< String > ()),
                            analysis.security_score, analysis.performance_score
                        );
                        if verbose && !analysis.issues.is_empty() {
                            for issue in &analysis.issues {
                                println!("    • {}", issue.yellow());
                            }
                        }
                    }
                }
                if !report.security_issues.is_empty() {
                    println!("\n🔒 Security Issues:");
                    for issue in &report.security_issues {
                        let severity_icon = match issue.severity.as_str() {
                            "critical" => "🚨",
                            "high" => "❌",
                            "medium" => "⚠️",
                            "low" => "ℹ️",
                            _ => "•",
                        };
                        println!(
                            "  {} {}:{} - {}", severity_icon, issue.file_path.split('/')
                            .last().unwrap_or(& issue.file_path), issue.line_number,
                            issue.description
                        );
                        if verbose {
                            println!("    SQL: {}", issue.sql_snippet.dimmed());
                            println!("    💡 {}", issue.suggestion);
                        }
                    }
                }
                if !report.performance_issues.is_empty() {
                    println!("\n⚡ Performance Issues:");
                    for issue in &report.performance_issues {
                        println!(
                            "  🐌 {}:{} - {}", issue.file_path.split('/').last()
                            .unwrap_or(& issue.file_path), issue.line_number, issue
                            .description
                        );
                        if verbose {
                            println!("    SQL: {}", issue.sql_snippet.dimmed());
                            println!("    💡 {}", issue.suggestion);
                        }
                    }
                }
                if !report.syntax_errors.is_empty() {
                    println!("\n❌ Syntax Errors:");
                    for error in &report.syntax_errors {
                        println!(
                            "  🔴 {}:{} - {}", error.file_path.split('/').last()
                            .unwrap_or(& error.file_path), error.line_number, error
                            .description
                        );
                        if verbose {
                            println!("    SQL: {}", error.sql_snippet.dimmed());
                        }
                    }
                }
                if !report.suggestions.is_empty() {
                    println!("\n💡 Suggestions:");
                    for suggestion in &report.suggestions {
                        println!("  • {}", suggestion.cyan());
                    }
                }
                println!("\n✅ Analysis complete!");
                let total_issues = report.security_issues.len()
                    + report.performance_issues.len() + report.syntax_errors.len();
                if total_issues == 0 {
                    println!("   All SQL queries look good!");
                } else {
                    println!("   Found {} issue(s) to address", total_issues);
                }
            }
            OutputFormat::Json => {
                let json = serde_json::to_string_pretty(report)
                    .unwrap_or_else(|_| "{}".to_string());
                println!("{}", json);
            }
            OutputFormat::Table => {
                println!(
                    "{:<30} {:<15} {:<15} {:<15} {:<10}", "File", "Queries", "Security",
                    "Performance", "Syntax"
                );
                println!("{}", "─".repeat(90));
                let mut file_stats = HashMap::new();
                for analysis in &report.macros_analyzed {
                    let entry = file_stats
                        .entry(&analysis.file_path)
                        .or_insert((0, 0, 0, 0));
                    entry.0 += 1;
                }
                for issue in &report.security_issues {
                    if let Some(entry) = file_stats.get_mut(&issue.file_path) {
                        entry.1 += 1;
                    }
                }
                for issue in &report.performance_issues {
                    if let Some(entry) = file_stats.get_mut(&issue.file_path) {
                        entry.2 += 1;
                    }
                }
                for error in &report.syntax_errors {
                    if let Some(entry) = file_stats.get_mut(&error.file_path) {
                        entry.3 += 1;
                    }
                }
                for (file_path, (queries, security, performance, syntax)) in file_stats {
                    let file_name = file_path.split('/').last().unwrap_or(&file_path);
                    println!(
                        "{:<30} {:<15} {:<15} {:<15} {:<10}", file_name, queries
                        .to_string(), security.to_string(), performance.to_string(),
                        syntax.to_string()
                    );
                }
            }
        }
    }
}
impl Tool for SqlMacroCheckTool {
    fn name(&self) -> &'static str {
        "sql-macro-check"
    }
    fn description(&self) -> &'static str {
        "Compile-time SQL query validation"
    }
    fn command(&self) -> Command {
        Command::new(self.name())
            .about(self.description())
            .long_about(
                "Analyze SQL queries in Rust code for security vulnerabilities, \
                        performance issues, and syntax errors. Supports multiple SQL libraries.

EXAMPLES:
    cm tool sql-macro-check --input src/
    cm tool sql-macro-check --workspace --security-only
    cm tool sql-macro-check --input src/db.rs --fix-suggestions",
            )
            .args(
                &[
                    Arg::new("input")
                        .long("input")
                        .short('i')
                        .help("Input directory or file to analyze")
                        .default_value("src/"),
                    Arg::new("workspace")
                        .long("workspace")
                        .help("Analyze all Rust files in workspace")
                        .action(clap::ArgAction::SetTrue),
                    Arg::new("security-only")
                        .long("security-only")
                        .help("Only check for security issues")
                        .action(clap::ArgAction::SetTrue),
                    Arg::new("performance-only")
                        .long("performance-only")
                        .help("Only check for performance issues")
                        .action(clap::ArgAction::SetTrue),
                    Arg::new("syntax-only")
                        .long("syntax-only")
                        .help("Only check for syntax errors")
                        .action(clap::ArgAction::SetTrue),
                    Arg::new("fix-suggestions")
                        .long("fix-suggestions")
                        .help("Show detailed fix suggestions")
                        .action(clap::ArgAction::SetTrue),
                    Arg::new("library")
                        .long("library")
                        .short('l')
                        .help("SQL library to check")
                        .default_value("auto")
                        .value_parser(["auto", "sqlx", "diesel", "sea-orm", "rusqlite"]),
                ],
            )
            .args(&common_options())
    }
    fn execute(&self, matches: &ArgMatches) -> Result<()> {
        let input = matches.get_one::<String>("input").unwrap();
        let workspace = matches.get_flag("workspace");
        let security_only = matches.get_flag("security-only");
        let performance_only = matches.get_flag("performance-only");
        let syntax_only = matches.get_flag("syntax-only");
        let fix_suggestions = matches.get_flag("fix-suggestions");
        let library = matches.get_one::<String>("library").unwrap();
        let output_format = parse_output_format(matches);
        let verbose = matches.get_flag("verbose");
        println!(
            "🔍 {} - Analyzing SQL Macros", "CargoMate SqlMacroCheck".bold().blue()
        );
        let mut all_analyses = Vec::new();
        let mut security_issues = Vec::new();
        let mut performance_issues = Vec::new();
        let mut syntax_errors = Vec::new();
        let files_to_analyze = if workspace {
            self.find_rust_files(".")?
        } else if Path::new(input).is_file() {
            vec![input.clone()]
        } else {
            self.find_rust_files(input)?
        };
        if files_to_analyze.is_empty() {
            println!("{}", "No Rust files found to analyze".yellow());
            return Ok(());
        }
        for file_path in &files_to_analyze {
            match self.analyze_sql_macros(file_path) {
                Ok(analyses) => {
                    for analysis in analyses {
                        all_analyses.push(analysis.clone());
                        for issue in &analysis.issues {
                            if issue.contains("injection")
                                || issue.contains("parameterized")
                                || issue.contains("deprecated")
                            {
                                security_issues
                                    .push(SecurityIssue {
                                        file_path: analysis.file_path.clone(),
                                        line_number: analysis.line_number,
                                        issue_type: "security_vulnerability".to_string(),
                                        description: issue.clone(),
                                        severity: if issue.contains("injection") {
                                            "high".to_string()
                                        } else {
                                            "medium".to_string()
                                        },
                                        sql_snippet: analysis.sql_query.clone(),
                                        suggestion: self.generate_security_suggestion(issue),
                                    });
                            } else {
                                performance_issues
                                    .push(PerformanceIssue {
                                        file_path: analysis.file_path.clone(),
                                        line_number: analysis.line_number,
                                        issue_type: "performance_issue".to_string(),
                                        description: issue.clone(),
                                        sql_snippet: analysis.sql_query.clone(),
                                        suggestion: self.generate_performance_suggestion(issue),
                                    });
                            }
                        }
                        let syntax_issues = self.check_sql_syntax(&analysis.sql_query);
                        for syntax_error in syntax_issues {
                            syntax_errors
                                .push(SyntaxError {
                                    file_path: analysis.file_path.clone(),
                                    line_number: analysis.line_number,
                                    error_type: syntax_error.error_type,
                                    description: syntax_error.description,
                                    sql_snippet: syntax_error.sql_snippet,
                                });
                        }
                    }
                }
                Err(e) => {
                    println!("⚠️  Failed to analyze {}: {}", file_path, e);
                }
            }
        }
        if all_analyses.is_empty() {
            println!("{}", "No SQL macros found in the codebase".yellow());
            return Ok(());
        }
        let final_analyses = if security_only || performance_only || syntax_only {
            all_analyses
                .into_iter()
                .filter(|analysis| {
                    if security_only {
                        analysis.security_score < 100
                    } else if performance_only {
                        analysis.performance_score < 100
                    } else {
                        false
                    }
                })
                .collect()
        } else {
            all_analyses
        };
        let final_security_issues = if security_only || !performance_only && !syntax_only
        {
            security_issues
        } else {
            Vec::new()
        };
        let final_performance_issues = if performance_only
            || !security_only && !syntax_only
        {
            performance_issues
        } else {
            Vec::new()
        };
        let final_syntax_errors = if syntax_only || !security_only && !performance_only {
            syntax_errors
        } else {
            Vec::new()
        };
        let suggestions = self.generate_suggestions(&final_analyses);
        let report = SqlAnalysisReport {
            files_analyzed: files_to_analyze.len(),
            sql_queries_found: final_analyses.len(),
            macros_analyzed: final_analyses,
            security_issues: final_security_issues,
            performance_issues: final_performance_issues,
            syntax_errors: final_syntax_errors,
            suggestions,
            timestamp: chrono::Utc::now().to_rfc3339(),
        };
        self.display_report(&report, output_format, verbose);
        Ok(())
    }
}
impl SqlMacroCheckTool {
    fn generate_security_suggestion(&self, issue: &str) -> String {
        if issue.contains("injection") {
            "Use parameterized queries with bound parameters instead of string concatenation"
                .to_string()
        } else if issue.contains("parameterized") {
            "Replace string literals with parameter placeholders (e.g., $1, ?, :param)"
                .to_string()
        } else if issue.contains("deprecated") {
            "Update to use current SQL features and avoid deprecated functions"
                .to_string()
        } else {
            "Review query for potential security vulnerabilities".to_string()
        }
    }
    fn generate_performance_suggestion(&self, issue: &str) -> String {
        if issue.contains("SELECT *") {
            "Specify column names explicitly to reduce data transfer and improve query performance"
                .to_string()
        } else if issue.contains("indexes") {
            "Add database indexes on frequently queried columns".to_string()
        } else if issue.contains("Cartesian") {
            "Add proper JOIN conditions to avoid Cartesian products".to_string()
        } else if issue.contains("inefficient") {
            "Consider using more efficient functions or query patterns".to_string()
        } else {
            "Review query execution plan and consider optimization".to_string()
        }
    }
}
impl Default for SqlMacroCheckTool {
    fn default() -> Self {
        Self::new()
    }
}