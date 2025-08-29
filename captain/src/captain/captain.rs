use anyhow::Result;
use clap::{Parser, Subcommand};
use colored::*;
#[derive(Parser, Debug)]
#[command(name = "captain")]
#[command(about = "🚢 Cargo Mate Captain - Advanced Development Tools")]
pub struct Args {
    #[command(subcommand)]
    pub command: Option<CaptainCommand>,
}
#[derive(Subcommand, Debug)]
pub enum CaptainCommand {
    Wtf { #[command(subcommand)] action: crate::captain::wtf::WtfAction },
    Shell { #[command(subcommand)] action: ShellAction },
    Version {
        #[command(subcommand)]
        action: crate::captain::version_commands::VersionAction,
    },
    Config { #[command(subcommand)] action: crate::captain::config::ConfigAction },
    License { #[command(subcommand)] action: LicenseAction },
    Security { #[command(subcommand)] action: SecurityAction },
    Encrypt { input: String, output: String },
    Log { #[command(subcommand)] action: LogAction },
    Optimize { #[command(subcommand)] action: OptimizeAction },
    Analyze { #[command(subcommand)] action: AnalyzeAction },
}
#[derive(Subcommand, Debug)]
pub enum ShellAction {
    Install,
    Status,
    Activate,
}
#[derive(Subcommand, Debug)]
pub enum LicenseAction {
    Status,
    Activate,
    Check,
}
#[derive(Subcommand, Debug)]
pub enum LogAction {
    Show,
    Analyze,
    Export,
}
#[derive(Subcommand, Debug)]
pub enum OptimizeAction {
    Aggressive,
    Balanced,
    Conservative,
}
#[derive(Subcommand, Debug)]
pub enum AnalyzeAction {
    Dependencies,
    Performance,
    Patterns,
}
#[derive(Subcommand, Debug)]
pub enum SecurityAction {
    Encrypt { input: String, output: String },
    Decrypt { input: String, output: String },
    Status,
}
pub fn run() -> Result<()> {
    println!(
        "{}", "🚢 CARGO MATE CAPTAIN - THE HEART AND SOUL OF CARGO MATE".bold().cyan()
    );
    println!();
    println!("{}", "WITHOUT THE CAPTAIN WE DONT KNOW WHERE TO GO".yellow());
    println!("{}", "Lost at sea is not fun".cyan());
    println!("   1. Run: cm install NOT cm on me");
    println!("   2. If that fails, find your Captain here:");
    println!("      https://get.cargo.do/captain/captain-linux-x86_64.enc");
    println!("      https://get.cargo.do/captain/captain-linux-aarch64.enc");
    println!("      https://get.cargo.do/captain/captain-macos-x86_64.enc");
    println!("      https://get.cargo.do/captain/captain-macos-aarch64.enc");
    println!("      https://get.cargo.do/captain/captain-windows-x86_64.enc");
    println!("   3. Access sophisticated AI, shell integration, and analytics");
    println!();
    println!("{}", "🔧 Available commands:".green());
    println!("   • wtf     - AI-powered development assistance");
    println!("   • version - Intelligent version management");
    println!("   • config  - Configuration management");
    println!("   • license - License management");
    println!("   • security- Security features");
    println!("   • encrypt - Binary encryption");
    println!("   • shell   - Shell integration");
    println!("   • log     - Captain's log and analytics");
    println!("   • optimize- Build optimization and performance");
    println!("   • analyze - Code analysis tools");
    println!();
    println!(
        "{}", "🚀 Upgrade to Pro for the full captain experience!".bold().green()
    );
    println!("{}", "Go to cargo.do/pro to join the Captains Crew".cyan());
    Ok(())
}
#[derive(Debug)]
pub struct CaptainInterface;
impl CaptainInterface {
    pub fn new() -> Result<Self> {
        println!("🚢 CaptainInterface requires the real captain binary.");
        Ok(CaptainInterface)
    }
    pub fn delegate_to_captain(
        &self,
        _message: &str,
        _event_type: &str,
        _data: serde_json::Value,
    ) -> Result<()> {
        println!("🚢 Captain delegation requires the real captain binary.");
        Ok(())
    }
}
#[derive(Debug)]
pub struct CaptainManager;
impl CaptainManager {
    pub fn new() -> Result<Self> {
        println!("🚢 CaptainManager requires the real captain binary.");
        Ok(CaptainManager)
    }
    pub fn is_installed(&self) -> Result<bool> {
        println!("🚢 Captain installation check requires the real captain binary.");
        Ok(false)
    }
    pub async fn install(&self) -> Result<()> {
        println!("🚢 Captain installation requires the real captain binary.");
        Ok(())
    }
}
pub fn handle_captain_command(command: CaptainCommand) -> Result<()> {
    if crate::captain::captain_status::is_captain_available() {
        let captain_path = crate::captain::captain_status::get_captain_path();
        if let Some(captain_binary) = captain_path {
            let args = command_to_args(&command);
            match std::process::Command::new(&captain_binary).args(&args).status() {
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
    match command {
        CaptainCommand::Wtf { action } => {
            crate::captain::wtf::handle_wtf_action(action)?;
        }
        CaptainCommand::Version { action } => {
            crate::captain::version_commands::handle_version(action)?;
        }
        CaptainCommand::Config { action } => {
            crate::captain::config::handle_config_action(action)?;
        }
        _ => {
            println!("🤖 This captain feature requires the real captain binary.");
            println!("💡 Run 'cm install' to install captain for full functionality.");
        }
    }
    Ok(())
}
fn command_to_args(command: &CaptainCommand) -> Vec<String> {
    match command {
        CaptainCommand::Wtf { action } => {
            let mut args = vec!["wtf".to_string()];
            args.extend(crate::captain::wtf::action_to_args(action));
            args
        }
        CaptainCommand::Version { action } => {
            let mut args = vec!["version".to_string()];
            args.extend(
                crate::captain::version_commands::version_action_to_args(action),
            );
            args
        }
        CaptainCommand::Config { action } => {
            let mut args = vec!["config".to_string()];
            match action {
                crate::captain::config::ConfigAction::Get { key } => {
                    args.extend(vec!["get".to_string(), key.clone()]);
                }
                crate::captain::config::ConfigAction::Set { key, value, local } => {
                    args.extend(vec!["set".to_string(), key.clone(), value.clone()]);
                    if *local {
                        args.push("--local".to_string());
                    }
                }
                crate::captain::config::ConfigAction::List => {
                    args.push("list".to_string());
                }
                crate::captain::config::ConfigAction::Init => {
                    args.push("init".to_string());
                }
                crate::captain::config::ConfigAction::Shortcut {
                    name,
                    command,
                    local,
                } => {
                    args.extend(
                        vec!["shortcut".to_string(), name.clone(), command.clone()],
                    );
                    if *local {
                        args.push("--local".to_string());
                    }
                }
                crate::captain::config::ConfigAction::Hook {
                    hook_type,
                    command,
                    local,
                } => {
                    args.extend(
                        vec!["hook".to_string(), hook_type.clone(), command.clone()],
                    );
                    if *local {
                        args.push("--local".to_string());
                    }
                }
            }
            args
        }
        CaptainCommand::License { action } => {
            let mut args = vec!["license".to_string()];
            match action {
                LicenseAction::Status => args.push("status".to_string()),
                LicenseAction::Activate => args.push("activate".to_string()),
                LicenseAction::Check => args.push("check".to_string()),
            }
            args
        }
        CaptainCommand::Security { action } => {
            let mut args = vec!["security".to_string()];
            match action {
                SecurityAction::Encrypt { input, output } => {
                    args.extend(
                        vec!["encrypt".to_string(), input.clone(), output.clone()],
                    );
                }
                SecurityAction::Decrypt { input, output } => {
                    args.extend(
                        vec!["decrypt".to_string(), input.clone(), output.clone()],
                    );
                }
                SecurityAction::Status => {
                    args.push("status".to_string());
                }
            }
            args
        }
        CaptainCommand::Encrypt { input, output } => {
            vec!["encrypt".to_string(), input.clone(), output.clone()]
        }
        CaptainCommand::Shell { action } => {
            let mut args = vec!["shell".to_string()];
            match action {
                ShellAction::Install => args.push("install".to_string()),
                ShellAction::Status => args.push("status".to_string()),
                ShellAction::Activate => args.push("activate".to_string()),
            }
            args
        }
        CaptainCommand::Log { action } => {
            let mut args = vec!["log".to_string()];
            match action {
                LogAction::Show => args.push("show".to_string()),
                LogAction::Analyze => args.push("analyze".to_string()),
                LogAction::Export => args.push("export".to_string()),
            }
            args
        }
        CaptainCommand::Optimize { action } => {
            let mut args = vec!["optimize".to_string()];
            match action {
                OptimizeAction::Aggressive => args.push("aggressive".to_string()),
                OptimizeAction::Balanced => args.push("balanced".to_string()),
                OptimizeAction::Conservative => args.push("conservative".to_string()),
            }
            args
        }
        CaptainCommand::Analyze { action } => {
            let mut args = vec!["analyze".to_string()];
            match action {
                AnalyzeAction::Dependencies => args.push("dependencies".to_string()),
                AnalyzeAction::Performance => args.push("performance".to_string()),
                AnalyzeAction::Patterns => args.push("patterns".to_string()),
            }
            args
        }
    }
}
pub fn main() -> Result<()> {
    run()
}