use anyhow::{Context, Result};
use colored::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use crate::log::Log;
use super::license;
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProjectConfig {
    pub project: ProjectSettings,
    pub shortcuts: HashMap<String, String>,
    pub auto_fix: AutoFixSettings,
    pub journey: JourneySettings,
    pub build: BuildSettings,
    pub version_control: VersionControlSettings,
    pub hooks: HookSettings,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProjectSettings {
    pub name: Option<String>,
    pub default_journey: Option<String>,
    pub theme: String,
    pub auto_checklist: bool,
    pub track_performance: bool,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AutoFixSettings {
    pub format_on_save: bool,
    pub clippy_on_build: bool,
    pub auto_deps_update: bool,
    pub fix_warnings: bool,
    pub suggest_fixes: bool,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct JourneySettings {
    pub auto_record: bool,
    pub share_templates: bool,
    pub max_recordings: usize,
    pub interactive_by_default: bool,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BuildSettings {
    pub default_profile: String,
    pub parallel_jobs: Option<usize>,
    pub target_dir: Option<PathBuf>,
    pub incremental: bool,
    pub cache_artifacts: bool,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct HookSettings {
    pub pre_build: Vec<String>,
    pub post_build: Vec<String>,
    pub on_error: Vec<String>,
    pub on_success: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VersionControlSettings {
    pub auto_git_commit: bool,
    pub auto_anchor_git: bool,
}
pub struct ConfigManager {
    global_config: ProjectConfig,
    local_config: Option<ProjectConfig>,
    global_path: PathBuf,
    local_path: PathBuf,
}
impl ConfigManager {
    pub fn new() -> Result<Self> {
        let global_path = dirs::home_dir()
            .context("Could not find home directory")?
            .join(".shipwreck")
            .join("config.toml");
        let local_path = PathBuf::from(".cg");
        let global_config = if global_path.exists() {
            let content = fs::read_to_string(&global_path)?;
            toml::from_str(&content)?
        } else {
            ProjectConfig::default()
        };
        let local_config = if local_path.exists() {
            let content = fs::read_to_string(&local_path)?;
            Some(toml::from_str(&content)?)
        } else {
            None
        };
        Ok(Self {
            global_config,
            local_config,
            global_path,
            local_path,
        })
    }
    pub fn get(&self, key: &str) -> Option<String> {
        let parts: Vec<&str> = key.split('.').collect();
        if let Some(ref local) = self.local_config {
            if let Some(value) = self.get_from_config(local, &parts) {
                return Some(value);
            }
        }
        self.get_from_config(&self.global_config, &parts)
    }
    fn get_from_config(&self, config: &ProjectConfig, parts: &[&str]) -> Option<String> {
        match parts {
            ["project", field] => {
                match *field {
                    "name" => config.project.name.clone(),
                    "default_journey" => config.project.default_journey.clone(),
                    "theme" => Some(config.project.theme.clone()),
                    "auto_checklist" => Some(config.project.auto_checklist.to_string()),
                    "track_performance" => {
                        Some(config.project.track_performance.to_string())
                    }
                    _ => None,
                }
            }
            ["shortcuts", key] => config.shortcuts.get(*key).cloned(),
            ["auto_fix", field] => {
                match *field {
                    "format_on_save" => Some(config.auto_fix.format_on_save.to_string()),
                    "clippy_on_build" => {
                        Some(config.auto_fix.clippy_on_build.to_string())
                    }
                    "auto_deps_update" => {
                        Some(config.auto_fix.auto_deps_update.to_string())
                    }
                    "fix_warnings" => Some(config.auto_fix.fix_warnings.to_string()),
                    "suggest_fixes" => Some(config.auto_fix.suggest_fixes.to_string()),
                    _ => None,
                }
            }
            ["build", field] => {
                match *field {
                    "default_profile" => Some(config.build.default_profile.clone()),
                    "parallel_jobs" => config.build.parallel_jobs.map(|j| j.to_string()),
                    "incremental" => Some(config.build.incremental.to_string()),
                    "cache_artifacts" => Some(config.build.cache_artifacts.to_string()),
                    _ => None,
                }
            }
            _ => None,
        }
    }
    pub fn set(&mut self, key: &str, value: &str, local: bool) -> Result<()> {
        let config = if local {
            self.local_config.get_or_insert_with(ProjectConfig::default)
        } else {
            &mut self.global_config
        };
        let parts: Vec<&str> = key.split('.').collect();
        match parts.as_slice() {
            ["project", field] => {
                match *field {
                    "name" => config.project.name = Some(value.to_string()),
                    "default_journey" => {
                        config.project.default_journey = Some(value.to_string());
                    }
                    "theme" => config.project.theme = value.to_string(),
                    "auto_checklist" => config.project.auto_checklist = value.parse()?,
                    "track_performance" => {
                        config.project.track_performance = value.parse()?;
                    }
                    _ => return Err(anyhow::anyhow!("Unknown project field: {}", field)),
                }
            }
            ["shortcuts", key] => {
                config.shortcuts.insert(key.to_string(), value.to_string());
            }
            ["auto_fix", field] => {
                match *field {
                    "format_on_save" => config.auto_fix.format_on_save = value.parse()?,
                    "clippy_on_build" => config.auto_fix.clippy_on_build = value.parse()?,
                    "auto_deps_update" => {
                        config.auto_fix.auto_deps_update = value.parse()?;
                    }
                    "fix_warnings" => config.auto_fix.fix_warnings = value.parse()?,
                    "suggest_fixes" => config.auto_fix.suggest_fixes = value.parse()?,
                    _ => return Err(anyhow::anyhow!("Unknown auto_fix field: {}", field)),
                }
            }
            ["build", field] => {
                match *field {
                    "default_profile" => config.build.default_profile = value.to_string(),
                    "parallel_jobs" => config.build.parallel_jobs = Some(value.parse()?),
                    "incremental" => config.build.incremental = value.parse()?,
                    "cache_artifacts" => config.build.cache_artifacts = value.parse()?,
                    _ => return Err(anyhow::anyhow!("Unknown build field: {}", field)),
                }
            }
            _ => return Err(anyhow::anyhow!("Unknown config key: {}", key)),
        }
        self.save(local)?;
        println!("✅ Config set: {} = {}", key.cyan(), value.green());
        Ok(())
    }
    pub fn add_shortcut(
        &mut self,
        name: &str,
        command: &str,
        local: bool,
    ) -> Result<()> {
        let config = if local {
            self.local_config.get_or_insert_with(ProjectConfig::default)
        } else {
            &mut self.global_config
        };
        config.shortcuts.insert(name.to_string(), command.to_string());
        self.save(local)?;
        println!("✅ Shortcut added: {} → {}", name.cyan(), command.green());
        Ok(())
    }
    pub fn get_shortcut(&self, name: &str) -> Option<String> {
        if let Some(ref local) = self.local_config {
            if let Some(cmd) = local.shortcuts.get(name) {
                return Some(cmd.clone());
            }
        }
        self.global_config.shortcuts.get(name).cloned()
    }
    pub fn list_shortcuts(&self) {
        println!("{}", "=== Shortcuts ===".blue().bold());
        let mut all_shortcuts = self.global_config.shortcuts.clone();
        if let Some(ref local) = self.local_config {
            for (name, cmd) in &local.shortcuts {
                all_shortcuts.insert(name.clone(), cmd.clone());
            }
        }
        if all_shortcuts.is_empty() {
            println!("No shortcuts defined");
        } else {
            for (name, cmd) in all_shortcuts {
                println!("  {} → {}", name.cyan(), cmd.green());
            }
        }
    }
    pub fn add_hook(
        &mut self,
        hook_type: &str,
        command: &str,
        local: bool,
    ) -> Result<()> {
        let config = if local {
            self.local_config.get_or_insert_with(ProjectConfig::default)
        } else {
            &mut self.global_config
        };
        match hook_type {
            "pre_build" => config.hooks.pre_build.push(command.to_string()),
            "post_build" => config.hooks.post_build.push(command.to_string()),
            "on_error" => config.hooks.on_error.push(command.to_string()),
            "on_success" => config.hooks.on_success.push(command.to_string()),
            _ => return Err(anyhow::anyhow!("Unknown hook type: {}", hook_type)),
        }
        self.save(local)?;
        println!("✅ Hook added: {} → {}", hook_type.cyan(), command.green());
        Ok(())
    }
    pub fn run_hooks(&self, hook_type: &str) -> Result<()> {
        let mut hooks = Vec::new();
        match hook_type {
            "pre_build" => {
                hooks.extend(self.global_config.hooks.pre_build.clone());
                if let Some(ref local) = self.local_config {
                    hooks.extend(local.hooks.pre_build.clone());
                }
            }
            "post_build" => {
                hooks.extend(self.global_config.hooks.post_build.clone());
                if let Some(ref local) = self.local_config {
                    hooks.extend(local.hooks.post_build.clone());
                }
            }
            "on_error" => {
                hooks.extend(self.global_config.hooks.on_error.clone());
                if let Some(ref local) = self.local_config {
                    hooks.extend(local.hooks.on_error.clone());
                }
            }
            "on_success" => {
                hooks.extend(self.global_config.hooks.on_success.clone());
                if let Some(ref local) = self.local_config {
                    hooks.extend(local.hooks.on_success.clone());
                }
            }
            _ => {}
        }
        for hook in hooks {
            println!("🎣 Running {} hook: {}", hook_type, hook.dimmed());
            std::process::Command::new("sh").arg("-c").arg(&hook).status()?;
        }
        Ok(())
    }
    pub fn init_local(&mut self) -> Result<()> {
        if self.local_path.exists() {
            println!("⚠️  Local config already exists");
            return Ok(());
        }
        let config = ProjectConfig::default();
        let toml = toml::to_string_pretty(&config)?;
        fs::write(&self.local_path, toml)?;
        self.local_config = Some(config);
        println!("✅ Created local config file: .cg");
        println!("   Edit this file to customize project-specific settings");
        Ok(())
    }
    pub fn show(&self) {
        println!("{}", "=== Configuration ===".blue().bold());
        if let Some(ref local) = self.local_config {
            println!("\n📁 Local Config (.cg):");
            self.display_config(local, "  ");
        }
        println!("\n🌍 Global Config:");
        self.display_config(&self.global_config, "  ");
    }
    fn display_config(&self, config: &ProjectConfig, prefix: &str) {
        println!("{}Project:", prefix);
        if let Some(ref name) = config.project.name {
            println!("{}  name: {}", prefix, name.green());
        }
        if let Some(ref journey) = config.project.default_journey {
            println!("{}  default_journey: {}", prefix, journey.green());
        }
        println!("{}  theme: {}", prefix, config.project.theme.green());
        println!("{}  auto_checklist: {}", prefix, config.project.auto_checklist);
        if !config.shortcuts.is_empty() {
            println!("{}Shortcuts:", prefix);
            for (name, cmd) in &config.shortcuts {
                println!("{}  {} → {}", prefix, name.cyan(), cmd);
            }
        }
        println!("{}Auto Fix:", prefix);
        println!("{}  format_on_save: {}", prefix, config.auto_fix.format_on_save);
        println!("{}  clippy_on_build: {}", prefix, config.auto_fix.clippy_on_build);
        println!("{}Build:", prefix);
        println!("{}  default_profile: {}", prefix, config.build.default_profile);
        println!("{}  incremental: {}", prefix, config.build.incremental);
    }
    pub fn save(&self, local: bool) -> Result<()> {
        if local {
            if let Some(ref config) = self.local_config {
                let toml = toml::to_string_pretty(config)?;
                fs::write(&self.local_path, toml)?;
            }
        } else {
            let toml = toml::to_string_pretty(&self.global_config)?;
            fs::create_dir_all(self.global_path.parent().unwrap())?;
            fs::write(&self.global_path, toml)?;
        }
        Ok(())
    }
    pub fn merge_with_env(&mut self) {
        if let Ok(val) = std::env::var("CM_DEFAULT_PROFILE") {
            self.global_config.build.default_profile = val;
        }
        if let Ok(val) = std::env::var("CM_PARALLEL_JOBS") {
            if let Ok(jobs) = val.parse() {
                self.global_config.build.parallel_jobs = Some(jobs);
            }
        }
        if let Ok(val) = std::env::var("CM_AUTO_FIX") {
            if let Ok(enabled) = val.parse() {
                self.global_config.auto_fix.format_on_save = enabled;
                self.global_config.auto_fix.clippy_on_build = enabled;
            }
        }
    }
    pub fn reset(&mut self) -> Result<()> {
        self.local_config = Some(ProjectConfig::default());
        self.save(true)?;
        println!("✅ Local configuration reset to defaults");
        Ok(())
    }
}
impl Default for ProjectConfig {
    fn default() -> Self {
        Self {
            project: ProjectSettings {
                name: None,
                default_journey: None,
                theme: "nautical".to_string(),
                auto_checklist: true,
                track_performance: true,
            },
            shortcuts: HashMap::new(),
            auto_fix: AutoFixSettings {
                format_on_save: false,
                clippy_on_build: false,
                auto_deps_update: false,
                fix_warnings: false,
                suggest_fixes: true,
            },
            journey: JourneySettings {
                auto_record: false,
                share_templates: false,
                max_recordings: 100,
                interactive_by_default: true,
            },
            build: BuildSettings {
                default_profile: "dev".to_string(),
                parallel_jobs: None,
                target_dir: None,
                incremental: true,
                cache_artifacts: true,
            },
            version_control: VersionControlSettings {
                auto_git_commit: true,
                auto_anchor_git: true,
            },
            hooks: HookSettings {
                pre_build: Vec::new(),
                post_build: Vec::new(),
                on_error: Vec::new(),
                on_success: Vec::new(),
            },
        }
    }
}
pub fn check_captain_authority(command: &str) -> Result<bool> {
    let log = Log::new();
    log.log(
        &format!(
            "Captain's authority check for command '{}' - all hands on deck!", command
        ),
        vec!["captain".to_string(), "authority".to_string()],
    )?;
    let license_manager = license::LicenseManager::new()?;
    match license_manager.enforce_license(command) {
        Ok(_) => Ok(true),
        Err(e) => {
            if e.to_string().contains("limit") {
                log.log(
                    "Command quota exceeded",
                    vec!["captain".to_string(), "authority".to_string()],
                )?;
            } else if e.to_string().contains("License not found") {
                log.log(
                    "Unauthorized vessel! No captain's papers found!",
                    vec!["captain".to_string(), "authority".to_string()],
                )?;
            } else {
                log.log(
                    "Mutiny alert! Authority check failed",
                    vec!["captain".to_string(), "authority".to_string()],
                )?;
            }
            Ok(false)
        }
    }
}