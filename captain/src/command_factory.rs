use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use clap::Parser;
use colored::*;
use cargo_mate::captain::version::IncrementPolicy;
use cargo_mate::cmd::checklist;
use cargo_mate::history;
use cargo_mate::cmd::init::init_cargo_mate;
use cargo_mate::cmd::activate::handle_activate;
use cargo_mate::cmd::register::handle_register;
use cargo_mate::cmd::help::show_help;
use cargo_mate::anchor;
use cargo_mate::captain::version::VersionManager;
use cargo_mate::checklist::show_checklist;
use cargo_mate::history::show_history;
#[derive(Debug, Clone, Default)]
pub struct CommandArgs {
    pub action: String,
    pub target: Option<String>,
    pub options: HashMap<String, String>,
    pub flags: Vec<String>,
    pub trailing_args: Vec<String>,
}
pub trait Command: Send + Sync {
    fn execute(&self, args: CommandArgs) -> Result<()>;
    fn name(&self) -> &'static str;
    /// Get the command description
    fn description(&self) -> &'static str;
    fn subcommands(&self) -> Vec<&'static str> {
        vec![]
    }
    /// Validate arguments before execution
    fn validate_args(&self, args: &CommandArgs) -> Result<()> {
        Ok(())
    }
}
/// Command factory that manages all available commands
pub struct CommandFactory {
    commands: HashMap<String, Arc<dyn Command>>,
    aliases: HashMap<String, String>,
}
impl CommandFactory {
    /// Create a new command factory and register all commands
    pub fn new() -> Self {
        let mut factory = Self {
            commands: HashMap::new(),
            aliases: HashMap::new(),
        };
        factory.register_all_commands();
        factory.add_alias("j", "journey");
        factory.add_alias("a", "anchor");
        factory.add_alias("l", "log");
        factory.add_alias("v", "version");
        factory.add_alias("c", "config");
        factory.add_alias("o", "optimize");
        factory
    }
    /// Register all available commands
    fn register_all_commands(&mut self) {}
    pub fn register(&mut self, command: Arc<dyn Command>) {
        self.commands.insert(command.name().to_string(), command);
    }
    pub fn add_alias(&mut self, alias: &str, command: &str) {
        self.aliases.insert(alias.to_string(), command.to_string());
    }
    pub fn execute(&self, name: &str, args: CommandArgs) -> Result<()> {
        let actual_name = self.aliases.get(name).map(|s| s.as_str()).unwrap_or(name);
        match self.commands.get(actual_name) {
            Some(command) => {
                command.validate_args(&args)?;
                self.execute_with_tracking(command.clone(), args)
            }
            None => {
                let suggestions = self.find_similar_commands(name);
                if !suggestions.is_empty() {
                    let suggestion_text = suggestions.join(", ");
                    Err(
                        anyhow::anyhow!(
                            "Unknown command: '{}'. Did you mean: {}?", name,
                            suggestion_text
                        ),
                    )
                } else {
                    Err(anyhow::anyhow!("Unknown command: '{}'", name))
                }
            }
        }
    }
    pub fn execute_function(
        &self,
        name: &str,
        action: &str,
        target: Option<&str>,
        remaining_args: &[String],
    ) -> Result<()> {
        match name {
            "strip" => {
                let strip_args = crate::strip::StripArgs::parse_from(&*remaining_args);
                crate::strip::handle_strip_command(strip_args)?;
                return Ok(());
            }
            "scat" => {
                use crate::scat::ScatCommand;
                let command = ScatCommand::Protect {
                    input: std::path::PathBuf::from("input_binary"),
                    output: std::path::PathBuf::from("output_binary"),
                    level: "standard".to_string(),
                };
                crate::scat::handle_scat_command(command)?;
                return Ok(());
            }
            "anchor" => {
                if remaining_args.is_empty() {
                    eprintln!(
                        "⚠️  No anchor action specified. Use 'cargo anchor --help' for usage."
                    );
                    std::process::exit(1);
                }
                match remaining_args[0].as_str() {
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
                            .unwrap_or_else(|| {
                                format!("Auto-saved via cargo anchor save")
                            });
                        manager.save(&name, &description)?;
                        Ok(())
                    }
                    "restore" => {
                        if remaining_args.len() < 2 {
                            eprintln!(
                                "⚠️  Anchor name required. Usage: cargo anchor restore <name>"
                            );
                            std::process::exit(1);
                        }
                        let manager = anchor::AnchorManager::new()?;
                        manager.restore(&remaining_args[1])?;
                        Ok(())
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
                        Ok(())
                    }
                    "show" => {
                        if remaining_args.len() < 2 {
                            eprintln!(
                                "⚠️  Anchor name required. Usage: cargo anchor show <name>"
                            );
                            std::process::exit(1);
                        }
                        let manager = anchor::AnchorManager::new()?;
                        manager.show(&remaining_args[1])?;
                        Ok(())
                    }
                    "diff" => {
                        if remaining_args.len() < 2 {
                            eprintln!(
                                "⚠️  Anchor name required. Usage: cargo anchor diff <name>"
                            );
                            std::process::exit(1);
                        }
                        let manager = anchor::AnchorManager::new()?;
                        manager.diff(&remaining_args[1])?;
                        Ok(())
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
                            manager.start_auto_update(&remaining_args[1])?;
                        } else {
                            manager.start_auto_update_background(&remaining_args[1])?;
                        }
                        Ok(())
                    }
                    "stop" => {
                        if remaining_args.len() < 2 {
                            eprintln!(
                                "⚠️  Anchor name required. Usage: cargo anchor stop <name>"
                            );
                            std::process::exit(1);
                        }
                        let manager = anchor::AnchorManager::new()?;
                        manager.stop_auto_update(&remaining_args[1])?;
                        Ok(())
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
                        Ok(())
                    }
                    Some("info") => {
                        version_manager.show_info();
                        Ok(())
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
                                version_manager.config.increment_policy = IncrementPolicy::Minor;
                                let result = version_manager.increment()?;
                                version_manager.config.increment_policy = original_policy;
                                result
                            }
                            "major" => {
                                let original_policy = version_manager
                                    .config
                                    .increment_policy
                                    .clone();
                                version_manager.config.increment_policy = IncrementPolicy::Major;
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
                        Ok(())
                    }
                    Some("set") => {
                        if remaining_args.len() < 2 {
                            eprintln!(
                                "⚠️  Version required. Usage: cargo version set <version>"
                            );
                            std::process::exit(1);
                        }
                        version_manager.set_version(&remaining_args[1])?;
                        Ok(())
                    }
                    Some("update-cargo") => {
                        version_manager.update_cargo_toml()?;
                        Ok(())
                    }
                    Some("config") => {
                        match remaining_args.get(1).map(|s| s.as_ref()) {
                            Some("enable") => {
                                version_manager.config.auto_increment = true;
                                version_manager.save_config()?;
                                println!("✅ Auto-increment enabled");
                                Ok(())
                            }
                            Some("disable") => {
                                version_manager.config.auto_increment = false;
                                version_manager.save_config()?;
                                println!("✅ Auto-increment disabled");
                                Ok(())
                            }
                            Some("policy") => {
                                if remaining_args.len() < 3 {
                                    eprintln!(
                                        "⚠️  Policy required. Usage: cargo version config policy <patch|minor|major>"
                                    );
                                    std::process::exit(1);
                                }
                                let policy = &remaining_args[2];
                                version_manager.config.increment_policy = match policy
                                    .as_str()
                                {
                                    "patch" => IncrementPolicy::Patch,
                                    "minor" => IncrementPolicy::Minor,
                                    "major" => IncrementPolicy::Major,
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
                                Ok(())
                            }
                            Some("show") => {
                                version_manager.show_info();
                                Ok(())
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
                show_checklist();
                Ok(())
            }
            "history" => {
                let limit = remaining_args
                    .get(0)
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(10);
                show_history(&vec!["all".to_string(), limit.to_string()]);
                Ok(())
            }
            "init" => {
                init_cargo_mate()?;
                Ok(())
            }
            "install" => {
                cargo_mate::captain::shell_integration::ShellIntegration::install()?;
                Ok(())
            }
            "activate" => {
                handle_activate()?;
                Ok(())
            }
            "register" => {
                let license_key = remaining_args.get(0).map(|s| s.to_string());
                let status = remaining_args.iter().any(|arg| arg == "--status");
                handle_register(license_key, status, false)?;
                Ok(())
            }
            "help" | "--help" | "-h" => {
                show_help()?;
                Ok(())
            }
            _ => {
                eprintln!("⚠️  Unknown command: {}", name);
                std::process::exit(1);
            }
        }
    }
    fn execute_with_tracking(
        &self,
        command: Arc<dyn Command>,
        args: CommandArgs,
    ) -> Result<()> {
        use colored::*;
        println!(
            "{} {} {}", "🚢".bold(), "Executing:".cyan(), command.name().yellow()
            .bold()
        );
        let result = command.execute(args);
        match &result {
            Ok(_) => {
                println!(
                    "{} {} {}", "✅".bold(), command.name().green(),
                    "completed successfully".green()
                );
            }
            Err(e) => {
                println!(
                    "{} {} failed: {}", "❌".bold(), command.name().red(), e.to_string()
                    .red()
                );
            }
        }
        result
    }
    fn find_similar_commands(&self, name: &str) -> Vec<String> {
        let mut suggestions = Vec::new();
        for cmd_name in self.commands.keys() {
            if self.is_similar(name, cmd_name) {
                suggestions.push(cmd_name.clone());
            }
        }
        for (alias, cmd) in &self.aliases {
            if self.is_similar(name, alias) {
                suggestions.push(format!("{} (alias for {})", alias, cmd));
            }
        }
        suggestions
    }
    fn is_similar(&self, a: &str, b: &str) -> bool {
        if a.starts_with(b) || b.starts_with(a) {
            return true;
        }
        let distance = self.levenshtein_distance(a, b);
        distance <= 2 && distance as f32 / a.len().max(b.len()) as f32 <= 0.4
    }
    fn levenshtein_distance(&self, a: &str, b: &str) -> usize {
        let a_chars: Vec<char> = a.chars().collect();
        let b_chars: Vec<char> = b.chars().collect();
        let a_len = a_chars.len();
        let b_len = b_chars.len();
        if a_len == 0 {
            return b_len;
        }
        if b_len == 0 {
            return a_len;
        }
        let mut matrix = vec![vec![0; b_len + 1]; a_len + 1];
        for i in 0..=a_len {
            matrix[i][0] = i;
        }
        for j in 0..=b_len {
            matrix[0][j] = j;
        }
        for i in 1..=a_len {
            for j in 1..=b_len {
                let cost = if a_chars[i - 1] == b_chars[j - 1] { 0 } else { 1 };
                matrix[i][j] = std::cmp::min(
                    std::cmp::min(matrix[i - 1][j] + 1, matrix[i][j - 1] + 1),
                    matrix[i - 1][j - 1] + cost,
                );
            }
        }
        matrix[a_len][b_len]
    }
    pub fn list_commands(&self) -> Vec<CommandInfo> {
        let mut commands: Vec<CommandInfo> = self
            .commands
            .values()
            .map(|cmd| CommandInfo {
                name: cmd.name().to_string(),
                description: cmd.description().to_string(),
                subcommands: cmd.subcommands().iter().map(|s| s.to_string()).collect(),
                aliases: self.get_aliases_for_command(cmd.name()),
            })
            .collect();
        commands.sort_by(|a, b| a.name.cmp(&b.name));
        commands
    }
    fn get_aliases_for_command(&self, command: &str) -> Vec<String> {
        self.aliases
            .iter()
            .filter(|(_, cmd)| *cmd == command)
            .map(|(alias, _)| alias.clone())
            .collect()
    }
}
#[derive(Debug, Clone)]
pub struct CommandInfo {
    pub name: String,
    pub description: String,
    pub subcommands: Vec<String>,
    pub aliases: Vec<String>,
}
impl std::fmt::Display for CommandInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:<15} - {}", self.name, self.description)?;
        if !self.aliases.is_empty() {
            write!(f, " (aliases: {})", self.aliases.join(", "))?;
        }
        if !self.subcommands.is_empty() {
            write!(f, "\n{:17} Subcommands: {}", "", self.subcommands.join(", "))?;
        }
        Ok(())
    }
}