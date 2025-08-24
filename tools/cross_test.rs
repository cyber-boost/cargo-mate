use super::{Tool, Result, ToolError, common_options, parse_output_format, OutputFormat};
use clap::{Arg, ArgMatches, Command};
use colored::*;
use std::path::Path;
use std::process::Command as ProcessCommand;
use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct CrossTestTool;

#[derive(Debug, Clone, Deserialize, Serialize)]
struct TestResult {
    platform: String,
    success: bool,
    duration: Option<f64>,
    output: String,
    errors: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize)]
struct CrossTestReport {
    test_name: String,
    platforms: Vec<TestResult>,
    summary: TestSummary,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct TestSummary {
    total_platforms: usize,
    successful: usize,
    failed: usize,
    total_duration: f64,
    fastest_platform: Option<String>,
    slowest_platform: Option<String>,
}

impl CrossTestTool {
    pub fn new() -> Self {
        Self
    }

    fn get_supported_platforms(&self) -> Vec<String> {
        vec![
            "x86_64-unknown-linux-gnu".to_string(),
            "x86_64-apple-darwin".to_string(),
            "aarch64-apple-darwin".to_string(),
            "x86_64-pc-windows-msvc".to_string(),
            "aarch64-unknown-linux-gnu".to_string(),
        ]
    }

    fn detect_current_platform(&self) -> String {
        let target = std::env::var("TARGET").unwrap_or_else(|_| {
            format!("{}-{}",
                std::env::consts::ARCH,
                std::env::consts::OS
            )
        });
        target
    }

    fn run_tests_for_platform(&self, platform: &str, test_filter: Option<&str>, verbose: bool) -> Result<TestResult> {
        println!("🧪 Testing on platform: {}", platform.cyan());

        let start_time = std::time::Instant::now();

        let mut cmd = ProcessCommand::new("cargo");
        cmd.arg("test");

        if platform != "current" && platform != self.detect_current_platform() {
            cmd.arg("--target").arg(platform);
        }

        if let Some(filter) = test_filter {
            cmd.arg(filter);
        }

        if verbose {
            cmd.arg("--").arg("--nocapture");
        } else {
            cmd.arg("--quiet");
        }

        let output = cmd.output()
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to run tests on {}: {}", platform, e)))?;

        let duration = start_time.elapsed().as_secs_f64();

        let success = output.status.success();
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        let mut errors = Vec::new();
        if !success {
            for line in stderr.lines() {
                if line.contains("error") || line.contains("FAILED") {
                    errors.push(line.to_string());
                }
            }
        }

        let combined_output = if verbose {
            format!("{}\n{}", stdout, stderr)
        } else {
            if success {
                "Tests passed successfully".to_string()
            } else {
                stderr.to_string()
            }
        };

        Ok(TestResult {
            platform: platform.to_string(),
            success,
            duration: Some(duration),
            output: combined_output,
            errors,
        })
    }

    fn run_cross_platform_tests(&self, platforms: &[String], test_filter: Option<&str>, parallel: bool, verbose: bool) -> Result<Vec<TestResult>> {
        let mut results = Vec::new();

        if parallel {
            println!("🚀 Running tests in parallel across platforms");
            // For now, we'll run sequentially - parallel execution would require more complex setup
        }

        for platform in platforms {
            match self.run_tests_for_platform(platform, test_filter, verbose) {
                Ok(result) => results.push(result),
                Err(e) => {
                    println!("❌ Failed to test on {}: {}", platform.red(), e);
                    results.push(TestResult {
                        platform: platform.clone(),
                        success: false,
                        duration: None,
                        output: format!("Test execution failed: {}", e),
                        errors: vec![e.to_string()],
                    });
                }
            }
        }

        Ok(results)
    }

