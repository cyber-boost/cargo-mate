use anyhow::{Context, Result};
use colored::*;
use regex::Regex;
use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
struct TestResult {
    command: String,
    flags: Vec<String>,
    success: bool,
    exit_code: Option<i32>,
    stdout: String,
    stderr: String,
    duration: Duration,
    error_message: Option<String>,
}

#[derive(Debug)]
struct BinaryTestReport {
    binary_path: PathBuf,
    binary_name: String,
    total_commands: usize,
    total_tests: usize,
    passed: usize,
    failed: usize,
    errors: usize,
    results: Vec<TestResult>,
    commands_found: Vec<String>,
    flags_found: BTreeMap<String, Vec<String>>,
    start_time: Instant,
    end_time: Option<Instant>,
}

fn find_binary(name: Option<&String>, path: Option<&PathBuf>) -> Result<PathBuf> {
    if let Some(p) = path {
        if p.exists() {
            return Ok(p.clone());
        }
        return Err(anyhow::anyhow!("Binary not found at path: {}", p.display()));
    }

    if let Some(n) = name {
        // Try to find binary in PATH
        if let Ok(output) = Command::new("which").arg(n).output() {
            if output.status.success() {
                let path_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !path_str.is_empty() {
                    return Ok(PathBuf::from(path_str));
                }
            }
        }

        // Try common locations
        let common_paths = vec![
            format!("/usr/bin/{}", n),
            format!("/usr/local/bin/{}", n),
            format!("./{}", n),
            format!("{}/.cargo/bin/{}", std::env::var("HOME").unwrap_or_default(), n),
        ];

        for p in common_paths {
            let path_buf = PathBuf::from(&p);
            if path_buf.exists() {
                return Ok(path_buf);
            }
        }

        return Err(anyhow::anyhow!("Binary '{}' not found in PATH or common locations", n));
    }

    Err(anyhow::anyhow!("Either --name or --path must be specified"))
}

fn run_command(
    binary: &Path,
    args: &[&str],
    timeout_seconds: u64,
) -> Result<(bool, Option<i32>, String, String, Duration)> {
    let start = Instant::now();
    let mut cmd = Command::new(binary);
    cmd.args(args);
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    let mut child = cmd.spawn()?;

    // Wait with timeout
    let timeout = Duration::from_secs(timeout_seconds);
    let mut timed_out = false;
    
    // Simple timeout check - try to wait, but don't block forever
    let output = loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let output = child.wait_with_output()?;
                break output;
            }
            Ok(None) => {
                if start.elapsed() > timeout {
                    let _ = child.kill();
                    timed_out = true;
                    // Wait for kill to complete and get output
                    // wait_with_output should work even after kill
                    match child.wait_with_output() {
                        Ok(output) => break output,
                        Err(_) => {
                            // Fallback: create a simple timeout indication
                            // The timed_out flag will handle the status
                            break std::process::Output {
                                status: std::process::ExitStatus::from_raw(1),
                                stdout: vec![],
                                stderr: b"Command timed out".to_vec(),
                            };
                        }
                    }
                }
                std::thread::sleep(Duration::from_millis(100));
            }
            Err(e) => {
                return Err(anyhow::anyhow!("Failed to wait for process: {}", e));
            }
        }
    };

    let duration = start.elapsed();
    let success = output.status.success() && !timed_out;
    let exit_code = if timed_out { None } else { output.status.code() };

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = if timed_out {
        "Command timed out".to_string()
    } else {
        String::from_utf8_lossy(&output.stderr).to_string()
    };

    Ok((success, exit_code, stdout, stderr, duration))
}

