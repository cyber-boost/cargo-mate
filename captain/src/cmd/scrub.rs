use anyhow::Result;
use crate::cmd::smune::ScrubAction;
use crate::scrubme::scrub::{ScrubOptions, CargoScrubber};
pub fn handle_scrub(action: ScrubAction) -> Result<()> {
    match action {
        ScrubAction::Run { dry_run, verbose, start, resume, min_depth, max_depth } => {
            let options = ScrubOptions {
                dry_run,
                verbose,
                start_dir: std::path::PathBuf::from(start),
                resume_from: resume,
                min_depth,
                max_depth,
                jobs: 4,
                min_size: None,
                sort_by_size: false,
                export_json: None,
                interactive: false,
                exclude_patterns: Vec::new(),
                stats_only: false,
                shine: false,
                encrypted_backups: false,
                profile: false,
                html_report: None,
                git_commit: false,
                max_undo_days: 30,
                ai_detect: false,
            };
            let scrubber = CargoScrubber::new(options);
            scrubber.scrub()?;
        }
        ScrubAction::Help => {
            println!("🧹 Cargo Scrub - System-wide Cargo Clean");
            println!();
            println!("USAGE:");
            println!("  cm scrub run [OPTIONS]");
            println!();
            println!("OPTIONS:");
            println!(
                "  --dry-run       Show what would be cleaned without actually doing it"
            );
            println!("  -v, --verbose   Verbose output");
            println!("  -s, --start DIR Start directory (default: /)");
            println!("  -r, --resume    Resume from specific project directory");
            println!("  --min-depth N   Minimum depth to search (default: 1)");
            println!("  --max-depth N   Maximum depth to search (default: 10)");
            println!();
            println!("EXAMPLES:");
            println!(
                "  cm scrub run --dry-run              # See what would be cleaned"
            );
            println!("  cm scrub run -v                      # Verbose output");
            println!("  cm scrub run -s /home                # Only search in /home");
            println!(
                "  cm scrub run -r my-project           # Resume from projects containing 'my-project'"
            );
        }
    }
    Ok(())
}