use anyhow::Result;
use colored::*;
use clap::Subcommand;
use crate::version::{VersionManager, IncrementPolicy};
#[derive(Subcommand, Debug)]
pub enum VersionAction {
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
#[derive(Subcommand, Debug, Clone, clap::ValueEnum)]
pub enum IncrementType {
    Patch,
    Minor,
    Major,
}
#[derive(Subcommand, Debug)]
pub enum VersionConfigAction {
    Enable,
    Disable,
    Policy { #[arg(value_enum)] policy: IncrementType },
    Show,
}
pub fn handle_version(action: VersionAction) -> Result<()> {
    match action {
        VersionAction::Init { version } => {
            let mut manager = VersionManager::new(None)?;
            manager.init(version)?;
        }
        VersionAction::Info => {
            let manager = VersionManager::new(None)?;
            manager.show_info();
        }
        VersionAction::Increment { increment_type } => {
            let mut manager = VersionManager::new(None)?;
            let new_version = match increment_type {
                IncrementType::Patch => manager.increment()?,
                IncrementType::Minor => {
                    let original_policy = manager.config.increment_policy.clone();
                    manager.config.increment_policy = IncrementPolicy::Minor;
                    let result = manager.increment()?;
                    manager.config.increment_policy = original_policy;
                    result
                }
                IncrementType::Major => {
                    let original_policy = manager.config.increment_policy.clone();
                    manager.config.increment_policy = IncrementPolicy::Major;
                    let result = manager.increment()?;
                    manager.config.increment_policy = original_policy;
                    result
                }
            };
            println!("✅ Version incremented to: {}", new_version.cyan());
        }
        VersionAction::Set { version } => {
            let mut manager = VersionManager::new(None)?;
            manager.set_version(&version)?;
        }
        VersionAction::History => {
            println!("📚 Version history feature coming soon!");
            println!("For now, check the .v file for version changes.");
        }
        VersionAction::UpdateCargo => {
            let manager = VersionManager::new(None)?;
            manager.update_cargo_toml()?;
        }
        VersionAction::Config { action } => {
            match action {
                VersionConfigAction::Enable => {
                    let mut manager = VersionManager::new(None)?;
                    manager.config.auto_increment = true;
                    manager.save_config()?;
                    println!("✅ Auto-increment enabled");
                }
                VersionConfigAction::Disable => {
                    let mut manager = VersionManager::new(None)?;
                    manager.config.auto_increment = false;
                    manager.save_config()?;
                    println!("✅ Auto-increment disabled");
                }
                VersionConfigAction::Policy { policy } => {
                    let mut manager = VersionManager::new(None)?;
                    manager.config.increment_policy = match policy {
                        IncrementType::Patch => IncrementPolicy::Patch,
                        IncrementType::Minor => IncrementPolicy::Minor,
                        IncrementType::Major => IncrementPolicy::Major,
                    };
                    manager.save_config()?;
                    println!("✅ Increment policy updated");
                }
                VersionConfigAction::Show => {
                    let manager = VersionManager::new(None)?;
                    manager.show_info();
                }
            }
        }
    }
    Ok(())
}