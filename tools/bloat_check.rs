use super::{Tool, ToolError, Result, OutputFormat, parse_output_format};
use clap::{Arg, ArgMatches, Command};
use std::path::Path;
use std::process::Command as ProcessCommand;
use std::collections::HashMap;
use colored::*;
use serde::{Serialize, Deserialize};

/// Represents binary size information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinarySizeInfo {
    pub path: String,
    pub total_size: u64,
    pub text_size: u64,
    pub data_size: u64,
    pub bss_size: u64,
    pub symbol_count: usize,
}

/// Size comparison between two binaries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SizeComparison {
    pub current: BinarySizeInfo,
    pub baseline: BinarySizeInfo,
    pub size_diff: i64,
    pub text_diff: i64,
    pub data_diff: i64,
    pub bss_diff: i64,
}

/// Information about a symbol in the binary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolSize {
    pub name: String,
    pub size: u64,
    pub symbol_type: String,
}

/// Optimization suggestion based on analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationSuggestion {
    pub category: String,
    pub description: String,
    pub impact: String,
    pub suggestion: String,
}

/// Build comparison between debug and release
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildComparison {
    pub debug_size: u64,
    pub release_size: u64,
    pub ratio: f64,
    pub savings: u64,
}

pub struct BloatCheckTool;

impl BloatCheckTool {
    pub fn new() -> Self {
        Self
    }

    fn analyze_binary_size(&self, binary_path: &str) -> Result<BinarySizeInfo> {
        if !Path::new(binary_path).exists() {
            return Err(ToolError::InvalidArguments(format!("Binary not found: {}", binary_path)));
        }

        // Get basic file size
        let metadata = std::fs::metadata(binary_path)?;
        let total_size = metadata.len();

        // Try to get more detailed size information using `size` command
        let size_output = match ProcessCommand::new("size")
            .arg("-A")  // System V format
            .arg("-d")  // decimal output
            .arg(binary_path)
            .output()
        {
            Ok(output) if output.status.success() => {
                String::from_utf8_lossy(&output.stdout).to_string()
            }
            _ => {
                // Fallback: try `size -B` (Berkeley format)
                match ProcessCommand::new("size")
                    .arg("-B")
                    .arg(binary_path)
                    .output()
                {
                    Ok(output) if output.status.success() => {
                        String::from_utf8_lossy(&output.stdout).to_string()
                    }
                    _ => {
                        // Last fallback: just use file size
                        format!("{} {} {} {}", total_size, 0, 0, 0)
                    }
                }
            }
        };

        let mut text_size = 0u64;
        let mut data_size = 0u64;
        let mut bss_size = 0u64;

        // Parse size output
        for line in size_output.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 4 {
                if let (Ok(t), Ok(d), Ok(b)) = (
                    parts[0].parse::<u64>(),
                    parts[1].parse::<u64>(),
                    parts[2].parse::<u64>(),
                ) {
                    text_size = t;
                    data_size = d;
                    bss_size = b;
                    break;
                }
            }
        }

        // Count symbols using nm
        let symbol_count = match ProcessCommand::new("nm")
            .arg("-C")  // demangle C++ symbols
            .arg("--print-size")
            .arg("--size-sort")
            .arg("-t")  // decimal output
            .arg("d")   // dynamic symbols
            .arg(binary_path)
            .output()
        {
            Ok(output) if output.status.success() => {
                String::from_utf8_lossy(&output.stdout)
                    .lines()
                    .filter(|line| !line.trim().is_empty())
                    .count()
            }
            _ => 0,
        };

