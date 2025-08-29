use anyhow::{Context, Result};
use atty;
use chrono::{DateTime, Utc};
use colored::*;
use handlebars::Handlebars;
use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use serde::{Deserialize, Serialize};
use shell_words;
use std::collections::HashMap;
use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use crate::captain;
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Journey {
    pub name: String,
    pub description: String,
    pub created: DateTime<Utc>,
    pub commands: Vec<JourneyCommand>,
    pub variables: HashMap<String, String>,
    pub checkpoints: Vec<Checkpoint>,
    pub environment: HashMap<String, String>,
    pub success_rate: f32,
    #[serde(default)]
    pub author: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub downloads: u32,
    #[serde(default)]
    pub rating: f32,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct JourneyCommand {
    pub command: String,
    pub args: Vec<String>,
    pub working_dir: PathBuf,
    pub expected_exit_code: i32,
    pub timeout_seconds: u64,
    pub capture_output: bool,
    pub pause_before: bool,
    pub pause_after: bool,
    pub description: Option<String>,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Checkpoint {
    pub name: String,
    pub command_index: usize,
    pub validation: CheckpointValidation,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum CheckpointValidation {
    FileExists(PathBuf),
    FileContains(PathBuf, String),
    CommandSucceeds(String),
    Custom(String),
}
pub struct JourneyRecorder {
    recording: Arc<Mutex<Vec<JourneyCommand>>>,
    is_recording: Arc<AtomicBool>,
    start_time: Instant,
    variables: Arc<Mutex<HashMap<String, String>>>,
}
impl JourneyRecorder {
    pub fn new() -> Self {
        Self {
            recording: Arc::new(Mutex::new(Vec::new())),
            is_recording: Arc::new(AtomicBool::new(false)),
            start_time: Instant::now(),
            variables: Arc::new(Mutex::new(HashMap::new())),
        }
    }
    pub fn is_recording(&self) -> bool {
        self.is_recording.load(Ordering::Relaxed)
    }
    pub fn start_recording(&self, name: &str) -> Result<()> {
        if self.is_recording.load(Ordering::Relaxed) {
            return Err(anyhow::anyhow!("Already recording a journey"));
        }
        self.is_recording.store(true, Ordering::Relaxed);
        println!("🎬 Recording journey: {}", name.cyan().bold());
        println!("⏺️  Press Ctrl+D to stop recording");
        let recording = self.recording.clone();
        let is_recording = self.is_recording.clone();
        thread::spawn(move || {
            Self::record_session(recording, is_recording);
        });
        Ok(())
    }
    fn record_session(
        recording: Arc<Mutex<Vec<JourneyCommand>>>,
        is_recording: Arc<AtomicBool>,
    ) {
        let is_interactive = atty::is(atty::Stream::Stdin)
            && atty::is(atty::Stream::Stdout);
        if !is_interactive {
            println!("⚠️  Journey recording requires an interactive terminal!");
            println!("💡 Please run this command directly in your terminal:");
            println!("   cm journey record <name>");
            println!("✅ Recording stopped - try again in an interactive terminal");
            is_recording.store(false, Ordering::Relaxed);
            return;
        }
        println!("⏹️  Press Ctrl+D to stop recording");
        println!("💡 Or type 'stop'/'exit' and press Enter");
        println!("📝 Type commands and press Enter to record them");
        let mut input = String::new();
        loop {
            input.clear();
            print!("$ ");
            std::io::stdout().flush().unwrap();
            match std::io::stdin().read_line(&mut input) {
                Ok(0) => {
                    println!("✅ Recording stopped by Ctrl+D");
                    break;
                }
                Ok(bytes_read) => {
                    let trimmed = input.trim().to_lowercase();
                    if trimmed == "stop" || trimmed == "exit" {
                        println!("✅ Recording stopped by command: {}", trimmed);
                        break;
                    } else if trimmed.is_empty() {
                        continue;
                    } else {
                        let parts: Vec<String> = shell_words::split(trimmed.as_str())
                            .unwrap_or_else(|_| vec![trimmed.clone()]);
                        if !parts.is_empty() {
                            let cmd = JourneyCommand {
                                command: parts[0].clone(),
                                args: parts[1..].to_vec(),
                                working_dir: std::env::current_dir()
                                    .unwrap_or_else(|_| PathBuf::from(".")),
                                expected_exit_code: 0,
                                timeout_seconds: 300,
                                capture_output: true,
                                pause_before: false,
                                pause_after: false,
                                description: None,
                            };
                            let mut rec = recording.lock().unwrap();
                            rec.push(cmd);
                            let command_count = rec.len();
                            println!(
                                "📝 Recorded: {} (total: {})", trimmed, command_count
                            );
                        }
                    }
                }
                Err(e) => {
                    println!("❌ Recording stopped due to input error: {}", e);
                    break;
                }
            }
        }
        is_recording.store(false, Ordering::Relaxed);
    }
    fn parse_command_from_buffer(buffer: &str) -> Option<JourneyCommand> {
        let lines: Vec<&str> = buffer.lines().collect();
        if lines.is_empty() {
            return None;
        }
        let last_line = lines.last()?;
        if !last_line.contains("$") && !last_line.contains("#") {
            return None;
        }
        let command_start = last_line.rfind('$').or_else(|| last_line.rfind('#'))?;
        let command_str = &last_line[command_start + 1..].trim();
        if command_str.is_empty() {
            return None;
        }
        let parts: Vec<String> = shell_words::split(command_str).ok()?;
        if parts.is_empty() {
            return None;
        }
        Some(JourneyCommand {
            command: parts[0].clone(),
            args: parts[1..].to_vec(),
            working_dir: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            expected_exit_code: 0,
            timeout_seconds: 300,
            capture_output: true,
            pause_before: false,
            pause_after: false,
            description: None,
        })
    }
    pub fn stop_recording(&self, name: &str, description: &str) -> Result<Journey> {
        self.is_recording.store(false, Ordering::Relaxed);
        let commands = self.recording.lock().unwrap().clone();
        let optimized_commands = self.optimize_commands(commands);
        let journey = Journey {
            name: name.to_string(),
            description: description.to_string(),
            created: Utc::now(),
            commands: optimized_commands,
            variables: self.variables.lock().unwrap().clone(),
            checkpoints: self.detect_checkpoints(&self.recording.lock().unwrap()),
            environment: self.capture_environment(),
            success_rate: 100.0,
            author: std::env::var("USER").ok(),
            tags: Vec::new(),
            downloads: 0,
            rating: 0.0,
        };
        self.save_journey(&journey)?;
        println!("✅ Journey '{}' recorded successfully!", name.green().bold());
        println!("📁 Saved to ~/.shipwreck/journeys/{}.json", name);
        Ok(journey)
    }
    fn optimize_commands(&self, commands: Vec<JourneyCommand>) -> Vec<JourneyCommand> {
        let mut optimized = Vec::new();
        let mut last_cmd: Option<JourneyCommand> = None;
        for cmd in commands {
            if let Some(ref last) = last_cmd {
                if last.command == "cargo" && cmd.command == "cargo" {
                    if last.args.get(0) == Some(&"check".to_string())
                        && cmd.args.get(0) == Some(&"check".to_string())
                    {
                        continue;
                    }
                }
                if last.command == "cd" && cmd.command == "cd" {
                    last_cmd = Some(cmd);
                    continue;
                }
            }
            if let Some(last) = last_cmd.take() {
                optimized.push(last);
            }
            last_cmd = Some(cmd);
        }
        if let Some(last) = last_cmd {
            optimized.push(last);
        }
        optimized
    }
    fn detect_checkpoints(&self, commands: &[JourneyCommand]) -> Vec<Checkpoint> {
        let mut checkpoints = Vec::new();
        for (i, cmd) in commands.iter().enumerate() {
            if cmd.command == "cargo" && cmd.args.get(0) == Some(&"build".to_string()) {
                checkpoints
                    .push(Checkpoint {
                        name: "Build Complete".to_string(),
                        command_index: i,
                        validation: CheckpointValidation::CommandSucceeds(
                            "cargo check".to_string(),
                        ),
                    });
            }
            if cmd.command == "cargo" && cmd.args.get(0) == Some(&"test".to_string()) {
                checkpoints
                    .push(Checkpoint {
                        name: "Tests Pass".to_string(),
                        command_index: i,
                        validation: CheckpointValidation::CommandSucceeds(
                            "cargo test --quiet".to_string(),
                        ),
                    });
            }
        }
        checkpoints
    }
    fn capture_environment(&self) -> HashMap<String, String> {
        let mut env = HashMap::new();
        for (key, value) in std::env::vars() {
            if key.starts_with("CARGO_") || key.starts_with("RUST_") {
                env.insert(key, value);
            }
        }
        env
    }
    fn save_journey(&self, journey: &Journey) -> Result<()> {
        let journey_dir = dirs::home_dir()
            .context("Could not find home directory")?
            .join(".shipwreck")
            .join("journeys");
        fs::create_dir_all(&journey_dir)?;
        let journey_file = journey_dir.join(format!("{}.json", journey.name));
        let json = serde_json::to_string_pretty(journey)?;
        fs::write(&journey_file, json)?;
        let template_dir = journey_dir.join("templates");
        fs::create_dir_all(&template_dir)?;
        if journey.success_rate > 95.0 {
            let template_file = template_dir.join(format!("{}.json", journey.name));
            fs::copy(&journey_file, template_file)?;
        }
        Ok(())
    }
}
pub struct JourneyPlayer {
    handlebars: Handlebars<'static>,
    variables: HashMap<String, String>,
    dry_run: bool,
    interactive: bool,
}
impl JourneyPlayer {
    pub fn new(dry_run: bool, interactive: bool) -> Self {
        Self {
            handlebars: Handlebars::new(),
            variables: HashMap::new(),
            dry_run,
            interactive,
        }
    }
    pub fn load_journey(&self, name: &str) -> Result<Journey> {
        let journey_file = dirs::home_dir()
            .context("Could not find home directory")?
            .join(".shipwreck")
            .join("journeys")
            .join(format!("{}.json", name));
        if !journey_file.exists() {
            return Err(anyhow::anyhow!("Journey '{}' not found", name));
        }
        let content = fs::read_to_string(&journey_file)?;
        let journey: Journey = serde_json::from_str(&content)?;
        Ok(journey)
    }
    pub fn play(&mut self, journey: &Journey) -> Result<()> {
        println!("🚢 Playing journey: {}", journey.name.cyan().bold());
        println!("📝 {}", journey.description);
        println!();
        self.collect_variables(&journey.variables)?;
        for (i, cmd) in journey.commands.iter().enumerate() {
            if let Some(checkpoint) = journey
                .checkpoints
                .iter()
                .find(|c| c.command_index == i)
            {
                println!("🏁 Checkpoint: {}", checkpoint.name.yellow());
            }
            if cmd.pause_before && self.interactive {
                println!("⏸️  Press Enter to continue...");
                let mut input = String::new();
                std::io::stdin().read_line(&mut input)?;
            }
            self.execute_command(cmd)?;
            if cmd.pause_after && self.interactive {
                println!("⏸️  Press Enter to continue...");
                let mut input = String::new();
                std::io::stdin().read_line(&mut input)?;
            }
            for checkpoint in &journey.checkpoints {
                if checkpoint.command_index == i {
                    self.validate_checkpoint(checkpoint)?;
                }
            }
        }
        println!("✅ Journey completed successfully!");
        Ok(())
    }
    fn collect_variables(&mut self, defaults: &HashMap<String, String>) -> Result<()> {
        for (key, default_value) in defaults {
            if self.interactive {
                print!(
                    "📝 Enter value for {} [{}]: ", key.cyan(), default_value.dimmed()
                );
                std::io::stdout().flush()?;
                let mut input = String::new();
                std::io::stdin().read_line(&mut input)?;
                let value = input.trim();
                if value.is_empty() {
                    self.variables.insert(key.clone(), default_value.clone());
                } else {
                    self.variables.insert(key.clone(), value.to_string());
                }
            } else {
                self.variables.insert(key.clone(), default_value.clone());
            }
        }
        Ok(())
    }
    fn execute_command(&self, cmd: &JourneyCommand) -> Result<()> {
        let command = self.substitute_variables(&cmd.command)?;
        if command.is_empty()
            || command
                .chars()
                .any(|c| c.is_control() && c != '\n' && c != '\r' && c != '\t')
        {
            println!("⚠️  Skipping invalid command: '{}'", command);
            return Ok(());
        }
        let args: Result<Vec<String>> = cmd
            .args
            .iter()
            .filter(|arg| {
                !arg.is_empty()
                    && !arg
                        .chars()
                        .any(|c| c.is_control() && c != '\n' && c != '\r' && c != '\t')
            })
            .map(|arg| self.substitute_variables(arg))
            .collect();
        let args = args?;
        if let Some(ref desc) = cmd.description {
            println!("📌 {}", desc.dimmed());
        }
        println!("$ {} {}", command.green(), args.join(" ").green());
        if self.dry_run {
            println!("  [DRY RUN - command not executed]");
            return Ok(());
        }
        if command == "cd" {
            if args.is_empty() {
                println!("⚠️  Skipping cd command with no arguments");
                return Ok(());
            }
            let target_dir = &args[0];
            let expanded_path = if target_dir.contains('~') {
                let output = Command::new("sh")
                    .arg("-c")
                    .arg(format!("echo {}", target_dir))
                    .output()
                    .map_err(|e| anyhow::anyhow!("Failed to expand path: {}", e))?;
                String::from_utf8_lossy(&output.stdout).trim().to_string()
            } else {
                target_dir.clone()
            };
            if std::path::Path::new(&expanded_path).exists() {
                println!("📁 Changing working directory to: {}", expanded_path.cyan());
                return Ok(());
            } else {
                println!("❌ Directory does not exist: {}", expanded_path);
                return Ok(());
            }
        }
        let needs_shell = command.contains('~')
            || args.iter().any(|arg| arg.contains('~')) || command.contains('$')
            || args.iter().any(|arg| arg.contains('$'));
        let status = if needs_shell {
            let full_command = if args.is_empty() {
                command.clone()
            } else {
                format!("{} {}", command, args.join(" "))
            };
            let mut process = Command::new("sh")
                .arg("-c")
                .arg(&full_command)
                .current_dir(&cmd.working_dir)
                .stdout(
                    if cmd.capture_output { Stdio::piped() } else { Stdio::inherit() },
                )
                .stderr(
                    if cmd.capture_output { Stdio::piped() } else { Stdio::inherit() },
                )
                .spawn()?;
            process.wait()?
        } else {
            let mut process = Command::new(&command)
                .args(&args)
                .current_dir(&cmd.working_dir)
                .stdout(
                    if cmd.capture_output { Stdio::piped() } else { Stdio::inherit() },
                )
                .stderr(
                    if cmd.capture_output { Stdio::piped() } else { Stdio::inherit() },
                )
                .spawn()?;
            process.wait()?
        };
        if !status.success() && cmd.expected_exit_code == 0 {
            return Err(
                anyhow::anyhow!(
                    "Command failed with exit code: {}", status.code().unwrap_or(- 1)
                ),
            );
        }
        Ok(())
    }
    fn substitute_variables(&self, template: &str) -> Result<String> {
        self.handlebars
            .render_template(template, &self.variables)
            .context("Failed to substitute variables")
    }
    fn validate_checkpoint(&self, checkpoint: &Checkpoint) -> Result<()> {
        match &checkpoint.validation {
            CheckpointValidation::FileExists(path) => {
                if !path.exists() {
                    return Err(
                        anyhow::anyhow!(
                            "Checkpoint failed: file {} does not exist", path.display()
                        ),
                    );
                }
            }
            CheckpointValidation::FileContains(path, content) => {
                let file_content = fs::read_to_string(path)?;
                if !file_content.contains(content) {
                    return Err(
                        anyhow::anyhow!(
                            "Checkpoint failed: file {} does not contain '{}'", path
                            .display(), content
                        ),
                    );
                }
            }
            CheckpointValidation::CommandSucceeds(cmd) => {
                let status = Command::new("sh").arg("-c").arg(cmd).status()?;
                if !status.success() {
                    return Err(
                        anyhow::anyhow!("Checkpoint failed: command '{}' failed", cmd),
                    );
                }
            }
            CheckpointValidation::Custom(script) => {
                let status = Command::new("sh").arg("-c").arg(script).status()?;
                if !status.success() {
                    return Err(
                        anyhow::anyhow!("Checkpoint failed: custom validation failed"),
                    );
                }
            }
        }
        println!("✅ Checkpoint passed: {}", checkpoint.name.green());
        Ok(())
    }
}
pub fn list_journeys() -> Result<Vec<String>> {
    let journey_dir = dirs::home_dir()
        .context("Could not find home directory")?
        .join(".shipwreck")
        .join("journeys");
    if !journey_dir.exists() {
        return Ok(Vec::new());
    }
    let mut journeys = Vec::new();
    for entry in fs::read_dir(&journey_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension() == Some(std::ffi::OsStr::new("json")) {
            if let Some(stem) = path.file_stem() {
                journeys.push(stem.to_string_lossy().to_string());
            }
        }
    }
    Ok(journeys)
}
pub fn export_journey(name: &str, output: &Path) -> Result<()> {
    let journey_file = dirs::home_dir()
        .context("Could not find home directory")?
        .join(".shipwreck")
        .join("journeys")
        .join(format!("{}.json", name));
    if !journey_file.exists() {
        return Err(anyhow::anyhow!("Journey '{}' not found", name));
    }
    fs::copy(&journey_file, output)?;
    println!("✅ Journey exported to {}", output.display());
    Ok(())
}
pub fn import_journey(path: &Path) -> Result<()> {
    let content = fs::read_to_string(path)?;
    let journey: Journey = serde_json::from_str(&content)?;
    let journey_dir = dirs::home_dir()
        .context("Could not find home directory")?
        .join(".shipwreck")
        .join("journeys");
    fs::create_dir_all(&journey_dir)?;
    let journey_file = journey_dir.join(format!("{}.json", journey.name));
    fs::write(&journey_file, content)?;
    println!("✅ Journey '{}' imported successfully!", journey.name.green());
    Ok(())
}
#[derive(Debug, Serialize, Deserialize)]
pub struct MarketplaceJourney {
    pub gist_id: String,
    pub name: String,
    pub description: String,
    pub author: String,
    pub tags: Vec<String>,
    pub downloads: u32,
    pub rating: f32,
    pub created: DateTime<Utc>,
    pub updated: DateTime<Utc>,
}
pub struct JourneyMarketplace;
impl JourneyMarketplace {
    pub fn publish(name: &str, tags: Vec<String>) -> Result<String> {
        let journey_file = dirs::home_dir()
            .context("Could not find home directory")?
            .join(".shipwreck")
            .join("journeys")
            .join(format!("{}.json", name));
        if !journey_file.exists() {
            return Err(anyhow::anyhow!("Journey '{}' not found", name));
        }
        let content = fs::read_to_string(&journey_file)?;
        let mut journey: Journey = serde_json::from_str(&content)?;
        journey.tags = tags;
        let json = serde_json::to_string_pretty(&journey)?;
        println!("📤 Publishing journey '{}' to GitHub Gist...", name.cyan());
        let gist_json = serde_json::json!(
            { "description" : format!("Cargo Mate Journey: {} - {}", journey.name,
            journey.description), "public" : true, "files" : { format!("{}.json", name) :
            { "content" : json } } }
        );
        let temp_file = std::env::temp_dir()
            .join(format!("cargo-mate-gist-{}.json", name));
        fs::write(&temp_file, gist_json.to_string())?;
        let output = Command::new("gh")
            .args(&["api", "gists"])
            .arg("--method")
            .arg("POST")
            .arg("--input")
            .arg(&temp_file)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()?;
        let _ = fs::remove_file(&temp_file);
        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!("GitHub CLI error: {}", error));
        }
        let result = String::from_utf8_lossy(&output.stdout);
        if result.trim().is_empty() {
            return Err(anyhow::anyhow!("GitHub CLI returned empty response"));
        }
        let gist_response: serde_json::Value = serde_json::from_str(&result)
            .map_err(|e| anyhow::anyhow!("Failed to parse GitHub response: {}", e))?;
        let gist_id = gist_response["id"]
            .as_str()
            .ok_or_else(|| {
                anyhow::anyhow!("Failed to get gist ID from response: {}", result)
            })?;
        let html_url = gist_response["html_url"]
            .as_str()
            .ok_or_else(|| {
                anyhow::anyhow!("Failed to get gist URL from response: {}", result)
            })?;
        println!("✅ Journey published successfully!");
        println!("🔗 Gist URL: {}", html_url.cyan());
        println!("📋 Share ID: {}", gist_id.green());
        Self::save_published_record(name, gist_id)?;
        Ok(gist_id.to_string())
    }
    pub fn download(gist_id: &str) -> Result<()> {
        println!("📥 Downloading journey from gist {}...", gist_id.cyan());
        let output = Command::new("gh")
            .args(&["api", &format!("gists/{}", gist_id)])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()?;
        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!("Failed to download gist: {}", error));
        }
        let gist_response: serde_json::Value = serde_json::from_slice(&output.stdout)?;
        let files = gist_response["files"]
            .as_object()
            .ok_or_else(|| anyhow::anyhow!("No files found in gist"))?;
        for (filename, file_data) in files {
            if filename.ends_with(".json") {
                let content = file_data["content"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("Failed to get file content"))?;
                let journey: Journey = serde_json::from_str(content)?;
                let journey_dir = dirs::home_dir()
                    .context("Could not find home directory")?
                    .join(".shipwreck")
                    .join("journeys");
                fs::create_dir_all(&journey_dir)?;
                let journey_file = journey_dir.join(format!("{}.json", journey.name));
                fs::write(&journey_file, content)?;
                println!(
                    "✅ Journey '{}' downloaded successfully!", journey.name.green()
                );
                println!("📝 Description: {}", journey.description);
                if let Some(author) = &journey.author {
                    println!("👤 Author: {}", author.cyan());
                }
                if !journey.tags.is_empty() {
                    println!("🏷️  Tags: {}", journey.tags.join(", "));
                }
                return Ok(());
            }
        }
        Err(anyhow::anyhow!("No valid journey file found in gist"))
    }
    pub fn search(query: &str) -> Result<Vec<MarketplaceJourney>> {
        println!("🔍 Searching for journeys matching '{}'...", query.cyan());
        let output = Command::new("gh")
            .args(&["api", "search/gists"])
            .arg("-X")
            .arg("GET")
            .arg("-f")
            .arg(format!("q=Cargo Mate Journey {}", query))
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()?;
        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!("Search failed: {}", error));
        }
        let search_response: serde_json::Value = serde_json::from_slice(&output.stdout)?;
        let items = search_response["items"]
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("No search results found"))?;
        let mut journeys = Vec::new();
        for item in items {
            if let (Some(id), Some(description)) = (
                item["id"].as_str(),
                item["description"].as_str(),
            ) {
                if description.starts_with("Cargo Mate Journey:") {
                    let parts: Vec<&str> = description.splitn(3, " - ").collect();
                    if parts.len() >= 2 {
                        let name = parts[0].replace("Cargo Mate Journey: ", "");
                        let desc = parts.get(1).unwrap_or(&"").to_string();
                        journeys
                            .push(MarketplaceJourney {
                                gist_id: id.to_string(),
                                name,
                                description: desc,
                                author: item["owner"]["login"]
                                    .as_str()
                                    .unwrap_or("unknown")
                                    .to_string(),
                                tags: Vec::new(),
                                downloads: 0,
                                rating: 0.0,
                                created: DateTime::parse_from_rfc3339(
                                        item["created_at"].as_str().unwrap_or(""),
                                    )
                                    .ok()
                                    .map(|dt| dt.with_timezone(&Utc))
                                    .unwrap_or_else(Utc::now),
                                updated: DateTime::parse_from_rfc3339(
                                        item["updated_at"].as_str().unwrap_or(""),
                                    )
                                    .ok()
                                    .map(|dt| dt.with_timezone(&Utc))
                                    .unwrap_or_else(Utc::now),
                            });
                    }
                }
            }
        }
        if journeys.is_empty() {
            println!("No journeys found matching your search.");
        } else {
            println!("Found {} journey(s):", journeys.len());
            for (i, journey) in journeys.iter().enumerate() {
                println!(
                    "\n{}. {} by {}", i + 1, journey.name.cyan(), journey.author.green()
                );
                println!("   {}", journey.description);
                println!("   ID: {}", journey.gist_id.dimmed());
            }
        }
        Ok(journeys)
    }
    pub fn list_published() -> Result<Vec<String>> {
        let published_file = dirs::home_dir()
            .context("Could not find home directory")?
            .join(".shipwreck")
            .join("journeys")
            .join(".published.json");
        if !published_file.exists() {
            return Ok(Vec::new());
        }
        let content = fs::read_to_string(&published_file)?;
        let published: HashMap<String, String> = serde_json::from_str(&content)?;
        let mut journeys = Vec::new();
        for (name, gist_id) in published {
            journeys.push(format!("{} ({})", name, gist_id));
        }
        Ok(journeys)
    }
    fn save_published_record(name: &str, gist_id: &str) -> Result<()> {
        let published_file = dirs::home_dir()
            .context("Could not find home directory")?
            .join(".shipwreck")
            .join("journeys")
            .join(".published.json");
        let mut published: HashMap<String, String> = if published_file.exists() {
            let content = fs::read_to_string(&published_file)?;
            serde_json::from_str(&content)?
        } else {
            HashMap::new()
        };
        published.insert(name.to_string(), gist_id.to_string());
        let json = serde_json::to_string_pretty(&published)?;
        fs::write(&published_file, json)?;
        Ok(())
    }
}
pub fn check_buoy_clearance(command: &str) -> Result<bool> {
    println!(
        "🛟 Buoy check! Verifying command '{}' through the navigation channel", command
        .cyan()
    );
    let license_manager = captain::license::LicenseManager::new()?;
    match license_manager.enforce_license(command) {
        Ok(_) => {
            println!(
                "✅ Clear sailing! Command '{}' passed all navigation buoys!", command
                .green()
            );
            println!("   🛟 All channel markers are green - proceed!");
            Ok(true)
        }
        Err(e) => {
            if e.to_string().contains("limit") {
                println!("⚠️  Red buoy alert! Usage quota exceeded!");
                println!("   🚧 Safe harbor: https://cargo.do/checkout");
                println!("   🛟 Drop anchor and upgrade to continue");
            } else if e.to_string().contains("License not found") {
                println!("❌ No navigation beacon detected!");
                println!("   📡 Register clearance with 'cm register <key>'");
            } else {
                println!(
                    "❌ Stormy waters! Navigation check failed: {}", e.to_string().red()
                );
                println!("   🛟 Seek safe harbor and contact support");
            }
            Ok(false)
        }
    }
}