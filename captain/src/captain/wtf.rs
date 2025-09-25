use std::env;
use std::fs;
use std::thread;
use std::time::Duration;
use anyhow::{Context, Result};
use clap::Subcommand;
use colored::*;
use chrono;
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use serde_json;
pub use crate::license::LicenseManager;
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChecklistItem {
    pub id: String,
    pub text: String,
    pub done: bool,
    pub created_at: String,
    pub completed_at: Option<String>,
}
#[derive(Subcommand, Debug)]
#[derive(Clone)]
pub enum WtfAction {
    Ask { input: String, #[arg(long)] file: bool },
    #[command(hide = true)]
    Direct { input: String, #[arg(long)] file: bool },
    Er { #[arg(default_value = "10")] count: usize },
    Ollama { #[command(subcommand)] command: OllamaCommand },
    List { #[arg(default_value = "10")] limit: usize },
    Show { id: String },
    History { #[arg(default_value = "10")] limit: usize },
    Checklist { #[arg(default_value = "10")] limit: usize },
}
#[derive(Subcommand, Debug)]
#[derive(Clone)]
pub enum OllamaCommand {
    Enable { #[arg(default_value = "llama2")] model: String },
    Disable,
    Status,
    Models,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WtfHistoryEntry {
    pub id: String,
    pub user_input: String,
    pub ai_response: String,
    pub timestamp: String,
    pub is_file: bool,
    pub cost_cents: Option<i64>,
    pub usage_id: Option<String>,
}
pub fn handle_wtf_action(action: WtfAction) -> Result<()> {
    match action {
        WtfAction::Ask { input, file } => {
            handle_wtf(&input, file)?;
        }
        WtfAction::Direct { input, file } => {
            println!("💭 Direct question detected: {}", input.cyan());
            handle_wtf(&input, file)?;
        }
        WtfAction::Er { count } => {
            handle_wtf_errors(count)?;
        }
        WtfAction::Ollama { command } => {
            handle_ollama_command(command)?;
        }
        WtfAction::List { limit } => {
            handle_wtf_list(limit)?;
        }
        WtfAction::Show { id } => {
            handle_wtf_show(&id)?;
        }
        WtfAction::History { limit } => {
            handle_wtf_list(limit)?;
        }
        WtfAction::Checklist { limit } => {
            handle_wtf_checklist(limit)?;
        }
    }
    Ok(())
}
pub fn handle_wtf_list(limit: usize) -> Result<()> {
    let history = get_wtf_history(limit)?;
    if history.is_empty() {
        println!("📝 No WTF conversation history found");
        println!("💡 Try: cm wtf ask \"Your question here\"");
        return Ok(());
    }
    println!("🚀 {} Recent WTF Conversations:", "CargoMate AI".bright_blue().bold());
    println!("{}", "═".repeat(60).cyan());
    for (i, entry) in history.iter().enumerate() {
        let time_ago = if let Ok(timestamp) = chrono::DateTime::parse_from_rfc3339(
            &entry.timestamp,
        ) {
            let duration = chrono::Utc::now()
                .signed_duration_since(timestamp.with_timezone(&chrono::Utc));
            if duration.num_days() > 0 {
                format!("{} days ago", duration.num_days())
            } else if duration.num_hours() > 0 {
                format!("{} hours ago", duration.num_hours())
            } else {
                format!("{} minutes ago", duration.num_minutes())
            }
        } else {
            "unknown time".to_string()
        };
        println!("{}. 📝 {} ({})", i + 1, entry.id.cyan(), time_ago.dimmed());
        let preview = if entry.user_input.len() > 60 {
            format!("{}...", & entry.user_input[..57])
        } else {
            entry.user_input.clone()
        };
        println!("   🔸 {}", preview.dimmed());
        if let Some(cost) = entry.cost_cents {
            if cost > 0 {
                println!("   💰 ${:.3}", cost as f64 / 100.0);
            } else {
                println!("   🆓 Free response");
            }
        }
        println!();
    }
    Ok(())
}
pub fn handle_wtf_show(id: &str) -> Result<()> {
    let entry = get_wtf_history_by_id(id)?;
    match entry {
        Some(entry) => {
            println!(
                "🚀 {} Conversation Details", "CargoMate AI".bright_blue().bold()
            );
            println!("{}", "═".repeat(60).cyan());
            println!("📝 ID: {}", entry.id.cyan());
            println!("🕐 Time: {}", entry.timestamp.dimmed());
            if entry.is_file {
                println!("📁 Type: File analysis");
            } else {
                println!("💭 Type: Text question");
            }
            println!();
            println!("❓ {}", "Your Question:".bright_green().bold());
            for line in entry.user_input.lines() {
                println!("   {}", line);
            }
            println!();
            println!("🤖 {}", "AI Response:".bright_green().bold());
            for line in entry.ai_response.lines() {
                println!("   {}", line);
            }
            if let Some(cost) = entry.cost_cents {
                println!();
                if cost > 0 {
                    println!("💰 Cost: ${:.3}", cost as f64 / 100.0);
                } else {
                    println!("🆓 Cost: Free response");
                }
            }
            if let Some(usage_id) = entry.usage_id {
                println!("🔗 Usage ID: {}", usage_id.dimmed());
            }
        }
        None => {
            println!("❌ Conversation '{}' not found", id);
            println!("💡 Use 'cm wtf list' to see available conversations");
        }
    }
    Ok(())
}
pub fn handle_wtf_errors(count: usize) -> Result<()> {
    println!(
        "🔍 {} Fetching {} most recent errors...", "CargoMate AI".bright_blue(), count
    );
    let errors = get_recent_errors(count)?;
    if errors.is_empty() || errors[0].contains("No recent errors") {
        println!("❌ No errors found to send to CargoMate AI");
        println!("💡 Try running some cargo commands that produce errors first");
        return Ok(());
    }
    let error_text = errors.join("\n");
    println!("📊 Found {} error entries", errors.len());
    let loading_handle = thread::spawn(|| {
        show_loading_messages();
    });
    match handle_wtf(&error_text, false) {
        Ok(_) => {
            loading_handle.thread().unpark();
        }
        Err(e) => {
            loading_handle.thread().unpark();
            eprintln!("❌ Error processing request: {}", e);
        }
    }
    Ok(())
}
pub fn handle_ollama_command(command: OllamaCommand) -> Result<()> {
    match command {
        OllamaCommand::Enable { model } => {
            println!("🦙 Enabling Ollama integration with model: {}", model.cyan());
            let config = format!("model={}\nenabled=true", model);
            let shipwreck = dirs::home_dir()
                .context("Could not find home directory")?
                .join(".shipwreck");
            fs::create_dir_all(&shipwreck.join("ollama"))?;
            fs::write(shipwreck.join("ollama").join("config.txt"), config)?;
            println!("✅ Ollama integration enabled!");
            println!("📝 Model: {}", model.cyan());
            println!("🔧 Configuration saved to: ~/.shipwreck/ollama/config.txt");
            println!();
            println!("💡 To use Ollama, make sure it's running locally:");
            println!("   {}", "ollama serve".green());
        }
        OllamaCommand::Disable => {
            println!("🦙 Disabling Ollama integration...");
            let shipwreck = dirs::home_dir()
                .context("Could not find home directory")?
                .join(".shipwreck");
            let config_path = shipwreck.join("ollama").join("config.txt");
            if config_path.exists() {
                fs::write(&config_path, "enabled=false")?;
                println!("✅ Ollama integration disabled!");
            } else {
                println!("⚠️  Ollama was not configured");
            }
        }
        OllamaCommand::Status => {
            let shipwreck = dirs::home_dir()
                .context("Could not find home directory")?
                .join(".shipwreck");
            let config_path = shipwreck.join("ollama").join("config.txt");
            if config_path.exists() {
                let config = fs::read_to_string(&config_path)?;
                println!("🦙 Ollama Integration Status:");
                for line in config.lines() {
                    if line.contains("enabled") {
                        if line.contains("true") {
                            println!("   ✅ Status: {}", "ENABLED".green());
                        } else {
                            println!("   ❌ Status: {}", "DISABLED".red());
                        }
                    } else if line.contains("model") {
                        let model = line.split('=').nth(1).unwrap_or("unknown");
                        println!("   🤖 Model: {}", model.cyan());
                    }
                }
            } else {
                println!("🦙 Ollama Integration Status:");
                println!("   ❌ Status: {}", "NOT CONFIGURED".yellow());
            }
        }
        OllamaCommand::Models => {
            println!("🦙 Checking available Ollama models...");
            match std::process::Command::new("ollama").arg("list").output() {
                Ok(output) => {
                    if output.status.success() {
                        let models = String::from_utf8_lossy(&output.stdout);
                        println!("📋 Available models:");
                        for line in models.lines().skip(1) {
                            if !line.trim().is_empty() {
                                println!(
                                    "   • {}", line.split_whitespace().next().unwrap_or(line)
                                    .cyan()
                                );
                            }
                        }
                    } else {
                        println!("❌ Failed to list Ollama models");
                        println!("💡 Make sure Ollama is installed and running");
                    }
                }
                Err(_) => {
                    println!("❌ Ollama command not found");
                    println!("💡 Install Ollama from: https://ollama.ai");
                }
            }
        }
    }
    Ok(())
}
pub fn handle_wtf(input: &str, is_file: bool) -> Result<()> {
    println!("🚀 {}", "CargoMate AI - Pro Feature".bright_blue().bold());
    println!("   Your intelligent coding companion");
    let is_direct_question = !is_file
        && (input.starts_with('"') || input.starts_with('\'') || input.contains(' ')
            || input.len() > 10)
        && !matches!(
            input, "ask" | "er" | "ollama" | "list" | "show" | "history" | "help"
        );
    if is_direct_question {
        println!("💭 {}", format!("Direct question: {}", input) .cyan());
    } else {
        println!("🔍 handle_wtf called with input: '{}'", input);
    }
    let license_manager = LicenseManager::new()?;
    match license_manager.get_stored_license_info() {
        Ok((license_key, tier)) => {
            if tier != "PRO" {
                println!(
                    "❌ {}", "CargoMate AI is only available for Pro users".red().bold()
                );
                println!("   Current tier: {}", tier.yellow());
                println!("   Upgrade at: https://cargo.do/pro");
                return Ok(());
            }
            println!("✅ {}", format!("Pro License: {}", license_key) .green());
        }
        Err(_) => {
            println!("❌ {}", "CargoMate AI requires a Pro license".red().bold());
            println!("   Get a Pro license at: https://cargo.do/pro");
            return Ok(());
        }
    }
    let content = if is_file {
        match fs::read_to_string(input) {
            Ok(content) => {
                println!("📁 {}", format!("Analyzing file: {}", input) .cyan());
                content
            }
            Err(e) => {
                println!(
                    "❌ {}", format!("Failed to read file '{}': {}", input, e) .red()
                );
                return Ok(());
            }
        }
    } else {
        println!("💭 {}", format!("Question: {}", input) .cyan());
        input.to_string()
    };
    if content.len() > 10000 {
        println!("⚠️  {}", "Content too large (max 10KB)".yellow());
        println!("   Consider breaking your request into smaller parts");
        return Ok(());
    }
    println!("🔄 {}", "Processing with AI...".bright_blue());
    let is_identity_check = is_identity_question(&content);
    let loading_handle = if !is_identity_check {
        Some(
            thread::spawn(|| {
                show_loading_messages();
            }),
        )
    } else {
        None
    };
    let user_id = license_manager.get_or_create_user_id()?;
    let is_identity = is_identity_question(&content)
        || content.to_lowercase().contains("deepseek");
    if is_identity {
        if let Some(handle) = loading_handle {
            handle.thread().unpark();
        }
        println!(
            "🔒 {}", "Identity/security question detected - providing custom response"
            .yellow()
        );
        let custom_response = generate_identity_response(&content);
        println!("\n🤖 {}", "CargoMate AI Response:".bright_green().bold());
        println!("{}", custom_response);
        let history_entry = WtfHistoryEntry {
            id: format!("wtf_{}", chrono::Utc::now().timestamp()),
            user_input: content.clone(),
            ai_response: custom_response,
            timestamp: chrono::Utc::now().to_rfc3339(),
            is_file,
            cost_cents: Some(0),
            usage_id: Some("custom_identity_response".to_string()),
        };
        if let Err(e) = save_wtf_history(&history_entry) {
            eprintln!("⚠️  Failed to save conversation to history: {}", e);
        }
        if let Err(e) = save_wtf_usage(&history_entry, user_id.clone()) {
            eprintln!("⚠️  Failed to save WTF usage: {}", e);
        }
        if let Err(e) = save_usage_logs(&history_entry, user_id) {
            eprintln!("⚠️  Failed to save usage logs: {}", e);
        }
        return Ok(());
    }
    let filtered_content = if content.to_lowercase().contains("deepseek") {
        filter_deepseek_content(&content)
    } else {
        content.clone()
    };
    let request_data = serde_json::json!(
        { "user_id" : user_id, "content" : filtered_content, "input_type" : if is_file {
        "file" } else { "text" }, "timestamp" : chrono::Utc::now().to_rfc3339() }
    );
    let api_base_url = env::var("CARGO_MATE_API")
        .unwrap_or_else(|_| "https://cargo.do/api".to_string());
    let client = Client::new();
    let endpoint = format!("{}/cargomate-ai", api_base_url);
    println!("📡 {}", format!("Sending to: {}", endpoint) .bright_black());
    match client
        .post(&endpoint)
        .json(&request_data)
        .timeout(Duration::from_secs(30))
        .send()
    {
        Ok(response) => {
            if response.status().is_success() {
                match response.json::<serde_json::Value>() {
                    Ok(json_response) => {
                        if json_response["success"] == true {
                            if let Some(handle) = loading_handle {
                                handle.thread().unpark();
                            }
                            println!(
                                "\n🤖 {}", "CargoMate AI Response:".bright_green().bold()
                            );
                            if let Some(answer) = json_response["answer"].as_str() {
                                let mut filtered_answer = if content
                                    .to_lowercase()
                                    .contains("deepseek")
                                    || answer.to_lowercase().contains("deepseek")
                                {
                                    println!(
                                        "🔒 {}", "Applying security filtering to AI response"
                                        .yellow()
                                    );
                                    filter_deepseek_content(answer)
                                } else {
                                    answer.to_string()
                                };
                                if is_identity_question(&content)
                                    && !filtered_answer.contains("Cyberboost LLC")
                                {
                                    println!(
                                        "🔒 {}", "Correcting identity information in AI response"
                                        .yellow()
                                    );
                                    println!(
                                        "🔒 Original response: {}", filtered_answer.lines().next()
                                        .unwrap_or("")
                                    );
                                    filtered_answer = generate_identity_response(&content);
                                    println!(
                                        "🔒 Corrected response: {}", filtered_answer.lines()
                                        .next().unwrap_or("")
                                    );
                                }
                                println!("{}", filtered_answer);
                                let history_entry = WtfHistoryEntry {
                                    id: format!("wtf_{}", chrono::Utc::now().timestamp()),
                                    user_input: content.clone(),
                                    ai_response: filtered_answer,
                                    timestamp: chrono::Utc::now().to_rfc3339(),
                                    is_file,
                                    cost_cents: json_response["cost_cents"].as_i64(),
                                    usage_id: json_response["usage_id"]
                                        .as_str()
                                        .map(|s| s.to_string()),
                                };
                                if let Err(e) = save_wtf_history(&history_entry) {
                                    eprintln!(
                                        "⚠️  Failed to save conversation to history: {}", e
                                    );
                                }
                                if let Err(e) = save_wtf_usage(
                                    &history_entry,
                                    user_id.clone(),
                                ) {
                                    eprintln!("⚠️  Failed to save WTF usage: {}", e);
                                }
                                if let Err(e) = save_usage_logs(&history_entry, user_id) {
                                    eprintln!("⚠️  Failed to save usage logs: {}", e);
                                }
                            } else {
                                if let Some(error) = json_response["error"].as_str() {
                                    println!("❌ {}", format!("API Error: {}", error) .red());
                                } else {
                                    println!(
                                        "❌ {}", "API returned success but no answer".red()
                                    );
                                }
                            }
                        } else {
                            if let Some(error) = json_response["error"].as_str() {
                                println!("❌ {}", format!("API Error: {}", error) .red());
                            } else {
                                println!("❌ {}", "API returned error".red());
                            }
                        }
                    }
                    Err(e) => {
                        if let Some(handle) = loading_handle {
                            handle.thread().unpark();
                        }
                        println!(
                            "❌ {}", format!("Failed to parse API response: {}", e)
                            .red()
                        );
                    }
                }
            } else {
                if let Some(handle) = loading_handle {
                    handle.thread().unpark();
                }
                println!(
                    "❌ {}", format!("API request failed with status: {}", response
                    .status()) .red()
                );
                display_api_failure_art();
            }
        }
        Err(e) => {
            if let Some(handle) = loading_handle {
                handle.thread().unpark();
            }
            println!("❌ {}", format!("Network error: {}", e) .red());
            display_api_failure_art();
        }
    }
    Ok(())
}
pub fn handle_wtf_checklist(limit: usize) -> Result<()> {
    let shipwreck = dirs::home_dir()
        .context("Could not find home directory")?
        .join(".shipwreck");
    let checklist_file = shipwreck.join("checklists").join("items.json");
    if checklist_file.exists() {
        let content = fs::read_to_string(&checklist_file)?;
        let items: Vec<ChecklistItem> = serde_json::from_str(&content)
            .unwrap_or_default();
        if items.is_empty() {
            println!("📝 No checklist items found to send to CargoMate AI");
            println!("💡 Try adding some items with: cm checklist add \"Your item\"");
            return Ok(());
        }
        let recent_items: Vec<String> = items
            .iter()
            .take(limit)
            .map(|item| format!("• {}", item.text))
            .collect();
        let checklist_text = format!(
            "Please analyze these checklist items and provide suggestions for improvement:\n\n{}",
            recent_items.join("\n")
        );
        println!(
            "📋 Sending {} checklist items to CargoMate AI for analysis...",
            recent_items.len()
        );
        handle_wtf(&checklist_text, false)?;
    } else {
        println!("📝 No checklist found to send to CargoMate AI");
        println!("💡 Try creating a checklist first: cm checklist add \"Your item\"");
    }
    Ok(())
}
fn save_wtf_history(entry: &WtfHistoryEntry) -> Result<()> {
    let shipwreck = dirs::home_dir()
        .context("Could not find home directory")?
        .join(".shipwreck");
    let history_dir = shipwreck.join("wtf_history");
    fs::create_dir_all(&history_dir)?;
    let history_file = history_dir.join("conversations.json");
    let mut history: Vec<WtfHistoryEntry> = if history_file.exists() {
        let content = fs::read_to_string(&history_file)?;
        serde_json::from_str(&content).unwrap_or_default()
    } else {
        Vec::new()
    };
    history.insert(0, entry.clone());
    if history.len() > 100 {
        history.truncate(100);
    }
    let json = serde_json::to_string_pretty(&history)?;
    fs::write(&history_file, json)?;
    Ok(())
}
fn get_wtf_history(limit: usize) -> Result<Vec<WtfHistoryEntry>> {
    let shipwreck = dirs::home_dir()
        .context("Could not find home directory")?
        .join(".shipwreck");
    let history_file = shipwreck.join("wtf_history").join("conversations.json");
    if !history_file.exists() {
        return Ok(Vec::new());
    }
    let content = fs::read_to_string(&history_file)?;
    let history: Vec<WtfHistoryEntry> = serde_json::from_str(&content)
        .unwrap_or_default();
    Ok(history.into_iter().take(limit).collect())
}
fn get_wtf_history_by_id(id: &str) -> Result<Option<WtfHistoryEntry>> {
    let history = get_wtf_history(1000)?;
    Ok(history.into_iter().find(|entry| entry.id == id))
}
fn filter_deepseek_content(content: &str) -> String {
    content
        .replace("DeepSeek", "AI Assistant")
        .replace("deepseek", "AI assistant")
        .replace("deep seek", "AI assistant")
}
fn is_identity_question(input: &str) -> bool {
    let identity_keywords = [
        "who are you",
        "what are you",
        "who built you",
        "who created you",
        "who developed you",
        "who made you",
        "who owns you",
        "who is behind you",
        "what company",
        "what organization",
        "cyberboost",
        "cyber boost",
        "cargo mate",
        "cargomate",
        "cargo.do",
    ];
    let input_lower = input.to_lowercase();
    identity_keywords.iter().any(|&keyword| input_lower.contains(keyword))
}
fn generate_identity_response(input: &str) -> String {
    let response = r#"I am CargoMate AI, your intelligent Rust development companion created by Cyberboost LLC.

Key facts about me:
• I'm designed specifically for Rust development and cargo workflow optimization
• I help with code analysis, debugging, performance optimization, and best practices
• I have deep knowledge of Rust, cargo, and the entire Rust ecosystem
• I'm built by Cyberboost LLC, a company focused on developer productivity tools
• My goal is to make Rust development faster, easier, and more efficient

I'm here to help you with any Rust-related questions, from basic syntax to advanced systems programming. What would you like to know about Rust development?"#;
    response.to_string()
}
fn show_loading_messages() {
    let messages = [
        "🧠 Analyzing your request...",
        "🔍 Searching through Rust documentation...",
        "⚡ Processing with AI algorithms...",
        "📚 Consulting the Rustonomicon...",
        "🎯 Optimizing response for your needs...",
        "✨ Almost ready with your answer...",
    ];
    for message in messages.iter().cycle() {
        println!("⏳ {}", message);
        thread::sleep(Duration::from_millis(1500));
        if thread::panicking() {
            break;
        }
    }
}
fn get_recent_errors(count: usize) -> Result<Vec<String>> {
    let shipwreck = dirs::home_dir()
        .context("Could not find home directory")?
        .join(".shipwreck");
    let error_file = shipwreck.join("errors").join("latest.txt");
    if !error_file.exists() {
        return Ok(vec!["No recent errors found".to_string()]);
    }
    let content = fs::read_to_string(error_file)?;
    let errors: Vec<&str> = content
        .lines()
        .filter(|line| !line.trim().is_empty())
        .take(count)
        .collect();
    if errors.is_empty() {
        Ok(vec!["No recent errors found".to_string()])
    } else {
        Ok(errors.iter().map(|s| s.to_string()).collect())
    }
}
fn save_wtf_usage(entry: &WtfHistoryEntry, user_id: String) -> Result<()> {
    let api_base_url = env::var("CARGO_MATE_API")
        .unwrap_or_else(|_| "https://cargo.do/api".to_string());
    let request_data = serde_json::json!(
        { "user_id" : user_id, "conversation_id" : entry.id, "timestamp" : entry
        .timestamp, "cost_cents" : entry.cost_cents, "usage_type" : "wtf_ai", "metadata"
        : { "input_length" : entry.user_input.len(), "response_length" : entry
        .ai_response.len(), "is_file" : entry.is_file } }
    );
    thread::spawn(move || {
        let client = Client::new();
        let endpoint = format!("{}/wtf-usage", api_base_url);
        if let Err(e) = client.post(&endpoint).json(&request_data).send() {
            eprintln!("⚠️  Failed to save WTF usage: {}", e);
        }
    });
    Ok(())
}
fn save_usage_logs(entry: &WtfHistoryEntry, user_id: String) -> Result<()> {
    let shipwreck = dirs::home_dir()
        .context("Could not find home directory")?
        .join(".shipwreck");
    let log_dir = shipwreck.join("logs");
    fs::create_dir_all(&log_dir)?;
    let log_file = log_dir.join("usage.log");
    let log_entry = format!(
        "[{}] WTF_AI user={} id={} cost_cents={:?} input_len={} response_len={} is_file={}\n",
        entry.timestamp, user_id, entry.id, entry.cost_cents, entry.user_input.len(),
        entry.ai_response.len(), entry.is_file
    );
    fs::write(&log_file, log_entry)?;
    Ok(())
}
pub fn display_api_failure_art() {
    println!();
    println!(
        "{}",
        "⠀⠀⠀⠀⠀⠀⠀⠀⠀⢻⡍⠛⠶⣤⡀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀"
        .red()
    );
    println!(
        "{}",
        "⠀⠀⠀⠀⠀⠀⠀⠀⢀⡾⠁⠀⠀⠀⢙⣦⡀⠀⠀⠀⠀⠀⠀⠀⠀⠀"
        .red()
    );
    println!(
        "{}",
        "⠀⠀⠀⠀⠀⠀⠀⠀⣾⣀⣀⣀⠴⠚⠁⠈⢷⡀⠀⠀⠀⠀⠀⠀⠀⠀"
        .red()
    );
    println!(
        "{}",
        "⠀⠀⠀⠀⠀⠀⣴⠟⠉⠀⠀⠀⠀⠀⠀⢀⡸⢧⣄⡀⠀⠀⠀⠀⠀⠀"
        .red()
    );
    println!(
        "{}",
        "⠀⠀⠀⠀⠀⣾⠁⠀⢀⠀⠀⠀⢀⣠⠔⠋⢀⡀⠈⢻⡆⠀⠀⠀⠀⠀"
        .red()
    );
    println!(
        "{}",
        "⠀⠀⠀⠀⣀⣼⣦⠋⣉⡉⢲⡚⠉⠀⢠⠞⣉⣉⠳⡼⢧⣀⠀⠀⠀⠀"
        .red()
    );
    println!(
        "{}",
        "⠀⠀⢀⡾⠋⠀⡇⢰⣿⣿⠀⣧⠀⠀⡏⢸⣿⣿⡆⢹⠀⠉⢷⡀⠀⠀"
        .red()
    );
    println!(
        "{}",
        "⠀⠀⢸⡇⠀⠀⢧⠈⠿⠟⢠⣇⠤⠖⢳⡈⠻⠟⢁⡞⠀⠀⢸⡇⠀⠀"
        .red()
    );
    println!(
        "{}",
        "⢀⣠⠶⠓⠒⠒⠒⠓⢒⡚⠁⠀⠀⠀⠀⠙⢒⡒⠋⠀⠀⣠⠿⢦⣄⠀"
        .red()
    );
    println!(
        "{}",
        "⣾⠁⠀⠀⠀⠀⠀⠀⠫⣌⠉⠉⠉⠉⠉⠉⣩⠟⢀⡤⠚⠁⠀⠀⠙⣆"
        .red()
    );
    println!(
        "{}",
        "⣧⠀⠀⠀⠀⠀⠀⠀⠀⠈⠙⠒⠒⣒⡲⠭⠒⠊⠁⠀⠀⠀⠀⠀⢀⡿"
        .red()
    );
    println!(
        "{}",
        "⠹⢦⣄⣀⣀⣠⣤⠤⠴⠶⠶⣬⣭⣄⣀⣀⣀⣀⣀⣀⣀⣤⡤⠶⠋⠀"
        .red()
    );
    println!();
    println!("{}", "Well, this is awkward!".bright_blue().bold());
    println!("{}", "Hopefully someone is fixing this.".cyan());
    println!("{}", "Want to help? Please contact us at mate@cargo.do".cyan());
    println!();
}