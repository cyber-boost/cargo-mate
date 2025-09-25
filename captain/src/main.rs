use anyhow::Result;
use chrono;
use clap::Parser;
use colored::*;
use serde::{Deserialize, Serialize};
use serde_json;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use reqwest;
use progress::{BuildProgressBar, BuildTracker, BuildRecord};
use std::sync::atomic::Ordering;
use std::time::Instant;
use std::collections::HashMap;
use std::process::Output;
use cargo_mate::cmd::smune::{Args, Commands, JourneyAction};
use cargo_mate::cmd::{
    init, help, log, map, mutiny, config, version, view, test, optimize, checklist,
    activate, journey, idea, tool, scrub, sweep, anchor, captain, tide, ddr,
};
use cargo_mate::probe;
use cargo_mate::scat;
use cargo_mate::cmd::smune::DockDockRustCommands;
use cargo_mate::cmd::ddr::DdrAction;
use cargo_mate::captain::{
    config::ConfigManager, wtf, license::LicenseManager,
    shell_integration::ShellIntegration, captain_status,
};
use cargo_mate::history;
use cargo_mate::strip;
use cargo_mate::admin_msg;
use cargo_mate::sweeping;
use cargo_mate::journey::JourneyPlayer;
use cargo_mate::display;
mod tools;
mod command_factory;
mod progress;
mod utils;
#[tokio::main]
async fn main() -> Result<()> {
    if let Err(e) = run().await {
        eprintln!("❌ Error: {}", e);
        cargo_mate::captain::wtf::display_api_failure_art();
        std::process::exit(1);
    }
    Ok(())
}
async fn run() -> Result<()> {
    init::ensure_initialized();
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
                let license_manager = LicenseManager::new()?;
                match command {
                    Commands::Init => {
                        license_manager.enforce_license("init")?;
                    }
                    Commands::Journey { .. } => {
                        license_manager.enforce_license("journey")?;
                    }
                    Commands::Anchor { .. } => {
                        license_manager.enforce_license("anchor")?;
                    }
                    Commands::Log { .. } => {
                        license_manager.enforce_license("log")?;
                    }
                    Commands::Tide { .. } => {
                        license_manager.enforce_license("tide")?;
                    }
                    Commands::Map { .. } => {
                        license_manager.enforce_license("map")?;
                    }
                    Commands::Mutiny { .. } => {
                        license_manager.enforce_license("mutiny")?;
                    }
                    Commands::Config { .. } => {
                        license_manager.enforce_license("config")?;
                    }
                    Commands::Version { .. } => {
                        license_manager.enforce_license("version")?;
                    }
                    Commands::View { .. } => {
                        license_manager.enforce_license("view")?;
                    }
                    Commands::Test => {
                        license_manager.enforce_license("test")?;
                    }
                    Commands::Probe { .. } => {
                        license_manager.enforce_license("probe")?;
                    }
                    Commands::Optimize { .. } => {
                        license_manager.enforce_license("optimize")?;
                    }
                    Commands::Checklist { .. } => {
                        license_manager.enforce_license("checklist")?;
                    }
                    Commands::History { .. } => {
                        license_manager.enforce_license("history")?;
                    }
                    Commands::Scrub { .. } => {
                        license_manager.enforce_license("scrub")?;
                    }
                    Commands::Sweep { .. } => {}
                    Commands::Install => {
                        license_manager.enforce_license("install")?;
                    }
                    Commands::Activate => {
                        license_manager.enforce_license("activate")?;
                    }
                    Commands::Exec { .. } => {
                        license_manager.enforce_license("exec")?;
                    }
                    Commands::Idea { .. } => {
                        license_manager.enforce_license("idea")?;
                    }
                    Commands::Wtf { .. } => {
                        license_manager.enforce_license("wtf")?;
                    }
                    Commands::User => {
                        license_manager.enforce_license("user")?;
                    }
                    Commands::Debug => {
                        license_manager.enforce_license("debug")?;
                    }
                    Commands::Strip(_) => {}
                    Commands::Scat { .. } => {
                        license_manager.enforce_license("scat")?;
                    }
                    Commands::Tool { .. } => {
                        license_manager.enforce_license("tool")?;
                    }
                    Commands::Ddr { .. } => {
                        license_manager.enforce_license("ddr")?;
                    }
                    Commands::Register { .. } => {}
                }
            }
        }
    }
    match args.command {
        Some(command) => execute_command(command),
        None => handle_default_command(),
    }
}
fn update_path_for_captain() {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let current_path = std::env::var("PATH").unwrap_or_default();
    let new_path = format!("{}/.shipwreck/bin:{}", home, current_path);
    std::env::set_var("PATH", new_path);
}
fn should_check_admin_msg(args: &Args) -> bool {
    match args.command {
        Some(Commands::Register { .. })
        | Some(Commands::Activate)
        | Some(Commands::Install) => false,
        _ => true,
    }
}
fn requires_license_check(args: &Args) -> bool {
    match args.command {
        Some(Commands::Wtf { .. }) | Some(Commands::Exec { .. }) => false,
        _ => true,
    }
}
fn is_cargo_command(args: &Args) -> bool {
    matches!(args.command, Some(Commands::Exec { .. })) || args.command.is_none()
}
fn get_command_name(args: &Args) -> String {
    match &args.command {
        Some(cmd) => cmd.name().to_string(),
        None => "build".to_string(),
    }
}
fn format_command(args: &Args) -> String {
    match &args.command {
        Some(Commands::Exec { cargo_args }) => format!("cargo {}", cargo_args.join(" ")),
        Some(cmd) => format!("cm {}", cmd.name()),
        None => "cargo build".to_string(),
    }
}
fn execute_command(command: Commands) -> Result<()> {
    let factory = command_factory::CommandFactory::new();
    match command {
        Commands::Init => init::handle_init(),
        Commands::Journey { action } => journey::handle_journey(action),
        Commands::Anchor { action } => anchor::handle_anchor(action),
        Commands::Log { action } => log::handle_log(action),
        Commands::Tide { action } => tide::handle_tide(action),
        Commands::Map { action } => map::handle_map_internal(action),
        Commands::Mutiny { action } => mutiny::handle_mutiny_internal(action),
        Commands::Config { action } => config::handle_config(action),
        Commands::Version { action } => version::handle_version(action),
        Commands::View { action } => view::handle_view(action),
        Commands::Test => test::handle_test(),
        Commands::Probe { action } => probe::handle_probe(action),
        Commands::Optimize { action } => optimize::handle_optimize(action),
        Commands::Checklist { action } => checklist::handle_checklist_internal(action),
        Commands::History { kind, limit } => {
            let args = vec![kind, limit.to_string()];
            history::show_history(&args);
            Ok(())
        }
        Commands::Ddr { action } => {
            let runtime = tokio::runtime::Runtime::new()?;
            runtime
                .block_on(async {
                    match action {
                        DockDockRustCommands::DockDockRust {
                            image,
                            target,
                            jobs,
                            config,
                            ..
                        } => {
                            let ddr_action = DdrAction::Build {
                                image: Some(image),
                                target,
                                jobs,
                                config,
                                use_config: false,
                            };
                            ddr::handle_ddr(Some(ddr_action)).await
                        }
                    }
                })
        }
        Commands::Scrub { action } => scrub::handle_scrub(action),
        Commands::Sweep { action } => sweep::handle_sweep(action),
        Commands::Install => activate::handle_install(),
        Commands::Activate => activate::handle_activate(),
        Commands::Exec { cargo_args } => {
            let args_refs: Vec<&str> = cargo_args.iter().map(|s| s.as_str()).collect();
            display::run_cargo_with_display(&args_refs);
            Ok(())
        }
        Commands::Register { license_key, status, remaining } => {
            let mut config = ConfigManager::new()?;
            if let Some(key) = license_key {
                config.set("license.key", &key, true)?;
            }
            if status {
                config.set("license.status", "true", true)?;
            }
            if remaining {
                config.set("license.remaining", "true", true)?;
            }
            Ok(())
        }
        Commands::Idea { idea } => idea::handle_idea(&idea),
        Commands::Wtf { action } => wtf::handle_wtf_action(action),
        Commands::User => {
            println!("👤 User management requires captain binary");
            Ok(())
        }
        Commands::Debug => {
            println!("🔍 Debug mode enabled");
            Ok(())
        }
        Commands::Strip(args) => strip::handle_strip_command(args),
        Commands::Scat { command } => scat::handle_scat_command(command),
        Commands::Tool { action } => tool::handle_tool(action),
    }
}
fn handle_default_command() -> Result<()> {
    let config = ConfigManager::new()?;
    let license_status = config
        .get("license.status")
        .unwrap_or_else(|| "inactive".to_string());
    if license_status == "active" {
        let journey_action = JourneyAction::Play {
            name: "default".to_string(),
            dry_run: false,
        };
        journey::handle_journey(journey_action)
    } else {
        match license_status.as_str() {
            "inactive" => {
                println!("⚠️  {}", "License is inactive".bright_yellow());
                help::show_help()?;
                Ok(())
            }
            _ => {
                println!("⚠️  {}", "License status unknown".bright_yellow());
                help::show_help()?;
                Ok(())
            }
        }
    }
}