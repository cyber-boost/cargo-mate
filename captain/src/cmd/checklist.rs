use anyhow::{Result, Context};
use colored::Colorize;
use crate::cmd::smune::ChecklistAction;
use serde::{Deserialize, Serialize};
pub fn handle_checklist_internal(action: ChecklistAction) -> Result<()> {
    let shipwreck = dirs::home_dir()
        .context("Could not find home directory")?
        .join(".shipwreck");
    let checklist_dir = shipwreck.join("checklists");
    std::fs::create_dir_all(&checklist_dir)?;
    let checklist_file = checklist_dir.join("items.json");
    let mut items: Vec<ChecklistItem> = if checklist_file.exists() {
        let content = std::fs::read_to_string(&checklist_file)?;
        if content.trim().is_empty() {
            Vec::new()
        } else {
            serde_json::from_str(&content).unwrap_or_else(|_| Vec::new())
        }
    } else {
        Vec::new()
    };
    match action {
        ChecklistAction::Show | ChecklistAction::List => {
            if items.is_empty() {
                println!("📋 Checklist is empty");
                println!("💡 Add items with: cm checklist add \"Your task here\"");
            } else {
                println!("📋 Current Checklist:");
                println!("{}", "═".repeat(60).cyan());
                for item in &items {
                    let status = if item.done { "✅" } else { "❌" };
                    let checkbox = if item.done { "☑️" } else { "☐" };
                    println!("{}. {} {} {}", item.id, checkbox, item.text, status);
                }
                println!();
                let done_count = items.iter().filter(|i| i.done).count();
                println!(
                    "📊 Progress: {}/{} items completed", done_count, items.len()
                );
            }
        }
        ChecklistAction::Add { item } => {
            let next_id = items.iter().map(|i| i.id).max().unwrap_or(0) + 1;
            let new_item = ChecklistItem {
                id: next_id,
                text: item.clone(),
                done: false,
                created_at: chrono::Utc::now().to_rfc3339(),
            };
            items.push(new_item);
            let content = serde_json::to_string_pretty(&items)?;
            std::fs::write(&checklist_file, content)?;
            println!("✅ Added item #{}: {}", next_id, item);
            println!("💡 Mark as done with: cm checklist done {}", next_id);
        }
        ChecklistAction::Done { items: item_ids } => {
            let ids_to_mark: Vec<usize> = item_ids
                .split(',')
                .filter_map(|s| s.trim().parse::<usize>().ok())
                .collect();
            let mut marked_count = 0;
            for item in &mut items {
                if ids_to_mark.contains(&item.id) && !item.done {
                    item.done = true;
                    marked_count += 1;
                }
            }
            if marked_count > 0 {
                let content = serde_json::to_string_pretty(&items)?;
                std::fs::write(&checklist_file, content)?;
                println!(
                    "✅ Marked {} item(s) as completed: {}", marked_count, item_ids
                );
            } else {
                println!(
                    "❌ No items were marked as done. Items may already be completed or not exist."
                );
            }
        }
        ChecklistAction::Clear { target } => {
            match target.as_str() {
                "all" => {
                    items.clear();
                    let content = serde_json::to_string_pretty(&items)?;
                    std::fs::write(&checklist_file, content)?;
                    println!("🗑️  Cleared all checklist items");
                }
                "done" => {
                    items.retain(|item| !item.done);
                    let content = serde_json::to_string_pretty(&items)?;
                    std::fs::write(&checklist_file, content)?;
                    println!("🗑️  Removed completed items from checklist");
                }
                _ => {
                    println!(
                        "❌ Invalid target. Use 'all' to clear everything or 'done' to remove completed items"
                    );
                }
            }
        }
    }
    Ok(())
}
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ChecklistItem {
    id: usize,
    text: String,
    done: bool,
    created_at: String,
}