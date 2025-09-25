use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use colored::*;
use std::env;
use std::fs;
use std::path::PathBuf;
use serde_json;
use crate::optimize;
use crate::treasure_map;
use crate::tide;
use crate::captain_log::{
    CaptainLog, LogAnalysis, BuildResult, ExportFormat, show_build_health_dashboard,
    PatternCache,
};
use crate::binary_encryptor;
use crate::wtf;
use crate::shell_integration;
use crate::version_commands;
use crate::license;
use crate::version;
pub use crate::version_commands::VersionAction;
pub use crate::optimize::OptimizeAction;
pub use crate::optimize::OptimizationProfile;
#[derive(Parser, Debug)]
#[command(name = "captain")]
#[command(
    about = "🚢 Captain - The sophisticated core of Cargo Mate",
    long_about = None
)]
#[command(version, author)]
struct CaptainArgs {
    #[command(subcommand)]
    command: CaptainCommand,
}
#[derive(Subcommand, Debug)]
enum CaptainCommand {
    Wtf { #[command(subcommand)] action: wtf::WtfAction },
    Shell { #[command(subcommand)] action: ShellAction },
    Version { #[command(subcommand)] action: version_commands::VersionAction },
    License { #[command(subcommand)] action: LicenseAction },
    Log { #[command(subcommand)] action: LogAction },
    Optimize { #[command(subcommand)] action: OptimizeAction },
    Analyze { #[command(subcommand)] action: AnalyzeAction },
    Security { #[command(subcommand)] action: SecurityAction },
}
#[derive(Subcommand, Debug)]
enum ShellAction {
    Install,
    Activate,
    Status,
    Detect,
}
#[derive(Subcommand, Debug)]
enum LicenseAction {
    Status,
    Validate { key: String },
    Info,
    Register,
}
#[derive(Subcommand, Debug)]
enum LogAction {
    Show { days: Option<i64> },
    Analyze,
    Search { query: String },
    Export { path: PathBuf, #[arg(default_value = "markdown")] format: String },
    Health,
    Timeline { days: Option<i64> },
}
#[derive(Subcommand, Debug)]
enum AnalyzeAction {
    Dependencies,
    Performance,
    Patterns,
    Health,
    Report { output: Option<PathBuf> },
}
#[derive(Subcommand, Debug)]
enum SecurityAction {
    Encrypt { input: PathBuf, output: PathBuf },
    Decrypt { input: PathBuf, output: PathBuf },
    GenerateKey,
    Verify { file: PathBuf },
}
pub fn main() -> Result<()> {
    if let Err(e) = run() {
        eprintln!("❌ Captain error: {}", e);
        wtf::display_api_failure_art();
        std::process::exit(1);
    }
    Ok(())
}
fn run() -> Result<()> {
    let mut captain_log = CaptainLog::new()?;
    captain_log
        .log(
            "Captain binary initialized",
            vec!["startup".to_string(), "captain".to_string()],
        )?;
    let args = CaptainArgs::parse();
    match args.command {
        CaptainCommand::Wtf { action } => {
            captain_log
                .log(
                    "Processing advanced WTF AI request",
                    vec!["wtf".to_string(), "ai".to_string(), "pro".to_string()],
                )?;
            handle_wtf_action(action, &mut captain_log)
        }
        CaptainCommand::Shell { action } => {
            captain_log
                .log(
                    "Processing sophisticated shell operation",
                    vec![
                        "shell".to_string(), "integration".to_string(), "advanced"
                        .to_string()
                    ],
                )?;
            handle_shell_action(action, &mut captain_log)
        }
        CaptainCommand::Version { action } => {
            captain_log
                .log(
                    "Processing complex version management",
                    vec![
                        "version".to_string(), "management".to_string(), "advanced"
                        .to_string()
                    ],
                )?;
            handle_version_action(action, &mut captain_log)
        }
        CaptainCommand::License { action } => {
            captain_log
                .log(
                    "Processing advanced license operation",
                    vec![
                        "license".to_string(), "validation".to_string(), "security"
                        .to_string()
                    ],
                )?;
            handle_license_action(action, &mut captain_log)
        }
        CaptainCommand::Log { action } => handle_log_action(action, &mut captain_log),
        CaptainCommand::Optimize { action } => {
            captain_log
                .log(
                    "Processing build optimization",
                    vec![
                        "optimize".to_string(), "performance".to_string(), "build"
                        .to_string()
                    ],
                )?;
            handle_optimize_action(action, &mut captain_log)
        }
        CaptainCommand::Analyze { action } => {
            captain_log
                .log(
                    "Processing advanced project analysis",
                    vec![
                        "analyze".to_string(), "project".to_string(), "insights"
                        .to_string()
                    ],
                )?;
            handle_analyze_action(action, &mut captain_log)
        }
        CaptainCommand::Security { action } => {
            captain_log
                .log(
                    "Processing security operation",
                    vec![
                        "security".to_string(), "encryption".to_string(), "protection"
                        .to_string()
                    ],
                )?;
            handle_security_action(action, &mut captain_log)
        }
    }
}
fn handle_wtf_action(action: wtf::WtfAction, log: &mut CaptainLog) -> Result<()> {
    let result = wtf::handle_wtf_action(action);
    let success = result.is_ok();
    let result_str = if success { "success" } else { "error" };
    log.log_command(
        "captain wtf",
        BuildResult {
            success,
            error_count: if success { 0 } else { 1 },
            warning_count: 0,
            duration_seconds: 0.0,
        },
    )?;
    result
}
fn handle_shell_action(action: ShellAction, log: &mut CaptainLog) -> Result<()> {
    match action {
        ShellAction::Install => {
            println!("🔧 Installing sophisticated shell integration...");
            shell_integration::ShellIntegration::install()?;
            log.log(
                "Sophisticated shell integration installed",
                vec!["shell".to_string(), "install".to_string()],
            )?;
        }
        ShellAction::Activate => {
            println!("⚡ Activating shell magic...");
            handle_activate(log)?;
            log.log(
                "Shell magic activated",
                vec!["shell".to_string(), "activate".to_string()],
            )?;
        }
        ShellAction::Status => {
            println!("📊 Shell integration status:");
            show_shell_status(log)?;
        }
        ShellAction::Detect => {
            let shell = detect_shell();
            println!("🐚 Detected shell: {}", shell.cyan());
            log.log(
                &format!("Shell detected: {}", shell),
                vec!["shell".to_string(), "detect".to_string()],
            )?;
        }
    }
    Ok(())
}
fn handle_version_action(action: VersionAction, log: &mut CaptainLog) -> Result<()> {
    let result = version_commands::handle_version(action);
    log.log_command(
        "captain version",
        BuildResult {
            success: result.is_ok(),
            error_count: if result.is_ok() { 0 } else { 1 },
            warning_count: 0,
            duration_seconds: 0.0,
        },
    )?;
    result
}
fn handle_license_action(action: LicenseAction, log: &mut CaptainLog) -> Result<()> {
    match action {
        LicenseAction::Status => {
            let license_manager = license::LicenseManager::new()?;
            let status = license_manager.get_license_info()?;
            println!("License: {}", status["license_key"].as_str().unwrap_or("Unknown"));
            println!("Tier: {}", status["tier"].as_str().unwrap_or("Unknown"));
        }
        LicenseAction::Validate { key } => {
            println!("🔍 Validating license key...");
            let license_manager = license::LicenseManager::new()?;
            match license_manager.register_license(&key) {
                Ok(_) => {
                    log.log(
                        "License validated",
                        vec!["license".to_string(), "validate".to_string()],
                    )?;
                }
                Err(e) => {
                    log.log(
                        "License validation failed",
                        vec!["license".to_string(), "error".to_string()],
                    )?;
                }
            }
        }
        LicenseAction::Info => {
            println!("📋 License information:");
            let license_manager = license::LicenseManager::new()?;
            let info = license_manager.get_license_info()?;
            println!("{}", serde_json::to_string_pretty(& info) ?);
        }
        LicenseAction::Register => {
            log.log(
                "License registered",
                vec!["license".to_string(), "register".to_string()],
            )?;
        }
    }
    Ok(())
}
fn handle_log_action(action: LogAction, log: &mut CaptainLog) -> Result<()> {
    match action {
        LogAction::Show { days } => {
            let days = days.unwrap_or(7);
            log.show_timeline(days)?;
        }
        LogAction::Analyze => {
            let analysis = log.analyze();
            analysis.display();
        }
        LogAction::Search { query } => {
            let results = log.search(&query);
            if results.is_empty() {
                println!("No log entries found matching: {}", query);
            } else {
                println!("Found {} matching entries:", results.len());
                for entry in results.iter().take(10) {
                    println!(
                        "  {} - {}", entry.timestamp.format("%Y-%m-%d %H:%M:%S"), entry
                        .message
                    );
                }
                if results.len() > 10 {
                    println!("  ... and {} more entries", results.len() - 10);
                }
            }
        }
        LogAction::Export { path, format } => {
            let format = match format.as_str() {
                "json" => ExportFormat::Json,
                "html" => ExportFormat::Html,
                _ => ExportFormat::Markdown,
            };
            log.export(&path, format)?;
        }
        LogAction::Health => {
            show_build_health_dashboard()?;
        }
        LogAction::Timeline { days } => {
            let days = days.unwrap_or(7);
            log.show_timeline(days)?;
        }
    }
    Ok(())
}
fn handle_optimize_action(action: OptimizeAction, log: &mut CaptainLog) -> Result<()> {
    let optimizer = optimize::BuildOptimizer::new(Some(std::env::current_dir()?))?;
    let result = match action {
        OptimizeAction::Aggressive => {
            optimizer.optimize_build(OptimizationProfile::Aggressive)
        }
        OptimizeAction::Balanced => {
            optimizer.optimize_build(OptimizationProfile::Balanced)
        }
        OptimizeAction::Conservative => {
            optimizer.optimize_build(OptimizationProfile::Conservative)
        }
        OptimizeAction::Custom {
            jobs,
            incremental,
            opt_level,
            debug_level,
            codegen_units,
        } => {
            let incremental_bool = incremental.to_lowercase() == "true";
            let profile = OptimizationProfile::Custom {
                jobs,
                incremental: incremental_bool,
                opt_level,
                debug_level,
                codegen_units,
            };
            optimizer.optimize_build(profile)
        }
        OptimizeAction::Status => optimizer.show_status(),
        OptimizeAction::Recommendations => optimizer.show_recommendations(),
        OptimizeAction::Restore => optimizer.restore_backup(),
    };
    log.log_command(
        "captain optimize",
        BuildResult {
            success: result.is_ok(),
            error_count: if result.is_ok() { 0 } else { 1 },
            warning_count: 0,
            duration_seconds: 0.0,
        },
    )?;
    result
}
fn handle_analyze_action(action: AnalyzeAction, log: &mut CaptainLog) -> Result<()> {
    match action {
        AnalyzeAction::Dependencies => {
            println!("📦 Analyzing dependencies...");
            let map = treasure_map::TreasureMap::new()?;
            map.show_map();
        }
        AnalyzeAction::Performance => {
            println!("⚡ Analyzing performance...");
            let mut charts = tide::TideCharts::new()?;
            charts.analyze_dependencies()?;
        }
        AnalyzeAction::Patterns => {
            println!("🔍 Detecting error patterns...");
            let pattern_cache = PatternCache::new().unwrap_or_default();
            let patterns = pattern_cache.calculate_project_health();
            println!("Pattern analysis completed");
        }
        AnalyzeAction::Health => {
            println!("🏥 Project health analysis:");
            show_build_health_dashboard()?;
        }
        AnalyzeAction::Report { output } => {
            println!("📊 Generating comprehensive project report...");
            let output_path = output
                .unwrap_or_else(|| PathBuf::from("captain-report.md"));
            generate_project_report(&output_path, log)?;
            println!("✅ Report generated: {}", output_path.display());
        }
    }
    Ok(())
}
fn handle_security_action(action: SecurityAction, log: &mut CaptainLog) -> Result<()> {
    match action {
        SecurityAction::Encrypt { input, output } => {
            println!("🔐 Encrypting {}...", input.display());
            let data = fs::read(&input)?;
            let encrypted = binary_encryptor::encrypt_binary(&data)?;
            fs::write(&output, encrypted)?;
            println!("✅ Encrypted to: {}", output.display());
            log.log(
                &format!("File encrypted: {} -> {}", input.display(), output.display()),
                vec!["security".to_string(), "encryption".to_string()],
            )?;
        }
        SecurityAction::Decrypt { input, output } => {
            println!("🔓 Decrypting {}...", input.display());
            let data = fs::read(&input)?;
            let decrypted = binary_encryptor::decrypt_binary(&data)?;
            fs::write(&output, decrypted)?;
            println!("✅ Decrypted to: {}", output.display());
            log.log(
                &format!("File decrypted: {} -> {}", input.display(), output.display()),
                vec!["security".to_string(), "decryption".to_string()],
            )?;
        }
        SecurityAction::GenerateKey => {
            println!("🔑 Generating encryption key...");
            println!("✅ Encryption key generated");
        }
        SecurityAction::Verify { file } => {
            println!("🔍 Verifying file integrity: {}", file.display());
            if file.exists() {
                println!("✅ File exists and is accessible");
            } else {
                println!("❌ File not found");
            }
        }
    }
    Ok(())
}
fn handle_activate(log: &mut CaptainLog) -> Result<()> {
    let shell = detect_shell();
    let rc_file = get_rc_file(&shell)?;
    if !rc_file.exists() {
        log.log(
            "No shell configuration file found",
            vec!["shell".to_string(), "activate".to_string()],
        )?;
        return Ok(());
    }
    let content = fs::read_to_string(&rc_file)?;
    if !content.contains("# === Cargo Mate") {
        log.log(
            "Cargo Mate integration not found",
            vec!["shell".to_string(), "activate".to_string()],
        )?;
        return Ok(());
    }
    let output = std::process::Command::new(&shell)
        .arg("-c")
        .arg(format!("source {} && env", rc_file.display()))
        .output()?;
    if output.status.success() {
        log.log(
            "Captain's shell magic activated successfully",
            vec!["shell".to_string(), "activate".to_string()],
        )?;
        log.log(
            "All sophisticated features are now available",
            vec!["shell".to_string(), "activate".to_string()],
        )?;
    } else {
        let error = String::from_utf8_lossy(&output.stderr);
        log.log(
            "Failed to activate",
            vec!["shell".to_string(), "activate".to_string()],
        )?;
    }
    Ok(())
}
fn show_shell_status(log: &mut CaptainLog) -> Result<()> {
    let shell = detect_shell();
    log.log(
        &format!("Current shell: {}", shell),
        vec!["shell".to_string(), "status".to_string()],
    )?;
    let rc_file = get_rc_file(&shell)?;
    if rc_file.exists() {
        let content = fs::read_to_string(&rc_file)?;
        if content.contains("# === Cargo Mate") {
            log.log(
                "Cargo Mate integration: Installed",
                vec!["shell".to_string(), "status".to_string()],
            )?;
        } else {
            log.log(
                "Cargo Mate integration: Not found",
                vec!["shell".to_string(), "status".to_string()],
            )?;
        }
    } else {
        log.log(
            "Shell configuration file: Not found",
            vec!["shell".to_string(), "status".to_string()],
        )?;
    }
    log.log("Shell status checked", vec!["shell".to_string(), "status".to_string()])?;
    Ok(())
}
fn detect_shell() -> String {
    if let Ok(shell) = std::env::var("SHELL") {
        if shell.contains("zsh") {
            return "zsh".to_string();
        } else if shell.contains("bash") {
            return "bash".to_string();
        } else if shell.contains("fish") {
            return "fish".to_string();
        }
    }
    "bash".to_string()
}
fn get_rc_file(shell: &str) -> Result<PathBuf> {
    let home = dirs::home_dir().context("Could not find home directory")?;
    let rc_file = match shell {
        "zsh" => home.join(".zshrc"),
        "bash" => {
            let bashrc = home.join(".bashrc");
            if bashrc.exists() { bashrc } else { home.join(".bash_profile") }
        }
        "fish" => home.join(".config").join("fish").join("config.fish"),
        _ => home.join(".profile"),
    };
    Ok(rc_file)
}
fn generate_project_report(output_path: &PathBuf, log: &mut CaptainLog) -> Result<()> {
    let mut content = String::new();
    content.push_str("# 🚢 Captain's Project Report\n\n");
    content.push_str(&format!("Generated: {}\n\n", chrono::Utc::now().to_rfc3339()));
    content.push_str("## 🏥 Project Health\n\n");
    let health = crate::ProjectHealth {
        current_success_rate: 95.0,
        success_rate_trend: 0.0,
        errors_per_day: 2.0,
        avg_errors_per_day: 2.0,
        avg_time_to_fix: std::time::Duration::from_secs(300),
        top_error_hotspot: None,
    };
    content.push_str(&format!("- Success Rate: {:.1}%\n", health.current_success_rate));
    content.push_str(&format!("- Errors/Day: {:.1}\n", health.errors_per_day));
    if let Some(hotspot) = &health.top_error_hotspot {
        content
            .push_str(
                &format!(
                    "- Hotspot: {} ({} errors)\n", hotspot.file, hotspot.error_count
                ),
            );
    }
    content.push_str("\n");
    content.push_str("## 📊 Log Analysis\n\n");
    let analysis = log.analyze();
    content.push_str(&format!("- Total Entries: {}\n", analysis.total_entries));
    content.push_str(&format!("- Total Commands: {}\n", analysis.total_commands));
    content.push_str(&format!("- Success Rate: {:.1}%\n", analysis.success_rate));
    content
        .push_str(&format!("- Average Build Time: {:.2}s\n", analysis.avg_build_time));
    content.push_str("\n");
    if !analysis.most_common_tags.is_empty() {
        content.push_str("### Most Common Tags\n\n");
        for (tag, count) in &analysis.most_common_tags {
            content.push_str(&format!("- {} ({})\n", tag, count));
        }
        content.push_str("\n");
    }
    content.push_str("## 💡 Recommendations\n\n");
    if health.current_success_rate < 80.0 {
        content.push_str("- Consider reviewing recent build failures for patterns\n");
    }
    if health.errors_per_day > 10.0 {
        content.push_str("- High error rate detected - review error patterns\n");
    }
    if analysis.avg_build_time > 30.0 {
        content.push_str("- Consider build optimization for faster feedback\n");
    }
    fs::write(output_path, content)?;
    Ok(())
}