fn parse_help_output(help_text: &str) -> Vec<String> {
    let mut commands = Vec::new();

    // Common patterns for extracting commands from help
    let patterns = vec![
        // "COMMANDS:" section
        Regex::new(r"(?m)^\s{2,}([a-zA-Z0-9_-]+)\s+.*$").unwrap(),
        // "SUBCOMMANDS:" section
        Regex::new(r"(?m)^\s{2,}([a-zA-Z0-9_-]+)\s*$").unwrap(),
        // Lines starting with command names
        Regex::new(r"(?m)^\s{0,2}([a-zA-Z0-9_-]+)\s+[A-Z]").unwrap(),
    ];

    let lines: Vec<&str> = help_text.lines().collect();
    let mut in_commands_section = false;

    for line in lines {
        let line_lower = line.to_lowercase();
        if line_lower.contains("commands:") || line_lower.contains("subcommands:") {
            in_commands_section = true;
            continue;
        }

        if in_commands_section {
            if line.trim().is_empty() || line.starts_with("  ") == false {
                if !line.trim().is_empty() && !line_lower.contains("options:") && !line_lower.contains("flags:") {
                    in_commands_section = false;
                }
            } else {
                // Extract command name (first word)
                let parts: Vec<&str> = line.trim().split_whitespace().collect();
                if let Some(cmd) = parts.first() {
                    let cmd_clean = cmd.trim().to_string();
                    if !cmd_clean.is_empty()
                        && cmd_clean != "COMMANDS"
                        && cmd_clean != "SUBCOMMANDS"
                        && !commands.contains(&cmd_clean)
                    {
                        commands.push(cmd_clean);
                    }
                }
            }
        }

        // Also try regex patterns
        for pattern in &patterns {
            if let Some(caps) = pattern.captures(line) {
                if let Some(cmd) = caps.get(1) {
                    let cmd_str = cmd.as_str().to_string();
                    if !cmd_str.is_empty() && !commands.contains(&cmd_str) {
                        commands.push(cmd_str);
                    }
                }
            }
        }
    }

    // Remove common false positives
    commands.retain(|c| {
        !matches!(
            c.as_str(),
            "USAGE" | "OPTIONS" | "FLAGS" | "ARGS" | "COMMANDS" | "SUBCOMMANDS" | "EXAMPLES"
        )
    });

    commands
}

fn parse_flags_from_help(help_text: &str) -> Vec<String> {
    let mut flags = Vec::new();
    let flag_pattern = Regex::new(r"(-{1,2}[a-zA-Z0-9_-]+)").unwrap();

    for line in help_text.lines() {
        for cap in flag_pattern.captures_iter(line) {
            if let Some(flag) = cap.get(1) {
                let flag_str = flag.as_str().to_string();
                if !flags.contains(&flag_str) && flag_str != "--help" && flag_str != "-h" {
                    flags.push(flag_str);
                }
            }
        }
    }

    flags
}

