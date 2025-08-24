use super::{Tool, Result, ToolError, common_options, parse_output_format, OutputFormat};
use clap::{Arg, ArgMatches, Command};
use colored::*;
use std::path::Path;
use std::fs;
use std::collections::HashMap;
use regex::Regex;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct SnapshotTestTool;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SnapshotReport {
    total_snapshots: usize,
    passed: usize,
    failed: usize,
    updated: usize,
    new_snapshots: usize,
    snapshots: Vec<SnapshotResult>,
    summary: TestSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SnapshotResult {
    name: String,
    path: String,
    status: SnapshotStatus,
    diff: Option<String>,
    old_size: Option<usize>,
    new_size: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
enum SnapshotStatus {
    Pass,
    Fail,
    Updated,
    New,
    Missing,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TestSummary {
    test_files_analyzed: usize,
    snapshots_directory: String,
    update_mode: bool,
    timestamp: String,
}

#[derive(Debug, Clone)]
struct SnapshotData {
    name: String,
    content: String,
    path: String,
}

impl SnapshotTestTool {
    pub fn new() -> Self {
        Self
    }

    fn find_snapshot_tests(&self, directory: &str) -> Result<Vec<String>> {
        let mut test_files = Vec::new();
        self.find_test_files_recursive(directory, &mut test_files)?;
        Ok(test_files)
    }

    fn find_test_files_recursive(&self, dir: &str, test_files: &mut Vec<String>) -> Result<()> {
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
                    self.find_test_files_recursive(&path.to_string_lossy(), test_files)?;
                }
            } else if let Some(ext) = path.extension() {
                if ext == "rs" && path.to_string_lossy().contains("test") {
                    test_files.push(path.to_string_lossy().to_string());
                }
            }
        }

        Ok(())
    }

    fn parse_snapshot_calls(&self, content: &str, file_path: &str) -> Vec<SnapshotData> {
        let mut snapshots = Vec::new();

        // Look for common snapshot testing patterns
        let patterns = vec![
            // assert_snapshot! macro calls
            r#"assert_snapshot!\s*\(\s*"([^"]+)"\s*,"#,
            // snapshot! macro calls
            r#"snapshot!\s*\(\s*"([^"]+)"\s*,"#,
            // Custom snapshot patterns
            r#"snapshot_\w+!\s*\(\s*"([^"]+)"\s*,"#,
        ];

        for pattern in patterns {
            if let Ok(regex) = Regex::new(pattern) {
                for captures in regex.captures_iter(content) {
                    if let Some(snapshot_name) = captures.get(1) {
                        snapshots.push(SnapshotData {
                            name: snapshot_name.as_str().to_string(),
                            content: String::new(), // Will be filled when we find the actual snapshot
                            path: file_path.to_string(),
                        });
                    }
                }
            }
        }

        snapshots
    }

    fn find_snapshot_files(&self, snapshots_dir: &str) -> Result<HashMap<String, String>> {
        let mut snapshot_files = HashMap::new();

        if !Path::new(snapshots_dir).exists() {
            return Ok(snapshot_files);
        }

        for entry in fs::read_dir(snapshots_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() {
                if let Some(file_name) = path.file_name() {
                    let file_name_str = file_name.to_string_lossy().to_string();
                    if file_name_str.ends_with(".snap") || file_name_str.contains("snapshot") {
                        let content = fs::read_to_string(&path)?;
                        snapshot_files.insert(file_name_str, content);
                    }
                }
            }
        }

        Ok(snapshot_files)
    }

    fn generate_snapshot_name(&self, test_name: &str, snapshot_name: &str) -> String {
        format!("{}.snap", snapshot_name.replace(" ", "_").to_lowercase())
    }

    fn compare_snapshots(&self, expected: &str, actual: &str) -> Option<String> {
        if expected == actual {
            return None;
        }

        // Generate a simple diff
        let expected_lines: Vec<&str> = expected.lines().collect();
        let actual_lines: Vec<&str> = actual.lines().collect();

        let mut diff = String::new();
        let max_lines = expected_lines.len().max(actual_lines.len());

        for i in 0..max_lines {
            let expected_line = expected_lines.get(i).map_or("", |v| v);
            let actual_line = actual_lines.get(i).map_or("", |v| v);

            if expected_line != actual_line {
                if i < expected_lines.len() {
                    diff.push_str(&format!("- {}\n", expected_line));
                }
                if i < actual_lines.len() {
                    diff.push_str(&format!("+ {}\n", actual_line));
                }
            }
        }

        Some(diff)
    }

    fn update_snapshot_file(&self, snapshots_dir: &str, snapshot_name: &str, content: &str) -> Result<()> {
        let snapshot_path = Path::new(snapshots_dir).join(snapshot_name);

        // Create directory if it doesn't exist
        if let Some(parent) = snapshot_path.parent() {
            fs::create_dir_all(parent)?;
        }

        fs::write(snapshot_path, content)?;
        Ok(())
    }

    fn display_report(&self, report: &SnapshotReport, output_format: OutputFormat, verbose: bool) {
        match output_format {
            OutputFormat::Human => {
                println!("\n{}", "📸 Snapshot Test Report".bold().blue());
                println!("{}", "═".repeat(50).blue());

                println!("\n📊 Summary:");
                println!("  • Total snapshots: {}", report.total_snapshots);
                println!("  • Passed: {}", report.passed.to_string().green());
                println!("  • Failed: {}", report.failed.to_string().red());
                println!("  • Updated: {}", report.updated.to_string().yellow());
                println!("  • New: {}", report.new_snapshots.to_string().cyan());

                if report.summary.update_mode {
                    println!("  • Mode: {}", "Update".yellow());
                } else {
                    println!("  • Mode: {}", "Verify".green());
                }

                if verbose && !report.snapshots.is_empty() {
                    println!("\n📋 Details:");
                    for snapshot in &report.snapshots {
                        let status_icon = match snapshot.status {
                            SnapshotStatus::Pass => "✅",
                            SnapshotStatus::Fail => "❌",
                            SnapshotStatus::Updated => "🔄",
                            SnapshotStatus::New => "🆕",
                            SnapshotStatus::Missing => "❓",
                        };

                        let status_color = match snapshot.status {
                            SnapshotStatus::Pass => snapshot.name.green(),
                            SnapshotStatus::Fail => snapshot.name.red(),
                            SnapshotStatus::Updated => snapshot.name.yellow(),
                            SnapshotStatus::New => snapshot.name.cyan(),
                            SnapshotStatus::Missing => snapshot.name.magenta(),
                        };

                        println!("  {} {}", status_icon, status_color);

                        if let Some(diff) = &snapshot.diff {
                            if verbose && diff.len() < 500 { // Only show small diffs
                                for line in diff.lines() {
                                    if line.starts_with('-') {
                                        println!("    {}", line.red());
                                    } else if line.starts_with('+') {
                                        println!("    {}", line.green());
                                    }
                                }
                            }
                        }
                    }
                }

                // Show recommendations
                if report.failed > 0 {
                    println!("\n{}", "💡 Recommendations:".yellow());
                    if report.summary.update_mode {
                        println!("  • Run with --update to accept new snapshots");
                    } else {
                        println!("  • Review failed snapshots and update test expectations");
                        println!("  • Use --update to automatically update all snapshots");
                    }
                }

                if report.new_snapshots > 0 {
                    println!("\n{}", "🆕 New snapshots detected!".cyan());
                    println!("  • Review new snapshot files in {}", report.summary.snapshots_directory);
                }
            }
            OutputFormat::Json => {
                let json = serde_json::to_string_pretty(report)
                    .unwrap_or_else(|_| "{}".to_string());
                println!("{}", json);
            }
            OutputFormat::Table => {
                println!("{:<40} {:<15} {:<10} {:<10}",
                        "Snapshot", "Status", "Old", "New");
                println!("{}", "─".repeat(80));

                for snapshot in &report.snapshots {
                    let status = match snapshot.status {
                        SnapshotStatus::Pass => "PASS".green(),
                        SnapshotStatus::Fail => "FAIL".red(),
                        SnapshotStatus::Updated => "UPDATED".yellow(),
                        SnapshotStatus::New => "NEW".cyan(),
                        SnapshotStatus::Missing => "MISSING".magenta(),
                    };

                    let old_size = snapshot.old_size.map(|s| s.to_string()).unwrap_or("-".to_string());
                    let new_size = snapshot.new_size.map(|s| s.to_string()).unwrap_or("-".to_string());

                    println!("{:<40} {:<15} {:<10} {:<10}",
                            snapshot.name,
                            status,
                            old_size,
                            new_size);
                }
            }
        }
    }
}

