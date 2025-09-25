use anyhow::{Context, Result};
use colored::*;
use chrono;
use std::env;
use dirs;
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize)]
struct IdeaHistoryEntry {
    id: String,
    idea: String,
    timestamp: String,
}
pub fn handle_idea(idea: &str) -> Result<()> {
    let license_manager = crate::captain::license::LicenseManager::new();
    match license_manager?.enforce_license("idea") {
        Ok(_) => {
            println!("💡 Idea: {}", idea);
            Ok(())
        }
        Err(e) => Err(e),
    }
}
pub fn save_idea_history(idea: &str) -> Result<()> {
    let home_dir = dirs::home_dir()
        .ok_or_else(|| anyhow::anyhow!("Could not find home directory"))?;
    let history_dir = home_dir.join(".shipwreck").join("ideas");
    std::fs::create_dir_all(&history_dir)?;
    let history_file = history_dir.join("ideas.json");
    let mut entries = if history_file.exists() {
        let content = std::fs::read_to_string(&history_file)?;
        serde_json::from_str(&content).unwrap_or_else(|_| Vec::new())
    } else {
        Vec::new()
    };
    let id = uuid::Uuid::new_v4().to_string();
    let timestamp = chrono::Utc::now().to_rfc3339();
    let entry = IdeaHistoryEntry {
        id,
        idea: idea.to_string(),
        timestamp,
    };
    entries.push(entry);
    let json = serde_json::to_string_pretty(&entries)?;
    std::fs::write(history_file, json)?;
    Ok(())
}
pub fn get_idea_history(limit: usize) -> Result<Vec<IdeaHistoryEntry>> {
    let home_dir = dirs::home_dir()
        .ok_or_else(|| anyhow::anyhow!("Could not find home directory"))?;
    let history_file = home_dir.join(".shipwreck").join("ideas").join("ideas.json");
    if !history_file.exists() {
        return Ok(Vec::new());
    }
    let content = std::fs::read_to_string(&history_file)?;
    let mut entries: Vec<IdeaHistoryEntry> = serde_json::from_str(&content)?;
    entries.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
    Ok(entries.into_iter().take(limit).collect())
}