use anyhow::{Context, Result};
use chrono::{DateTime, Utc, Duration};
use colored::*;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
#[derive(Debug, Serialize, Deserialize)]
struct AdminMessageResponse {
    success: bool,
    has_message: bool,
    rate_limited: Option<bool>,
    message: Option<AdminMessage>,
}
#[derive(Debug, Serialize, Deserialize)]
struct AdminMessage {
    id: i32,
    title: String,
    content: String,
    #[serde(rename = "type")]
    message_type: String,
    priority: String,
}
#[derive(Debug, Serialize, Deserialize)]
struct MessageCheckRecord {
    last_checks: Vec<DateTime<Utc>>,
    seen_message_ids: Vec<i32>,
}
impl Default for MessageCheckRecord {
    fn default() -> Self {
        Self {
            last_checks: Vec::new(),
            seen_message_ids: Vec::new(),
        }
    }
}
fn get_message_record_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".shipwreck")
        .join("admin_msg_record.json")
}
fn load_check_record() -> Result<MessageCheckRecord> {
    let path = get_message_record_path();
    if !path.exists() {
        return Ok(MessageCheckRecord::default());
    }
    let content = fs::read_to_string(&path)?;
    let record: MessageCheckRecord = serde_json::from_str(&content)?;
    Ok(record)
}
fn save_check_record(record: &MessageCheckRecord) -> Result<()> {
    let path = get_message_record_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(record)?;
    fs::write(&path, json)?;
    Ok(())
}
fn should_check_messages() -> Result<bool> {
    let mut record = load_check_record()?;
    let now = Utc::now();
    let one_day_ago = now - Duration::hours(24);
    record.last_checks.retain(|check| *check > one_day_ago);
    if record.last_checks.len() >= 3 {
        return Ok(false);
    }
    if let Some(last_check) = record.last_checks.last() {
        let four_hours_ago = now - Duration::hours(4);
        if *last_check > four_hours_ago {
            return Ok(false);
        }
    }
    Ok(true)
}
fn record_message_check() -> Result<()> {
    let mut record = load_check_record()?;
    record.last_checks.push(Utc::now());
    save_check_record(&record)?;
    Ok(())
}
fn has_seen_message(message_id: i32) -> Result<bool> {
    let record = load_check_record()?;
    Ok(record.seen_message_ids.contains(&message_id))
}
fn mark_message_seen(message_id: i32) -> Result<()> {
    let mut record = load_check_record()?;
    if !record.seen_message_ids.contains(&message_id) {
        record.seen_message_ids.push(message_id);
        save_check_record(&record)?;
    }
    Ok(())
}
fn get_user_id() -> Result<String> {
    let user_id_path = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".shipwreck")
        .join("user.id");
    if user_id_path.exists() {
        fs::read_to_string(&user_id_path)
            .map(|s| s.trim().to_string())
            .context("Failed to read user ID")
    } else {
        Ok(format!("CM-USER-{}", Utc::now().timestamp()))
    }
}
pub async fn check_and_display_message() -> Result<()> {
    if !should_check_messages()? {
        return Ok(());
    }
    record_message_check()?;
    let user_id = get_user_id()?;
    let version = env!("CARGO_PKG_VERSION");
    let client = reqwest::Client::new();
    let response = client
        .get("https://cargo.do/api/admin/msg")
        .query(&[("user_id", user_id.as_str()), ("version", version)])
        .timeout(std::time::Duration::from_secs(5))
        .send()
        .await;
    match response {
        Ok(resp) if resp.status().is_success() => {
            if let Ok(data) = resp.json::<AdminMessageResponse>().await {
                if data.success && data.has_message {
                    if let Some(message) = data.message {
                        if !has_seen_message(message.id)? {
                            display_admin_message(&message);
                            mark_message_seen(message.id)?;
                            let _ = client
                                .post("https://cargo.do/api/admin/msg")
                                .form(
                                    &[
                                        ("user_id", user_id.as_str()),
                                        ("message_id", &message.id.to_string()),
                                        ("mark_seen", "true"),
                                    ],
                                )
                                .timeout(std::time::Duration::from_secs(2))
                                .send()
                                .await;
                        }
                    }
                }
            }
        }
        _ => {}
    }
    Ok(())
}
fn display_admin_message(message: &AdminMessage) {
    println!();
    let border_color = match message.message_type.as_str() {
        "error" => "red",
        "warning" => "yellow",
        "success" => "green",
        _ => "cyan",
    };
    match message.priority.as_str() {
        "critical" => {
            println!(
                "{}",
                "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
                .red().bold()
            );
            println!("{} {}", "🚨".red(), message.title.red().bold());
            println!(
                "{}",
                "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
                .red().bold()
            );
        }
        "urgent" => {
            println!(
                "{}",
                "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
                .yellow()
            );
            println!("{} {}", "⚠️ ".yellow(), message.title.yellow().bold());
            println!(
                "{}",
                "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
                .yellow()
            );
        }
        _ => {
            let line = "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━";
            match border_color {
                "red" => println!("{}", line.red()),
                "yellow" => println!("{}", line.yellow()),
                "green" => println!("{}", line.green()),
                _ => println!("{}", line.cyan()),
            }
            let icon = match message.message_type.as_str() {
                "error" => "❌",
                "warning" => "⚠️",
                "success" => "✅",
                _ => "ℹ️",
            };
            println!("{} {}", icon, message.title.bold());
            match border_color {
                "red" => println!("{}", line.red()),
                "yellow" => println!("{}", line.yellow()),
                "green" => println!("{}", line.green()),
                _ => println!("{}", line.cyan()),
            }
        }
    }
    println!();
    for line in message.content.lines() {
        println!("  {}", line);
    }
    println!();
    match message.priority.as_str() {
        "critical" => {
            println!(
                "{}",
                "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
                .red().bold()
            )
        }
        "urgent" => {
            println!(
                "{}",
                "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
                .yellow()
            )
        }
        _ => {
            let line = "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━";
            match border_color {
                "red" => println!("{}", line.red()),
                "yellow" => println!("{}", line.yellow()),
                "green" => println!("{}", line.green()),
                _ => println!("{}", line.cyan()),
            }
        }
    }
    println!();
}
pub async fn force_check_message() -> Result<()> {
    let mut record = load_check_record()?;
    record.last_checks.clear();
    save_check_record(&record)?;
    check_and_display_message().await
}