fn test_binary_systematically(
    binary: &Path,
    timeout_seconds: u64,
    max_depth: Option<usize>,
) -> Result<BinaryTestReport> {
    let start_time = Instant::now();
    let binary_name = binary
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();

    println!("🔍 Analyzing binary: {}", binary_name.bright_cyan());
    println!("   Path: {}", binary.display().to_string().dimmed());
    println!();

    // Step 1: Get main help
    println!("📖 Step 1: Getting main help...");
    let (help_success, _, help_stdout, help_stderr, _) =
        run_command(binary, &["--help"], timeout_seconds)
            .or_else(|_| run_command(binary, &["-h"], timeout_seconds))
            .or_else(|_| run_command(binary, &["help"], timeout_seconds))?;

    if !help_success {
        println!("⚠️  Warning: Could not get help output");
    }

    let all_help = format!("{}\n{}", help_stdout, help_stderr);
    println!("   Help output length: {} bytes", all_help.len());

    // Step 2: Parse commands
    println!("🔎 Step 2: Parsing commands...");
    let mut commands = parse_help_output(&all_help);
    
    // Limit depth if specified
    if let Some(max) = max_depth {
        commands.truncate(max);
    }

    println!("   Found {} command(s)", commands.len());
    if !commands.is_empty() {
        println!("   Commands: {}", commands.join(", ").bright_white());
    }
    println!();

    // Step 3: For each command, get its help and parse flags
    println!("🚀 Step 3: Testing commands and flags...");
    let mut flags_map: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut results = Vec::new();

    for (cmd_idx, command) in commands.iter().enumerate() {
        println!("   [{}/{}] Testing command: {}", cmd_idx + 1, commands.len(), command.bright_yellow());

        // Get command help
        let (cmd_help_success, _, cmd_help_stdout, cmd_help_stderr, _) = run_command(
            binary,
            &[command.as_str(), "--help"],
            timeout_seconds,
        )
        .or_else(|_| run_command(binary, &[command.as_str(), "-h"], timeout_seconds))
        .or_else(|_| run_command(binary, &[command.as_str(), "help"], timeout_seconds))
        .unwrap_or_else(|_| (false, None, String::new(), String::new(), Duration::ZERO));

        let cmd_help = format!("{}\n{}", cmd_help_stdout, cmd_help_stderr);
        let flags = parse_flags_from_help(&cmd_help);
        flags_map.insert(command.clone(), flags.clone());

        // Test command without flags
        let (success, exit_code, stdout, stderr, duration) =
            run_command(binary, &[command.as_str()], timeout_seconds)?;

        results.push(TestResult {
            command: command.clone(),
            flags: vec![],
            success,
            exit_code,
            stdout: stdout.clone(),
            stderr: stderr.clone(),
            duration,
            error_message: if success { None } else { Some(stderr.clone()) },
        });

        // Test command with each flag
        for flag in &flags {
            let (success, exit_code, stdout, stderr, duration) =
                run_command(binary, &[command.as_str(), flag.as_str()], timeout_seconds)?;

            results.push(TestResult {
                command: command.clone(),
                flags: vec![flag.clone()],
                success,
                exit_code,
                stdout,
                stderr,
                duration,
                error_message: if success { None } else { Some(stderr.clone()) },
            });
        }

        // Test command with help flag
        if cmd_help_success {
            results.push(TestResult {
                command: command.clone(),
                flags: vec!["--help".to_string()],
                success: true,
                exit_code: Some(0),
                stdout: cmd_help_stdout,
                stderr: cmd_help_stderr,
                duration: Duration::ZERO,
                error_message: None,
            });
        }
    }

    let end_time = Some(Instant::now());
    let passed = results.iter().filter(|r| r.success).count();
    let failed = results.iter().filter(|r| !r.success).count();
    let errors = results.iter().filter(|r| r.error_message.is_some()).count();

    Ok(BinaryTestReport {
        binary_path: binary.to_path_buf(),
        binary_name,
        total_commands: commands.len(),
        total_tests: results.len(),
        passed,
        failed,
        errors,
        results,
        commands_found: commands,
        flags_found: flags_map,
        start_time,
        end_time,
    })
}

fn generate_report_markdown(report: &BinaryTestReport) -> String {
    let mut content = String::new();

    content.push_str("# 🔬 Binary Test Report\n\n");
    content.push_str(&format!("**Binary**: `{}`\n", report.binary_name));
    content.push_str(&format!("**Path**: `{}`\n", report.binary_path.display()));
    content.push_str(&format!("**Test Duration**: {:.2}s\n", 
        report.end_time.map(|e| (e - report.start_time).as_secs_f64()).unwrap_or(0.0)));
    content.push_str("\n");

    // Summary
    content.push_str("## Summary\n\n");
    content.push_str(&format!("- **Total Commands Found**: {}\n", report.total_commands));
    content.push_str(&format!("- **Total Tests Run**: {}\n", report.total_tests));
    content.push_str(&format!("- **✅ Passed**: {}\n", report.passed));
    content.push_str(&format!("- **❌ Failed**: {}\n", report.failed));
    content.push_str(&format!("- **⚠️ Errors**: {}\n", report.errors));
    content.push_str("\n");

    // Commands found
    if !report.commands_found.is_empty() {
        content.push_str("## Commands Found\n\n");
        for cmd in &report.commands_found {
            content.push_str(&format!("- `{}`\n", cmd));
        }
        content.push_str("\n");
    }

    // Flags found
    if !report.flags_found.is_empty() {
        content.push_str("## Flags by Command\n\n");
        for (cmd, flags) in &report.flags_found {
            if !flags.is_empty() {
                content.push_str(&format!("### `{}`\n\n", cmd));
                for flag in flags {
                    content.push_str(&format!("- `{}`\n", flag));
                }
                content.push_str("\n");
            }
        }
    }

    // Test results
    content.push_str("## Test Results\n\n");

    // Group by command
    let mut by_command: BTreeMap<String, Vec<&TestResult>> = BTreeMap::new();
    for result in &report.results {
        by_command.entry(result.command.clone()).or_insert_with(Vec::new).push(result);
    }

    for (command, cmd_results) in by_command {
        content.push_str(&format!("### Command: `{}`\n\n", command));

        for (idx, result) in cmd_results.iter().enumerate() {
            let status_icon = if result.success { "✅" } else { "❌" };
            let flag_str = if result.flags.is_empty() {
                "(no flags)".to_string()
            } else {
                result.flags.join(" ")
            };

            content.push_str(&format!("#### Test {}: `{} {}` {}\n\n", 
                idx + 1, command, flag_str, status_icon));
            
            content.push_str(&format!("- **Status**: {}\n", 
                if result.success { "PASS" } else { "FAIL" }));
            content.push_str(&format!("- **Exit Code**: {}\n", 
                result.exit_code.map(|c| c.to_string()).unwrap_or_else(|| "N/A".to_string())));
            content.push_str(&format!("- **Duration**: {:.3}s\n", result.duration.as_secs_f64()));

            if !result.stdout.is_empty() {
                content.push_str("\n**Stdout**:\n```\n");
                let stdout_preview = if result.stdout.len() > 500 {
                    format!("{}...\n(truncated)", &result.stdout[..500])
                } else {
                    result.stdout.clone()
                };
                content.push_str(&stdout_preview);
                content.push_str("\n```\n");
            }

            if !result.stderr.is_empty() {
                content.push_str("\n**Stderr**:\n```\n");
                let stderr_preview = if result.stderr.len() > 500 {
                    format!("{}...\n(truncated)", &result.stderr[..500])
                } else {
                    result.stderr.clone()
                };
                content.push_str(&stderr_preview);
                content.push_str("\n```\n");
            }

            if let Some(err_msg) = &result.error_message {
                content.push_str(&format!("\n**Error**: {}\n", err_msg));
            }

            content.push_str("\n---\n\n");
        }
    }

    // Footer
    let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
    content.push_str(&format!("---\n*Generated on {} by Cargo Mate*\n", timestamp));

    content
}

