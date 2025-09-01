use anyhow::Result;
use crate::cmd::smune::MapAction;
use crate::captain::treasure_map::TreasureMap;
use colored::Colorize;
pub fn handle_map_internal(action: MapAction) -> Result<()> {
    let map = TreasureMap::new()?;
    match action {
        MapAction::Show => {
            map.show_map();
        }
        MapAction::Analyze => {
            let analysis = map.analyze();
            analysis.display();
        }
        MapAction::Export { path } => {
            map.export_dot(path.to_string_lossy().as_ref())?;
        }
        MapAction::Path { from, to } => {
            if let Some(path) = map.find_path(&from, &to) {
                println!("📍 Path from {} to {}:", from.cyan(), to.cyan());
                for (i, node) in path.iter().enumerate() {
                    println!("  {}. {}", i + 1, node);
                }
            } else {
                println!("No path found between {} and {}", from, to);
            }
        }
    }
    Ok(())
}