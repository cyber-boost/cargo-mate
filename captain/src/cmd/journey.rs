use anyhow::Result;
use colored::*;
use crate::journey;
use crate::cmd::smune::JourneyAction;
pub fn handle_journey(action: JourneyAction) -> Result<()> {
    match action {
        JourneyAction::Record { name } => {
            let recorder = journey::JourneyRecorder::new();
            recorder.start_recording(&name)?;
            while recorder.is_recording() {
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
            recorder.stop_recording(&name, "User recorded journey")?;
        }
        JourneyAction::Play { name, dry_run } => {
            let mut player = journey::JourneyPlayer::new(dry_run, true);
            let journey = player.load_journey(&name)?;
            player.play(&journey)?;
        }
        JourneyAction::List => {
            let journeys = journey::list_journeys()?;
            if journeys.is_empty() {
                println!("No journeys found");
            } else {
                println!("📚 Available journeys:");
                for name in journeys {
                    println!("  • {}", name.cyan());
                }
            }
        }
        JourneyAction::Export { name, output } => {
            journey::export_journey(&name, &output)?;
        }
        JourneyAction::Import { path } => {
            journey::import_journey(&path)?;
        }
        JourneyAction::Publish { name, tags } => {
            journey::JourneyMarketplace::publish(&name, tags)?;
        }
        JourneyAction::Download { gist_id } => {
            journey::JourneyMarketplace::download(&gist_id)?;
        }
        JourneyAction::Search { query } => {
            journey::JourneyMarketplace::search(&query)?;
        }
        JourneyAction::Published => {
            let published = journey::JourneyMarketplace::list_published()?;
            if published.is_empty() {
                println!("No published journeys found");
            } else {
                println!("📤 Your published journeys:");
                for journey in published {
                    println!("  • {}", journey.cyan());
                }
            }
        }
    }
    Ok(())
}