fn save_bin_history(content: &str, timestamp: &str, binary_name: &str) -> Result<PathBuf> {
    let home = dirs::home_dir()
        .context("Could not find home directory")?;
    let test_bin_dir = home.join(".shipwreck").join("test-bin");
    fs::create_dir_all(&test_bin_dir)?;

    let safe_name = binary_name.replace("/", "_").replace("\\", "_");
    let filename = format!("cm-bin-{}-{}.md", safe_name, timestamp);
    let history_path = test_bin_dir.join(&filename);
    fs::write(&history_path, content)?;

    Ok(history_path)
}

fn list_bin_history() -> Result<Vec<PathBuf>> {
    let home = dirs::home_dir()
        .context("Could not find home directory")?;
    let test_bin_dir = home.join(".shipwreck").join("test-bin");

    if !test_bin_dir.exists() {
        return Ok(Vec::new());
    }

    let mut reports: Vec<PathBuf> = fs::read_dir(&test_bin_dir)?
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path().is_file()
                && e.path().extension().map(|ext| ext == "md").unwrap_or(false)
                && e.path()
                    .file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n.starts_with("cm-bin-"))
                    .unwrap_or(false)
        })
        .map(|e| e.path())
        .collect();

    reports.sort();
    reports.reverse();
    Ok(reports)
}

