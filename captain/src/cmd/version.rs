use anyhow::Result;
use crate::captain::version::{VersionManager, IncrementPolicy};
use crate::cmd::smune::{VersionAction, VersionConfigAction, IncrementType};
use colored::Colorize;
pub fn handle_version(action: VersionAction) -> Result<()> {
    let mut version_manager = VersionManager::new(None)?;
    match action {
        VersionAction::Init { version } => {
            version_manager.init(Some(version.unwrap_or_else(|| "0.1.0".to_string())))?;
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
                    version_manager.config.increment_policy = IncrementPolicy::Minor;
                    let result = version_manager.increment()?;
                    version_manager.config.increment_policy = original_policy;
                    result
                }
                IncrementType::Major => {
                    let original_policy = version_manager
                        .config
                        .increment_policy
                        .clone();
                    version_manager.config.increment_policy = IncrementPolicy::Major;
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
                        IncrementType::Patch => IncrementPolicy::Patch,
                        IncrementType::Minor => IncrementPolicy::Minor,
                        IncrementType::Major => IncrementPolicy::Major,
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