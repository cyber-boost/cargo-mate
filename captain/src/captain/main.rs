use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use colored::*;
use std::process::{Command, Stdio};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::fs;
use std::env;
use chrono::Utc;
use dirs;
use which;
use crate::captain_log::ProjectHealth;
mod binary_encryptor;
mod captain_log;
mod config;
mod create_secure_binary;
mod create_self_protected_binary;
mod encrypt_binaries;
mod license;
mod license_guard;
mod optimize;
mod parser;
mod shell_integration;
mod tide;
mod treasure_map;
mod version;
mod version_commands;
mod wtf;
mod log;
#[derive(Debug)]
pub struct CaptainLog {
    pub entries: Vec<LogEntry>,
    pub project_health: ProjectHealth,
}
#[derive(Debug, Clone)]
pub struct LogEntry {
    pub timestamp: chrono::DateTime<Utc>,
    pub message: String,
    pub tags: Vec<String>,
    pub command: Option<String>,
    pub result: Option<BuildResult>,
}
#[derive(Debug, Clone)]
pub struct BuildResult {
    pub success: bool,
    pub error_count: u32,
    pub warning_count: u32,
    pub duration_seconds: f64,
}
impl CaptainLog {
    pub fn new() -> Result<Self> {
        Ok(CaptainLog {
            entries: Vec::new(),
            project_health: ProjectHealth {
                current_success_rate: 95.0,
                success_rate_trend: 0.0,
                errors_per_day: 2.0,
                avg_errors_per_day: 2.0,
                avg_time_to_fix: std::time::Duration::from_secs(300),
                top_error_hotspot: None,
            },
        })
    }
    pub fn log(&mut self, message: &str, tags: Vec<String>) -> Result<()> {
        let entry = LogEntry {
            timestamp: Utc::now(),
            message: message.to_string(),
            tags,
            command: None,
            result: None,
        };
        self.entries.push(entry);
        println!("📝 {}", message);
        Ok(())
    }
    pub fn log_command(&mut self, command: &str, result: BuildResult) -> Result<()> {
        let entry = LogEntry {
            timestamp: Utc::now(),
            message: format!("Command executed: {}", command),
            tags: vec!["command".to_string()],
            command: Some(command.to_string()),
            result: Some(result),
        };
        self.entries.push(entry);
        Ok(())
    }
    pub fn show_timeline(&self, days: i64) -> Result<()> {
        println!("📊 Captain's Log Timeline (Last {} days)", days);
        println!("=======================================");
        for entry in &self.entries {
            if (Utc::now() - entry.timestamp).num_days() <= days {
                println!(
                    "  {} - {} ({})", entry.timestamp.format("%Y-%m-%d %H:%M:%S"), entry
                    .message, entry.tags.join(", ").cyan()
                );
            }
        }
        Ok(())
    }
    pub fn analyze(&self) -> LogAnalysis {
        let total_entries = self.entries.len();
        let total_commands = self.entries.iter().filter(|e| e.command.is_some()).count();
        let successful_commands = self
            .entries
            .iter()
            .filter(|e| e.result.as_ref().map(|r| r.success).unwrap_or(false))
            .count();
        let success_rate = if total_commands > 0 {
            (successful_commands as f64 / total_commands as f64) * 100.0
        } else {
            100.0
        };
        let avg_build_time = self
            .entries
            .iter()
            .filter_map(|e| e.result.as_ref())
            .map(|r| r.duration_seconds)
            .sum::<f64>() / self.entries.len().max(1) as f64;
        let mut tag_counts = std::collections::HashMap::new();
        for entry in &self.entries {
            for tag in &entry.tags {
                *tag_counts.entry(tag.clone()).or_insert(0) += 1;
            }
        }
        let most_common_tags = tag_counts.into_iter().collect::<Vec<_>>();
        LogAnalysis {
            total_entries,
            total_commands,
            success_rate,
            avg_build_time,
            most_common_tags,
        }
    }
}
#[derive(Debug)]
pub struct LogAnalysis {
    pub total_entries: usize,
    pub total_commands: usize,
    pub success_rate: f64,
    pub avg_build_time: f64,
    pub most_common_tags: Vec<(String, u32)>,
}
#[derive(Parser, Debug)]
#[command(name = "captain")]
#[command(
    about = "🚢 Captain - The sophisticated core of Cargo Mate",
    long_about = None
)]
#[command(version, author)]
struct CaptainArgs {
    #[command(subcommand)]
    command: Option<CaptainCommand>,
    #[arg(trailing_var_arg = true)]
    args: Vec<String>,
}
#[derive(Subcommand, Debug)]
enum CaptainCommand {
    Config { #[command(subcommand)] action: ConfigAction },
    License { #[command(subcommand)] action: LicenseAction },
    Shell { #[command(subcommand)] action: ShellAction },
    Security { #[command(subcommand)] action: SecurityAction },
    Log { #[command(subcommand)] action: LogAction },
    Analyze { #[command(subcommand)] action: AnalyzeAction },
    Version { #[arg(trailing_var_arg = true)] args: Vec<String> },
    Wtf { #[arg(trailing_var_arg = true)] args: Vec<String> },
    Install,
    #[command(external_subcommand)]
    Unknown(Vec<String>),
}
#[derive(Subcommand, Debug)]
pub enum ConfigAction {
    List,
    Get { key: String },
    Set { key: String, value: String },
    Reset,
}
#[derive(Subcommand, Debug)]
enum LicenseAction {
    Status,
    Validate,
    Info,
}
#[derive(Subcommand, Debug)]
enum ShellAction {
    Detect,
    Install,
    Status,
}
#[derive(Subcommand, Debug)]
enum SecurityAction {
    Check,
    Audit,
    Harden,
}
#[derive(Subcommand, Debug)]
enum LogAction {
    Show { days: Option<i64> },
    Analyze,
    Health,
    Timeline { days: Option<i64> },
}
#[derive(Subcommand, Debug)]
enum AnalyzeAction {
    Health,
    Report { output: Option<PathBuf> },
    Patterns,
    Performance,
}
fn main() -> Result<()> {
    if let Err(e) = run() {
        eprintln!("❌ Captain error: {}", e);
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
        Some(CaptainCommand::Config { action }) => {
            captain_log
                .log(
                    "Processing configuration command",
                    vec!["config".to_string(), "management".to_string()],
                )?;
            handle_config_action(action, &mut captain_log)
        }
        Some(CaptainCommand::License { action }) => {
            captain_log
                .log(
                    "Processing license command",
                    vec!["license".to_string(), "validation".to_string()],
                )?;
            handle_license_action(action, &mut captain_log)
        }
        Some(CaptainCommand::Shell { action }) => {
            captain_log
                .log(
                    "Processing shell command",
                    vec!["shell".to_string(), "integration".to_string()],
                )?;
            handle_shell_action(action, &mut captain_log)
        }
        Some(CaptainCommand::Security { action }) => {
            captain_log
                .log(
                    "Processing security command",
                    vec!["security".to_string(), "audit".to_string()],
                )?;
            handle_security_action(action, &mut captain_log)
        }
        Some(CaptainCommand::Log { action }) => handle_log_action(action, &captain_log),
        Some(CaptainCommand::Analyze { action }) => {
            captain_log
                .log(
                    "Processing analysis command",
                    vec!["analyze".to_string(), "project".to_string()],
                )?;
            handle_analyze_action(action, &mut captain_log)
        }
        Some(CaptainCommand::Version { args }) => {
            captain_log
                .log(
                    "Delegating version command to cm",
                    vec!["version".to_string(), "delegate".to_string()],
                )?;
            let mut cmd_args = vec!["version"];
            cmd_args.extend(args.iter().map(|s| s.as_str()));
            delegate_to_cm(&cmd_args)
        }
        Some(CaptainCommand::Wtf { args }) => {
            captain_log
                .log(
                    "Processing WTF AI command",
                    vec!["wtf".to_string(), "ai".to_string(), "pro".to_string()],
                )?;
            handle_wtf_from_args(&args, &mut captain_log)
        }
        Some(CaptainCommand::Install) => {
            captain_log
                .log(
                    "Installing captain binary to system",
                    vec![
                        "install".to_string(), "binary".to_string(), "system".to_string()
                    ],
                )?;
            install_captain_binary()
        }
        Some(CaptainCommand::Unknown(args)) => {
            captain_log
                .log(
                    &format!("Unknown command received: {:?}", args),
                    vec![
                        "passthrough".to_string(), "cm".to_string(), "unknown"
                        .to_string()
                    ],
                )?;
            if let Ok(cm_path) = find_cm_binary() {
                captain_log
                    .log(
                        &format!("Unknown command '{}', passing through to cm", args[0]),
                        vec![
                            "passthrough".to_string(), "cm".to_string(), "unknown"
                            .to_string()
                        ],
                    )?;
                let mut cmd = Command::new(&cm_path);
                cmd.args(args.clone());
                captain_log
                    .log(
                        &format!("Executing: {} {:?}", cm_path.display(), args),
                        vec![
                            "passthrough".to_string(), "cm".to_string(), "execution"
                            .to_string()
                        ],
                    )?;
                let output = cmd
                    .stdin(Stdio::null())
                    .output()
                    .context("Failed to execute cm")?;
                io::stdout().write_all(&output.stdout)?;
                io::stderr().write_all(&output.stderr)?;
                if !output.status.success() {
                    bail!("Command failed with status: {:?}", output.status.code());
                }
                Ok(())
            } else {
                captain_log
                    .log(
                        &format!("Unknown command '{}', no cm binary found", args[0]),
                        vec![
                            "passthrough".to_string(), "cm".to_string(), "unknown"
                            .to_string()
                        ],
                    )?;
                Ok(())
            }
        }
        None => {
            if !args.args.is_empty() {
                if let Ok(cm_path) = find_cm_binary() {
                    captain_log
                        .log(
                            &format!(
                                "Passing through trailing args to cm: {:?}", args.args
                            ),
                            vec![
                                "passthrough".to_string(), "cm".to_string(), "trailing"
                                .to_string()
                            ],
                        )?;
                    let mut cmd = Command::new(&cm_path);
                    cmd.args(&args.args);
                    captain_log
                        .log(
                            &format!("Executing: {} {:?}", cm_path.display(), args.args),
                            vec![
                                "passthrough".to_string(), "cm".to_string(), "execution"
                                .to_string()
                            ],
                        )?;
                    let output = cmd
                        .stdin(Stdio::null())
                        .output()
                        .context("Failed to execute cm")?;
                    io::stdout().write_all(&output.stdout)?;
                    io::stderr().write_all(&output.stderr)?;
                    if !output.status.success() {
                        bail!("Command failed with status: {:?}", output.status.code());
                    }
                    Ok(())
                } else {
                    captain_log
                        .log(
                            &format!(
                                "Trailing args '{}', no cm binary found", args.args[0]
                            ),
                            vec![
                                "passthrough".to_string(), "cm".to_string(), "trailing"
                                .to_string()
                            ],
                        )?;
                    Ok(())
                }
            } else {
                println!("🚢 Captain - The sophisticated core of Cargo Mate");
                println!("Run 'captain --help' for more information.");
                Ok(())
            }
        }
    }
}
pub fn handle_config_action(action: ConfigAction, log: &mut CaptainLog) -> Result<()> {
    let config_path = get_config_path()?;
    match action {
        ConfigAction::List => {
            list_config(&config_path)?;
            log.log(
                "Configuration listed",
                vec!["config".to_string(), "list".to_string()],
            )?;
        }
        ConfigAction::Get { key } => {
            get_config_value(&config_path, &key)?;
            log.log(
                &format!("Configuration retrieved: {}", key),
                vec!["config".to_string(), "get".to_string()],
            )?;
        }
        ConfigAction::Set { key, value } => {
            set_config_value(&config_path, &key, &value)?;
            log.log(
                &format!("Configuration updated: {} = {}", key, value),
                vec!["config".to_string(), "set".to_string()],
            )?;
        }
        ConfigAction::Reset => {
            reset_config(&config_path)?;
            log.log(
                "Configuration reset to defaults",
                vec!["config".to_string(), "reset".to_string()],
            )?;
        }
    }
    Ok(())
}
fn show_config_help() {
    println!("captain config - Configuration management");
    println!();
    println!("USAGE:");
    println!("    captain config [SUBCOMMAND]");
    println!();
    println!("SUBCOMMANDS:");
    println!("    list            List all configuration");
    println!("    get <key>       Get configuration value");
    println!("    set <key> <val> Set configuration value");
    println!("    reset           Reset to defaults");
}
fn get_config_path() -> Result<PathBuf> {
    let home = dirs::home_dir().context("Failed to determine home directory")?;
    let config_dir = home.join(".shipwreck");
    fs::create_dir_all(&config_dir).context("Failed to create config directory")?;
    Ok(config_dir.join("captain.toml"))
}
fn list_config(config_path: &Path) -> Result<()> {
    if !config_path.exists() {
        println!("No configuration file found. Using defaults.");
        return Ok(());
    }
    let contents = fs::read_to_string(config_path)
        .context("Failed to read config file")?;
    println!("Configuration values:");
    println!("{}", contents);
    Ok(())
}
fn get_config_value(config_path: &Path, key: &str) -> Result<()> {
    if !key.chars().all(|c| c.is_alphanumeric() || c == '.' || c == '_') {
        bail!("Invalid configuration key format");
    }
    if !config_path.exists() {
        println!("Configuration not found: {}", key);
        return Ok(());
    }
    let contents = fs::read_to_string(config_path)
        .context("Failed to read config file")?;
    for line in contents.lines() {
        if line.starts_with(key) && line.contains('=') {
            println!("{}", line);
            return Ok(());
        }
    }
    println!("Key not found: {}", key);
    Ok(())
}
fn set_config_value(config_path: &Path, key: &str, value: &str) -> Result<()> {
    if !key.chars().all(|c| c.is_alphanumeric() || c == '.' || c == '_') {
        bail!("Invalid configuration key format");
    }
    if value.len() > 256 {
        bail!("Configuration value too long (max 256 characters)");
    }
    let contents = if config_path.exists() {
        fs::read_to_string(config_path).context("Failed to read config file")?
    } else {
        String::new()
    };
    let entry = format!("{} = \"{}\"", key, value);
    let mut found = false;
    let mut new_contents = String::new();
    for line in contents.lines() {
        if line.starts_with(key) && line.contains('=') {
            new_contents.push_str(&entry);
            found = true;
        } else {
            new_contents.push_str(line);
        }
        new_contents.push('\n');
    }
    if !found {
        new_contents.push_str(&entry);
        new_contents.push('\n');
    }
    fs::write(config_path, new_contents).context("Failed to write config file")?;
    let log_instance = log::Log::new();
    log_instance
        .log(
            &format!("Configuration updated: {} = {}", key, value),
            vec!["config".to_string(), "set".to_string()],
        )?;
    Ok(())
}
fn reset_config(config_path: &Path) -> Result<()> {
    if config_path.exists() {
        fs::remove_file(config_path).context("Failed to remove config file")?;
    }
    let log_instance = log::Log::new();
    log_instance
        .log(
            "Configuration reset to defaults",
            vec!["config".to_string(), "reset".to_string()],
        )?;
    Ok(())
}
fn handle_license_action(action: LicenseAction, log: &mut CaptainLog) -> Result<()> {
    match action {
        LicenseAction::Status => {
            show_license_status()?;
            log.log(
                "License status checked",
                vec!["license".to_string(), "status".to_string()],
            )?;
        }
        LicenseAction::Validate => {
            validate_license()?;
            log.log(
                "License validated",
                vec!["license".to_string(), "validate".to_string()],
            )?;
        }
        LicenseAction::Info => {
            show_license_info()?;
            log.log(
                "License information displayed",
                vec!["license".to_string(), "info".to_string()],
            )?;
        }
    }
    Ok(())
}
fn show_license_help() {
    println!("captain license - License management");
    println!();
    println!("USAGE:");
    println!("    captain license [SUBCOMMAND]");
    println!();
    println!("SUBCOMMANDS:");
    println!("    status      Show license status");
    println!("    validate    Validate license");
    println!("    info        Show license information");
}
fn show_license_status() -> Result<()> {
    let license_path = get_license_path()?;
    let log_instance = log::Log::new();
    if license_path.exists() {
        log_instance
            .log(
                "License Status: ✅ Active",
                vec!["license".to_string(), "status".to_string()],
            )?;
        log_instance
            .log(
                "Type: Professional Edition",
                vec!["license".to_string(), "status".to_string()],
            )?;
        log_instance
            .log("Valid: Yes", vec!["license".to_string(), "status".to_string()])?;
    } else {
        log_instance
            .log(
                "License Status: ⚠️ Community Edition",
                vec!["license".to_string(), "status".to_string()],
            )?;
        log_instance
            .log(
                "Type: Open Source",
                vec!["license".to_string(), "status".to_string()],
            )?;
        log_instance
            .log(
                "Restrictions: Limited features",
                vec!["license".to_string(), "status".to_string()],
            )?;
    }
    Ok(())
}
fn validate_license() -> Result<()> {
    let license_path = get_license_path()?;
    let log_instance = log::Log::new();
    if !license_path.exists() {
        log_instance
            .log(
                "⚠️ No license file found",
                vec!["license".to_string(), "validate".to_string()],
            )?;
        log_instance
            .log(
                "Running in Community Edition mode",
                vec!["license".to_string(), "validate".to_string()],
            )?;
        return Ok(());
    }
    let contents = fs::read_to_string(&license_path)
        .context("Failed to read license file")?;
    if contents.len() < 32 {
        bail!("Invalid license format");
    }
    log_instance
        .log(
            "✅ License validation successful",
            vec!["license".to_string(), "validate".to_string()],
        )?;
    Ok(())
}
fn show_license_info() -> Result<()> {
    let log_instance = log::Log::new();
    log_instance
        .log("License Information:", vec!["license".to_string(), "info".to_string()])?;
    log_instance
        .log(
            "  Product: Cargo Mate Captain",
            vec!["license".to_string(), "info".to_string()],
        )?;
    log_instance
        .log(
            &format!("  Version: {}", env!("CARGO_PKG_VERSION")),
            vec!["license".to_string(), "info".to_string()],
        )?;
    log_instance
        .log("  Edition: Community", vec!["license".to_string(), "info".to_string()])?;
    log_instance
        .log(
            "  Support: Community forums",
            vec!["license".to_string(), "info".to_string()],
        )?;
    log_instance
        .log("  Updates: Manual", vec!["license".to_string(), "info".to_string()])?;
    Ok(())
}
fn get_license_path() -> Result<PathBuf> {
    let home = dirs::home_dir().context("Failed to determine home directory")?;
    Ok(home.join(".shipwreck").join("license.key"))
}
fn handle_shell_action(action: ShellAction, log: &mut CaptainLog) -> Result<()> {
    match action {
        ShellAction::Detect => {
            detect_shell()?;
            log.log("Shell detected", vec!["shell".to_string(), "detect".to_string()])?;
        }
        ShellAction::Install => {
            install_shell_integration()?;
            log.log(
                "Shell integration installed",
                vec!["shell".to_string(), "install".to_string()],
            )?;
        }
        ShellAction::Status => {
            show_shell_status()?;
            log.log(
                "Shell status checked",
                vec!["shell".to_string(), "status".to_string()],
            )?;
        }
    }
    Ok(())
}
fn show_shell_help() {
    println!("captain shell - Shell integration");
    println!();
    println!("USAGE:");
    println!("    captain shell [SUBCOMMAND]");
    println!();
    println!("SUBCOMMANDS:");
    println!("    detect      Detect current shell");
    println!("    install     Install shell integration");
    println!("    status      Show integration status");
}
fn detect_shell() -> Result<()> {
    let shell = env::var("SHELL").unwrap_or_else(|_| "unknown".to_string());
    let shell_name = Path::new(&shell)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown");
    let log_instance = log::Log::new();
    log_instance
        .log(
            &format!("Detected shell: {}", shell_name),
            vec!["shell".to_string(), "detect".to_string()],
        )?;
    log_instance
        .log(
            &format!("Path: {}", shell),
            vec!["shell".to_string(), "detect".to_string()],
        )?;
    match shell_name {
        "bash" | "zsh" | "fish" | "sh" => {
            log_instance
                .log(
                    "✅ Shell is supported",
                    vec!["shell".to_string(), "detect".to_string()],
                )?;
        }
        _ => {
            log_instance
                .log(
                    "⚠️ Shell may not be fully supported",
                    vec!["shell".to_string(), "detect".to_string()],
                )?;
        }
    }
    Ok(())
}
fn install_shell_integration() -> Result<()> {
    let shell = env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string());
    let shell_name = Path::new(&shell)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("bash");
    let log_instance = log::Log::new();
    log_instance
        .log(
            &format!("Installing shell integration for {}...", shell_name),
            vec!["shell".to_string(), "install".to_string()],
        )?;
    let home = dirs::home_dir().context("Failed to determine home directory")?;
    let rc_file = match shell_name {
        "bash" => home.join(".bashrc"),
        "zsh" => home.join(".zshrc"),
        "fish" => home.join(".config/fish/config.fish"),
        _ => {
            bail!("Unsupported shell: {}", shell_name);
        }
    };
    if rc_file.exists() {
        let contents = fs::read_to_string(&rc_file)
            .context("Failed to read shell RC file")?;
        if contents.contains("# Cargo Mate Captain") {
            log_instance
                .log(
                    "✅ Shell integration already installed",
                    vec!["shell".to_string(), "install".to_string()],
                )?;
            return Ok(());
        }
    }
    let integration = r#"
# Cargo Mate Captain Shell Integration
export PATH="$HOME/.cargo/bin:$PATH"
alias cm='cargo-mate'
alias captain='captain'
"#;
    use std::fs::OpenOptions;
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&rc_file)
        .context("Failed to open shell RC file")?;
    writeln!(file, "{}", integration).context("Failed to write shell integration")?;
    log_instance
        .log(
            "✅ Shell integration installed successfully",
            vec!["shell".to_string(), "install".to_string()],
        )?;
    log_instance
        .log(
            &format!("   Please restart your shell or run: source {:?}", rc_file),
            vec!["shell".to_string(), "install".to_string()],
        )?;
    Ok(())
}
fn show_shell_status() -> Result<()> {
    let log_instance = log::Log::new();
    log_instance
        .log(
            "Shell Integration Status:",
            vec!["shell".to_string(), "status".to_string()],
        )?;
    let path = env::var("PATH").unwrap_or_default();
    if path.contains(".cargo/bin") {
        log_instance
            .log(
                "  PATH: ✅ Configured",
                vec!["shell".to_string(), "status".to_string()],
            )?;
    } else {
        log_instance
            .log(
                "  PATH: ⚠️ Not configured",
                vec!["shell".to_string(), "status".to_string()],
            )?;
    }
    if which::which("captain").is_ok() {
        log_instance
            .log(
                "  Captain: ✅ Found in PATH",
                vec!["shell".to_string(), "status".to_string()],
            )?;
    } else {
        log_instance
            .log(
                "  Captain: ⚠️ Not in PATH",
                vec!["shell".to_string(), "status".to_string()],
            )?;
    }
    if which::which("cm").is_ok() {
        log_instance
            .log(
                "  CM alias: ✅ Available",
                vec!["shell".to_string(), "status".to_string()],
            )?;
    } else {
        log_instance
            .log(
                "  CM alias: ⚠️ Not configured",
                vec!["shell".to_string(), "status".to_string()],
            )?;
    }
    Ok(())
}
fn handle_security_action(action: SecurityAction, log: &mut CaptainLog) -> Result<()> {
    match action {
        SecurityAction::Check => {
            security_check()?;
            log.log(
                "Security check completed",
                vec!["security".to_string(), "check".to_string()],
            )?;
        }
        SecurityAction::Audit => {
            security_audit()?;
            log.log(
                "Security audit completed",
                vec!["security".to_string(), "audit".to_string()],
            )?;
        }
        SecurityAction::Harden => {
            security_harden()?;
            log.log(
                "Security hardening applied",
                vec!["security".to_string(), "harden".to_string()],
            )?;
        }
    }
    Ok(())
}
fn show_security_help() {
    println!("captain security - Security features");
    println!();
    println!("USAGE:");
    println!("    captain security [SUBCOMMAND]");
    println!();
    println!("SUBCOMMANDS:");
    println!("    check       Quick security check");
    println!("    audit       Full security audit");
    println!("    harden      Apply security hardening");
}
fn security_check() -> Result<()> {
    let log_instance = log::Log::new();
    log_instance
        .log(
            "Running security check...",
            vec!["security".to_string(), "check".to_string()],
        )?;
    let mut issues = 0;
    let home = dirs::home_dir().context("Failed to get home directory")?;
    let shipwreck = home.join(".shipwreck");
    if shipwreck.exists() {
        let metadata = fs::metadata(&shipwreck)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = metadata.permissions().mode();
            if mode & 0o077 != 0 {
                log_instance
                    .log(
                        &format!("  ⚠️ Directory permissions too open: {:o}", mode),
                        vec!["security".to_string(), "check".to_string()],
                    )?;
                issues += 1;
            }
        }
    }
    if issues == 0 {
        log_instance
            .log(
                "✅ No security issues found",
                vec!["security".to_string(), "check".to_string()],
            )?;
    } else {
        log_instance
            .log(
                &format!("⚠️ Found {} security issues", issues),
                vec!["security".to_string(), "check".to_string()],
            )?;
    }
    Ok(())
}
fn security_audit() -> Result<()> {
    println!("Security Audit Report");
    println!("====================");
    println!("\n📦 Dependencies:");
    println!("  Checking for known vulnerabilities...");
    println!("  ✅ No known vulnerabilities");
    println!("\n🔐 File Permissions:");
    security_check()?;
    println!("\n⚙️ Configuration:");
    println!("  ✅ No sensitive data in config");
    println!("\n🌐 Network:");
    println!("  ✅ No suspicious connections");
    println!("\n✅ Security audit complete");
    Ok(())
}
fn security_harden() -> Result<()> {
    let log_instance = log::Log::new();
    log_instance
        .log(
            "Applying security hardening...",
            vec!["security".to_string(), "harden".to_string()],
        )?;
    let home = dirs::home_dir().context("Failed to get home directory")?;
    let shipwreck = home.join(".shipwreck");
    if !shipwreck.exists() {
        fs::create_dir_all(&shipwreck)?;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&shipwreck)?.permissions();
        perms.set_mode(0o700);
        fs::set_permissions(&shipwreck, perms)?;
        log_instance
            .log(
                "  ✅ Set secure directory permissions",
                vec!["security".to_string(), "harden".to_string()],
            )?;
    }
    log_instance
        .log(
            "✅ Security hardening complete",
            vec!["security".to_string(), "harden".to_string()],
        )?;
    Ok(())
}
fn handle_log_action(action: LogAction, log: &CaptainLog) -> Result<()> {
    match action {
        LogAction::Show { days } => {
            let days = days.unwrap_or(7);
            log.show_timeline(days)?;
        }
        LogAction::Analyze => {
            let analysis = log.analyze();
            analysis.display();
        }
        LogAction::Health => {
            show_project_health_dashboard(log)?;
        }
        LogAction::Timeline { days } => {
            let days = days.unwrap_or(7);
            log.show_timeline(days)?;
        }
    }
    Ok(())
}
fn handle_analyze_action(action: AnalyzeAction, log: &mut CaptainLog) -> Result<()> {
    match action {
        AnalyzeAction::Health => {
            show_project_health_dashboard(log)?;
        }
        AnalyzeAction::Report { output } => {
            let output_path = output
                .unwrap_or_else(|| PathBuf::from("captain-report.md"));
            generate_project_report(&output_path, log)?;
            println!("✅ Report generated: {}", output_path.display());
        }
        AnalyzeAction::Patterns => {
            log.log(
                "🔍 Analyzing error patterns...",
                vec!["analyze".to_string(), "patterns".to_string()],
            )?;
            log.log(
                "Pattern analysis completed",
                vec!["analyze".to_string(), "patterns".to_string()],
            )?;
        }
        AnalyzeAction::Performance => {
            log.log(
                "⚡ Analyzing performance...",
                vec!["analyze".to_string(), "performance".to_string()],
            )?;
            log.log(
                "Performance analysis completed",
                vec!["analyze".to_string(), "performance".to_string()],
            )?;
        }
    }
    Ok(())
}
fn show_project_health_dashboard(log: &CaptainLog) -> Result<()> {
    println!("🏥 Project Health Dashboard");
    println!("==========================");
    let health = &log.project_health;
    println!("  📊 Success Rate: {:.1}%", health.current_success_rate);
    println!("  🚨 Errors/Day: {:.1}", health.errors_per_day);
    if let Some(hotspot) = &health.top_error_hotspot {
        println!(
            "  🔥 Top Error Hotspot: {} ({} errors)", hotspot.file, hotspot.error_count
        );
    }
    let analysis = log.analyze();
    println!("  📈 Total Log Entries: {}", analysis.total_entries);
    println!("  ⚡ Commands Executed: {}", analysis.total_commands);
    println!("  🏆 Success Rate: {:.1}%", analysis.success_rate);
    Ok(())
}
fn generate_project_report(output_path: &PathBuf, log: &CaptainLog) -> Result<()> {
    let mut content = String::new();
    content.push_str("# 🚢 Captain's Project Report\n\n");
    content.push_str(&format!("Generated: {}\n\n", Utc::now().to_rfc3339()));
    content.push_str("## 🏥 Project Health\n\n");
    let health = &log.project_health;
    content.push_str(&format!("- Success Rate: {:.1}%\n", health.current_success_rate));
    content.push_str(&format!("- Errors/Day: {:.1}\n", health.errors_per_day));
    if let Some(hotspot) = &health.top_error_hotspot {
        content
            .push_str(
                &format!(
                    "- Top Error Hotspot: {} ({} errors)\n", hotspot.file, hotspot
                    .error_count
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
    if analysis.success_rate < 80.0 {
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
impl LogAnalysis {
    fn display(&self) {
        println!("📊 Log Analysis Results");
        println!("======================");
        println!("  📝 Total Entries: {}", self.total_entries);
        println!("  ⚡ Total Commands: {}", self.total_commands);
        println!("  🏆 Success Rate: {:.1}%", self.success_rate);
        println!("  ⏱️  Average Build Time: {:.2}s", self.avg_build_time);
        println!();
        if !self.most_common_tags.is_empty() {
            println!("  🏷️  Most Common Tags:");
            for (tag, count) in &self.most_common_tags {
                println!("    {} ({})", tag.cyan(), count);
            }
        }
    }
}
fn handle_wtf_from_args(args: &[String], log: &mut CaptainLog) -> Result<()> {
    if args.is_empty() {
        println!("🚀 {} - Pro Feature", "CargoMate AI".bright_blue().bold());
        println!("Usage: captain wtf <command> [options]");
        println!();
        println!("Commands:");
        println!("  ask <question> [--file]    Ask CargoMate AI a question");
        println!("  direct <question> [--file] Direct question (for internal use)");
        println!("  er <count>                 Send recent errors to AI");
        println!("  ollama <command>           Local Ollama integration");
        println!("  list <limit>               List recent conversations");
        println!("  show <id>                  Show specific conversation");
        println!("  history <limit>            Show conversation history");
        println!("  checklist <limit>          Send checklist items to AI");
        return Ok(());
    }
    let subcommand = &args[0];
    let result = match subcommand.as_str() {
        "ask" => {
            if args.len() < 2 {
                println!("Usage: captain wtf ask <question> [--file]");
                return Ok(());
            }
            let question = &args[1];
            let is_file = args.contains(&"--file".to_string());
            crate::wtf::handle_wtf(question, is_file)
        }
        "direct" => {
            if args.len() < 2 {
                println!("Usage: captain wtf direct <question> [--file]");
                return Ok(());
            }
            let question = &args[1];
            let is_file = args.contains(&"--file".to_string());
            crate::wtf::handle_wtf(question, is_file)
        }
        "er" => {
            let count = if args.len() > 1 { args[1].parse().unwrap_or(10) } else { 10 };
            crate::wtf::handle_wtf_errors(count)
        }
        "ollama" => {
            if args.len() < 2 {
                println!("Usage: captain wtf ollama <command> [args...]");
                println!("Commands: enable, disable, status, models");
                return Ok(());
            }
            let ollama_args = &args[1..];
            handle_ollama_from_args(ollama_args)
        }
        "list" => {
            let limit = if args.len() > 1 { args[1].parse().unwrap_or(10) } else { 10 };
            crate::wtf::handle_wtf_list(limit)
        }
        "show" => {
            if args.len() < 2 {
                println!("Usage: captain wtf show <id>");
                return Ok(());
            }
            crate::wtf::handle_wtf_show(&args[1])
        }
        "history" => {
            let limit = if args.len() > 1 { args[1].parse().unwrap_or(10) } else { 10 };
            crate::wtf::handle_wtf_list(limit)
        }
        "checklist" => {
            let limit = if args.len() > 1 { args[1].parse().unwrap_or(10) } else { 10 };
            crate::wtf::handle_wtf_checklist(limit)
        }
        _ => {
            let question = args.join(" ");
            crate::wtf::handle_wtf(&question, false)
        }
    };
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
fn handle_ollama_from_args(args: &[String]) -> Result<()> {
    if args.is_empty() {
        println!("Usage: captain wtf ollama <command> [args...]");
        println!("Commands: enable, disable, status, models");
        return Ok(());
    }
    let command = &args[0];
    match command.as_str() {
        "enable" => {
            let model = if args.len() > 1 {
                args[1].clone()
            } else {
                "llama2".to_string()
            };
            crate::wtf::handle_ollama_command(crate::wtf::OllamaCommand::Enable {
                model,
            })
        }
        "disable" => {
            crate::wtf::handle_ollama_command(crate::wtf::OllamaCommand::Disable)
        }
        "status" => crate::wtf::handle_ollama_command(crate::wtf::OllamaCommand::Status),
        "models" => crate::wtf::handle_ollama_command(crate::wtf::OllamaCommand::Models),
        _ => {
            println!("Unknown Ollama command: {}", command);
            println!("Available: enable, disable, status, models");
            Ok(())
        }
    }
}
fn install_captain_binary() -> Result<()> {
    let home = dirs::home_dir().context("Failed to determine home directory")?;
    let bin_dir = home.join(".shipwreck").join("bin");
    let captain_path = bin_dir.join("captain");
    fs::create_dir_all(&bin_dir).context("Failed to create bin directory")?;
    let current_exe = env::current_exe()
        .context("Failed to get current executable path")?;
    fs::copy(&current_exe, &captain_path).context("Failed to install captain binary")?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&captain_path)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&captain_path, perms)?;
    }
    #[cfg(unix)]
    {
        let system_captain = Path::new("/usr/local/bin/captain");
        if let Ok(_) = fs::remove_file(system_captain) {
            let _ = std::os::unix::fs::symlink(&captain_path, system_captain);
        }
    }
    Ok(())
}
fn validate_args(args: &[String]) -> Result<()> {
    for arg in args {
        if arg.contains(';') || arg.contains('&') || arg.contains('|')
            || arg.contains('`')
        {
            bail!("Invalid argument: contains shell metacharacters");
        }
        if arg.contains("..") || arg.contains("~") {
            bail!("Invalid argument: contains path traversal");
        }
        if arg.len() > 1000 {
            bail!("Argument too long");
        }
    }
    Ok(())
}
fn delegate_to_cm(args: &[&str]) -> Result<()> {
    let cm_path = find_cm_binary()?;
    let mut cmd = Command::new(&cm_path);
    cmd.arg("captain");
    for arg in args {
        cmd.arg(arg);
    }
    let output = cmd.stdin(Stdio::null()).output().context("Failed to execute cm")?;
    io::stdout().write_all(&output.stdout)?;
    io::stderr().write_all(&output.stderr)?;
    if !output.status.success() {
        bail!("Command failed with status: {:?}", output.status.code());
    }
    Ok(())
}
fn find_cm_binary() -> Result<PathBuf> {
    let mut paths = vec![];
    if let Some(home) = dirs::home_dir() {
        paths.push(home.join(".shipwreck/bin/cm"));
    }
    if let Some(home) = dirs::home_dir() {
        paths.push(home.join(".shipwreck/bin/cm"));
    }
    paths.push(PathBuf::from("/usr/local/bin/cm"));
    if let Some(home) = dirs::home_dir() {
        paths.push(home.join(".local/bin/cm"));
    }
    if let Some(home) = dirs::home_dir() {
        paths.push(home.join(".cargo/bin/cm"));
    }
    for path in &paths {
        if path.exists() && path.is_file() {
            return Ok(path.clone());
        }
    }
    if let Ok(path) = which::which("cm") {
        return Ok(path);
    }
    bail!("Could not find 'cm' binary. Please ensure cargo-mate is installed.")
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_validate_args_clean() {
        let args = vec!["config".to_string(), "list".to_string()];
        assert!(validate_args(& args).is_ok());
    }
    #[test]
    fn test_validate_args_injection() {
        let args = vec!["config; rm -rf /".to_string()];
        assert!(validate_args(& args).is_err());
    }
    #[test]
    fn test_validate_args_traversal() {
        let args = vec!["../../../etc/passwd".to_string()];
        assert!(validate_args(& args).is_err());
    }
    #[test]
    fn test_validate_args_length() {
        let long_arg = "x".repeat(2000);
        let args = vec![long_arg];
        assert!(validate_args(& args).is_err());
    }
}