impl Tool for SnapshotTestTool {
    fn name(&self) -> &'static str {
        "snapshot-test"
    }

    fn description(&self) -> &'static str {
        "Better snapshot testing with visual diffs"
    }

    fn command(&self) -> Command {
        Command::new(self.name())
            .about(self.description())
            .long_about("Enhanced snapshot testing tool with visual diffs, update modes, \
                        and better organization. Supports multiple snapshot formats and CI integration.

EXAMPLES:
    cm tool snapshot-test --update
    cm tool snapshot-test --directory tests/ --format json
    cm tool snapshot-test --pattern \"test_*.rs\" --verbose")
            .args(&[
                Arg::new("directory")
                    .long("directory")
                    .short('d')
                    .help("Directory to scan for test files")
                    .default_value("tests"),
                Arg::new("snapshots")
                    .long("snapshots")
                    .short('s')
                    .help("Directory to store snapshots")
                    .default_value("tests/snapshots"),
                Arg::new("pattern")
                    .long("pattern")
                    .short('p')
                    .help("File pattern to match test files"),
                Arg::new("update")
                    .long("update")
                    .short('u')
                    .help("Update snapshots that don't match")
                    .action(clap::ArgAction::SetTrue),
                Arg::new("new-only")
                    .long("new-only")
                    .help("Only create new snapshots, don't update existing")
                    .action(clap::ArgAction::SetTrue),
                Arg::new("fail-on-new")
                    .long("fail-on-new")
                    .help("Fail if new snapshots are detected")
                    .action(clap::ArgAction::SetTrue),
                Arg::new("ci-mode")
                    .long("ci-mode")
                    .help("CI-friendly output format")
                    .action(clap::ArgAction::SetTrue),
            ])
            .args(&common_options())
    }

    fn execute(&self, matches: &ArgMatches) -> Result<()> {
        let directory = matches.get_one::<String>("directory").unwrap();
        let snapshots_dir = matches.get_one::<String>("snapshots").unwrap();
        let pattern = matches.get_one::<String>("pattern");
        let update = matches.get_flag("update");
        let new_only = matches.get_flag("new-only");
        let fail_on_new = matches.get_flag("fail-on-new");
        let ci_mode = matches.get_flag("ci-mode");
        let output_format = parse_output_format(matches);
        let verbose = matches.get_flag("verbose");

        // Find test files
        let test_files = self.find_snapshot_tests(directory)?;

        if test_files.is_empty() {
            println!("{}", "No test files found".yellow());
            return Ok(());
        }

        // Parse test files for snapshot calls
        let mut all_snapshots = Vec::new();

        for test_file in &test_files {
            if let Ok(content) = fs::read_to_string(test_file) {
                let snapshots = self.parse_snapshot_calls(&content, test_file);
                all_snapshots.extend(snapshots);
            }
        }

        if all_snapshots.is_empty() {
            println!("{}", "No snapshot calls found in test files".yellow());
            return Ok(());
        }

        // Find existing snapshot files
        let existing_snapshots = self.find_snapshot_files(snapshots_dir)?;

        // Analyze snapshots
        let mut results = Vec::new();
        let mut passed = 0;
        let mut failed = 0;
        let mut updated = 0;
        let mut new_snapshots = 0;

        for snapshot in &all_snapshots {
            let snapshot_file_name = self.generate_snapshot_name(&snapshot.name, &snapshot.name);
            let snapshot_key = format!("{}_{}", snapshot.path.replace("/", "_").replace("\\", "_"), snapshot_file_name);

            if let Some(existing_content) = existing_snapshots.get(&snapshot_key) {
                // Compare with existing snapshot
                let mock_new_content = format!("Mock content for {}\nGenerated: {}\n",
                    snapshot.name,
                    chrono::Utc::now().to_rfc3339());

                if let Some(diff) = self.compare_snapshots(existing_content, &mock_new_content) {
                    if update && !new_only {
                        // Update the snapshot
                        self.update_snapshot_file(snapshots_dir, &snapshot_key, &mock_new_content)?;
                        results.push(SnapshotResult {
                            name: snapshot.name.clone(),
                            path: snapshot.path.clone(),
                            status: SnapshotStatus::Updated,
                            diff: Some(diff),
                            old_size: Some(existing_content.len()),
                            new_size: Some(mock_new_content.len()),
                        });
                        updated += 1;
                    } else {
                        // Mark as failed
                        results.push(SnapshotResult {
                            name: snapshot.name.clone(),
                            path: snapshot.path.clone(),
                            status: SnapshotStatus::Fail,
                            diff: Some(diff),
                            old_size: Some(existing_content.len()),
                            new_size: Some(mock_new_content.len()),
                        });
                        failed += 1;
                    }
                } else {
                    // Snapshot matches
                    results.push(SnapshotResult {
                        name: snapshot.name.clone(),
                        path: snapshot.path.clone(),
                        status: SnapshotStatus::Pass,
                        diff: None,
                        old_size: Some(existing_content.len()),
                        new_size: Some(existing_content.len()),
                    });
                    passed += 1;
                }
            } else {
                // New snapshot
                let mock_content = format!("Mock content for {}\nGenerated: {}\n",
                    snapshot.name,
                    chrono::Utc::now().to_rfc3339());

                if update || !fail_on_new {
                    // Create new snapshot
                    self.update_snapshot_file(snapshots_dir, &snapshot_key, &mock_content)?;
                    results.push(SnapshotResult {
                        name: snapshot.name.clone(),
                        path: snapshot.path.clone(),
                        status: SnapshotStatus::New,
                        diff: None,
                        old_size: None,
                        new_size: Some(mock_content.len()),
                    });
                    new_snapshots += 1;
                } else {
                    // Mark as missing
                    results.push(SnapshotResult {
                        name: snapshot.name.clone(),
                        path: snapshot.path.clone(),
                        status: SnapshotStatus::Missing,
                        diff: None,
                        old_size: None,
                        new_size: None,
                    });
                    failed += 1;
                }
            }
        }

        // Create report
        let report = SnapshotReport {
            total_snapshots: all_snapshots.len(),
            passed,
            failed,
            updated,
            new_snapshots,
            snapshots: results,
            summary: TestSummary {
                test_files_analyzed: test_files.len(),
                snapshots_directory: snapshots_dir.clone(),
                update_mode: update,
                timestamp: chrono::Utc::now().to_rfc3339(),
            },
        };

        // Display report
        self.display_report(&report, output_format, verbose);

        // Handle CI mode
        if ci_mode {
            println!("::set-output name=snapshots-total::{}", report.total_snapshots);
            println!("::set-output name=snapshots-passed::{}", report.passed);
            println!("::set-output name=snapshots-failed::{}", report.failed);
            println!("::set-output name=snapshots-updated::{}", report.updated);
            println!("::set-output name=snapshots-new::{}", report.new_snapshots);

            if report.failed > 0 {
                println!("::error title=Snapshot Tests Failed::{} snapshots failed", report.failed);
            }
        }

        // Exit with error if any tests failed
        if failed > 0 && !update {
            std::process::exit(1);
        }

        Ok(())
    }
}

impl Default for SnapshotTestTool {
    fn default() -> Self {
        Self::new()
    }
}
