use super::{Tool, Result, ToolError, common_options, parse_output_format, OutputFormat};
use clap::{Arg, ArgMatches, Command};
use colored::*;
use std::path::Path;
use std::process::Command as ProcessCommand;
use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct BenchDiffTool;

#[derive(Debug, Deserialize, Serialize)]
struct BenchmarkResult {
    name: String,
    time: String,
    throughput: Option<String>,
}

#[derive(Debug, serde::Serialize)]
struct BenchmarkComparison {
    name: String,
    before_time: f64,
    after_time: f64,
    improvement: f64,
    regression: bool,
}

impl BenchDiffTool {
    pub fn new() -> Self {
        Self
    }

    fn parse_time_to_ns(&self, time_str: &str) -> Result<f64> {
        // Handle formats like "1.23 ns/iter", "2.34 µs/iter", "5.67 ms/iter", etc.
        let parts: Vec<&str> = time_str.split_whitespace().collect();
        if parts.is_empty() {
            return Err(ToolError::ExecutionFailed("Invalid time format".to_string()));
        }

        let (value_str, unit) = if parts.len() >= 2 {
            (parts[0], parts[1])
        } else {
            (time_str, "ns")
        };

        let value: f64 = value_str.parse()
            .map_err(|_| ToolError::ExecutionFailed("Cannot parse time value".to_string()))?;

        let multiplier = match unit {
            "ns" | "ns/iter" => 1.0,
            "µs" | "µs/iter" => 1_000.0,
            "ms" | "ms/iter" => 1_000_000.0,
            "s" | "s/iter" => 1_000_000_000.0,
            _ => 1.0, // Assume nanoseconds if unit is unclear
        };

        Ok(value * multiplier)
    }

    fn run_benchmark(&self, commit: &str) -> Result<Vec<BenchmarkResult>> {
        println!("📊 Running benchmarks for commit: {}", commit.yellow());

        // Checkout the commit
        let checkout_result = ProcessCommand::new("git")
            .args(&["checkout", commit])
            .output()
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to checkout commit: {}", e)))?;

        if !checkout_result.status.success() {
            return Err(ToolError::ExecutionFailed("Git checkout failed".to_string()));
        }

        // Run cargo bench
        let bench_result = ProcessCommand::new("cargo")
            .args(&["bench", "--message-format", "json"])
            .output()
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to run cargo bench: {}", e)))?;

        if !bench_result.status.success() {
            return Err(ToolError::ExecutionFailed("Cargo bench failed".to_string()));
        }

        // Parse the output (simplified for now)
        let output = String::from_utf8_lossy(&bench_result.stdout);
        let mut results = Vec::new();

        // Simple parsing of cargo bench output
        for line in output.lines() {
            if line.contains("test ") && line.contains("time:") {
                if let Some(test_name) = self.extract_test_name(line) {
                    if let Some(time_str) = self.extract_time(line) {
                        results.push(BenchmarkResult {
                            name: test_name,
                            time: time_str,
                            throughput: None,
                        });
                    }
                }
            }
        }

        Ok(results)
    }

    fn extract_test_name(&self, line: &str) -> Option<String> {
        if let Some(start) = line.find("test ") {
            let after_test = &line[start + 5..];
            if let Some(end) = after_test.find(" ...") {
                return Some(after_test[..end].to_string());
            }
        }
        None
    }

    fn extract_time(&self, line: &str) -> Option<String> {
        if let Some(time_start) = line.find("time: [") {
            let after_time = &line[time_start + 7..];
            if let Some(end) = after_time.find(']') {
                return Some(after_time[..end].to_string());
            }
        }
        None
    }

    fn compare_benchmarks(&self, before: &[BenchmarkResult], after: &[BenchmarkResult]) -> Vec<BenchmarkComparison> {
        let mut comparisons = Vec::new();
        let mut before_map: HashMap<String, f64> = HashMap::new();

        // Convert before results to nanoseconds
        for result in before {
            if let Ok(ns) = self.parse_time_to_ns(&result.time) {
                before_map.insert(result.name.clone(), ns);
            }
        }

        // Compare with after results
        for result in after {
            if let Ok(after_ns) = self.parse_time_to_ns(&result.time) {
                if let Some(before_ns) = before_map.get(&result.name) {
                    let improvement = ((before_ns - after_ns) / before_ns) * 100.0;
                    comparisons.push(BenchmarkComparison {
                        name: result.name.clone(),
                        before_time: *before_ns,
                        after_time: after_ns,
                        improvement,
                        regression: improvement < 0.0,
                    });
                }
            }
        }

        comparisons.sort_by(|a, b| a.name.cmp(&b.name));
        comparisons
    }

    fn format_time(&self, ns: f64) -> String {
        if ns >= 1_000_000_000.0 {
            format!("{:.2}s", ns / 1_000_000_000.0)
        } else if ns >= 1_000_000.0 {
            format!("{:.2}ms", ns / 1_000_000.0)
        } else if ns >= 1_000.0 {
            format!("{:.2}µs", ns / 1_000.0)
        } else {
            format!("{:.2}ns", ns)
        }
    }

