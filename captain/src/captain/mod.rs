pub mod captain;
pub mod captain_log;
pub mod captain_status;
pub mod config;
pub mod embedder;
pub mod license;
pub mod license_guard;
pub mod optimize;
pub mod parser;
pub mod shell_integration;
pub mod tide;
pub mod treasure_map;
pub mod version;
pub mod version_commands;
pub mod wtf;
use anyhow::Result;
use clap::{Parser, Subcommand};
#[derive(Parser, Debug)]
#[command(name = "captain")]
#[command(about = "Advanced captain binary with real implementations")]
pub struct CaptainCli {
    #[command(subcommand)]
    pub command: Option<CaptainCommands>,
    #[arg(trailing_var_arg = true)]
    pub args: Vec<String>,
}
#[derive(Subcommand, Debug, Clone)]
pub enum CaptainCommands {
    Install,
    Update,
    Status,
    Run { args: Vec<String> },
}
pub fn run_captain() -> Result<()> {
    let cli = CaptainCli::parse();
    match cli.command {
        Some(CaptainCommands::Install) => {
            log::info!("🚢 Installing captain binary...");
            crate::cmd::captain::auto_install_captain()?;
            log::info!("✅ Captain binary installed successfully!");
            Ok(())
        }
        Some(CaptainCommands::Update) => {
            log::info!("🚢 Updating captain binary...");
            crate::cmd::captain::auto_install_captain()?;
            log::info!("✅ Captain binary updated successfully!");
            Ok(())
        }
        Some(CaptainCommands::Status) => {
            let status = captain_status::get_captain_status();
            log::info!("🚢 Captain Status:");
            log::info!(
                "   Installed: {}", if status.is_installed { "✅ Yes" } else { "❌ No"
                }
            );
            if let Some(path) = status.binary_path {
                log::info!("   Location: {}", path.display());
            }
            log::info!(
                "   Version: {}", status.version.unwrap_or_else(|| "Unknown".to_string())
            );
            Ok(())
        }
        Some(CaptainCommands::Run { args }) => {
            let args_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
            let output = embedder::extract_and_execute_captain(&args_refs)?;
            if !output.stdout.is_empty() {
                print!("{}", String::from_utf8_lossy(& output.stdout));
            }
            if !output.stderr.is_empty() {
                eprint!("{}", String::from_utf8_lossy(& output.stderr));
            }
            Ok(())
        }
        None => {
            if cli.args.is_empty() {
                log::info!("🚢 Captain - Advanced CLI tool");
                log::info!("   Use 'captain --help' for more information");
                Ok(())
            } else {
                let args_refs: Vec<&str> = cli.args.iter().map(|s| s.as_str()).collect();
                let output = embedder::extract_and_execute_captain(&args_refs)?;
                if !output.stdout.is_empty() {
                    print!("{}", String::from_utf8_lossy(& output.stdout));
                }
                if !output.stderr.is_empty() {
                    eprint!("{}", String::from_utf8_lossy(& output.stderr));
                }
                Ok(())
            }
        }
    }
}