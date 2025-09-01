use anyhow::Result;
use crate::captain::config::ConfigAction;
pub fn handle_config(action: crate::captain::config::ConfigAction) -> Result<()> {
    crate::captain::config::handle_config_action(action)
}
pub fn handle_config_internal(action: ConfigAction) -> Result<()> {
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