    fn display_comparison(&self, comparisons: &[BenchmarkComparison], format: OutputFormat) {
        match format {
            OutputFormat::Json => {
                println!("{}", serde_json::to_string_pretty(comparisons).unwrap());
            }
            OutputFormat::Table => {
                println!("{:<40} {:<15} {:<15} {:<12}", "Benchmark", "Before", "After", "Change");
                println!("{}", "─".repeat(85));

                for comp in comparisons {
                    let change_color = if comp.regression {
                        comp.improvement.to_string().red()
                    } else {
                        comp.improvement.to_string().green()
                    };

                    println!("{:<40} {:<15} {:<15} {:>+6.2}%",
                             comp.name,
                             self.format_time(comp.before_time),
                             self.format_time(comp.after_time),
                             change_color);
                }
            }
            OutputFormat::Human => {
                println!("{}", "📊 Benchmark Comparison Results".bold().blue());
                println!("{}", "═".repeat(60).blue());

                let mut improved = 0;
                let mut regressed = 0;

                for comp in comparisons {
                    let status = if comp.regression {
                        regressed += 1;
                        "📉 REGRESSION".red().bold()
                    } else {
                        improved += 1;
                        "📈 IMPROVED".green().bold()
                    };

                    println!("{} {}", status, comp.name.bold());
                    println!("   Before: {} | After: {} | Change: {:>+6.2}%",
                             self.format_time(comp.before_time).cyan(),
                             self.format_time(comp.after_time).cyan(),
                             comp.improvement);
                    println!();
                }

                println!("{}", "Summary:".bold());
                println!("  {} Improved", format!("{} ✅", improved).green());
                println!("  {} Regressed", format!("{} ❌", regressed).red());
            }
        }
    }
}

impl Tool for BenchDiffTool {
    fn name(&self) -> &'static str {
        "bench-diff"
    }

    fn description(&self) -> &'static str {
        "Compare benchmark results between commits"
    }

    fn command(&self) -> Command {
        Command::new(self.name())
            .about(self.description())
            .long_about("Compare cargo bench results between two commits to identify performance changes")
            .args(&[
                Arg::new("from")
                    .long("from")
                    .short('f')
                    .help("Starting commit (default: HEAD~1)")
                    .default_value("HEAD~1"),
                Arg::new("to")
                    .long("to")
                    .short('t')
                    .help("Ending commit (default: HEAD)")
                    .default_value("HEAD"),
                Arg::new("threshold")
                    .long("threshold")
                    .help("Minimum percentage change to report (default: 5.0)")
                    .default_value("5.0"),
                Arg::new("save")
                    .long("save")
                    .help("Save results to .cargo-mate/benchmarks/"),
            ])
            .args(&common_options())
    }

    fn execute(&self, matches: &ArgMatches) -> Result<()> {
        let from_commit = matches.get_one::<String>("from").unwrap();
        let to_commit = matches.get_one::<String>("to").unwrap();
        let threshold: f64 = matches.get_one::<String>("threshold")
            .unwrap()
            .parse()
            .map_err(|_| ToolError::InvalidArguments("Invalid threshold value".to_string()))?;
        let save_results = matches.get_flag("save");
        let output_format = parse_output_format(matches);
        let verbose = matches.get_flag("verbose");

        // Check if we're in a git repository
        if !Path::new(".git").exists() {
            return Err(ToolError::ExecutionFailed("Not in a git repository".to_string()));
        }

        println!("🚀 {} - Comparing benchmark performance", "CargoMate BenchDiff".bold().blue());
        println!("   From: {} | To: {} | Threshold: ±{}%", from_commit, to_commit, threshold);
        println!();

        // Store current commit to restore later
        let current_commit = ProcessCommand::new("git")
            .args(&["rev-parse", "HEAD"])
            .output()
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to get current commit: {}", e)))?;

        let current_commit = String::from_utf8_lossy(&current_commit.stdout).trim().to_string();

        if verbose {
            println!("📍 Current commit: {}", current_commit.dimmed());
        }

        // Run benchmarks for the "from" commit
        let before_results = self.run_benchmark(from_commit)?;

        // Run benchmarks for the "to" commit
        let after_results = self.run_benchmark(to_commit)?;

        // Restore original commit
        ProcessCommand::new("git")
            .args(&["checkout", &current_commit])
            .output()
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to restore commit: {}", e)))?;

        if verbose {
            println!("✅ Restored to original commit: {}", current_commit.dimmed());
        }

        // Compare results
        let comparisons = self.compare_benchmarks(&before_results, &after_results);

        // Filter by threshold
        let significant_changes: Vec<_> = comparisons.into_iter()
            .filter(|comp| comp.improvement.abs() >= threshold)
            .collect();

        if significant_changes.is_empty() {
            println!("📊 No significant changes detected (threshold: ±{}%)", threshold);
        } else {
            self.display_comparison(&significant_changes, output_format);
        }

        // Save results if requested
        if save_results {
            self.save_results(&significant_changes, from_commit, to_commit)?;
        }

        Ok(())
    }
}

impl BenchDiffTool {
    fn save_results(&self, comparisons: &[BenchmarkComparison], from: &str, to: &str) -> Result<()> {
        use std::fs;

        let cargo_mate_dir = Path::new(".cargo-mate");
        let benchmarks_dir = cargo_mate_dir.join("benchmarks");

        fs::create_dir_all(&benchmarks_dir)
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to create benchmarks dir: {}", e)))?;

        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
        let filename = format!("bench_diff_{}_to_{}_{}.json", from, to, timestamp);
        let filepath = benchmarks_dir.join(filename);

        let results = serde_json::json!({
            "from_commit": from,
            "to_commit": to,
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "comparisons": comparisons,
            "threshold": 5.0
        });

        fs::write(&filepath, serde_json::to_string_pretty(&results).unwrap())
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to save results: {}", e)))?;

        println!("💾 Results saved to: {}", filepath.display().to_string().cyan());
        Ok(())
    }
}

impl Default for BenchDiffTool {
    fn default() -> Self {
        Self::new()
    }
}
