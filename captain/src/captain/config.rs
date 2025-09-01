use anyhow::{Context, Result};
use clap::Subcommand;
use std::collections::HashMap;
use std::process::Command;
use colored::*;
#[derive(Subcommand, Debug, Clone)]
pub enum ConfigAction {
    Set { key: String, value: String, #[arg(long)] local: bool },
    Get { key: String },
    List,
    Init,
    Shortcut { name: String, command: String, #[arg(long)] local: bool },
    Hook { hook_type: String, command: String, #[arg(long)] local: bool },
}
pub struct ConfigManager;
impl ConfigManager {
    pub fn new() -> Result<Self> {
        println!(
            "⚙️ {}", "Advanced configuration requires the captain binary"
            .bright_blue()
        );
        println!("   Delegating configuration operations to captain...");
        Ok(ConfigManager)
    }
    pub fn load(&self) -> Result<HashMap<String, String>> {
        println!("⚙️ {}", "Loading configuration from captain binary".bright_blue());
        self.delegate_to_captain(vec!["config", "load"]).map(|_| HashMap::new())
    }
    pub fn save(&self, _config: HashMap<String, String>) -> Result<()> {
        println!(
            "⚙️ {}", "Saving configuration requires captain binary".bright_blue()
        );
        self.delegate_to_captain(vec!["config", "save"])?;
        Ok(())
    }
    pub fn merge_with_env(&self) -> Result<HashMap<String, String>> {
        println!(
            "⚙️ {}", "Merging with environment requires captain binary".bright_blue()
        );
        self.delegate_to_captain(vec!["config", "merge"]).map(|_| HashMap::new())
    }
    pub fn get(&self, key: &str) -> Option<String> {
        println!(
            "⚙️ {}", format!("Getting config '{}' requires captain binary", key)
            .bright_blue()
        );
        let _ = self.delegate_to_captain(vec!["config", "get", key]);
        None
    }
    pub fn init_local(&self) -> Result<()> {
        println!(
            "⚙️ {}", "Initializing local config requires captain binary"
            .bright_blue()
        );
        self.delegate_to_captain(vec!["config", "init", "--local"])?;
        Ok(())
    }
    pub fn set(&mut self, key: &str, value: &str, local: bool) -> Result<()> {
        println!(
            "⚙️ {}", format!("Setting config '{}' requires captain binary", key)
            .bright_blue()
        );
        let args = if local {
            vec!["config", "set", key, value, "--local"]
        } else {
            vec!["config", "set", key, value]
        };
        self.delegate_to_captain(args)?;
        Ok(())
    }
    pub fn show(&self) -> Result<()> {
        println!(
            "⚙️ {}", "Showing configuration requires captain binary".bright_blue()
        );
        self.delegate_to_captain(vec!["config", "list"])?;
        Ok(())
    }
    pub fn add_shortcut(
        &mut self,
        name: &str,
        command: &str,
        local: bool,
    ) -> Result<()> {
        println!(
            "⚙️ {}", format!("Adding shortcut '{}' requires captain binary", name)
            .bright_blue()
        );
        let args = if local {
            vec!["config", "shortcut", name, command, "--local"]
        } else {
            vec!["config", "shortcut", name, command]
        };
        self.delegate_to_captain(args)?;
        Ok(())
    }
    pub fn add_hook(
        &mut self,
        hook_type: &str,
        command: &str,
        local: bool,
    ) -> Result<()> {
        println!(
            "⚙️ {}", format!("Adding {} hook requires captain binary", hook_type)
            .bright_blue()
        );
        let args = if local {
            vec!["config", "hook", hook_type, command, "--local"]
        } else {
            vec!["config", "hook", hook_type, command]
        };
        self.delegate_to_captain(args)?;
        Ok(())
    }
    fn delegate_to_captain(&self, args: Vec<&str>) -> Result<bool> {
        let captain_path = match crate::captain::captain_status::find_captain_binary() {
            Some(path) => path,
            None => {
                println!("❌ {}", "Advanced captain binary not found".red().bold());
                println!(
                    "🔄 {}", "Auto-downloading captain binary from get.cargo.do/"
                    .cyan()
                );
                match crate::captain::captain_status::auto_download_captain() {
                    Ok(path) => path,
                    Err(e) => {
                        println!(
                            "❌ {}", format!("Failed to download captain: {}", e) .red()
                        );
                        println!("💡 {}", "Please run: cm captain install".cyan());
                        println!("   Or upgrade at: https://cargo.do/pro");
                        println!();
                        println!(
                            "💡 {}",
                            "Configuration features require the captain binary:".cyan()
                        );
                        println!("   • Configuration persistence");
                        println!("   • Project settings management");
                        println!("   • Custom shortcuts and hooks");
                        println!("   • Local vs global configuration");
                        return Ok(true);
                    }
                }
            }
        };
        let output = Command::new(&captain_path)
            .args(&args)
            .output()
            .context("Failed to execute captain binary for config operation")?;
        if !output.stdout.is_empty() {
            print!("{}", String::from_utf8_lossy(& output.stdout));
        }
        if !output.stderr.is_empty() {
            eprint!("{}", String::from_utf8_lossy(& output.stderr));
        }
        Ok(output.status.success())
    }
}
pub fn load_captain_config() -> Result<HashMap<String, String>> {
    println!(
        "⚙️ {}", "Loading captain configuration requires captain binary"
        .bright_blue()
    );
    let config_manager = ConfigManager::new()?;
    config_manager.load()
}
pub fn save_captain_config(config: HashMap<String, String>) -> Result<()> {
    println!(
        "⚙️ {}", "Saving captain configuration requires captain binary".bright_blue()
    );
    let config_manager = ConfigManager::new()?;
    config_manager.save(config)
}
pub fn handle_config_action(action: ConfigAction) -> Result<()> {
    println!(
        "⚙️ {}", "Advanced configuration management requires the captain binary"
        .bright_blue()
    );
    println!("   Delegating configuration action to captain...");
    let args: Vec<String> = match action {
        ConfigAction::Set { key, value, local } => {
            if local {
                vec![
                    "config".to_string(), "set".to_string(), key, value, "--local"
                    .to_string()
                ]
            } else {
                vec!["config".to_string(), "set".to_string(), key, value]
            }
        }
        ConfigAction::Get { key } => vec!["config".to_string(), "get".to_string(), key],
        ConfigAction::List => vec!["config".to_string(), "list".to_string()],
        ConfigAction::Init => vec!["config".to_string(), "init".to_string()],
        ConfigAction::Shortcut { name, command, local } => {
            if local {
                vec![
                    "config".to_string(), "shortcut".to_string(), name, command,
                    "--local".to_string()
                ]
            } else {
                vec!["config".to_string(), "shortcut".to_string(), name, command]
            }
        }
        ConfigAction::Hook { hook_type, command, local } => {
            if local {
                vec![
                    "config".to_string(), "hook".to_string(), hook_type, command,
                    "--local".to_string()
                ]
            } else {
                vec!["config".to_string(), "hook".to_string(), hook_type, command]
            }
        }
    };
    let str_args: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    let captain_path = match crate::captain::captain_status::find_captain_binary() {
        Some(path) => path,
        None => {
            println!("❌ {}", "Advanced captain binary not found".red().bold());
            println!("   Please run: cm captain install");
            println!("   Or upgrade at: https://cargo.do/pro");
            println!();
            println!(
                "💡 {}", "Configuration features require the captain binary:".cyan()
            );
            println!("   • Configuration persistence");
            println!("   • Project settings management");
            println!("   • Custom shortcuts and hooks");
            println!("   • Local vs global configuration");
            return Ok(());
        }
    };
    let output = Command::new(&captain_path)
        .args(&str_args)
        .output()
        .context("Failed to execute captain binary for config action")?;
    if !output.stdout.is_empty() {
        print!("{}", String::from_utf8_lossy(& output.stdout));
    }
    if !output.stderr.is_empty() {
        eprint!("{}", String::from_utf8_lossy(& output.stderr));
    }
    if !output.status.success() {
        println!(
            "❌ {}", format!("Captain binary exited with status: {}", output.status)
            .red()
        );
    }
    Ok(())
}