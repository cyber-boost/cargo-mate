use super::{Tool, Result, ToolError, common_options, parse_output_format, OutputFormat};
use clap::{Arg, ArgMatches, Command};
use colored::*;
use std::path::Path;
use std::process::Command as ProcessCommand;
use std::fs;
use regex::Regex;

#[derive(Debug, Clone)]
pub struct CoverageGuardTool;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct CoverageReport {
    current_coverage: f64,
    minimum_threshold: f64,
    threshold_met: bool,
    coverage_diff: Option<f64>,
    branch: Option<String>,
    commit: Option<String>,
    timestamp: String,
    details: CoverageDetails,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct CoverageDetails {
    lines_covered: usize,
    lines_total: usize,
    functions_covered: usize,
    functions_total: usize,
    branches_covered: usize,
    branches_total: usize,
    files_analyzed: usize,
}

impl CoverageGuardTool {
    pub fn new() -> Self {
        Self
    }

    fn run_coverage_analysis(&self) -> Result<CoverageReport> {
        // Try different coverage tools in order of preference
        if self.is_tool_available("grcov") {
            self.run_grcov()
        } else if self.is_tool_available("tarpaulin") {
            self.run_tarpaulin()
        } else if self.is_tool_available("cargo-llvm-cov") {
            self.run_llvm_cov()
        } else {
            Err(ToolError::ExecutionFailed("No coverage tool found. Install grcov, tarpaulin, or cargo-llvm-cov".to_string()))
        }
    }

    fn is_tool_available(&self, tool: &str) -> bool {
        ProcessCommand::new(tool)
            .arg("--version")
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }

    fn run_grcov(&self) -> Result<CoverageReport> {
        let output = ProcessCommand::new("grcov")
            .args(&[".", "--output-type", "lcov", "--output-path", "/tmp/coverage.lcov"])
            .output()
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to run grcov: {}", e)))?;

        if !output.status.success() {
            return Err(ToolError::ExecutionFailed("grcov command failed".to_string()));
        }

        self.parse_lcov_file("/tmp/coverage.lcov")
    }

    fn run_tarpaulin(&self) -> Result<CoverageReport> {
        let output = ProcessCommand::new("cargo")
            .args(&["tarpaulin", "--out", "Json", "--output-dir", "/tmp"])
            .output()
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to run tarpaulin: {}", e)))?;

        if !output.status.success() {
            return Err(ToolError::ExecutionFailed("tarpaulin command failed".to_string()));
        }

        self.parse_tarpaulin_json("/tmp/tarpaulin-report.json")
    }

    fn run_llvm_cov(&self) -> Result<CoverageReport> {
        let output = ProcessCommand::new("cargo")
            .args(&["llvm-cov", "report", "--json", "--output-path", "/tmp/coverage.json"])
            .output()
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to run llvm-cov: {}", e)))?;

        if !output.status.success() {
            return Err(ToolError::ExecutionFailed("llvm-cov command failed".to_string()));
        }

        self.parse_llvm_cov_json("/tmp/coverage.json")
    }

