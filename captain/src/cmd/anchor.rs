use anyhow::Result;
use colored::*;
use crate::anchor;
use crate::cmd::smune::AnchorAction;
pub fn handle_anchor(action: AnchorAction) -> Result<()> {
    let manager = anchor::AnchorManager::new()?;
    match action {
        AnchorAction::Save { name, message } => {
            let msg = message.unwrap_or_else(|| "Manual anchor point".to_string());
            manager.save(&name, &msg)?;
        }
        AnchorAction::Restore { name } => {
            manager.restore(&name)?;
        }
        AnchorAction::List => {
            let anchors = manager.list()?;
            if anchors.is_empty() {
                println!("No anchors found");
            } else {
                println!("⚓ Saved anchors:");
                for anchor in anchors {
                    anchor.display();
                }
            }
        }
        AnchorAction::Show { name } => {
            manager.show(&name)?;
        }
        AnchorAction::Diff { name } => {
            manager.diff(&name)?;
        }
        AnchorAction::Auto { name, foreground } => {
            if foreground {
                manager.start_auto_update(&name)?;
            } else {
                manager.start_auto_update_background(&name)?;
            }
        }
        AnchorAction::Stop { name } => {
            manager.stop_auto_update(&name)?;
        }
    }
    Ok(())
}