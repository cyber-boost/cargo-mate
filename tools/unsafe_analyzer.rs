use super::{Tool, Result, ToolError, common_options, parse_output_format, OutputFormat};
use clap::{Arg, ArgMatches, Command};
use colored::*;
use std::path::Path;
use std::fs;
use std::collections::HashMap;
use syn::{parse_file, Item, ItemFn, FnArg, Pat, Type, visit::Visit, visit};
use quote::ToTokens;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct UnsafeAnalyzerTool;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct UnsafeAnalysisReport {
    files_analyzed: usize,
    functions_with_unsafe: usize,
    total_unsafe_blocks: usize,
    unsafe_usage: Vec<UnsafeUsage>,
    risk_assessment: RiskAssessment,
    recommendations: Vec<String>,
    statistics: UnsafeStatistics,
    timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct UnsafeUsage {
    file_path: String,
    function_name: String,
    line_number: usize,
    unsafe_type: String,
    context: String,
    risk_level: String,
    justification_required: bool,
    suggestion: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RiskAssessment {
    overall_risk: String,
    high_risk_functions: usize,
    medium_risk_functions: usize,
    low_risk_functions: usize,
    critical_functions: usize,
    risk_score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct UnsafeStatistics {
    total_functions: usize,
    functions_with_unsafe: usize,
    unsafe_blocks: usize,
    unsafe_static_access: usize,
    unsafe_pointer_ops: usize,
    unsafe_ffi_calls: usize,
    unsafe_transmutes: usize,
}

struct UnsafeVisitor {
    unsafe_usages: Vec<UnsafeUsage>,
    current_function: Option<String>,
    current_file: String,
    in_unsafe_block: bool,
    in_unsafe_fn: bool,
}

impl UnsafeVisitor {
    fn new(file_path: String) -> Self {
        Self {
            unsafe_usages: Vec::new(),
            current_function: None,
            current_file: file_path,
            in_unsafe_block: false,
            in_unsafe_fn: false,
        }
    }

    fn add_unsafe_usage(&mut self, unsafe_type: String, context: String, line: usize) {
        let function_name = self.current_function.clone().unwrap_or("global".to_string());

        let (risk_level, justification_required) = self.assess_risk(&unsafe_type, &context);
        let suggestion = self.generate_suggestion(&unsafe_type, &context);

        self.unsafe_usages.push(UnsafeUsage {
            file_path: self.current_file.clone(),
            function_name,
            line_number: line,
            unsafe_type,
            context,
            risk_level,
            justification_required,
            suggestion,
        });
    }

    fn assess_risk(&self, unsafe_type: &str, context: &str) -> (String, bool) {
        match unsafe_type {
            "raw_pointer_dereference" => {
                if context.contains("null") || context.contains("uninitialized") {
                    ("critical".to_string(), true)
                } else {
                    ("high".to_string(), true)
                }
            }
            "static_mut_access" => ("high".to_string(), true),
            "ffi_call" => {
                if context.contains("extern") && context.contains("fn") {
                    ("medium".to_string(), true)
                } else {
                    ("low".to_string(), false)
                }
            }
            "transmute" => ("high".to_string(), true),
            "union_access" => ("medium".to_string(), true),
            "unsafe_trait_impl" => ("medium".to_string(), true),
            "inline_assembly" => ("critical".to_string(), true),
            "raw_pointer_arithmetic" => ("high".to_string(), true),
            _ => ("low".to_string(), false),
        }
    }

    fn generate_suggestion(&self, unsafe_type: &str, context: &str) -> String {
        match unsafe_type {
            "raw_pointer_dereference" => {
                "Use safe alternatives like Box, Rc, Arc, or check for null pointers before dereferencing".to_string()
            }
            "static_mut_access" => {
                "Use thread-safe alternatives like Mutex, RwLock, or atomic operations".to_string()
            }
            "ffi_call" => {
                "Ensure proper error handling and memory management when interfacing with C code".to_string()
            }
            "transmute" => {
                "Use safe casting methods or ensure type compatibility before transmuting".to_string()
            }
            "union_access" => {
                "Use enums with proper discriminant checking instead of unions".to_string()
            }
            "unsafe_trait_impl" => {
                "Document safety requirements and provide safe wrapper methods".to_string()
            }
            "inline_assembly" => {
                "Minimize assembly usage and thoroughly document safety invariants".to_string()
            }
            "raw_pointer_arithmetic" => {
                "Use safe collection types and bounds checking".to_string()
            }
            _ => "Review unsafe usage and ensure all safety invariants are maintained".to_string(),
        }
    }
}

impl<'ast> Visit<'ast> for UnsafeVisitor {
    fn visit_item_fn(&mut self, node: &'ast syn::ItemFn) {
        let previous_function = self.current_function.clone();
        self.current_function = Some(node.sig.ident.to_string());
        self.in_unsafe_fn = node.sig.unsafety.is_some();

        if self.in_unsafe_fn {
            let context = format!("fn {}() -> {}", node.sig.ident, node.sig.output.to_token_stream());
            self.add_unsafe_usage(
                "unsafe_function".to_string(),
                context,
                0, // We'll need to track line numbers better
            );
        }

        visit::visit_item_fn(self, node);
        self.current_function = previous_function;
        self.in_unsafe_fn = false;
    }

    fn visit_expr_unsafe(&mut self, node: &'ast syn::ExprUnsafe) {
        let previous_unsafe_block = self.in_unsafe_block;
        self.in_unsafe_block = true;

        let context = node.block.to_token_stream().to_string();
        self.add_unsafe_usage(
            "unsafe_block".to_string(),
            context,
            0,
        );

        visit::visit_expr_unsafe(self, node);
        self.in_unsafe_block = previous_unsafe_block;
    }

    fn visit_expr_call(&mut self, node: &'ast syn::ExprCall) {
        if self.in_unsafe_block || self.in_unsafe_fn {
            let call_str = node.to_token_stream().to_string();

            // Check for FFI calls
            if call_str.contains("extern") || call_str.contains("::c_") {
                self.add_unsafe_usage(
                    "ffi_call".to_string(),
                    call_str,
                    0,
                );
            }
        }

        visit::visit_expr_call(self, node);
    }

    fn visit_expr_unary(&mut self, node: &'ast syn::ExprUnary) {
        if self.in_unsafe_block || self.in_unsafe_fn {
            let unary_str = node.to_token_stream().to_string();

            // Check for raw pointer operations
            if unary_str.contains('*') && (unary_str.contains("const") || unary_str.contains("mut")) {
                self.add_unsafe_usage(
                    "raw_pointer_dereference".to_string(),
                    unary_str,
                    0,
                );
            }
        }

        visit::visit_expr_unary(self, node);
    }

    fn visit_expr_path(&mut self, node: &'ast syn::ExprPath) {
        if self.in_unsafe_block || self.in_unsafe_fn {
            let path_str = node.to_token_stream().to_string();

            // Check for static mut access
            if path_str.contains("static") && path_str.contains("mut") {
                self.add_unsafe_usage(
                    "static_mut_access".to_string(),
                    path_str,
                    0,
                );
            }
        }

        visit::visit_expr_path(self, node);
    }
}

impl UnsafeAnalyzerTool {
    pub fn new() -> Self {
        Self
    }

    fn find_rust_files(&self, directory: &str) -> Result<Vec<String>> {
        let mut files = Vec::new();
        self.find_rust_files_recursive(directory, &mut files)?;
        Ok(files)
    }

    fn find_rust_files_recursive(&self, dir: &str, files: &mut Vec<String>) -> Result<()> {
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

    fn analyze_file(&self, file_path: &str) -> Result<Vec<UnsafeUsage>> {
        let content = fs::read_to_string(file_path)?;

        // Use regex to find unsafe patterns since syn visitor is complex
        let mut usages = Vec::new();

        // Find unsafe functions
        let unsafe_fn_pattern = regex::Regex::new(r"unsafe\s+fn\s+(\w+)").unwrap();
        for captures in unsafe_fn_pattern.captures_iter(&content) {
            if let Some(fn_name) = captures.get(1) {
                usages.push(UnsafeUsage {
                    file_path: file_path.to_string(),
                    function_name: fn_name.as_str().to_string(),
                    line_number: 0, // Would need line number calculation
                    unsafe_type: "unsafe_function".to_string(),
                    context: format!("unsafe fn {}", fn_name.as_str()),
                    risk_level: "medium".to_string(),
                    justification_required: true,
                    suggestion: "Document safety requirements and provide safe wrapper functions".to_string(),
                });
            }
        }

        // Find unsafe blocks
        let unsafe_block_pattern = regex::Regex::new(r"unsafe\s*\{([^}]*)\}").unwrap();
        for captures in unsafe_block_pattern.captures_iter(&content) {
            if let Some(block_content) = captures.get(1) {
                let block_text = block_content.as_str();

                // Analyze what's inside the unsafe block
                if block_text.contains("transmute") {
                    usages.push(UnsafeUsage {
                        file_path: file_path.to_string(),
                        function_name: "unknown".to_string(),
                        line_number: 0,
                        unsafe_type: "transmute".to_string(),
                        context: block_text.to_string(),
                        risk_level: "high".to_string(),
                        justification_required: true,
                        suggestion: "Use safe casting methods or ensure type compatibility".to_string(),
                    });
                }

                if block_text.contains("*const") || block_text.contains("*mut") {
                    usages.push(UnsafeUsage {
                        file_path: file_path.to_string(),
                        function_name: "unknown".to_string(),
                        line_number: 0,
                        unsafe_type: "raw_pointer_dereference".to_string(),
                        context: block_text.to_string(),
                        risk_level: "high".to_string(),
                        justification_required: true,
                        suggestion: "Use safe alternatives like Box, Rc, Arc".to_string(),
                    });
                }

                if block_text.contains("static mut") {
                    usages.push(UnsafeUsage {
                        file_path: file_path.to_string(),
                        function_name: "unknown".to_string(),
                        line_number: 0,
                        unsafe_type: "static_mut_access".to_string(),
                        context: block_text.to_string(),
                        risk_level: "high".to_string(),
                        justification_required: true,
                        suggestion: "Use thread-safe alternatives like Mutex, RwLock".to_string(),
                    });
                }

                if block_text.contains("extern") && block_text.contains("fn") {
                    usages.push(UnsafeUsage {
                        file_path: file_path.to_string(),
                        function_name: "unknown".to_string(),
                        line_number: 0,
                        unsafe_type: "ffi_call".to_string(),
                        context: block_text.to_string(),
                        risk_level: "medium".to_string(),
                        justification_required: true,
                        suggestion: "Ensure proper error handling and memory management".to_string(),
                    });
                }
            }
        }

        Ok(usages)
    }

    fn calculate_statistics(&self, usages: &[UnsafeUsage]) -> UnsafeStatistics {
        let mut stats = UnsafeStatistics {
            total_functions: 0,
            functions_with_unsafe: 0,
            unsafe_blocks: 0,
            unsafe_static_access: 0,
            unsafe_pointer_ops: 0,
            unsafe_ffi_calls: 0,
            unsafe_transmutes: 0,
        };

        let mut functions_with_unsafe = std::collections::HashSet::new();

        for usage in usages {
            match usage.unsafe_type.as_str() {
                "unsafe_function" => {
                    stats.functions_with_unsafe += 1;
                    functions_with_unsafe.insert(&usage.function_name);
                }
                "unsafe_block" => {
                    stats.unsafe_blocks += 1;
                }
                "static_mut_access" => {
                    stats.unsafe_static_access += 1;
                }
                "raw_pointer_dereference" => {
                    stats.unsafe_pointer_ops += 1;
                }
                "ffi_call" => {
                    stats.unsafe_ffi_calls += 1;
                }
                "transmute" => {
                    stats.unsafe_transmutes += 1;
                }
                _ => {}
            }
        }

        stats.functions_with_unsafe = functions_with_unsafe.len();
        stats
    }

    fn assess_risk(&self, usages: &[UnsafeUsage]) -> RiskAssessment {
        let critical_count = usages.iter().filter(|u| u.risk_level == "critical").count();
        let high_count = usages.iter().filter(|u| u.risk_level == "high").count();
        let medium_count = usages.iter().filter(|u| u.risk_level == "medium").count();
        let low_count = usages.iter().filter(|u| u.risk_level == "low").count();

        let total_risky = critical_count + high_count + medium_count;
        let risk_score = if usages.is_empty() {
            0.0
        } else {
            (critical_count * 10 + high_count * 7 + medium_count * 4 + low_count * 1) as f64 / usages.len() as f64
        };

        let overall_risk = if risk_score >= 8.0 || critical_count > 0 {
            "critical"
        } else if risk_score >= 6.0 || high_count > 0 {
            "high"
        } else if risk_score >= 3.0 || medium_count > 0 {
            "medium"
        } else {
            "low"
        };

        RiskAssessment {
            overall_risk: overall_risk.to_string(),
            high_risk_functions: high_count,
            medium_risk_functions: medium_count,
            low_risk_functions: low_count,
            critical_functions: critical_count,
            risk_score,
        }
    }

    fn generate_recommendations(&self, usages: &[UnsafeUsage], risk: &RiskAssessment) -> Vec<String> {
        let mut recommendations = Vec::new();

        if risk.overall_risk == "critical" {
            recommendations.push("🚨 CRITICAL: Review all unsafe code immediately - high risk of memory safety issues".to_string());
        }

        if usages.iter().any(|u| u.unsafe_type == "raw_pointer_dereference") {
            recommendations.push("Replace raw pointer operations with safe alternatives (Box, Rc, Arc, RefCell)".to_string());
        }

        if usages.iter().any(|u| u.unsafe_type == "static_mut_access") {
            recommendations.push("Replace static mut access with thread-safe alternatives (Mutex, RwLock, atomic types)".to_string());
        }

        if usages.iter().any(|u| u.unsafe_type == "ffi_call") {
            recommendations.push("Add comprehensive error handling and memory management for FFI calls".to_string());
        }

        if usages.iter().any(|u| u.unsafe_type == "transmute") {
            recommendations.push("Avoid transmute where possible, use safe casting methods".to_string());
        }

        // General recommendations
        recommendations.push("Document all unsafe blocks with safety comments explaining invariants".to_string());
        recommendations.push("Create safe wrapper functions around unsafe operations".to_string());
        recommendations.push("Use tools like Miri to test unsafe code under different scenarios".to_string());
        recommendations.push("Consider gradual migration to safe alternatives where possible".to_string());
        recommendations.push("Run regular code reviews focusing on unsafe code blocks".to_string());

        recommendations
    }

    fn display_report(&self, report: &UnsafeAnalysisReport, output_format: OutputFormat, verbose: bool) {
        match output_format {
            OutputFormat::Human => {
                println!("\n🔍 {} - Unsafe Code Analysis Report", "CargoMate UnsafeAnalyzer".bold().blue());
                println!("{}", "═".repeat(65).blue());

                println!("\n📊 Summary:");
                println!("  • Files Analyzed: {}", report.files_analyzed);
                println!("  • Functions with Unsafe: {}", report.functions_with_unsafe);
                println!("  • Total Unsafe Blocks: {}", report.total_unsafe_blocks);
                println!("  • Overall Risk Level: {}", report.risk_assessment.overall_risk.red());
                println!("  • Risk Score: {:.1}/10", report.risk_assessment.risk_score);

                println!("\n📈 Risk Assessment:");
                println!("  • Critical Risk Functions: {}", report.risk_assessment.critical_functions);
                println!("  • High Risk Functions: {}", report.risk_assessment.high_risk_functions);
                println!("  • Medium Risk Functions: {}", report.risk_assessment.medium_risk_functions);
                println!("  • Low Risk Functions: {}", report.risk_assessment.low_risk_functions);

                println!("\n📋 Unsafe Statistics:");
                println!("  • Functions with Unsafe: {}", report.statistics.functions_with_unsafe);
                println!("  • Unsafe Blocks: {}", report.statistics.unsafe_blocks);
                println!("  • Static Mut Access: {}", report.statistics.unsafe_static_access);
                println!("  • Pointer Operations: {}", report.statistics.unsafe_pointer_ops);
                println!("  • FFI Calls: {}", report.statistics.unsafe_ffi_calls);
                println!("  • Transmutes: {}", report.statistics.unsafe_transmutes);

                if !report.unsafe_usage.is_empty() {
                    println!("\n⚠️  Unsafe Usage Details:");
                    for usage in &report.unsafe_usage {
                        let risk_icon = match usage.risk_level.as_str() {
                            "critical" => "🚨",
                            "high" => "❌",
                            "medium" => "⚠️",
                            "low" => "ℹ️",
                            _ => "•",
                        };

                        println!("  {} {}::{} - {} (Line {})",
                                risk_icon,
                                usage.file_path.split('/').last().unwrap_or(&usage.file_path),
                                usage.function_name,
                                usage.unsafe_type.red(),
                                usage.line_number);

                        if verbose {
                            println!("    📝 Context: {}", usage.context.dimmed());
                            if usage.justification_required {
                                println!("    ⚠️  Justification Required: {}", "Yes".red());
                            }
                            println!("    💡 Suggestion: {}", usage.suggestion.cyan());
                        }
                    }
                }

                if !report.recommendations.is_empty() {
                    println!("\n💡 Recommendations:");
                    for rec in &report.recommendations {
                        println!("  • {}", rec.cyan());
                    }
                }

                println!("\n✅ Analysis complete!");
                if report.unsafe_usage.is_empty() {
                    println!("   No unsafe code found - excellent!");
                } else {
                    println!("   Found {} unsafe usage(s) to review", report.unsafe_usage.len());
                }
            }
            OutputFormat::Json => {
                let json = serde_json::to_string_pretty(report)
                    .unwrap_or_else(|_| "{}".to_string());
                println!("{}", json);
            }
            OutputFormat::Table => {
                println!("{:<30} {:<20} {:<15} {:<12} {:<15}",
                        "File", "Function", "Type", "Risk", "Justification");
                println!("{}", "─".repeat(100));

                for usage in &report.unsafe_usage {
                    let file_name = usage.file_path.split('/').last().unwrap_or(&usage.file_path);
                    let justification = if usage.justification_required { "Required" } else { "Optional" };
                    println!("{:<30} {:<20} {:<15} {:<12} {:<15}",
                            file_name.chars().take(29).collect::<String>(),
                            usage.function_name.chars().take(19).collect::<String>(),
                            usage.unsafe_type.chars().take(14).collect::<String>(),
                            usage.risk_level,
                            justification);
                }
            }
        }
    }
}

impl Tool for UnsafeAnalyzerTool {
    fn name(&self) -> &'static str {
        "unsafe-analyzer"
    }

    fn description(&self) -> &'static str {
        "Detailed analysis of unsafe code usage"
    }

    fn command(&self) -> Command {
        Command::new(self.name())
            .about(self.description())
            .long_about("Analyze unsafe code usage in Rust projects, identifying potential \
                        memory safety issues, performance concerns, and areas for improvement.

EXAMPLES:
    cm tool unsafe-analyzer --input src/
    cm tool unsafe-analyzer --workspace --risk-threshold high
    cm tool unsafe-analyzer --input src/lib.rs --focus-functions")
            .args(&[
                Arg::new("input")
                    .long("input")
                    .short('i')
                    .help("Input directory or file to analyze")
                    .default_value("src/"),
                Arg::new("workspace")
                    .long("workspace")
                    .help("Analyze all Rust files in workspace")
                    .action(clap::ArgAction::SetTrue),
                Arg::new("risk-threshold")
                    .long("risk-threshold")
                    .help("Minimum risk level to report")
                    .default_value("low")
                    .value_parser(["low", "medium", "high", "critical"]),
                Arg::new("focus-functions")
                    .long("focus-functions")
                    .help("Only analyze functions, ignore other unsafe usage")
                    .action(clap::ArgAction::SetTrue),
                Arg::new("ignore-test-files")
                    .long("ignore-test-files")
                    .help("Skip analysis of test files")
                    .action(clap::ArgAction::SetTrue),
                Arg::new("show-context")
                    .long("show-context")
                    .help("Show full context for unsafe blocks")
                    .action(clap::ArgAction::SetTrue),
                Arg::new("export-sarif")
                    .long("export-sarif")
                    .help("Export results in SARIF format for CI")
                    .action(clap::ArgAction::SetTrue),
            ])
            .args(&common_options())
    }

    fn execute(&self, matches: &ArgMatches) -> Result<()> {
        let input = matches.get_one::<String>("input").unwrap();
        let workspace = matches.get_flag("workspace");
        let risk_threshold = matches.get_one::<String>("risk_threshold").unwrap();
        let focus_functions = matches.get_flag("focus-functions");
        let ignore_test_files = matches.get_flag("ignore-test-files");
        let show_context = matches.get_flag("show-context");
        let export_sarif = matches.get_flag("export-sarif");
        let output_format = parse_output_format(matches);
        let verbose = matches.get_flag("verbose");

        println!("🔍 {} - Analyzing Unsafe Code", "CargoMate UnsafeAnalyzer".bold().blue());

        let files_to_analyze = if workspace {
            self.find_rust_files(".")?
        } else if Path::new(input).is_file() {
            vec![input.clone()]
        } else {
            self.find_rust_files(input)?
        };

        let filtered_files = if ignore_test_files {
            files_to_analyze.into_iter()
                .filter(|file| !file.contains("test") && !file.contains("/tests/"))
                .collect()
        } else {
            files_to_analyze
        };

        if filtered_files.is_empty() {
            println!("{}", "No Rust files found to analyze".yellow());
            return Ok(());
        }

        let mut all_usages = Vec::new();

        for file_path in &filtered_files {
            match self.analyze_file(file_path) {
                Ok(usages) => {
                    for usage in usages {
                        // Filter by risk threshold
                        let include_usage = match risk_threshold.as_str() {
                            "critical" => usage.risk_level == "critical",
                            "high" => usage.risk_level == "critical" || usage.risk_level == "high",
                            "medium" => usage.risk_level == "critical" || usage.risk_level == "high" || usage.risk_level == "medium",
                            "low" => true,
                            _ => true,
                        };

                        // Filter by focus if specified
                        let include_by_focus = if focus_functions {
                            usage.unsafe_type == "unsafe_function"
                        } else {
                            true
                        };

                        if include_usage && include_by_focus {
                            all_usages.push(usage);
                        }
                    }
                }
                Err(e) => {
                    if verbose {
                        println!("⚠️  Failed to analyze {}: {}", file_path, e);
                    }
                }
            }
        }

        // Calculate statistics
        let statistics = self.calculate_statistics(&all_usages);

        // Assess risk
        let risk_assessment = self.assess_risk(&all_usages);

        // Generate recommendations
        let recommendations = self.generate_recommendations(&all_usages, &risk_assessment);

        // Create report
        let report = UnsafeAnalysisReport {
            files_analyzed: filtered_files.len(),
            functions_with_unsafe: statistics.functions_with_unsafe,
            total_unsafe_blocks: statistics.unsafe_blocks,
            unsafe_usage: all_usages,
            risk_assessment,
            recommendations,
            statistics,
            timestamp: chrono::Utc::now().to_rfc3339(),
        };

        // Export SARIF if requested
        if export_sarif {
            let sarif_report = self.generate_sarif_report(&report)?;
            fs::write("unsafe-analysis.sarif", sarif_report)?;
            println!("📄 SARIF report exported to unsafe-analysis.sarif");
        }

        // Display results
        self.display_report(&report, output_format, verbose);

        Ok(())
    }
}

impl UnsafeAnalyzerTool {
    fn generate_sarif_report(&self, report: &UnsafeAnalysisReport) -> Result<String> {
        let sarif = serde_json::json!({
            "version": "2.1.0",
            "$schema": "https://schemastore.azurewebsites.net/schemas/json/sarif-2.1.0.json",
            "runs": [{
                "tool": {
                    "driver": {
                        "name": "CargoMate UnsafeAnalyzer",
                        "version": "1.0.0",
                        "informationUri": "https://example.com",
                        "rules": [{
                            "id": "unsafe-usage",
                            "name": "UnsafeCodeUsage",
                            "shortDescription": {
                                "text": "Unsafe code usage detected"
                            },
                            "fullDescription": {
                                "text": "Usage of unsafe Rust code that may compromise memory safety"
                            },
                            "help": {
                                "text": "Review unsafe code usage and ensure all safety invariants are maintained"
                            },
                            "properties": {
                                "category": "security",
                                "impact": "high"
                            }
                        }]
                    }
                },
                "results": report.unsafe_usage.iter().map(|usage| {
                    serde_json::json!({
                        "ruleId": "unsafe-usage",
                        "level": match usage.risk_level.as_str() {
                            "critical" => "error",
                            "high" => "warning",
                            "medium" => "note",
                            _ => "note"
                        },
                        "message": {
                            "text": format!("{}: {}", usage.unsafe_type, usage.context)
                        },
                        "locations": [{
                            "physicalLocation": {
                                "artifactLocation": {
                                    "uri": usage.file_path
                                },
                                "region": {
                                    "startLine": usage.line_number
                                }
                            }
                        }],
                        "properties": {
                            "riskLevel": usage.risk_level,
                            "suggestion": usage.suggestion
                        }
                    })
                }).collect::<Vec<_>>()
            }]
        });

        Ok(serde_json::to_string_pretty(&sarif)?)
    }
}

impl Default for UnsafeAnalyzerTool {
    fn default() -> Self {
        Self::new()
    }
}