        Ok(BinarySizeInfo {
            path: binary_path.to_string(),
            total_size,
            text_size,
            data_size,
            bss_size,
            symbol_count,
        })
    }

    fn analyze_size_changes(&self, current_path: &str, baseline_path: &str) -> Result<SizeComparison> {
        let current = self.analyze_binary_size(current_path)?;
        let baseline = self.analyze_binary_size(baseline_path)?;

        // Calculate differences before moving the values
        let size_diff = current.total_size as i64 - baseline.total_size as i64;
        let text_diff = current.text_size as i64 - baseline.text_size as i64;
        let data_diff = current.data_size as i64 - baseline.data_size as i64;
        let bss_diff = current.bss_size as i64 - baseline.bss_size as i64;

        Ok(SizeComparison {
            current,
            baseline,
            size_diff,
            text_diff,
            data_diff,
            bss_diff,
        })
    }

    fn find_largest_symbols(&self, binary_path: &str) -> Result<Vec<SymbolSize>> {
        let output = ProcessCommand::new("nm")
            .arg("-C")  // demangle C++ symbols
            .arg("--print-size")
            .arg("--size-sort")
            .arg("-r")  // reverse sort (largest first)
            .arg("-t")  // decimal output
            .arg("d")   // dynamic symbols
            .arg(binary_path)
            .output()
            .map_err(|e| ToolError::ExecutionFailed(format!("nm command failed: {}", e)))?;

        if !output.status.success() {
            return Err(ToolError::ExecutionFailed(
                String::from_utf8_lossy(&output.stderr).to_string()
            ));
        }

        let mut symbols = Vec::new();

        for line in String::from_utf8_lossy(&output.stdout).lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 3 {
                if let Ok(size) = parts[0].parse::<u64>() {
                    let symbol_type = parts[1].to_string();
                    let name = parts[2..].join(" ");

                    symbols.push(SymbolSize {
                        name,
                        size,
                        symbol_type,
                    });

                    // Limit to top 20 symbols to avoid too much output
                    if symbols.len() >= 20 {
                        break;
                    }
                }
            }
        }

        Ok(symbols)
    }

    fn generate_optimization_suggestions(&self, analysis: &BinarySizeInfo) -> Vec<OptimizationSuggestion> {
        let mut suggestions = Vec::new();

        // Size-based suggestions
        if analysis.total_size > 50 * 1024 * 1024 { // > 50MB
            suggestions.push(OptimizationSuggestion {
                category: "Binary Size".to_string(),
                description: "Large binary detected".to_string(),
                impact: "High".to_string(),
                suggestion: "Consider enabling link-time optimization (LTO) in release builds".to_string(),
            });
        }

        if analysis.text_size > 20 * 1024 * 1024 { // > 20MB text section
            suggestions.push(OptimizationSuggestion {
                category: "Code Size".to_string(),
                description: "Large text section".to_string(),
                impact: "Medium".to_string(),
                suggestion: "Review inlining decisions and consider #[inline(never)] for large functions".to_string(),
            });
        }

        if analysis.data_size > 10 * 1024 * 1024 { // > 10MB data section
            suggestions.push(OptimizationSuggestion {
                category: "Data Size".to_string(),
                description: "Large data section".to_string(),
                impact: "Medium".to_string(),
                suggestion: "Review static data usage and consider lazy initialization".to_string(),
            });
        }

        if analysis.bss_size > 5 * 1024 * 1024 { // > 5MB BSS
            suggestions.push(OptimizationSuggestion {
                category: "Memory Usage".to_string(),
                description: "Large uninitialized data section".to_string(),
                impact: "Low".to_string(),
                suggestion: "Review large static arrays and consider dynamic allocation".to_string(),
            });
        }

        // General optimization suggestions
        suggestions.push(OptimizationSuggestion {
            category: "Build Optimization".to_string(),
            description: "General size optimizations".to_string(),
            impact: "Low".to_string(),
            suggestion: "Use cargo build --release with strip = true in Cargo.toml".to_string(),
        });

        suggestions.push(OptimizationSuggestion {
            category: "Dependency Analysis".to_string(),
            description: "Check for unused dependencies".to_string(),
            impact: "Medium".to_string(),
            suggestion: "Run cargo-udeps to find unused dependencies".to_string(),
        });

        suggestions
    }

    fn analyze_debug_vs_release(&self, debug_path: &str, release_path: &str) -> Result<BuildComparison> {
        let debug_info = self.analyze_binary_size(debug_path)?;
        let release_info = self.analyze_binary_size(release_path)?;

        let debug_size = debug_info.total_size;
        let release_size = release_info.total_size;
        let ratio = if release_size > 0 { debug_size as f64 / release_size as f64 } else { 1.0 };
        let savings = debug_size.saturating_sub(release_size);

        Ok(BuildComparison {
            debug_size,
            release_size,
            ratio,
            savings,
        })
    }

    fn format_size(&self, bytes: u64) -> String {
        const UNITS: &[&str] = &["B", "KB", "MB", "GB"];
        let mut size = bytes as f64;
        let mut unit_index = 0;

        while size >= 1024.0 && unit_index < UNITS.len() - 1 {
            size /= 1024.0;
            unit_index += 1;
        }

        format!("{:.1} {}", size, UNITS[unit_index])
    }

    fn format_diff(&self, diff: i64) -> String {
        if diff == 0 {
            "±0 B".to_string()
        } else if diff > 0 {
            format!("+{}", self.format_size(diff as u64))
        } else {
            format!("-{}", self.format_size((-diff) as u64))
        }
    }

    fn colorize_diff(&self, diff: i64, threshold: f64) -> ColoredString {
        let abs_diff = diff.abs() as f64;
        let color = if abs_diff > threshold as f64 {
            diff.to_string().red()
        } else if abs_diff > threshold * 0.7 {
            diff.to_string().yellow()
        } else {
            diff.to_string().green()
        };
        color
    }
}