    fn generate_summary(&self, results: &[TestResult]) -> TestSummary {
        let total_platforms = results.len();
        let successful = results.iter().filter(|r| r.success).count();
        let failed = total_platforms - successful;

        let total_duration: f64 = results.iter()
            .filter_map(|r| r.duration)
            .sum();

        let mut fastest = None;
        let mut slowest = None;
        let mut min_duration = f64::INFINITY;
        let mut max_duration = 0.0;

        for result in results {
            if let Some(duration) = result.duration {
                if duration < min_duration {
                    min_duration = duration;
                    fastest = Some(result.platform.clone());
                }
                if duration > max_duration {
                    max_duration = duration;
                    slowest = Some(result.platform.clone());
                }
            }
        }

        TestSummary {
            total_platforms,
            successful,
            failed,
            total_duration,
            fastest_platform: fastest,
            slowest_platform: slowest,
        }
    }

    fn display_results(&self, results: &[TestResult], summary: &TestSummary, format: OutputFormat, verbose: bool) -> Result<()> {
        match format {
            OutputFormat::Json => {
                let report = CrossTestReport {
                    test_name: "cross-platform-tests".to_string(),
                    platforms: results.to_vec(),
                    summary: (*summary).clone(),
                };
                println!("{}", serde_json::to_string_pretty(&report).unwrap());
            }
            OutputFormat::Table => {
                println!("{:<25} {:<10} {:<12} {:<15}", "Platform", "Status", "Duration", "Errors");
                println!("{}", "─".repeat(65));

                for result in results {
                    let status = if result.success {
                        "✅ PASS".green().to_string()
                    } else {
                        "❌ FAIL".red().to_string()
                    };

                    let duration = result.duration
                        .map(|d| format!("{:.2}s", d))
                        .unwrap_or("N/A".to_string());

                    let error_count = result.errors.len().to_string();

                    println!("{:<25} {:<10} {:<12} {:<15}",
                            result.platform,
                            status,
                            duration,
                            error_count);
                }
            }
            OutputFormat::Human => {
                println!("{}", "🌐 Cross-Platform Test Results".bold().blue());
                println!("{}", "═".repeat(50).blue());

                println!("📊 Summary:");
                println!("   Platforms tested: {}", summary.total_platforms);
                println!("   ✅ Passed: {}", summary.successful.to_string().green());
                println!("   ❌ Failed: {}", summary.failed.to_string().red());
                println!("   ⏱️  Total time: {:.2}s", summary.total_duration);

                if let Some(fastest) = &summary.fastest_platform {
                    println!("   🏃 Fastest: {}", fastest.cyan());
                }
                if let Some(slowest) = &summary.slowest_platform {
                    println!("   🐌 Slowest: {}", slowest.yellow());
                }

                if verbose {
                    println!("\n📋 Detailed Results:");
                    for result in results {
                        println!("\n{}: {}", result.platform.bold(),
                                if result.success { "PASSED".green() } else { "FAILED".red() });

                        if let Some(duration) = result.duration {
                            println!("   Duration: {:.2}s", duration);
                        }

                        if !result.errors.is_empty() {
                            println!("   Errors:");
                            for error in &result.errors {
                                println!("     {}", error.red());
                            }
                        }

                        if verbose && !result.output.is_empty() {
                            println!("   Output:");
                            for line in result.output.lines() {
                                println!("     {}", line);
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn validate_platforms(&self, requested_platforms: &[String]) -> Result<Vec<String>> {
        let supported = self.get_supported_platforms();
        let mut valid_platforms = Vec::new();

        for platform in requested_platforms {
            if platform == "current" {
                valid_platforms.push(self.detect_current_platform());
            } else if supported.contains(platform) {
                valid_platforms.push(platform.clone());
            } else {
                println!("⚠️  Platform {} not fully supported, will attempt anyway", platform.yellow());
                valid_platforms.push(platform.clone());
            }
        }

        Ok(valid_platforms)
    }
}

impl Tool for CrossTestTool {
    fn name(&self) -> &'static str {
        "cross-test"
    }

    fn description(&self) -> &'static str {
        "Run tests across different platforms and architectures"
    }

    fn command(&self) -> Command {
        Command::new(self.name())
            .about(self.description())
            .long_about("Test your Rust code across multiple platforms using Docker or native compilation. Helps catch platform-specific bugs early.")
            .args(&[
                Arg::new("platforms")
                    .long("platforms")
                    .short('p')
                    .help("Comma-separated list of platforms to test")
                    .default_value("current"),
                Arg::new("test-filter")
                    .long("test-filter")
                    .short('f')
                    .help("Filter tests by name pattern"),
                Arg::new("docker")
                    .long("docker")
                    .help("Use Docker for cross-platform testing")
                    .action(clap::ArgAction::SetTrue),
                Arg::new("parallel")
                    .long("parallel")
                    .help("Run tests in parallel (experimental)")
                    .action(clap::ArgAction::SetTrue),
                Arg::new("list-platforms")
                    .long("list-platforms")
                    .help("List supported platforms")
                    .action(clap::ArgAction::SetTrue),
                Arg::new("report")
                    .long("report")
                    .help("Generate detailed test report")
                    .action(clap::ArgAction::SetTrue),
                Arg::new("failing-only")
                    .long("failing-only")
                    .help("Only show results for failed platforms")
                    .action(clap::ArgAction::SetTrue),
            ])
            .args(&common_options())
    }

    fn execute(&self, matches: &ArgMatches) -> Result<()> {
        let platforms_str = matches.get_one::<String>("platforms").unwrap();
        let test_filter = matches.get_one::<String>("test-filter");
        let docker = matches.get_flag("docker");
        let parallel = matches.get_flag("parallel");
        let list_platforms = matches.get_flag("list-platforms");
        let report = matches.get_flag("report");
        let failing_only = matches.get_flag("failing-only");
        let output_format = parse_output_format(matches);
        let verbose = matches.get_flag("verbose");
        let dry_run = matches.get_flag("dry-run");

        if list_platforms {
            println!("{}", "Supported Platforms:".bold().blue());
            for platform in &self.get_supported_platforms() {
                println!("  • {}", platform.cyan());
            }
            println!("\n💡 Use 'current' to test on your current platform");
            return Ok(());
        }

        if docker {
            println!("🐳 Docker support not yet implemented");
            println!("   This would run tests in Docker containers for each platform");
            return Ok(());
        }

        println!("🌐 {} - Running cross-platform tests", "CargoMate CrossTest".bold().blue());

        let requested_platforms: Vec<String> = if platforms_str == "current" {
            vec!["current".to_string()]
        } else {
            platforms_str.split(',').map(|s| s.trim().to_string()).collect()
        };

        let platforms = self.validate_platforms(&requested_platforms)?;

        if dry_run {
            println!("🔍 Dry run - would test on platforms: {:?}", platforms);
            if let Some(filter) = test_filter {
                println!("   Test filter: {}", filter);
            }
            return Ok(());
        }

        println!("🧪 Testing on {} platform(s): {}", platforms.len(), platforms.join(", ").cyan());

        let results = self.run_cross_platform_tests(&platforms, test_filter.map(|s| s.as_str()), parallel, verbose)?;
        let summary = self.generate_summary(&results);

        let display_results = if failing_only {
            results.iter().filter(|r| !r.success).cloned().collect::<Vec<_>>()
        } else {
            results.clone()
        };

        self.display_results(&display_results, &summary, output_format, verbose)?;

        if summary.failed > 0 {
            println!("\n⚠️  {} platform(s) had test failures", summary.failed.to_string().yellow());
            println!("   Use --verbose to see detailed error output");
        } else {
            println!("\n🎉 All platforms passed tests successfully!");
        }

        Ok(())
    }
}

impl Default for CrossTestTool {
    fn default() -> Self {
        Self::new()
    }
}
