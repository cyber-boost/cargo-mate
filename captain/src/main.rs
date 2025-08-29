use anyhow::{Context, Result};
use chrono;
use clap::{Parser, Subcommand, ValueEnum};
use colored::*;
use serde::{Deserialize, Serialize};
use serde_json;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use reqwest;
use crate::captain::config::ConfigAction;
mod anchor;
mod admin_msg;
mod affiliate;
mod captain;
mod captain_log;
mod checklist;
mod display;
mod history;
mod journey;
mod mutiny;
mod parser;
mod smart_parser;
mod strip;
mod scat;
mod tide;
mod treasure_map;
mod version;
mod optimize;
mod scrub;
mod user;
mod tools;
use crate::version::VersionManager;
#[derive(Parser, Debug)]
#[command(name = "cm")]
#[command(
    about = "🚢 Cargo Mate - A better cargo wrapper with superpowers",
    long_about = None
)]
#[command(version, author)]
struct Args {
    #[command(subcommand)]
    command: Option<Commands>,
    #[arg(trailing_var_arg = true)]
    args: Vec<String>,
}
#[derive(Subcommand, Debug)]
enum Commands {
    Init,
    Journey { #[command(subcommand)] action: JourneyAction },
    Anchor { #[command(subcommand)] action: AnchorAction },
    Log { #[command(subcommand)] action: LogAction },
    Tide { #[command(subcommand)] action: TideAction },
    Map { #[command(subcommand)] action: MapAction },
    Mutiny { #[command(subcommand)] action: MutinyAction },
    Config { #[command(subcommand)] action: ConfigAction },
    Version { #[command(subcommand)] action: VersionAction },
    View { #[command(subcommand)] action: ViewAction },
    Test,
    Optimize { #[command(subcommand)] action: OptimizeAction },
    Checklist { #[command(subcommand)] action: ChecklistAction },
    History {
        #[arg(default_value = "summary")]
        kind: String,
        #[arg(default_value = "50")]
        limit: usize,
    },
    Scrub { #[command(subcommand)] action: ScrubAction },
    Install,
    Activate,
    Exec {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        cargo_args: Vec<String>,
    },
    Register {
        license_key: Option<String>,
        #[arg(long)]
        status: bool,
        #[arg(long)]
        remaining: bool,
    },
    Idea { idea: String },
    Wtf { #[command(subcommand)] action: crate::captain::wtf::WtfAction },
    User,
    Debug,
    Strip(crate::strip::StripArgs),
    Scat(crate::scat::ScatArgs),
    Tool { #[command(subcommand)] action: ToolAction },
}
#[derive(Subcommand, Debug)]
enum JourneyAction {
    Record { name: String },
    Play { name: String, #[arg(long)] dry_run: bool },
    List,
    Export { name: String, output: PathBuf },
    Import { path: PathBuf },
    Publish { name: String, #[arg(long)] tags: Vec<String> },
    Download { gist_id: String },
    Search { query: String },
    Published,
}
#[derive(Subcommand, Debug)]
enum AnchorAction {
    Save { name: String, #[arg(long)] message: Option<String> },
    Restore { name: String },
    List,
    Show { name: String },
    Diff { name: String },
    Auto { name: String, #[arg(long)] foreground: bool },
    Stop { name: String },
}
#[derive(Subcommand, Debug)]
enum LogAction {
    Add { message: String, #[arg(long)] tags: Vec<String> },
    Search { query: String },
    Timeline { #[arg(default_value = "7")] days: i64 },
    Export { path: PathBuf, #[arg(long, default_value = "markdown")] format: String },
    Analyze,
    Track { command: String },
}
#[derive(Subcommand, Debug)]
enum TideAction {
    Show,
    Analyze,
    Export { path: PathBuf },
}
#[derive(Subcommand, Debug)]
enum MapAction {
    Show,
    Analyze,
    Export { path: PathBuf },
    Path { from: String, to: String },
}
#[derive(Subcommand, Debug)]
enum MutinyAction {
    Activate { reason: String },
    Deactivate,
    AllowWarnings,
    SkipTests,
    Force,
    Yolo,
    Status,
}
#[derive(Subcommand, Debug)]
enum VersionAction {
    Init {
        #[arg(help = "Initial version number (e.g., 1.0.0)")]
        version: Option<String>,
    },
    Info,
    Increment {
        #[arg(help = "Increment type")]
        #[arg(value_enum)]
        #[arg(default_value = "patch")]
        increment_type: IncrementType,
    },
    Set { #[arg(help = "New version number")] version: String },
    History,
    UpdateCargo,
    Config { #[command(subcommand)] action: VersionConfigAction },
}
#[derive(Subcommand, Debug, Clone, Copy, ValueEnum)]
enum IncrementType {
    Patch,
    Minor,
    Major,
}
#[derive(Subcommand, Debug)]
enum VersionConfigAction {
    Enable,
    Disable,
    Policy { #[arg(value_enum)] policy: IncrementType },
    Show,
}
fn handle_tool_command(action: ToolAction) -> Result<()> {
    match action {
        ToolAction::List => {
            tools::list_tools();
        }
        ToolAction::Help { name } => {
            tools::show_tool_help(&name);
        }
        ToolAction::Run { name, args } => {
            tools::run_tool(&name, &args)?;
        }
        ToolAction::Execute(args) => {
            if args.is_empty() {
                tools::list_tools();
            } else {
                let tool_name = &args[0];
                let tool_args = &args[1..];
                tools::run_tool(tool_name, tool_args)?;
            }
        }
    }
    Ok(())
}
#[tokio::main]
async fn main() -> Result<()> {
    if let Err(e) = run().await {
        eprintln!("❌ Error: {}", e);
        crate::captain::wtf::display_api_failure_art();
        std::process::exit(1);
    }
    Ok(())
}
async fn run() -> Result<()> {
    ensure_initialized();
    let protection_key = crate::captain::protection::get_protection_key();
    if crate::captain::protection::is_captain_drunk() {
        eprintln!(
            "CAPTAIN_DRUNK: Using embedded fallback protection key ({}...)", &
            protection_key[..8]
        );
    } else if crate::captain::protection::is_captain_sober() {
        eprintln!(
            "CAPTAIN_SOBER: Using remote protection key ({}...)", & protection_key[..8]
        );
    } else if crate::captain::protection::is_captain_cached() {
        eprintln!(
            "CAPTAIN_CACHE: Using cached protection key ({}...)", & protection_key[..8]
        );
    }
    let captain_available = crate::captain::captain_status::is_captain_available();
    if !captain_available {
        if let Some(captain_path) = crate::captain::captain_status::find_captain_binary() {
            eprintln!("⚠️  Captain binary found at: {}", captain_path);
            eprintln!(
                "   But verification failed - may need PROTECT_KEY environment variable"
            );
            eprintln!(
                "   Current PROTECT_KEY: {}", std::env::var("PROTECT_KEY")
                .unwrap_or_else(| _ | "NOT SET".to_string())
            );
            eprintln!();
            eprintln!("💡 Try setting PROTECT_KEY or check if the key has rotated");
            eprintln!("   Download from: https://get.cargo.do/captain/");
            eprintln!();
        } else {
            eprintln!("⚠️  Captain binary not found");
        }
        if std::env::var("CM_NO_AUTO_INSTALL").is_ok() {
            eprintln!("   Auto-install disabled by CM_NO_AUTO_INSTALL");
            eprintln!("   Some advanced features will be unavailable.");
            eprintln!();
            eprintln!("💡 For full functionality, install Captain manually:");
            eprintln!("   Download from: https://get.cargo.do/captain/");
            eprintln!();
            std::env::set_var("CARGO_MATE_LIMITED_MODE", "1");
            initialize_fallback_mode()?;
        } else {
            match auto_install_captain().await {
                Ok(_) => {
                    eprintln!("✅ Captain installed successfully!");
                    std::env::set_var("CARGO_MATE_FULL_MODE", "1");
                    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
                    let current_path = std::env::var("PATH").unwrap_or_default();
                    let new_path = format!("{}/.shipwreck/bin:{}", home, current_path);
                    std::env::set_var("PATH", new_path);
                }
                Err(e) => {
                    eprintln!("⚠️  Failed to auto-install captain: {}", e);
                    eprintln!("   Running in limited mode");
                    eprintln!();
                    eprintln!("💡 You can manually install Captain:");
                    eprintln!("   Download from: https://get.cargo.do/mate");
                    eprintln!();
                    std::env::set_var("CARGO_MATE_LIMITED_MODE", "1");
                    initialize_fallback_mode()?;
                }
            }
        }
    } else {
        eprintln!("✅ Captain binary detected - full functionality enabled");
        std::env::set_var("CARGO_MATE_FULL_MODE", "1");
    }
    let raw_args: Vec<String> = std::env::args().collect();
    if raw_args.len() >= 3 && raw_args[1] == "wtf" {
        let first_arg = &raw_args[2];
        let is_not_subcommand = !matches!(
            first_arg.as_str(), "list" | "show" | "history" | "checklist" | "help" |
            "--help" | "-h" | "--version"
        );
        if is_not_subcommand {
            let is_direct_question = if raw_args.len() == 3 {
                first_arg.starts_with('"') || first_arg.starts_with('\'')
                    || first_arg.contains(' ') || first_arg.len() > 2
            } else {
                true
            };
            if is_direct_question {
                let question = if raw_args.len() == 3 {
                    first_arg.clone()
                } else {
                    raw_args[2..].join(" ")
                };
                println!("💭 Detected direct question: {}", question.cyan());
                if let Err(e) = crate::captain::wtf::handle_wtf(&question, false) {
                    eprintln!("❌ Error: {}", e);
                    std::process::exit(1);
                }
                return Ok(());
            }
        }
    }
    let args = Args::parse();
    if !matches!(args.command, Some(Commands::Register { .. }) | None) {
        let should_check = match &args.command {
            Some(cmd) => !matches!(cmd, Commands::Activate | Commands::Install),
            None => true,
        };
        if should_check {
            std::thread::spawn(|| {
                let runtime = tokio::runtime::Runtime::new().unwrap();
                runtime
                    .block_on(async {
                        let _ = admin_msg::check_and_display_message().await;
                    });
            });
        }
    }
    if let Some(ref command) = args.command {
        match command {
            Commands::Register { .. } => {}
            _ => {
                let license_manager = crate::captain::license::LicenseManager::new()?;
                match command {
                    Commands::Init => license_manager.enforce_license("init")?,
                    Commands::Journey { .. } => {
                        license_manager.enforce_license("journey")?
                    }
                    Commands::Anchor { .. } => license_manager.enforce_license("anchor")?,
                    Commands::Log { .. } => license_manager.enforce_license("log")?,
                    Commands::Tide { .. } => license_manager.enforce_license("tide")?,
                    Commands::Map { .. } => license_manager.enforce_license("map")?,
                    Commands::Mutiny { .. } => license_manager.enforce_license("mutiny")?,
                    Commands::Config { .. } => license_manager.enforce_license("config")?,
                    Commands::Version { .. } => {
                        license_manager.enforce_license("version")?
                    }
                    Commands::View { .. } => license_manager.enforce_license("view")?,
                    Commands::Optimize { .. } => {
                        license_manager.enforce_license("optimize")?
                    }
                    Commands::Test => license_manager.enforce_license("test")?,
                    Commands::Checklist { .. } => {
                        license_manager.enforce_license("checklist")?
                    }
                    Commands::History { .. } => {
                        license_manager.enforce_license("history")?
                    }
                    Commands::Scrub { .. } => license_manager.enforce_license("scrub")?,
                    Commands::Install => license_manager.enforce_license("install")?,
                    Commands::Activate => license_manager.enforce_license("activate")?,
                    Commands::Idea { .. } => license_manager.enforce_license("idea")?,
                    Commands::Wtf { .. } => license_manager.enforce_license("wtf")?,
                    Commands::User => license_manager.enforce_license("user")?,
                    Commands::Debug => license_manager.enforce_license("debug")?,
                    Commands::Strip(_) => license_manager.enforce_license("strip")?,
                    Commands::Scat(_) => license_manager.enforce_license("scat")?,
                    Commands::Exec { .. } => {}
                    Commands::Tool { .. } => license_manager.enforce_license("tool")?,
                    Commands::Register { .. } => unreachable!(),
                };
            }
        }
    }
    let mut config = crate::captain::config::ConfigManager::new()?;
    config.merge_with_env();
    match args.command {
        None => {
            if Path::new("Cargo.toml").exists() {
                if let Some(journey) = config.get("project.default_journey") {
                    println!("🚢 Running default journey: {}", journey.cyan());
                    let mut player = journey::JourneyPlayer::new(false, false);
                    let journey = player.load_journey(&journey)?;
                    player.play(&journey)?;
                } else {
                    println!("🚢 Auto-building release...");
                    run_cargo_with_wrapper(&["build", "--release"]);
                }
            } else {
                show_help();
            }
        }
        Some(Commands::Init) => init_cargo_mate()?,
        Some(Commands::Journey { action }) => handle_journey(action)?,
        Some(Commands::Anchor { action }) => handle_anchor(action)?,
        Some(Commands::Log { action }) => handle_log(action)?,
        Some(Commands::Tide { action }) => handle_tide(action)?,
        Some(Commands::Map { action }) => handle_map(action)?,
        Some(Commands::Mutiny { action }) => handle_mutiny(action)?,
        Some(Commands::Config { action }) => handle_config(action)?,
        Some(Commands::Version { action }) => handle_version(action)?,
        Some(Commands::View { action }) => handle_view(action)?,
        Some(Commands::Optimize { action }) => handle_optimize(action)?,
        Some(Commands::Test) => handle_test()?,
        Some(Commands::Checklist { action }) => handle_checklist(action)?,
        Some(Commands::History { kind, limit }) => {
            history::show_history(&[kind, limit.to_string()]);
            return Ok(());
        }
        Some(Commands::Scrub { action }) => handle_scrub(action)?,
        Some(Commands::Install) => {
            crate::captain::shell_integration::ShellIntegration::install()?;
            if let Err(e) = affiliate::show_affiliate_program_info() {
                eprintln!("Warning: Could not show affiliate info: {}", e);
            }
            return Ok(());
        }
        Some(Commands::Activate) => handle_activate()?,
        Some(Commands::Register { license_key, status, remaining }) => {
            handle_register(license_key, status, remaining)?
        }
        Some(Commands::Idea { idea }) => handle_idea(&idea)?,
        Some(Commands::Wtf { action }) => {
            if crate::captain::captain_status::is_captain_available() {
                let captain_path = crate::captain::captain_status::get_captain_path();
                if let Some(captain_binary) = captain_path {
                    let raw_args: Vec<String> = std::env::args().collect();
                    let wtf_args = if let Some(wtf_pos) = raw_args
                        .iter()
                        .position(|arg| arg == "wtf")
                    {
                        if wtf_pos + 1 < raw_args.len() {
                            raw_args[wtf_pos + 1..].to_vec()
                        } else {
                            vec![]
                        }
                    } else {
                        vec![]
                    };
                    match std::process::Command::new(&captain_binary)
                        .args(&wtf_args)
                        .status()
                    {
                        Ok(status) if status.success() => {
                            return Ok(());
                        }
                        Ok(status) => {
                            eprintln!("⚠️  Captain exited with status: {}", status);
                            return Ok(());
                        }
                        Err(e) => {
                            eprintln!("⚠️  Failed to execute captain: {}", e);
                        }
                    }
                }
            }
            let mut logger = crate::captain::captain::CaptainInterface::new()?;
            logger
                .delegate_to_captain(
                    "Processing WTF AI request",
                    "wtf_ai",
                    serde_json::json!({ "action" : "ask", "source" : "direct_command" }),
                )?;
            crate::captain::wtf::handle_wtf_action(action)?;
            return Ok(());
        }
        Some(Commands::User) => handle_user()?,
        Some(Commands::Config { action }) => {
            handle_config(action)?;
            return Ok(());
        }
        Some(Commands::Debug) => {
            let license_manager = crate::captain::license::LicenseManager::new()?;
            license_manager.debug_command_counters()?;
            println!();
            println!("🧭 Captain Status:");
            println!("{}", crate ::captain::captain_status::get_captain_status_info());
            return Ok(());
        }
        Some(Commands::Strip(args)) => {
            crate::strip::handle_strip_command(args)?;
            return Ok(());
        }
        Some(Commands::Scat(args)) => {
            crate::scat::handle_scat_command(args)?;
            return Ok(());
        }
        Some(Commands::Tool { action }) => {
            handle_tool_command(action)?;
            return Ok(());
        }
        Some(Commands::Exec { cargo_args }) => {
            let args: Vec<&str> = cargo_args.iter().map(|s| s.as_str()).collect();
            if !args.is_empty() && is_cm_command(args[0]) {
                handle_cm_command(&args)?;
            } else {
                if let Err(e) = version::pre_operation_hook(None) {
                    eprintln!("⚠️  Version auto-increment failed: {}", e);
                }
                let modified_args = if args.len() >= 2 && args[0] == "cargo"
                    && args[1] == "publish"
                {
                    let mut new_args = args.to_vec();
                    let has_allow_dirty = new_args
                        .iter()
                        .any(|&arg| arg == "--allow-dirty");
                    if !has_allow_dirty {
                        new_args.insert(2, "--allow-dirty");
                    }
                    new_args
                } else {
                    args.to_vec()
                };
                display::run_cargo_passthrough(&modified_args);
                if let Err(e) = version::post_operation_hook(None, true) {
                    eprintln!("⚠️  Version post-operation hook failed: {}", e);
                }
            }
            return Ok(());
        }
    }
    Ok(())
}
fn is_cm_command(cmd: &str) -> bool {
    matches!(
        cmd, "anchor" | "journey" | "log" | "tide" | "map" | "mutiny" | "config" |
        "version" | "view" | "optimize" | "test" | "history" | "init" | "install" |
        "activate" | "register" | "idea" | "wtf" | "checklist" | "add" | "done" | "clear"
        | "show" | "list" | "user" | "debug" | "help" | "--help" | "-h" | "tool" |
        "tools" | "strip" | "scat"
    )
}
fn handle_cm_command(args: &[&str]) -> Result<()> {
    if args.is_empty() {
        return Ok(());
    }
    let cmd = args[0];
    let remaining_args = &args[1..];
    let license_manager = crate::captain::license::LicenseManager::new()?;
    license_manager.enforce_license(cmd)?;
    match cmd {
        "strip" => {
            let strip_args = crate::strip::StripArgs::parse_from(&*remaining_args);
            crate::strip::handle_strip_command(strip_args)?;
            return Ok(());
        }
        "scat" => {
            let scat_args = crate::scat::ScatArgs::parse_from(&*remaining_args);
            crate::scat::handle_scat_command(scat_args)?;
            return Ok(());
        }
        "anchor" => {
            if remaining_args.is_empty() {
                eprintln!(
                    "⚠️  No anchor action specified. Use 'cargo anchor --help' for usage."
                );
                std::process::exit(1);
            }
            match remaining_args[0] {
                "save" => {
                    if remaining_args.len() < 2 {
                        eprintln!(
                            "⚠️  Anchor name required. Usage: cargo anchor save <name>"
                        );
                        std::process::exit(1);
                    }
                    let name = remaining_args[1].to_string();
                    let message = if remaining_args.len() >= 4
                        && remaining_args[2] == "--message"
                    {
                        Some(remaining_args[3].to_string())
                    } else {
                        None
                    };
                    let manager = anchor::AnchorManager::new()?;
                    let description = message
                        .unwrap_or_else(|| format!("Auto-saved via cargo anchor save"));
                    manager.save(&name, &description)?;
                }
                "restore" => {
                    if remaining_args.len() < 2 {
                        eprintln!(
                            "⚠️  Anchor name required. Usage: cargo anchor restore <name>"
                        );
                        std::process::exit(1);
                    }
                    let manager = anchor::AnchorManager::new()?;
                    manager.restore(remaining_args[1])?;
                }
                "list" => {
                    let manager = anchor::AnchorManager::new()?;
                    let anchors = manager.list()?;
                    if anchors.is_empty() {
                        println!("⚓ No anchors found");
                    } else {
                        println!("⚓ Available anchors:");
                        for anchor in anchors {
                            println!(
                                "⚓ {} - {} ({} files)", anchor.name.cyan().bold(), anchor
                                .timestamp.format("%Y-%m-%d %H:%M:%S").to_string().dimmed(),
                                anchor.files_count
                            );
                            println!("   {}", anchor.description.dimmed());
                        }
                    }
                }
                "show" => {
                    if remaining_args.len() < 2 {
                        eprintln!(
                            "⚠️  Anchor name required. Usage: cargo anchor show <name>"
                        );
                        std::process::exit(1);
                    }
                    let manager = anchor::AnchorManager::new()?;
                    manager.show(remaining_args[1])?;
                }
                "diff" => {
                    if remaining_args.len() < 2 {
                        eprintln!(
                            "⚠️  Anchor name required. Usage: cargo anchor diff <name>"
                        );
                        std::process::exit(1);
                    }
                    let manager = anchor::AnchorManager::new()?;
                    manager.diff(remaining_args[1])?;
                }
                "auto" => {
                    if remaining_args.len() < 2 {
                        eprintln!(
                            "⚠️  Anchor name required. Usage: cargo anchor auto <name> [--foreground]"
                        );
                        std::process::exit(1);
                    }
                    let manager = anchor::AnchorManager::new()?;
                    let foreground = remaining_args.len() > 2
                        && remaining_args[2] == "--foreground";
                    if foreground {
                        manager.start_auto_update(remaining_args[1])?;
                    } else {
                        manager.start_auto_update_background(remaining_args[1])?;
                    }
                }
                "stop" => {
                    if remaining_args.len() < 2 {
                        eprintln!(
                            "⚠️  Anchor name required. Usage: cargo anchor stop <name>"
                        );
                        std::process::exit(1);
                    }
                    let manager = anchor::AnchorManager::new()?;
                    manager.stop_auto_update(remaining_args[1])?;
                }
                _ => {
                    eprintln!(
                        "⚠️  Unknown anchor action: {}. Use 'cargo anchor --help' for usage.",
                        remaining_args[0]
                    );
                    std::process::exit(1);
                }
            }
        }
        "journey" => {
            eprintln!("🚧 Journey commands not yet implemented for exec routing");
            eprintln!("💡 Use 'cm journey' directly for now");
            std::process::exit(1);
        }
        "log" => {
            eprintln!("🚧 Log commands not yet implemented for exec routing");
            eprintln!("💡 Use 'cm log' directly for now");
            std::process::exit(1);
        }
        "tide" => {
            eprintln!("🚧 Tide commands not yet implemented for exec routing");
            eprintln!("💡 Use 'cm tide' directly for now");
            std::process::exit(1);
        }
        "map" => {
            eprintln!("🚧 Map commands not yet implemented for exec routing");
            eprintln!("💡 Use 'cm map' directly for now");
            std::process::exit(1);
        }
        "mutiny" => {
            eprintln!("🚧 Mutiny commands not yet implemented for exec routing");
            eprintln!("💡 Use 'cm mutiny' directly for now");
            std::process::exit(1);
        }
        "config" => {
            eprintln!("🚧 Config commands not yet implemented for exec routing");
            eprintln!("💡 Use 'cm config' directly for now");
            std::process::exit(1);
        }
        "version" => {
            let mut version_manager = VersionManager::new(None)?;
            match remaining_args.get(0).map(|s| s.as_ref()) {
                Some("history") => {
                    version_manager.show_history()?;
                }
                Some("info") => {
                    version_manager.show_info();
                }
                Some("increment") => {
                    let increment_type = remaining_args
                        .get(1)
                        .map(|s| s.as_ref())
                        .unwrap_or("patch");
                    let new_version = match increment_type {
                        "patch" => version_manager.increment()?,
                        "minor" => {
                            let original_policy = version_manager
                                .config
                                .increment_policy
                                .clone();
                            version_manager.config.increment_policy = version::IncrementPolicy::Minor;
                            let result = version_manager.increment()?;
                            version_manager.config.increment_policy = original_policy;
                            result
                        }
                        "major" => {
                            let original_policy = version_manager
                                .config
                                .increment_policy
                                .clone();
                            version_manager.config.increment_policy = version::IncrementPolicy::Major;
                            let result = version_manager.increment()?;
                            version_manager.config.increment_policy = original_policy;
                            result
                        }
                        _ => {
                            eprintln!(
                                "⚠️  Unknown increment type: {}. Use patch, minor, or major.",
                                increment_type
                            );
                            std::process::exit(1);
                        }
                    };
                    println!("✅ Version incremented to: {}", new_version.cyan());
                }
                Some("set") => {
                    if remaining_args.len() < 2 {
                        eprintln!(
                            "⚠️  Version required. Usage: cargo version set <version>"
                        );
                        std::process::exit(1);
                    }
                    version_manager.set_version(remaining_args[1])?;
                }
                Some("update-cargo") => {
                    version_manager.update_cargo_toml()?;
                }
                Some("config") => {
                    match remaining_args.get(1).map(|s| s.as_ref()) {
                        Some("enable") => {
                            version_manager.config.auto_increment = true;
                            version_manager.save_config()?;
                            println!("✅ Auto-increment enabled");
                        }
                        Some("disable") => {
                            version_manager.config.auto_increment = false;
                            version_manager.save_config()?;
                            println!("✅ Auto-increment disabled");
                        }
                        Some("policy") => {
                            if remaining_args.len() < 3 {
                                eprintln!(
                                    "⚠️  Policy required. Usage: cargo version config policy <patch|minor|major>"
                                );
                                std::process::exit(1);
                            }
                            let policy = remaining_args[2];
                            version_manager.config.increment_policy = match policy {
                                "patch" => version::IncrementPolicy::Patch,
                                "minor" => version::IncrementPolicy::Minor,
                                "major" => version::IncrementPolicy::Major,
                                _ => {
                                    eprintln!(
                                        "⚠️  Unknown policy: {}. Use patch, minor, or major.",
                                        policy
                                    );
                                    std::process::exit(1);
                                }
                            };
                            version_manager.save_config()?;
                            println!("✅ Increment policy updated");
                        }
                        Some("show") => {
                            version_manager.show_info();
                        }
                        _ => {
                            eprintln!(
                                "⚠️  Unknown config action. Use: enable, disable, policy, show"
                            );
                            std::process::exit(1);
                        }
                    }
                }
                _ => {
                    eprintln!(
                        "⚠️  Unknown version action. Available: history, info, increment, set, update-cargo, config"
                    );
                    std::process::exit(1);
                }
            }
        }
        "view" => {
            eprintln!("🚧 View commands not yet implemented for exec routing");
            eprintln!("💡 Use 'cm view' directly for now");
            std::process::exit(1);
        }
        "optimize" => {
            eprintln!("🚧 Optimize commands not yet implemented for exec routing");
            eprintln!("💡 Use 'cm optimize' directly for now");
            std::process::exit(1);
        }
        "checklist" => {
            checklist::show_checklist();
        }
        "history" => {
            let limit = remaining_args.get(0).and_then(|s| s.parse().ok()).unwrap_or(10);
            history::show_history(&vec!["all".to_string(), limit.to_string()]);
        }
        "init" => {
            init_cargo_mate()?;
        }
        "install" => {
            crate::captain::shell_integration::ShellIntegration::install()?;
        }
        "activate" => {
            handle_activate()?;
        }
        "register" => {
            let license_key = remaining_args.get(0).map(|s| s.to_string());
            let status = remaining_args.contains(&"--status");
            handle_register(license_key, status, false)?;
        }
        "help" | "--help" | "-h" => {
            println!(
                "{}", "🚢 Cargo Mate - A better cargo wrapper with superpowers".bold()
            );
            println!();
            println!("{}", "USAGE:".bold());
            println!("    cargo <COMMAND>    - Run cargo commands through cargo-mate");
            println!("    cm <COMMAND>       - Direct cargo-mate access");
            println!("    cg <COMMAND>       - Quick shortcut");
            println!();
            println!("{}", "CARGO-MATE COMMANDS:".bold());
            println!(
                "    anchor     - Save and restore project states (with auto-update)"
            );
            println!("    journey    - Journey recording and playback");
            println!("    log        - Captain's log for build notes");
            println!("    tide       - Performance tracking and visualization");
            println!("    map        - Dependency visualization");
            println!("    mutiny     - Override cargo restrictions");
            println!("    config     - Configuration management");
            println!("    version    - Version management and auto-incrementing");
            println!("    view       - View build results and artifacts");
            println!("    optimize   - Build performance optimization");
            println!("    checklist  - Show error/warning checklist");
            println!("    history    - Show command history");
            println!(
                "    wtf        - CargoMate AI (Pro only) - Ask questions about your code"
            );
            println!("    idea       - Submit ideas for Cargo Mate");
            println!("    init       - Initialize cargo-mate in current project");
            println!("    install    - Install shell integration");
            println!("    activate   - Activate shell integration immediately");
            println!("    user       - Show user information and license status");
            println!("    scrub      - System wide cargo clean");
            println!();
        }
        _ => {
            eprintln!("⚠️  Unknown command: {}", cmd);
            std::process::exit(1);
        }
    }
    Ok(())
}
fn ensure_initialized() {
    let shipwreck = dirs::home_dir()
        .expect("Could not find home directory")
        .join(".shipwreck");
    if !shipwreck.exists() {
        println!("⚓ First run! Setting up Cargo Mate...");
        std::fs::create_dir_all(&shipwreck.join("errors"))
            .expect("Failed to create errors directory");
        std::fs::create_dir_all(&shipwreck.join("warnings"))
            .expect("Failed to create warnings directory");
        std::fs::create_dir_all(&shipwreck.join("checklists"))
            .expect("Failed to create checklists directory");
        std::fs::create_dir_all(&shipwreck.join("history"))
            .expect("Failed to create history directory");
        std::fs::create_dir_all(&shipwreck.join("wtf_history"))
            .expect("Failed to create WTF history directory");
        std::fs::create_dir_all(&shipwreck.join("idea_history"))
            .expect("Failed to create idea history directory");
        if let Err(e) = auto_install_shell_integration() {
            eprintln!("⚠️  Auto-setup failed: {}", e);
            println!("💡 Run 'cm install' manually if needed");
        }
    }
}
fn initialize_fallback_mode() -> Result<()> {
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    let shipwreck_dir = home.join(".shipwreck");
    let _ = fs::create_dir_all(&shipwreck_dir);
    let config_file = shipwreck_dir.join("config.toml");
    if !config_file.exists() {
        let basic_config = r#"
[user]
mode = "limited"
captain_installed = false

[features]
basic_commands = true
advanced_features = false

[fallback]
reason = "captain binary not found"
timestamp = "2024-01-01"
"#;
        let _ = fs::write(&config_file, basic_config);
    }
    let history_dir = shipwreck_dir.join("history");
    let _ = fs::create_dir_all(&history_dir);
    let anchors_dir = shipwreck_dir.join("anchors");
    let _ = fs::create_dir_all(&anchors_dir);
    let journeys_dir = shipwreck_dir.join("journeys");
    let _ = fs::create_dir_all(&journeys_dir);
    eprintln!("📂 Fallback mode initialized with basic directories");
    eprintln!("✅ Basic cargo commands will work");
    eprintln!("⚠️  Advanced features require captain binary");
    Ok(())
}
fn detect_platform() -> Result<String> {
    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;
    let platform = match (os, arch) {
        ("linux", "x86_64") => {
            if std::path::Path::new("/etc/alpine-release").exists() {
                "x86_64-unknown-linux-musl"
            } else {
                "x86_64-unknown-linux-gnu"
            }
        }
        ("linux", "aarch64") => "aarch64-unknown-linux-gnu",
        ("macos", "x86_64") => "x86_64-apple-darwin",
        ("macos", "aarch64") => "aarch64-apple-darwin",
        ("windows", "x86_64") => "x86_64-pc-windows-gnu",
        _ => return Err(anyhow::anyhow!("Unsupported platform: {}-{}", os, arch)),
    };
    Ok(platform.to_string())
}
async fn auto_install_captain() -> Result<()> {
    eprintln!("🔍 Captain binary not found. Installing automatically...");
    let platform = detect_platform()?;
    eprintln!("📦 Detected platform: {}", platform);
    let home = std::env::var("HOME").context("HOME not set")?;
    let shipwreck_bin = PathBuf::from(&home).join(".shipwreck").join("bin");
    fs::create_dir_all(&shipwreck_bin)?;
    let base_url = std::env::var("CAPTAIN_DOWNLOAD_URL")
        .unwrap_or_else(|_| "https://get.cargo.do/captain".to_string());
    let archive_name = format!("captain-{}.tar.gz", platform);
    let download_url = format!("{}/{}", base_url, archive_name);
    eprintln!("📥 Downloading from: {}", download_url);
    let client = reqwest::Client::new();
    let response = client.get(&download_url).send().await?;
    if !response.status().is_success() {
        return Err(anyhow::anyhow!("Failed to download captain: {}", response.status()));
    }
    let bytes = response.bytes().await?;
    let temp_dir = std::env::temp_dir();
    let archive_path = temp_dir.join(&archive_name);
    fs::write(&archive_path, bytes)?;
    eprintln!("📂 Extracting captain...");
    let output = std::process::Command::new("tar")
        .args(&["-xzf", archive_path.to_str().unwrap()])
        .current_dir(&temp_dir)
        .output()?;
    if !output.status.success() {
        return Err(anyhow::anyhow!("Failed to extract archive"));
    }
    let extracted_dir = temp_dir.join(format!("captain-{}", platform));
    let captain_enc = extracted_dir.join("captain.enc");
    if !captain_enc.exists() {
        return Err(anyhow::anyhow!("captain.enc not found in archive"));
    }
    let captain_dest = shipwreck_bin.join("captain");
    fs::copy(&captain_enc, &captain_dest)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&captain_dest)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&captain_dest, perms)?;
    }
    fs::remove_file(&archive_path).ok();
    fs::remove_dir_all(&extracted_dir).ok();
    eprintln!("✅ Captain installed successfully to: {}", captain_dest.display());
    crate::captain::captain_status::mark_captain_installed(
        captain_dest.to_str().unwrap(),
    )?;
    if !std::env::var("PATH").unwrap_or_default().contains(".shipwreck/bin") {
        println!();
        println!("⚠️  Add ~/.shipwreck/bin to your PATH:");
        println!("   export PATH=\"$HOME/.shipwreck/bin:$PATH\"");
    }
    Ok(())
}
fn auto_install_shell_integration() -> Result<()> {
    let shell = detect_shell();
    let rc_file = get_rc_file(&shell)?;
    if rc_file.exists() {
        let content = std::fs::read_to_string(&rc_file)?;
        if content.contains("# === Cargo Mate") {
            return handle_activate();
        }
    }
    add_shell_integration(&rc_file, &shell)?;
    handle_activate()?;
    Ok(())
}
fn init_cargo_mate() -> Result<()> {
    println!("🚢 Initializing Cargo Mate...");
    let mut config = crate::captain::config::ConfigManager::new()?;
    config.init_local()?;
    println!("✅ Local config created: .cg");
    println!("🔧 Setting up shell integration...");
    let shell = detect_shell();
    let rc_file = get_rc_file(&shell)?;
    if rc_file.exists() {
        let content = std::fs::read_to_string(&rc_file)?;
        if content.contains("# === Cargo Mate") {
            eprintln!("⚠️  Shell integration already installed");
        } else {
            add_shell_integration(&rc_file, &shell)?;
        }
    } else {
        add_shell_integration(&rc_file, &shell)?;
    }
    eprintln!("📁 Error logs will be stored in ~/.shipwreck/");
    println!();
    println!("🎉 Cargo Mate initialized successfully!");
    println!();
    println!(
        "⚡ {}", "Shell integration added. To activate immediately, run one of these:"
        .yellow()
    );
    println!("   {} {}", "source".green(), format!("{}", rc_file.display()) .cyan());
    println!("   {} {}", "cm".green(), "activate".cyan());
    println!("   {}", "Or restart your terminal".dimmed());
    println!();
    println!("📚 {}", "Available commands after activation:".yellow());
    println!("   {} - Run cargo through cargo-mate", "cargo".cyan());
    println!("   {} - Direct cargo-mate access", "cm".cyan());
    println!("   {} - Quick shortcut", "cg".cyan());
    println!();
    println!("💡 {}", "Tip: Run 'cm activate' anytime to activate integration".blue());
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
fn add_shell_integration(rc_file: &PathBuf, shell: &str) -> Result<()> {
    use std::fs::OpenOptions;
    use std::io::Write;
    if rc_file.exists() {
        let backup = rc_file.with_extension("bak.cargo-mate");
        std::fs::copy(rc_file, &backup)?;
        println!("📋 Backed up {} to {}", rc_file.display(), backup.display());
    }
    let integration_code = match shell {
        "fish" => {
            r#"
# === Cargo Mate (cm) Integration ===
function cargo
    cm exec $argv
end

# Note: cm binary should be in PATH
alias cg='cm'
# === End Cargo Mate Integration ===
"#
        }
        _ => {
            r#"
# === Cargo Mate (cm) Integration ===
cargo() {
    cm exec "$@"
}
# Note: cm binary should be in PATH
alias cg='cm'
# === End Cargo Mate Integration ===
"#
        }
    };
    let mut file = OpenOptions::new().create(true).append(true).open(rc_file)?;
    writeln!(file, "{}", integration_code)?;
    println!("✅ Shell integration added to {}", rc_file.display());
    Ok(())
}
fn handle_journey(action: JourneyAction) -> Result<()> {
    match action {
        JourneyAction::Record { name } => {
            let recorder = journey::JourneyRecorder::new();
            recorder.start_recording(&name)?;
            while recorder.is_recording() {
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
            recorder.stop_recording(&name, "User recorded journey")?;
        }
        JourneyAction::Play { name, dry_run } => {
            let mut player = journey::JourneyPlayer::new(dry_run, true);
            let journey = player.load_journey(&name)?;
            player.play(&journey)?;
        }
        JourneyAction::List => {
            let journeys = journey::list_journeys()?;
            if journeys.is_empty() {
                println!("No journeys found");
            } else {
                println!("📚 Available journeys:");
                for name in journeys {
                    println!("  • {}", name.cyan());
                }
            }
        }
        JourneyAction::Export { name, output } => {
            journey::export_journey(&name, &output)?;
        }
        JourneyAction::Import { path } => {
            journey::import_journey(&path)?;
        }
        JourneyAction::Publish { name, tags } => {
            journey::JourneyMarketplace::publish(&name, tags)?;
        }
        JourneyAction::Download { gist_id } => {
            journey::JourneyMarketplace::download(&gist_id)?;
        }
        JourneyAction::Search { query } => {
            journey::JourneyMarketplace::search(&query)?;
        }
        JourneyAction::Published => {
            let published = journey::JourneyMarketplace::list_published()?;
            if published.is_empty() {
                println!("No published journeys found");
            } else {
                println!("📤 Your published journeys:");
                for journey in published {
                    println!("  • {}", journey.cyan());
                }
            }
        }
    }
    Ok(())
}
fn handle_anchor(action: AnchorAction) -> Result<()> {
    let manager = anchor::AnchorManager::new()?;
    match action {
        AnchorAction::Save { name, message } => {
            let msg = message.unwrap_or_else(|| "Manual anchor point".to_string());
            manager.save(&name, &msg)?;
        }
        AnchorAction::Restore { name } => {
            manager.restore(&name)?;
        }
        AnchorAction::List => {
            let anchors = manager.list()?;
            if anchors.is_empty() {
                println!("No anchors found");
            } else {
                println!("⚓ Saved anchors:");
                for anchor in anchors {
                    anchor.display();
                }
            }
        }
        AnchorAction::Show { name } => {
            manager.show(&name)?;
        }
        AnchorAction::Diff { name } => {
            manager.diff(&name)?;
        }
        AnchorAction::Auto { name, foreground } => {
            if foreground {
                manager.start_auto_update(&name)?;
            } else {
                manager.start_auto_update_background(&name)?;
            }
        }
        AnchorAction::Stop { name } => {
            manager.stop_auto_update(&name)?;
        }
    }
    Ok(())
}
fn handle_log(action: LogAction) -> Result<()> {
    let mut log = captain_log::CaptainLog::new()?;
    match action {
        LogAction::Add { message, tags } => {
            log.log(&message, tags)?;
        }
        LogAction::Search { query } => {
            let results = log.search(&query);
            if results.is_empty() {
                println!("No matching log entries found");
            } else {
                println!("Found {} entries:", results.len());
                for entry in results {
                    println!(
                        "  {} - {}", entry.timestamp.format("%Y-%m-%d %H:%M:%S"), entry
                        .message
                    );
                }
            }
        }
        LogAction::Timeline { days } => {
            log.show_timeline(days)?;
        }
        LogAction::Export { path, format } => {
            let fmt = match format.as_str() {
                "json" => captain_log::ExportFormat::Json,
                "html" => captain_log::ExportFormat::Html,
                _ => captain_log::ExportFormat::Markdown,
            };
            log.export(&path, fmt)?;
        }
        LogAction::Analyze => {
            let analysis = log.analyze();
            analysis.display();
        }
        LogAction::Track { command } => {
            println!("🔍 Starting enhanced tracking for: {}", command.cyan());
            let session_id = format!(
                "{}-{:x}", command.replace(" ", "_"), chrono::Utc::now().timestamp()
            );
            match run_tracked_command(&command, &session_id) {
                Ok(_) => println!("✅ Command tracked successfully"),
                Err(e) => eprintln!("❌ Tracking failed: {}", e),
            }
        }
    }
    Ok(())
}
fn handle_tide(action: TideAction) -> Result<()> {
    let mut charts = tide::TideCharts::new()?;
    match action {
        TideAction::Show => {
            charts.show_interactive()?;
        }
        TideAction::Analyze => {
            charts.analyze_dependencies()?;
        }
        TideAction::Export { path } => {
            charts.export_csv(&path)?;
        }
    }
    Ok(())
}
fn handle_map(action: MapAction) -> Result<()> {
    let map = treasure_map::TreasureMap::new()?;
    match action {
        MapAction::Show => {
            map.show_map();
        }
        MapAction::Analyze => {
            let analysis = map.analyze();
            analysis.display();
        }
        MapAction::Export { path } => {
            map.export_dot(&path)?;
        }
        MapAction::Path { from, to } => {
            if let Some(path) = map.find_path(&from, &to) {
                println!("📍 Path from {} to {}:", from.cyan(), to.cyan());
                for (i, node) in path.iter().enumerate() {
                    println!("  {}. {}", i + 1, node);
                }
            } else {
                println!("No path found between {} and {}", from, to);
            }
        }
    }
    Ok(())
}
fn handle_mutiny(action: MutinyAction) -> Result<()> {
    let mut mutiny = mutiny::MutinyMode::new()?;
    match action {
        MutinyAction::Activate { reason } => {
            mutiny.activate(&reason)?;
        }
        MutinyAction::Deactivate => {
            mutiny.deactivate()?;
        }
        MutinyAction::AllowWarnings => {
            mutiny.allow_warnings()?;
        }
        MutinyAction::SkipTests => {
            mutiny.skip_tests()?;
        }
        MutinyAction::Force => {
            mutiny.force_build()?;
        }
        MutinyAction::Yolo => {
            mutiny.yolo_mode()?;
        }
        MutinyAction::Status => {
            mutiny.status();
        }
    }
    Ok(())
}
fn handle_config(action: ConfigAction) -> Result<()> {
    let mut config = crate::captain::config::ConfigManager::new()?;
    match action {
        ConfigAction::Set { key, value, local } => {
            config.set(&key, &value, local)?;
        }
        ConfigAction::Get { key } => {
            if let Some(value) = config.get(&key) {
                println!("{}", value);
            } else {
                println!("Config key '{}' not found", key);
            }
        }
        ConfigAction::List => {
            config.show();
        }
        ConfigAction::Init => {
            config.init_local()?;
        }
        ConfigAction::Shortcut { name, command, local } => {
            config.add_shortcut(&name, &command, local)?;
        }
        ConfigAction::Hook { hook_type, command, local } => {
            config.add_hook(&hook_type, &command, local)?;
        }
    }
    Ok(())
}
fn handle_version(action: VersionAction) -> Result<()> {
    let mut version_manager = version::VersionManager::new(None)?;
    match action {
        VersionAction::Init { version } => {
            version_manager.init(version)?;
        }
        VersionAction::Info => {
            version_manager.show_info();
        }
        VersionAction::Increment { increment_type } => {
            let new_version = match increment_type {
                IncrementType::Patch => version_manager.increment()?,
                IncrementType::Minor => {
                    let original_policy = version_manager
                        .config
                        .increment_policy
                        .clone();
                    version_manager.config.increment_policy = version::IncrementPolicy::Minor;
                    let result = version_manager.increment()?;
                    version_manager.config.increment_policy = original_policy;
                    result
                }
                IncrementType::Major => {
                    let original_policy = version_manager
                        .config
                        .increment_policy
                        .clone();
                    version_manager.config.increment_policy = version::IncrementPolicy::Major;
                    let result = version_manager.increment()?;
                    version_manager.config.increment_policy = original_policy;
                    result
                }
            };
            println!("✅ Version incremented to: {}", new_version.cyan());
        }
        VersionAction::Set { version } => {
            version_manager.set_version(&version)?;
        }
        VersionAction::History => {
            let version_manager = VersionManager::new(None)?;
            version_manager.show_history()?;
        }
        VersionAction::UpdateCargo => {
            version_manager.update_cargo_toml()?;
        }
        VersionAction::Config { action } => {
            match action {
                VersionConfigAction::Enable => {
                    version_manager.config.auto_increment = true;
                    version_manager.save_config()?;
                    println!("✅ Auto-increment enabled");
                }
                VersionConfigAction::Disable => {
                    version_manager.config.auto_increment = false;
                    version_manager.save_config()?;
                    println!("✅ Auto-increment disabled");
                }
                VersionConfigAction::Policy { policy } => {
                    version_manager.config.increment_policy = match policy {
                        IncrementType::Patch => version::IncrementPolicy::Patch,
                        IncrementType::Minor => version::IncrementPolicy::Minor,
                        IncrementType::Major => version::IncrementPolicy::Major,
                    };
                    version_manager.save_config()?;
                    println!("✅ Increment policy updated");
                }
                VersionConfigAction::Show => {
                    version_manager.show_info();
                }
            }
        }
    }
    Ok(())
}
#[derive(Subcommand, Debug)]
enum ViewAction {
    Errors,
    Artifacts,
    Scripts,
    History,
    Checklist,
    All,
    Latest,
    Open,
}
#[derive(Subcommand, Debug)]
enum ChecklistAction {
    Show,
    Add { item: String },
    Done { items: String },
    Clear { #[arg(default_value = "all")] target: String },
    List,
}
#[derive(Subcommand, Debug)]
enum OptimizeAction {
    Aggressive,
    Balanced,
    Conservative,
    Custom {
        #[arg(default_value = "4")]
        jobs: u32,
        #[arg(default_value = "true")]
        incremental: String,
        #[arg(default_value = "1")]
        opt_level: u32,
        #[arg(default_value = "1")]
        debug_level: u32,
        #[arg(default_value = "128")]
        codegen_units: u32,
    },
    Status,
    Recommendations,
    Restore,
}
#[derive(Subcommand, Debug)]
enum ScrubAction {
    Run {
        #[arg(long)]
        dry_run: bool,
        #[arg(short, long)]
        verbose: bool,
        #[arg(short, long, default_value = "/")]
        start: String,
        #[arg(short, long)]
        resume: Option<String>,
        #[arg(long, default_value = "1")]
        min_depth: usize,
        #[arg(long, default_value = "10")]
        max_depth: usize,
    },
    Help,
}
#[derive(Subcommand, Debug)]
enum ToolAction {
    List,
    Help { name: String },
    Run { name: String, #[arg(trailing_var_arg = true)] args: Vec<String> },
    #[command(external_subcommand)]
    Execute(Vec<String>),
}
fn handle_view(action: ViewAction) -> Result<()> {
    let shipwreck = dirs::home_dir()
        .context("Could not find home directory")?
        .join(".shipwreck");
    match action {
        ViewAction::Errors => {
            let error_file = shipwreck.join("errors").join("latest.txt");
            if error_file.exists() {
                println!("🔴 Latest Errors:");
                println!("{}", "═".repeat(50).red());
                let content = fs::read_to_string(error_file)?;
                println!("{}", content);
            } else {
                println!("✅ No errors found");
            }
        }
        ViewAction::Artifacts => {
            let artifact_file = shipwreck.join("artifacts").join("latest.txt");
            if artifact_file.exists() {
                println!("📦 Generated Artifacts:");
                println!("{}", "═".repeat(50).blue());
                let content = fs::read_to_string(artifact_file)?;
                println!("{}", content);
            } else {
                println!("📁 No artifacts found");
            }
        }
        ViewAction::Scripts => {
            let script_file = shipwreck.join("scripts").join("latest.txt");
            if script_file.exists() {
                println!("🔨 Build Scripts:");
                println!("{}", "═".repeat(50).yellow());
                let content = fs::read_to_string(script_file)?;
                println!("{}", content);
            }
        }
        ViewAction::History => {
            history::show_history(&["detailed".to_string(), "100".to_string()]);
        }
        ViewAction::Checklist => {
            checklist::show_checklist();
        }
        ViewAction::All => {
            println!("🔍 Complete Build Results:");
            println!("{}", "═".repeat(60).cyan());
            let error_file = shipwreck.join("errors").join("latest.txt");
            if error_file.exists() {
                println!("🔴 Errors:");
                let content = fs::read_to_string(error_file)?;
                println!("{}", content);
                println!();
            }
            let warning_file = shipwreck.join("warnings").join("latest.txt");
            if warning_file.exists() {
                println!("⚠️  Warnings:");
                let content = fs::read_to_string(warning_file)?;
                println!("{}", content);
                println!();
            }
            let artifact_file = shipwreck.join("artifacts").join("latest.txt");
            if artifact_file.exists() {
                println!("📦 Artifacts:");
                let content = fs::read_to_string(artifact_file)?;
                println!("{}", content);
                println!();
            }
            let script_file = shipwreck.join("scripts").join("latest.txt");
            if script_file.exists() {
                println!("🔨 Build Scripts:");
                let content = fs::read_to_string(script_file)?;
                println!("{}", content);
            }
        }
        ViewAction::Latest => {
            println!("🔍 Latest Build Issues:");
            println!("{}", "═".repeat(50).cyan());
            let error_file = shipwreck.join("errors").join("latest.txt");
            if error_file.exists() {
                let content = fs::read_to_string(error_file)?;
                if !content.trim().is_empty() {
                    println!("🔴 Errors:");
                    println!("{}", content);
                    println!();
                }
            }
            let warning_file = shipwreck.join("warnings").join("latest.txt");
            if warning_file.exists() {
                let content = fs::read_to_string(warning_file)?;
                if !content.trim().is_empty() {
                    println!("⚠️  Warnings:");
                    println!("{}", content);
                }
            }
        }
        ViewAction::Open => {
            use std::process::Command;
            let target_dir = std::env::current_dir()?.join("target");
            if target_dir.exists() {
                println!("🚀 Opening target directory in file explorer...");
                #[cfg(target_os = "linux")]
                {
                    let _ = Command::new("xdg-open").arg(&target_dir).spawn();
                }
                #[cfg(target_os = "macos")]
                {
                    let _ = Command::new("open").arg(&target_dir).spawn();
                }
                #[cfg(target_os = "windows")]
                {
                    let _ = Command::new("explorer").arg(&target_dir).spawn();
                }
            } else {
                println!("❌ Target directory not found");
            }
        }
    }
    Ok(())
}
fn handle_register(
    license_key: Option<String>,
    status: bool,
    remaining: bool,
) -> Result<()> {
    let license_manager = crate::captain::license::LicenseManager::new()?;
    if remaining {
        match license_manager.check_remaining_commands() {
            Ok(-1) => println!("🚀 Unlimited commands (Pro license active)"),
            Ok(remaining) if remaining > 0 => {
                println!(
                    "📊 {} commands remaining today", remaining.to_string().green()
                )
            }
            Ok(0) => println!("❌ Daily limit reached (0 commands remaining)"),
            Ok(remaining) => {
                println!("⚠️  Unexpected remaining commands value: {}", remaining)
            }
            Err(e) => println!("❌ Error: {}", e.to_string().red()),
        }
    } else if status {
        match license_manager.get_license_info() {
            Ok(info) => {
                println!("🦀 Current License Status:");
                println!(
                    "   License Key: {}", info["license_key"].as_str()
                    .unwrap_or("Unknown").yellow()
                );
                println!(
                    "   Tier: {}", info["tier"].as_str().unwrap_or("Unknown").green()
                );
                let daily_usage = info["daily_usage_count"].as_i64().unwrap_or(0);
                let daily_limit = info["daily_limit"].as_i64().unwrap_or(0);
                let remaining = info["remaining_commands"].as_i64().unwrap_or(0);
                if status && !is_build_process() {
                    if info["tier"] == "FREE" {
                        println!(
                            "   Daily Usage: {}/{} commands", daily_usage.to_string()
                            .yellow(), daily_limit.to_string().cyan()
                        );
                        if remaining > 0 {
                            println!(
                                "   Remaining Today: {} commands", remaining.to_string()
                                .green()
                            );
                        } else {
                            println!("   Status: {}", "LIMIT REACHED".red());
                            println!("   💡 Upgrade to Pro for unlimited commands!");
                        }
                    } else {
                        println!("   Usage: {}", "UNLIMITED".green());
                        println!(
                            "   Total Commands: {}", daily_usage.to_string().cyan()
                        );
                    }
                }
                if status {
                    if let Some(usage) = info["usage"].as_object() {
                        if let Some(total) = usage.get("total_commands") {
                            println!(
                                "   All-time Commands: {}", total.as_i64().unwrap_or(0)
                                .to_string().blue()
                            );
                        }
                        if let Some(today) = usage.get("today_commands") {
                            if today.as_i64().unwrap_or(0) > 0 {
                                println!(
                                    "   Commands Today: {}", today.as_i64().unwrap_or(0)
                                    .to_string().yellow()
                                );
                            }
                        }
                        if let Some(last_used) = usage.get("last_used") {
                            if let Some(last) = last_used.as_str() {
                                println!("   Last Used: {}", last.cyan());
                            }
                        }
                    }
                }
                if status {
                    match license_manager.is_license_expired() {
                        Ok(true) => {
                            println!("   ⚠️  License Status: {}", "EXPIRED".red())
                        }
                        Ok(false) => {
                            println!("   ✅ License Status: {}", "ACTIVE".green())
                        }
                        Err(_) => {
                            println!("   ❓ License Status: {}", "UNKNOWN".yellow())
                        }
                    }
                }
            }
            Err(e) => {
                println!("❌ No license registered: {}", e.to_string().red());
                println!("💡 Register a Pro license with: cm register <license-key>");
                println!(
                    "Go to Cargo.do/pro or use Cargo Mate for free with 10 commands per day!"
                );
            }
        }
    } else if let Some(key) = license_key {
        match license_manager.register_license(&key) {
            Ok(_) => {
                println!("✅ License registered successfully!");
                println!("🔄 Testing license validity...");
                match license_manager.check_license_status() {
                    Ok(validation) => {
                        if validation.valid {
                            println!("✅ License is valid and active!");
                            if validation.tier == "PRO" {
                                println!(
                                    "🚀 You now have unlimited access to all features!"
                                );
                            }
                        } else {
                            println!(
                                "⚠️  License registered but validation failed:"
                            );
                            if let Some(error) = validation.error {
                                println!("   {}", error.yellow());
                            }
                        }
                    }
                    Err(e) => {
                        println!(
                            "❌ License validation error: {}", e.to_string().red()
                        );
                    }
                }
            }
            Err(e) => {
                println!("❌ Failed to register license: {}", e.to_string().red());
                println!("💡 Make sure your license key is correct and active");
            }
        }
    } else {
        println!("🦀 Cargo Mate License Management");
        println!();
        println!("USAGE:");
        println!("  cm register <license-key>    Register your Pro license");
        println!("  cm register --status         Check detailed license status");
        println!("  cm register --remaining      Show remaining commands count");
        println!();
        println!("EXAMPLES:");
        println!("  cm register CM-ABC12-DEF34-GHI56");
        println!("  cm register --status");
        println!("  cm register --remaining");
        println!();
        println!(
            "💡 Pro licenses provide unlimited commands. Free tier: 10 commands/day"
        );
        println!("   Get a Pro license at: https://cargo.do/pro");
        println!("   Use for free: 10 commands per day, resets daily at midnight UTC");
    }
    Ok(())
}
fn is_build_process() -> bool {
    std::env::var("CARGO").is_ok() || std::env::var("RUSTC").is_ok()
        || std::env::var("CARGO_MANIFEST_DIR").is_ok()
        || std::env::var("CARGO_PKG_NAME").is_ok()
}
fn handle_idea(idea: &str) -> Result<()> {
    println!("💡 Submitting idea: {}", idea.yellow());
    println!("🔗 Testing API communication...");
    if let Err(e) = save_idea_history(idea) {
        eprintln!("⚠️  Failed to save idea to history: {}", e);
    }
    let license_manager = crate::captain::license::LicenseManager::new()?;
    let api_base_url = env::var("CARGO_MATE_API")
        .unwrap_or_else(|_| "https://cargo.do/api".to_string());
    let user_id = license_manager.get_or_create_user_id()?;
    let idea_data = serde_json::json!(
        { "user_id" : user_id, "idea" : idea, "timestamp" : chrono::Utc::now()
        .to_rfc3339(), "source" : "cargo-mate-cli" }
    );
    let client = reqwest::blocking::Client::new();
    let endpoint = format!("{}/idea/{}", api_base_url, urlencoding::encode(idea));
    println!("📡 Sending to: {}", endpoint.cyan());
    match client
        .post(&endpoint)
        .json(&idea_data)
        .timeout(std::time::Duration::from_secs(10))
        .send()
    {
        Ok(response) => {
            if response.status().is_success() {
                println!("✅ Idea submitted successfully!");
                println!("🚀 API communication working perfectly");
                println!(
                    "📊 Response status: {}", response.status().to_string().green()
                );
                match response.json::<serde_json::Value>() {
                    Ok(json_response) => {
                        println!(
                            "📋 Server response: {}", serde_json::to_string_pretty(&
                            json_response).unwrap_or_else(| _ | "Unable to format JSON"
                            .to_string())
                        );
                    }
                    Err(_) => {
                        println!("📋 Response received (non-JSON format)");
                    }
                }
            } else {
                println!(
                    "⚠️  API responded with error: {}", response.status().to_string()
                    .yellow()
                );
                match response.text() {
                    Ok(error_text) => println!("   Error details: {}", error_text),
                    Err(_) => println!("   Unable to read error details"),
                }
            }
        }
        Err(e) => {
            crate::captain::wtf::display_api_failure_art();
            println!("❌ API communication failed: {}", e.to_string().red());
            println!("🔍 This could indicate:");
            println!("   • Network connectivity issues");
            println!("   • API server is down");
            println!("   • Firewall blocking the request");
            println!("   • DNS resolution problems");
            println!("💡 Try testing with: curl -X POST {}", endpoint);
        }
    }
    Ok(())
}
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct IdeaHistoryEntry {
    id: String,
    idea: String,
    timestamp: String,
}
fn save_idea_history(idea: &str) -> Result<()> {
    let shipwreck = dirs::home_dir()
        .context("Could not find home directory")?
        .join(".shipwreck");
    let history_dir = shipwreck.join("idea_history");
    std::fs::create_dir_all(&history_dir)?;
    let history_file = history_dir.join("history.json");
    let mut history: Vec<IdeaHistoryEntry> = if history_file.exists() {
        let content = std::fs::read_to_string(&history_file)?;
        serde_json::from_str(&content).unwrap_or_default()
    } else {
        Vec::new()
    };
    let entry = IdeaHistoryEntry {
        id: format!("idea_{}", chrono::Utc::now().timestamp()),
        idea: idea.to_string(),
        timestamp: chrono::Utc::now().to_rfc3339(),
    };
    history.insert(0, entry);
    history.truncate(50);
    let json = serde_json::to_string_pretty(&history)?;
    std::fs::write(&history_file, json)?;
    Ok(())
}
fn get_idea_history(limit: usize) -> Result<Vec<IdeaHistoryEntry>> {
    let shipwreck = dirs::home_dir()
        .context("Could not find home directory")?
        .join(".shipwreck");
    let history_file = shipwreck.join("idea_history").join("history.json");
    if history_file.exists() {
        let content = std::fs::read_to_string(&history_file)?;
        let history: Vec<IdeaHistoryEntry> = serde_json::from_str(&content)
            .unwrap_or_default();
        Ok(history.into_iter().take(limit).collect())
    } else {
        Ok(Vec::new())
    }
}
fn get_recent_errors(count: usize) -> Result<Vec<String>> {
    let shipwreck = dirs::home_dir()
        .context("Could not find home directory")?
        .join(".shipwreck");
    let error_file = shipwreck.join("errors").join("latest.txt");
    if error_file.exists() {
        let content = std::fs::read_to_string(&error_file)?;
        let errors: Vec<String> = content
            .lines()
            .filter(|line| !line.trim().is_empty())
            .take(count)
            .map(|s| s.to_string())
            .collect();
        if errors.is_empty() {
            Ok(vec!["No recent errors found in the error logs.".to_string()])
        } else {
            Ok(errors)
        }
    } else {
        Ok(
            vec![
                "No error log file found. Try running some cargo commands first."
                .to_string()
            ],
        )
    }
}
fn show_loading_messages() {
    let messages = [
        "⚓ Hoisting the sails... preparing to set sail for knowledge!",
        "🌊 Riding the waves... surfing through the digital ocean!",
        "🧭 Checking the compass... navigating to the answer!",
        "🚢 Batten down the hatches... stormy seas of computation ahead!",
        "🦜 Arr, matey! Consulting the ancient tomes of wisdom!",
        "⚡ Charging the canons... ready to fire the knowledge salvo!",
        "🧜‍♀️ Singing sea shanties... luring the answers from the deep!",
        "🗺️ Reading the treasure map... X marks the spot of knowledge!",
        "🦈 Dodging digital sharks... swimming towards the answer!",
        "🌟 Aligning the stars... consulting the celestial database!",
    ];
    let mut index = 0;
    let start_time = std::time::Instant::now();
    while start_time.elapsed().as_secs() < 30 {
        println!("⏳ {}", messages[index]);
        std::thread::sleep(std::time::Duration::from_secs(3));
        index = (index + 1) % messages.len();
    }
}
fn handle_user() -> Result<()> {
    let license_manager = crate::captain::license::LicenseManager::new()?;
    license_manager.show_user_info()?;
    Ok(())
}
fn handle_activate() -> Result<()> {
    println!("⚡ Activating Cargo Mate shell integration...");
    let shell = detect_shell();
    let rc_file = get_rc_file(&shell)?;
    if !rc_file.exists() {
        println!("❌ No shell configuration file found: {}", rc_file.display());
        println!("💡 Run 'cm init' first to set up shell integration");
        return Ok(());
    }
    let content = fs::read_to_string(&rc_file)?;
    if !content.contains("# === Cargo Mate") {
        println!("❌ Cargo Mate integration not found in {}", rc_file.display());
        println!("💡 Run 'cm init' first to set up shell integration");
        return Ok(());
    }
    println!("🔄 Sourcing {}", rc_file.display().to_string().cyan());
    let output = std::process::Command::new(&shell)
        .arg("-c")
        .arg(format!("source {} && env", rc_file.display()))
        .output()?;
    if output.status.success() {
        println!("✅ Shell integration activated successfully!");
        println!();
        println!("🚢 {}", "You can now use:".yellow());
        println!("   {} - cargo commands go through cargo-mate", "cargo".cyan());
        println!("   {} - direct cargo-mate access", "cm".cyan());
        println!("   {} - quick alias", "cg".cyan());
        println!();
        println!("🎯 {}", "Try it:".green());
        println!("   cargo --version");
        println!("   cm --help");
    } else {
        let error = String::from_utf8_lossy(&output.stderr);
        println!("❌ Failed to activate integration: {}", error);
        println!(
            "💡 You can manually run: {}", format!("source {}", rc_file.display())
            .green()
        );
    }
    Ok(())
}
fn parse_bool(s: &str) -> Result<bool, std::num::ParseIntError> {
    match s.to_lowercase().as_str() {
        "true" | "1" | "yes" | "on" => Ok(true),
        "false" | "0" | "no" | "off" => Ok(false),
        _ => Ok(s.parse::<u8>()? != 0),
    }
}
fn handle_optimize(action: OptimizeAction) -> Result<()> {
    let optimizer = optimize::BuildOptimizer::new(None)?;
    match action {
        OptimizeAction::Aggressive => {
            optimizer.optimize_build(optimize::OptimizationProfile::Aggressive)?;
        }
        OptimizeAction::Balanced => {
            optimizer.optimize_build(optimize::OptimizationProfile::Balanced)?;
        }
        OptimizeAction::Conservative => {
            optimizer.optimize_build(optimize::OptimizationProfile::Conservative)?;
        }
        OptimizeAction::Custom {
            jobs,
            incremental,
            opt_level,
            debug_level,
            codegen_units,
        } => {
            let incremental_bool = incremental.to_lowercase() == "true";
            let profile = optimize::OptimizationProfile::Custom {
                jobs,
                incremental: incremental_bool,
                opt_level,
                debug_level,
                codegen_units,
            };
            optimizer.optimize_build(profile)?;
        }
        OptimizeAction::Status => {
            optimizer.show_status()?;
        }
        OptimizeAction::Recommendations => {
            optimizer.show_recommendations()?;
        }
        OptimizeAction::Restore => {
            optimizer.restore_backup()?;
        }
    }
    Ok(())
}
fn handle_scrub(action: ScrubAction) -> Result<()> {
    match action {
        ScrubAction::Run { dry_run, verbose, start, resume, min_depth, max_depth } => {
            let options = scrub::ScrubOptions {
                dry_run,
                verbose,
                start_dir: std::path::PathBuf::from(start),
                resume_from: resume,
                min_depth,
                max_depth,
            };
            let scrubber = scrub::CargoScrubber::new(options);
            scrubber.scrub()?;
        }
        ScrubAction::Help => {
            println!("🧹 Cargo Scrub - System-wide Cargo Clean");
            println!();
            println!("USAGE:");
            println!("  cm scrub run [OPTIONS]");
            println!();
            println!("OPTIONS:");
            println!(
                "  --dry-run       Show what would be cleaned without actually doing it"
            );
            println!("  -v, --verbose   Verbose output");
            println!("  -s, --start DIR Start directory (default: /)");
            println!("  -r, --resume    Resume from specific project directory");
            println!("  --min-depth N   Minimum depth to search (default: 1)");
            println!("  --max-depth N   Maximum depth to search (default: 10)");
            println!();
            println!("EXAMPLES:");
            println!(
                "  cm scrub run --dry-run              # See what would be cleaned"
            );
            println!("  cm scrub run -v                      # Verbose output");
            println!("  cm scrub run -s /home                # Only search in /home");
            println!(
                "  cm scrub run -r my-project           # Resume from projects containing 'my-project'"
            );
        }
    }
    Ok(())
}
fn handle_test() -> Result<()> {
    println!("🧪 Running test command that will generate and log an error...");
    let shipwreck = dirs::home_dir()
        .context("Could not find home directory")?
        .join(".shipwreck");
    std::fs::create_dir_all(shipwreck.join("errors"))?;
    let error_file = shipwreck.join("errors").join("latest.txt");
    let error_message = format!(
        "🧪 Test Error: This is a deliberate test error from the test command\nTime: {}\nCommand: cm test\nError: Test error - demonstrating error logging functionality\n",
        chrono::Utc::now().to_rfc3339()
    );
    std::fs::write(&error_file, error_message)?;
    println!("📝 Error logged to: {}", error_file.display());
    println!("✅ Test error successfully logged!");
    println!("💡 Now run 'cm view errors' to see this error");
    Ok(())
}
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ChecklistItem {
    id: usize,
    text: String,
    done: bool,
    created_at: String,
}
fn handle_checklist(action: ChecklistAction) -> Result<()> {
    let shipwreck = dirs::home_dir()
        .context("Could not find home directory")?
        .join(".shipwreck");
    let checklist_dir = shipwreck.join("checklists");
    std::fs::create_dir_all(&checklist_dir)?;
    let checklist_file = checklist_dir.join("items.json");
    let mut items: Vec<ChecklistItem> = if checklist_file.exists() {
        let content = std::fs::read_to_string(&checklist_file)?;
        if content.trim().is_empty() {
            Vec::new()
        } else {
            serde_json::from_str(&content).unwrap_or_else(|_| Vec::new())
        }
    } else {
        Vec::new()
    };
    match action {
        ChecklistAction::Show | ChecklistAction::List => {
            if items.is_empty() {
                println!("📋 Checklist is empty");
                println!("💡 Add items with: cm checklist add \"Your task here\"");
            } else {
                println!("📋 Current Checklist:");
                println!("{}", "═".repeat(60).cyan());
                for item in &items {
                    let status = if item.done { "✅" } else { "❌" };
                    let checkbox = if item.done { "☑️" } else { "☐" };
                    println!("{}. {} {} {}", item.id, checkbox, item.text, status);
                }
                println!();
                let done_count = items.iter().filter(|i| i.done).count();
                println!(
                    "📊 Progress: {}/{} items completed", done_count, items.len()
                );
            }
        }
        ChecklistAction::Add { item } => {
            let next_id = items.iter().map(|i| i.id).max().unwrap_or(0) + 1;
            let new_item = ChecklistItem {
                id: next_id,
                text: item.clone(),
                done: false,
                created_at: chrono::Utc::now().to_rfc3339(),
            };
            items.push(new_item);
            let content = serde_json::to_string_pretty(&items)?;
            std::fs::write(&checklist_file, content)?;
            println!("✅ Added item #{}: {}", next_id, item);
            println!("💡 Mark as done with: cm checklist done {}", next_id);
        }
        ChecklistAction::Done { items: item_ids } => {
            let ids_to_mark: Vec<usize> = item_ids
                .split(',')
                .filter_map(|s| s.trim().parse::<usize>().ok())
                .collect();
            let mut marked_count = 0;
            for item in &mut items {
                if ids_to_mark.contains(&item.id) && !item.done {
                    item.done = true;
                    marked_count += 1;
                }
            }
            if marked_count > 0 {
                let content = serde_json::to_string_pretty(&items)?;
                std::fs::write(&checklist_file, content)?;
                println!(
                    "✅ Marked {} item(s) as completed: {}", marked_count, item_ids
                );
            } else {
                println!(
                    "❌ No items were marked as done. Items may already be completed or not exist."
                );
            }
        }
        ChecklistAction::Clear { target } => {
            match target.as_str() {
                "all" => {
                    items.clear();
                    let content = serde_json::to_string_pretty(&items)?;
                    std::fs::write(&checklist_file, content)?;
                    println!("🗑️  Cleared all checklist items");
                }
                "done" => {
                    items.retain(|item| !item.done);
                    let content = serde_json::to_string_pretty(&items)?;
                    std::fs::write(&checklist_file, content)?;
                    println!("🗑️  Removed completed items from checklist");
                }
                _ => {
                    println!(
                        "❌ Invalid target. Use 'all' to clear everything or 'done' to remove completed items"
                    );
                }
            }
        }
    }
    Ok(())
}
fn show_help() {
    println!("{}", "🚢 Cargo Mate (cm) - A Rustic Journey".bold());
    println!();
    println!("{}", "USAGE:".yellow());
    println!("  cm                      Auto-build or run default build");
    println!("  cm <command>            Run cm command or pass to cargo");
    println!();
    println!("{}", "SPECIAL COMMANDS:".yellow());
    println!("  cm wtf                  🤖 Ask CargoMate AI a question");
    println!("  cm idea                 💡 Submit an idea for Cargo Mate");
    println!("  cm journey              🎬 Record and play command sequences");
    println!("  cm anchor               ⚓ Save and restore project states");
    println!("  cm log                  📝 Captain's log for build notes");
    println!("  cm tide                 🌊 Performance tracking charts");
    println!("  cm map                  🗺️  Dependency visualization");
    println!("  cm mutiny               🏴‍☠️ Override cargo restrictions");
    println!("  cm config               ⚙️  Configuration management");
    println!("  cm version              🚢 Version management and auto-incrementing");
    println!("  cm view                 🔍 View build results and artifacts");
    println!("  cm optimize             🚀 Build performance optimization");
    println!("  cm checklist            📋 Show error/warning checklist");
    println!("  cm scrub                🧹 System-wide cargo clean");
    println!("  cm history              📊 Show build history");
    println!("  cm install              🔧 Install shell integration");
    println!("  cm user                 👤 Show user information and license status");
    println!(
        "  cm affiliate            💰 Manage affiliate program & earning opportunities"
    );
    println!();
    println!("{}", "EXAMPLES:".yellow());
    println!("  cm journey record build-flow    # Record a build sequence");
    println!("  cm anchor save before-refactor  # Save current state");
    println!("  cm mutiny allow-warnings        # Temporarily allow warnings");
    println!("  cm map show                      # Show dependency tree");
    println!("  cm wtf er 10                     # Send recent errors to CargoMate AI");
    println!(
        "  cm wtf checklist 5               # Send 5 recent checklist items to CargoMate AI"
    );
    println!("  cm scrub run --dry-run           # Preview system-wide cargo clean");
    println!("  cm wtf ollama enable llama2      # Configure local Ollama integration");
    println!();
    println!("Run 'cm <command> --help' for more information on a command.");
}
fn run_cargo_with_wrapper(args: &[&str]) {
    if !args.is_empty() {
        let license_manager = match crate::captain::license::LicenseManager::new() {
            Ok(lm) => lm,
            Err(e) => {
                eprintln!("❌ Failed to initialize license system: {}", e);
                crate::captain::wtf::display_api_failure_art();
                std::process::exit(1);
            }
        };
        if let Err(e) = license_manager.enforce_license(&format!("cargo-{}", args[0])) {
            eprintln!("❌ License enforcement failed: {}", e);
            crate::captain::wtf::display_api_failure_art();
            std::process::exit(1);
        }
    }
    if let Err(e) = version::pre_operation_hook(None) {
        eprintln!("⚠️  Version auto-increment failed: {}", e);
    }
    display::run_cargo_with_display(args);
    if let Ok(mut log) = captain_log::CaptainLog::new() {
        let build_result = captain_log::BuildResult {
            success: true,
            error_count: 0,
            warning_count: 0,
            duration_seconds: 0.0,
        };
        if let Err(e) = log
            .log_command(&format!("cargo {}", args.join(" ")), build_result)
        {
            eprintln!("⚠️  Captain's Log recording failed: {}", e);
        }
        println!("\n📝 {}", "Captain's Log: Session recorded".dimmed());
    }
    if let Err(e) = version::post_operation_hook(None, true) {
        eprintln!("⚠️  Version post-operation hook failed: {}", e);
    }
}
fn run_tracked_command(command: &str, session_id: &str) -> Result<()> {
    use std::process::Command;
    use std::io::{BufRead, BufReader};
    let parts: Vec<&str> = command.split_whitespace().collect();
    if parts.is_empty() {
        return Err(anyhow::anyhow!("Empty command"));
    }
    let mut log = captain_log::CaptainLog::new()?;
    let parser = captain_log::CargoOutputParser::new();
    let mut cmd = Command::new(parts[0]);
    cmd.args(&parts[1..]);
    if parts[0] == "cargo" {
        cmd.arg("--message-format=json");
    }
    let start_time = std::time::Instant::now();
    let mut child = cmd
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()?;
    if let Some(stdout) = child.stdout.take() {
        let reader = BufReader::new(stdout);
        for line in reader.lines() {
            let line = line?;
            println!("{}", line);
            if let Some(msg) = parser.parse_message(&line)? {
                if let Some(diagnostic) = msg.message {
                    let entry = parser
                        .create_log_entry_from_diagnostic(&diagnostic, session_id);
                    log.log(&entry.message, entry.tags)?;
                }
            }
        }
    }
    if let Some(stderr) = child.stderr.take() {
        let reader = BufReader::new(stderr);
        for line in reader.lines() {
            let line = line?;
            eprintln!("{}", line);
            if let Some(msg) = parser.parse_message(&line)? {
                if let Some(diagnostic) = msg.message {
                    let entry = parser
                        .create_log_entry_from_diagnostic(&diagnostic, session_id);
                    log.log(&entry.message, entry.tags)?;
                }
            }
        }
    }
    let status = child.wait()?;
    let duration = start_time.elapsed();
    let build_result = captain_log::BuildResult {
        success: status.success(),
        error_count: 0,
        warning_count: 0,
        duration_seconds: duration.as_secs_f64(),
    };
    log.log_command(command, build_result)?;
    println!("\n🔍 Analysis:");
    let entries = log.get_recent(1000);
    let detector = captain_log::PatternDetector::new(
        entries.into_iter().cloned().collect(),
    );
    let recurring = detector.find_recurring_errors();
    if !recurring.is_empty() {
        println!("\n⚠️  Recurring Issues:");
        for (error_key, count, _) in recurring.into_iter().take(5) {
            println!("   {} ({})", error_key.cyan(), count);
        }
    }
    let regressions = detector.detect_build_time_regression();
    if !regressions.is_empty() {
        println!("\n📈 Build Time Regressions:");
        for (command, old_time, new_time) in regressions {
            println!(
                "   {}: {:.2}s → {:.2}s ({:.1}%)", command.cyan(), old_time, new_time,
                ((new_time - old_time) / old_time) * 100.0
            );
        }
    }
    Ok(())
}