    fn parse_lcov_file(&self, file_path: &str) -> Result<CoverageReport> {
        let content = fs::read_to_string(file_path)
            .map_err(|e| ToolError::IoError(e))?;

        let mut lines_total = 0;
        let mut lines_covered = 0;
        let mut functions_total = 0;
        let mut functions_covered = 0;
        let mut branches_total = 0;
        let mut branches_covered = 0;
        let mut files_count = 0;

        let lines_regex = Regex::new(r"LF:(\d+)").unwrap();
        let lines_hit_regex = Regex::new(r"LH:(\d+)").unwrap();
        let functions_regex = Regex::new(r"FNF:(\d+)").unwrap();
        let functions_hit_regex = Regex::new(r"FNH:(\d+)").unwrap();
        let branches_regex = Regex::new(r"BRF:(\d+)").unwrap();
        let branches_hit_regex = Regex::new(r"BRH:(\d+)").unwrap();

        for line in content.lines() {
            if line.starts_with("SF:") {
                files_count += 1;
            } else if let Some(captures) = lines_regex.captures(line) {
                lines_total += captures[1].parse::<usize>().unwrap_or(0);
            } else if let Some(captures) = lines_hit_regex.captures(line) {
                lines_covered += captures[1].parse::<usize>().unwrap_or(0);
            } else if let Some(captures) = functions_regex.captures(line) {
                functions_total += captures[1].parse::<usize>().unwrap_or(0);
            } else if let Some(captures) = functions_hit_regex.captures(line) {
                functions_covered += captures[1].parse::<usize>().unwrap_or(0);
            } else if let Some(captures) = branches_regex.captures(line) {
                branches_total += captures[1].parse::<usize>().unwrap_or(0);
            } else if let Some(captures) = branches_hit_regex.captures(line) {
                branches_covered += captures[1].parse::<usize>().unwrap_or(0);
            }
        }

        let coverage_percentage = if lines_total > 0 {
            (lines_covered as f64 / lines_total as f64) * 100.0
        } else {
            0.0
        };

        let details = CoverageDetails {
            lines_covered,
            lines_total,
            functions_covered,
            functions_total,
            branches_covered,
            branches_total,
            files_analyzed: files_count,
        };

        Ok(CoverageReport {
            current_coverage: coverage_percentage,
            minimum_threshold: 80.0, // Default threshold
            threshold_met: coverage_percentage >= 80.0,
            coverage_diff: None,
            branch: self.get_current_branch(),
            commit: self.get_current_commit(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            details,
        })
    }

    fn parse_tarpaulin_json(&self, file_path: &str) -> Result<CoverageReport> {
        let content = fs::read_to_string(file_path)
            .map_err(|e| ToolError::IoError(e))?;

        let json: serde_json::Value = serde_json::from_str(&content)
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to parse JSON: {}", e)))?;

        let coverage_percentage = json["coverage_percentage"]
            .as_f64()
            .unwrap_or(0.0);

        let lines_covered = json["covered_lines"]
            .as_u64()
            .unwrap_or(0) as usize;

        let lines_total = json["total_lines"]
            .as_u64()
            .unwrap_or(0) as usize;

        let details = CoverageDetails {
            lines_covered,
            lines_total,
            functions_covered: 0, // Tarpaulin doesn't provide this
            functions_total: 0,
            branches_covered: 0,
            branches_total: 0,
            files_analyzed: 1,
        };

        Ok(CoverageReport {
            current_coverage: coverage_percentage,
            minimum_threshold: 80.0,
            threshold_met: coverage_percentage >= 80.0,
            coverage_diff: None,
            branch: self.get_current_branch(),
            commit: self.get_current_commit(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            details,
        })
    }

    fn parse_llvm_cov_json(&self, file_path: &str) -> Result<CoverageReport> {
        let content = fs::read_to_string(file_path)
            .map_err(|e| ToolError::IoError(e))?;

        let json: serde_json::Value = serde_json::from_str(&content)
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to parse JSON: {}", e)))?;

        let data = json["data"][0]["totals"].clone();

        let lines_covered = data["lines"]["covered"]
            .as_u64()
            .unwrap_or(0) as usize;

        let lines_total = data["lines"]["count"]
            .as_u64()
            .unwrap_or(0) as usize;

        let functions_covered = data["functions"]["covered"]
            .as_u64()
            .unwrap_or(0) as usize;

        let functions_total = data["functions"]["count"]
            .as_u64()
            .unwrap_or(0) as usize;

        let branches_covered = data["branches"]["covered"]
            .as_u64()
            .unwrap_or(0) as usize;

        let branches_total = data["branches"]["count"]
            .as_u64()
            .unwrap_or(0) as usize;

        let coverage_percentage = if lines_total > 0 {
            (lines_covered as f64 / lines_total as f64) * 100.0
        } else {
            0.0
        };

        let details = CoverageDetails {
            lines_covered,
            lines_total,
            functions_covered,
            functions_total,
            branches_covered,
            branches_total,
            files_analyzed: 1,
        };

        Ok(CoverageReport {
            current_coverage: coverage_percentage,
            minimum_threshold: 80.0,
            threshold_met: coverage_percentage >= 80.0,
            coverage_diff: None,
            branch: self.get_current_branch(),
            commit: self.get_current_commit(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            details,
        })
    }

    fn get_current_branch(&self) -> Option<String> {
        ProcessCommand::new("git")
            .args(&["rev-parse", "--abbrev-ref", "HEAD"])
            .output()
            .ok()
            .filter(|output| output.status.success())
            .and_then(|output| String::from_utf8_lossy(&output.stdout).trim().to_string().into())
    }

    fn get_current_commit(&self) -> Option<String> {
        ProcessCommand::new("git")
            .args(&["rev-parse", "HEAD"])
            .output()
            .ok()
            .filter(|output| output.status.success())
            .and_then(|output| String::from_utf8_lossy(&output.stdout).trim()[..8].to_string().into())
    }

    fn get_baseline_coverage(&self, baseline: &str) -> Result<f64> {
        // Get coverage from git history or stored baseline
        let output = ProcessCommand::new("git")
            .args(&["show", &format!("{}:coverage.json", baseline)])
            .output()
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to get baseline: {}", e)))?;

        if !output.status.success() {
            return Err(ToolError::ExecutionFailed(format!("Baseline commit {} not found or missing coverage data", baseline)));
        }

        let content = String::from_utf8_lossy(&output.stdout);
        let json: serde_json::Value = serde_json::from_str(&content)
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to parse baseline JSON: {}", e)))?;

        Ok(json["current_coverage"].as_f64().unwrap_or(0.0))
    }

    fn display_report(&self, report: &CoverageReport, output_format: OutputFormat, verbose: bool) {
        match output_format {
            OutputFormat::Human => {
                println!("\n{}", "📊 Coverage Guard Report".bold().blue());
                println!("{}", "═".repeat(50).blue());

                println!("\n📈 Current Coverage: {:.2}%", report.current_coverage);

                if let Some(threshold) = report.minimum_threshold.into() {
                    println!("🎯 Minimum Threshold: {:.2}%", threshold);
                }

                let status = if report.threshold_met {
                    "✅ PASSED".green()
                } else {
                    "❌ FAILED".red()
                };
                println!("📋 Status: {}", status);

                if let Some(branch) = &report.branch {
                    println!("🌿 Branch: {}", branch);
                }

                if let Some(commit) = &report.commit {
                    println!("🔗 Commit: {}", commit);
                }

                if verbose {
                    println!("\n📊 Detailed Metrics:");
                    println!("  • Lines: {}/{} ({:.2}%)",
                            report.details.lines_covered,
                            report.details.lines_total,
                            if report.details.lines_total > 0 {
                                (report.details.lines_covered as f64 / report.details.lines_total as f64) * 100.0
                            } else { 0.0 });

                    if report.details.functions_total > 0 {
                        println!("  • Functions: {}/{} ({:.2}%)",
                                report.details.functions_covered,
                                report.details.functions_total,
                                (report.details.functions_covered as f64 / report.details.functions_total as f64) * 100.0);
                    }

                    if report.details.branches_total > 0 {
                        println!("  • Branches: {}/{} ({:.2}%)",
                                report.details.branches_covered,
                                report.details.branches_total,
                                (report.details.branches_covered as f64 / report.details.branches_total as f64) * 100.0);
                    }

                    println!("  • Files Analyzed: {}", report.details.files_analyzed);
                }

                if let Some(diff) = report.coverage_diff {
                    let diff_status = if diff >= 0.0 {
                        format!("+{:.2}%", diff).green()
                    } else {
                        format!("{:.2}%", diff).red()
                    };
                    println!("📊 Coverage Change: {}", diff_status);
                }

                if !report.threshold_met {
                    println!("\n{}", "⚠️  Coverage threshold not met!".yellow().bold());
                    println!("💡 To fix this:");
                    println!("   1. Add more tests to increase coverage");
                    println!("   2. Use --threshold to set a custom minimum");
                    println!("   3. Use --baseline to compare against previous commit");
                }
            }
            OutputFormat::Json => {
                let json = serde_json::to_string_pretty(report)
                    .unwrap_or_else(|_| "{}".to_string());
                println!("{}", json);
            }
            OutputFormat::Table => {
                println!("{:<20} {:<15} {:<15} {:<10}",
                        "Metric", "Covered", "Total", "Percentage");
                println!("{}", "─".repeat(70));

                println!("{:<20} {:<15} {:<15} {:.2}%",
                        "Lines",
                        report.details.lines_covered.to_string(),
                        report.details.lines_total.to_string(),
                        if report.details.lines_total > 0 {
                            (report.details.lines_covered as f64 / report.details.lines_total as f64) * 100.0
                        } else { 0.0 });

                if report.details.functions_total > 0 {
                    println!("{:<20} {:<15} {:<15} {:.2}%",
                            "Functions",
                            report.details.functions_covered.to_string(),
                            report.details.functions_total.to_string(),
                            (report.details.functions_covered as f64 / report.details.functions_total as f64) * 100.0);
                }

                if report.details.branches_total > 0 {
                    println!("{:<20} {:<15} {:<15} {:.2}%",
                            "Branches",
                            report.details.branches_covered.to_string(),
                            report.details.branches_total.to_string(),
                            (report.details.branches_covered as f64 / report.details.branches_total as f64) * 100.0);
                }
            }
        }
    }
}

impl Tool for CoverageGuardTool {
    fn name(&self) -> &'static str {
        "coverage-guard"
    }

    fn description(&self) -> &'static str {
        "Block commits/PRs if coverage drops below threshold"
    }

    fn command(&self) -> Command {
        Command::new(self.name())
            .about(self.description())
            .long_about("Monitor code coverage and prevent commits when coverage drops below \
                        acceptable thresholds. Supports multiple coverage tools and CI/CD integration.

EXAMPLES:
    cm tool coverage-guard --threshold 85.0
    cm tool coverage-guard --baseline main --fail-on-drop
    cm tool coverage-guard --tool grcov --output-format json")
            .args(&[
                Arg::new("threshold")
                    .long("threshold")
                    .short('t')
                    .help("Minimum coverage threshold (percentage)")
                    .default_value("80.0"),
                Arg::new("baseline")
                    .long("baseline")
                    .short('b')
                    .help("Baseline commit/branch for comparison"),
                Arg::new("tool")
                    .long("tool")
                    .help("Coverage tool to use (grcov, tarpaulin, llvm-cov)")
                    .default_value("auto"),
                Arg::new("fail-on-drop")
                    .long("fail-on-drop")
                    .help("Fail if coverage drops from baseline")
                    .action(clap::ArgAction::SetTrue),
                Arg::new("store-baseline")
                    .long("store-baseline")
                    .help("Store current coverage as baseline")
                    .action(clap::ArgAction::SetTrue),
                Arg::new("ci-mode")
                    .long("ci-mode")
                    .help("CI-friendly output format")
                    .action(clap::ArgAction::SetTrue),
            ])
            .args(&common_options())
    }

    fn execute(&self, matches: &ArgMatches) -> Result<()> {
        let threshold = matches.get_one::<String>("threshold")
            .unwrap()
            .parse::<f64>()
            .map_err(|_| ToolError::InvalidArguments("Invalid threshold value".to_string()))?;

        let baseline = matches.get_one::<String>("baseline");
        let tool = matches.get_one::<String>("tool").unwrap();
        let fail_on_drop = matches.get_flag("fail-on-drop");
        let store_baseline = matches.get_flag("store-baseline");
        let ci_mode = matches.get_flag("ci-mode");
        let output_format = parse_output_format(matches);
        let verbose = matches.get_flag("verbose");

        // Run coverage analysis
        let mut report = self.run_coverage_analysis()?;

        // Set custom threshold
        report.minimum_threshold = threshold;
        report.threshold_met = report.current_coverage >= threshold;

        // Compare with baseline if specified
        if let Some(baseline_commit) = baseline {
            match self.get_baseline_coverage(baseline_commit) {
                Ok(baseline_cov) => {
                    report.coverage_diff = Some(report.current_coverage - baseline_cov);

                    if fail_on_drop && report.coverage_diff.unwrap() < 0.0 {
                        report.threshold_met = false;
                    }
                }
                Err(e) => {
                    if verbose {
                        println!("⚠️  Could not get baseline coverage: {}", e);
                    }
                }
            }
        }

        // Store baseline if requested
        if store_baseline {
            let baseline_data = serde_json::to_string_pretty(&report)
                .map_err(|e| ToolError::ExecutionFailed(format!("Failed to serialize: {}", e)))?;

            fs::write("coverage.json", baseline_data)
                .map_err(|e| ToolError::IoError(e))?;

            println!("✅ Baseline coverage stored in coverage.json");
        }

        // Display report
        self.display_report(&report, output_format, verbose);

        // Handle CI mode
        if ci_mode {
            if report.threshold_met {
                println!("::set-output name=coverage-passed::true");
                println!("::set-output name=coverage-percentage::{:.2}", report.current_coverage);
            } else {
                println!("::set-output name=coverage-passed::false");
                println!("::set-output name=coverage-percentage::{:.2}", report.current_coverage);
                println!("::error title=Coverage Check Failed::Coverage {:.2}% is below threshold {:.2}%",
                        report.current_coverage, threshold);
            }
        }

        // Exit with error if threshold not met
        if !report.threshold_met {
            std::process::exit(1);
        }

        Ok(())
    }
}

impl Default for CoverageGuardTool {
    fn default() -> Self {
        Self::new()
    }
}