impl Tool for BloatCheckTool {
    fn name(&self) -> &'static str {
        "bloat-check"
    }

    fn description(&self) -> &'static str {
        "Analyze binary size and suggest optimizations"
    }

    fn command(&self) -> Command {
        Command::new(self.name())
            .about(self.description())
            .long_about(
                "Analyze binary size and suggest optimizations.\n\
                 \n\
                 This tool helps you understand what's contributing to your binary size:\n\
                 • Track size changes between builds\n\
                 • Identify largest functions and data structures\n\
                 • Compare debug vs release builds\n\
                 • Generate optimization recommendations\n\
                 \n\
                 EXAMPLES:\n\
                 cm tool bloat-check --binary target/release/myapp --symbols\n\
                 cm tool bloat-check --binary target/release/myapp --baseline old-build/myapp\n\
                 cm tool bloat-check --debug-compare --optimize"
            )
            .args(&[
                Arg::new("binary")
                    .long("binary")
                    .short('b')
                    .help("Path to binary to analyze")
                    .default_value("target/release/cargo-mate"),
                Arg::new("baseline")
                    .long("baseline")
                    .help("Path to baseline binary for comparison"),
                Arg::new("threshold")
                    .long("threshold")
                    .short('t')
                    .help("Size change threshold percentage")
                    .default_value("5.0"),
                Arg::new("symbols")
                    .long("symbols")
                    .short('s')
                    .help("Show largest symbols")
                    .action(clap::ArgAction::SetTrue),
                Arg::new("debug-compare")
                    .long("debug-compare")
                    .help("Compare debug vs release builds")
                    .action(clap::ArgAction::SetTrue),
                Arg::new("optimize")
                    .long("optimize")
                    .short('o')
                    .help("Generate optimization suggestions")
                    .action(clap::ArgAction::SetTrue),
                Arg::new("report")
                    .long("report")
                    .help("Generate detailed size report")
                    .action(clap::ArgAction::SetTrue),
            ])
            .args(&super::common_options())
    }

    fn execute(&self, matches: &ArgMatches) -> Result<()> {
        let binary_path = matches.get_one::<String>("binary").unwrap();
        let baseline_path = matches.get_one::<String>("baseline");
        let threshold = matches.get_one::<String>("threshold")
            .unwrap()
            .parse::<f64>()
            .unwrap_or(5.0);
        let show_symbols = matches.get_flag("symbols");
        let debug_compare = matches.get_flag("debug-compare");
        let optimize = matches.get_flag("optimize");
        let report = matches.get_flag("report");
        let verbose = matches.get_flag("verbose");
        let dry_run = matches.get_flag("dry-run");
        let output_format = parse_output_format(matches);

        if dry_run {
            println!("🔍 Would analyze binary: {}", binary_path);
            return Ok(());
        }

        match output_format {
            OutputFormat::Human => {
                println!("📊 {} - {}", "Binary Size Analysis".bold(), self.description().cyan());

                // Main analysis
                match self.analyze_binary_size(binary_path) {
                    Ok(analysis) => {
                        println!("\n📁 Binary: {}", analysis.path.bold());
                        println!("📏 Size: {}", self.format_size(analysis.total_size).green().bold());
                        println!("🔢 Symbols: {}", analysis.symbol_count.to_string().cyan());

                        if analysis.text_size > 0 || analysis.data_size > 0 || analysis.bss_size > 0 {
                            println!("\n📈 Section Sizes:");
                            if analysis.text_size > 0 {
                                println!("  Text (code): {}", self.format_size(analysis.text_size));
                            }
                            if analysis.data_size > 0 {
                                println!("  Data (initialized): {}", self.format_size(analysis.data_size));
                            }
                            if analysis.bss_size > 0 {
                                println!("  BSS (uninitialized): {}", self.format_size(analysis.bss_size));
                            }
                        }

                        // Baseline comparison
                        if let Some(baseline) = baseline_path {
                            match self.analyze_size_changes(binary_path, baseline) {
                                Ok(comparison) => {
                                    println!("\n📊 Size Changes (compared to {}):", baseline);
                                    println!("  Total size: {} ({:.1}%)",
                                        self.colorize_diff(comparison.size_diff, threshold * analysis.total_size as f64 / 100.0),
                                        (comparison.size_diff as f64 / comparison.baseline.total_size as f64 * 100.0)
                                    );
                                    if comparison.text_diff != 0 {
                                        println!("  Text section: {} ({:.1}%)",
                                            self.colorize_diff(comparison.text_diff, threshold * analysis.text_size as f64 / 100.0),
                                            (comparison.text_diff as f64 / comparison.baseline.text_size as f64 * 100.0)
                                        );
                                    }
                                    if comparison.data_diff != 0 {
                                        println!("  Data section: {} ({:.1}%)",
                                            self.colorize_diff(comparison.data_diff, threshold * analysis.data_size as f64 / 100.0),
                                            (comparison.data_diff as f64 / comparison.baseline.data_size as f64 * 100.0)
                                        );
                                    }
                                    if comparison.bss_diff != 0 {
                                        println!("  BSS section: {} ({:.1}%)",
                                            self.colorize_diff(comparison.bss_diff, threshold * analysis.bss_size as f64 / 100.0),
                                            (comparison.bss_diff as f64 / comparison.baseline.bss_size as f64 * 100.0)
                                        );
                                    }
                                }
                                Err(e) => {
                                    if verbose {
                                        println!("⚠️  Could not analyze baseline: {}", e);
                                    }
                                }
                            }
                        }

                        // Show largest symbols
                        if show_symbols {
                            match self.find_largest_symbols(binary_path) {
                                Ok(symbols) if !symbols.is_empty() => {
                                    println!("\n🔍 Largest Symbols:");
                                    for (i, symbol) in symbols.iter().enumerate() {
                                        println!("  {}. {} ({} bytes) - {}",
                                            i + 1,
                                            symbol.name.cyan(),
                                            symbol.size.to_string().yellow(),
                                            symbol.symbol_type
                                        );
                                    }
                                }
                                Ok(_) => {
                                    if verbose {
                                        println!("\n⚠️  No symbol information available");
                                    }
                                }
                                Err(e) => {
                                    if verbose {
                                        println!("\n⚠️  Could not analyze symbols: {}", e);
                                    }
                                }
                            }
                        }

                        // Debug vs Release comparison
                        if debug_compare {
                            let debug_path = binary_path.replace("release", "debug");
                            if Path::new(&debug_path).exists() {
                                match self.analyze_debug_vs_release(&debug_path, binary_path) {
                                    Ok(comparison) => {
                                        println!("\n🔧 Debug vs Release Comparison:");
                                        println!("  Debug build: {}", self.format_size(comparison.debug_size));
                                        println!("  Release build: {}", self.format_size(comparison.release_size));
                                        println!("  Size ratio: {:.1}x", comparison.ratio);
                                        println!("  Space savings: {}", self.format_size(comparison.savings).green());
                                    }
                                    Err(e) => {
                                        if verbose {
                                            println!("\n⚠️  Could not compare builds: {}", e);
                                        }
                                    }
                                }
                            } else if verbose {
                                println!("\n⚠️  Debug build not found at: {}", debug_path);
                            }
                        }

                        // Optimization suggestions
                        if optimize {
                            let suggestions = self.generate_optimization_suggestions(&analysis);
                            if !suggestions.is_empty() {
                                println!("\n💡 Optimization Suggestions:");
                                for suggestion in suggestions {
                                    let impact_color = match suggestion.impact.as_str() {
                                        "High" => suggestion.impact.red().bold(),
                                        "Medium" => suggestion.impact.yellow().bold(),
                                        _ => suggestion.impact.green().bold(),
                                    };
                                    println!("  • [{}] {}: {}",
                                        impact_color,
                                        suggestion.category.bold(),
                                        suggestion.suggestion
                                    );
                                }
                            }
                        }

                        // Detailed report
                        if report {
                            println!("\n📋 Detailed Analysis Report:");
                            println!("═══════════════════════════════════════════════");
                            println!("Binary Path: {}", analysis.path);
                            println!("Total Size: {}", self.format_size(analysis.total_size));
                            println!("Symbol Count: {}", analysis.symbol_count);
                            println!("Text Section: {}", self.format_size(analysis.text_size));
                            println!("Data Section: {}", self.format_size(analysis.data_size));
                            println!("BSS Section: {}", self.format_size(analysis.bss_size));

                            if let Some(baseline) = baseline_path {
                                if let Ok(comparison) = self.analyze_size_changes(binary_path, baseline) {
                                    println!("\nSize Changes:");
                                    println!("Total: {}", self.format_diff(comparison.size_diff));
                                    println!("Text: {}", self.format_diff(comparison.text_diff));
                                    println!("Data: {}", self.format_diff(comparison.data_diff));
                                    println!("BSS: {}", self.format_diff(comparison.bss_diff));
                                }
                            }
                        }
                    }
                    Err(e) => {
                        return Err(ToolError::ExecutionFailed(format!("Failed to analyze binary: {}", e)));
                    }
                }
            }
            OutputFormat::Json => {
                let analysis = self.analyze_binary_size(binary_path)?;
                let mut json_output = serde_json::json!({
                    "binary": analysis.path,
                    "total_size": analysis.total_size,
                    "text_size": analysis.text_size,
                    "data_size": analysis.data_size,
                    "bss_size": analysis.bss_size,
                    "symbol_count": analysis.symbol_count,
                });

                if let Some(baseline) = baseline_path {
                    if let Ok(comparison) = self.analyze_size_changes(binary_path, baseline) {
                        json_output["size_changes"] = serde_json::json!({
                            "total_diff": comparison.size_diff,
                            "text_diff": comparison.text_diff,
                            "data_diff": comparison.data_diff,
                            "bss_diff": comparison.bss_diff,
                        });
                    }
                }

                if show_symbols {
                    if let Ok(symbols) = self.find_largest_symbols(binary_path) {
                        json_output["largest_symbols"] = serde_json::to_value(&symbols).unwrap();
                    }
                }

                if optimize {
                    let suggestions = self.generate_optimization_suggestions(&analysis);
                    json_output["optimization_suggestions"] = serde_json::to_value(&suggestions).unwrap();
                }

                println!("{}", serde_json::to_string_pretty(&json_output).unwrap());
            }
            OutputFormat::Table => {
                let analysis = self.analyze_binary_size(binary_path)?;
                println!("┌─ Binary Size Analysis ──────────────────────┐");
                println!("│ Binary: {:<35} │", analysis.path);
                println!("│ Size: {:<37} │", self.format_size(analysis.total_size));
                println!("│ Symbols: {:<34} │", analysis.symbol_count.to_string());
                println!("│ Text: {:<37} │", self.format_size(analysis.text_size));
                println!("│ Data: {:<37} │", self.format_size(analysis.data_size));
                println!("│ BSS: {:<38} │", self.format_size(analysis.bss_size));
                println!("└─────────────────────────────────────────────┘");
            }
        }

        Ok(())
    }
}
