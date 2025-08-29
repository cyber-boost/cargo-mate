use anyhow::Result;
use clap::Subcommand;
use std::collections::HashMap;
#[derive(Subcommand, Debug)]
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
        Ok(ConfigManager)
    }
    pub fn load(&self) -> Result<HashMap<String, String>> {
        Ok(HashMap::new())
    }
    pub fn save(&self, _config: HashMap<String, String>) -> Result<()> {
        Ok(())
    }
    pub fn merge_with_env(&self) -> Result<HashMap<String, String>> {
        Ok(HashMap::new())
    }
    pub fn get(&self, _key: &str) -> Option<String> {
        None
    }
    pub fn init_local(&self) -> Result<()> {
        Ok(())
    }
    pub fn set(&mut self, _key: &str, _value: &str, _local: bool) -> Result<()> {
        Ok(())
    }
    pub fn show(&self) -> Result<()> {
        Ok(())
    }
    pub fn add_shortcut(
        &mut self,
        _name: &str,
        _command: &str,
        _local: bool,
    ) -> Result<()> {
        Ok(())
    }
    pub fn add_hook(
        &mut self,
        _hook_type: &str,
        _command: &str,
        _local: bool,
    ) -> Result<()> {
        Ok(())
    }
}
pub fn load_captain_config() -> Result<HashMap<String, String>> {
    Ok(HashMap::new())
}
pub fn save_captain_config(_config: HashMap<String, String>) -> Result<()> {
    Ok(())
}
pub fn handle_config_action(_action: ConfigAction) -> Result<()> {
    eprintln!("⚙️ Advanced configuration management requires the captain binary.");
    eprintln!("💡 Download captain with: cm install");
    eprintln!("   Captain provides configuration persistence and advanced settings.");
    Ok(())
}