pub fn handle_bin(
    action: Option<crate::cmd::smune::BinAction>,
    path: Option<PathBuf>,
    name: Option<String>,
    out: Option<PathBuf>,
    timeout_seconds: u64,
    max_depth: Option<usize>,
) -> Result<()> {
    // Handle subcommands
    if let Some(action) = action {
        match action {
            crate::cmd::smune::BinAction::History => {
                return handle_bin_history();
            }
            crate::cmd::smune::BinAction::Show { name } => {
                return handle_bin_show(&name);
            }
            crate::cmd::smune::BinAction::Find { query } => {
                return handle_bin_find(&query);
            }
            crate::cmd::smune::BinAction::Delete { all } => {
                return handle_bin_delete(all);
            }
        }
    }

    // Main testing functionality
    println!("{}", "🔬 Binary Systematic Testing".bright_cyan().bold());
    println!();

    let binary = find_binary(name.as_ref(), path.as_ref())?;

    let report = test_binary_systematically(&binary, timeout_seconds, max_depth)?;

    let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
    let report_content = generate_report_markdown(&report);

    // Determine output path
    let output_path = if let Some(out_path) = out {
        if out_path.is_absolute() {
            out_path
        } else {
            std::env::current_dir()?.join(out_path)
        }
    } else {
        std::env::current_dir()?.join(format!("cm-bin-{}-{}.md", report.binary_name, timestamp))
    };

    // Write to output file
    fs::write(&output_path, &report_content)
        .with_context(|| format!("Failed to write report to: {}", output_path.display()))?;

    // Save to history
    let history_path = save_bin_history(&report_content, &timestamp.to_string(), &report.binary_name)?;

    println!();
    println!("✅ Testing complete!");
    println!("   Output: {}", output_path.display().to_string().cyan());
    println!("   History: {}", history_path.display().to_string().dimmed());
    println!();
    println!("📊 Results:");
    println!("   Commands: {}", report.total_commands.to_string().bright_white());
    println!("   Tests: {}", report.total_tests.to_string().bright_white());
    println!("   ✅ Passed: {}", report.passed.to_string().bright_green());
    println!("   ❌ Failed: {}", report.failed.to_string().bright_red());
    println!("   ⚠️  Errors: {}", report.errors.to_string().bright_yellow());

    Ok(())
}

fn handle_bin_history() -> Result<()> {
    println!("{}", "📚 Binary Test History".bright_cyan().bold());
    println!();

    let reports = list_bin_history()?;

    if reports.is_empty() {
        println!("No test history found.");
        println!("💡 Test a binary with: cm bin --name <binary> or cm bin --path <path>");
        return Ok(());
    }

    println!("Found {} test report(s):\n", reports.len());

    for (i, report_path) in reports.iter().enumerate() {
        if let Some(file_name) = report_path.file_name().and_then(|n| n.to_str()) {
            println!("  {}. {}", i + 1, file_name.cyan());
            println!("     Path: {}", report_path.display().to_string().dimmed());
            println!();
        }
    }

    println!("💡 View a report with: cm bin show <name>");
    println!("💡 Search reports with: cm bin find <query>");
    println!("💡 Delete reports with: cm bin delete --all");

    Ok(())
}

fn handle_bin_show(name: &str) -> Result<()> {
    let reports = list_bin_history()?;

    let report_path = reports
        .iter()
        .find(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.contains(name))
                .unwrap_or(false)
        })
        .or_else(|| reports.first());

    match report_path {
        Some(path) => {
            let content = fs::read_to_string(path)?;
            println!("{}", content);
            Ok(())
        }
        None => Err(anyhow::anyhow!("Test report '{}' not found", name)),
    }
}

fn handle_bin_find(query: &str) -> Result<()> {
    println!("{}", format!("🔍 Searching for: {}", query).bright_cyan().bold());
    println!();

    let reports = list_bin_history()?;
    let mut found = Vec::new();

    for report_path in &reports {
        if let Ok(content) = fs::read_to_string(report_path) {
            if content.to_lowercase().contains(&query.to_lowercase()) {
                found.push(report_path.clone());
            }
        }
    }

    if found.is_empty() {
        println!("No reports found matching '{}'", query);
        return Ok(());
    }

    println!("Found {} matching report(s):\n", found.len());

    for (i, report_path) in found.iter().enumerate() {
        if let Some(file_name) = report_path.file_name().and_then(|n| n.to_str()) {
            println!("  {}. {}", i + 1, file_name.cyan());
            println!("     Path: {}", report_path.display().to_string().dimmed());
            println!();
        }
    }

    Ok(())
}

fn handle_bin_delete(all: bool) -> Result<()> {
    let reports = list_bin_history()?;

    if reports.is_empty() {
        println!("No test reports to delete.");
        return Ok(());
    }

    if all {
        println!("🗑️  Deleting all {} test report(s)...", reports.len());
        for report_path in &reports {
            fs::remove_file(report_path)?;
            println!("   Deleted: {}", report_path.file_name().and_then(|n| n.to_str()).unwrap_or("unknown"));
        }
        println!("✅ All test reports deleted.");
    } else {
        println!("💡 Use --all flag to delete all test reports");
        println!("   Example: cm bin delete --all");
    }

    Ok(())
}

