use anyhow::Result;
use std::path::PathBuf;
use crate::cmd::smune::ScrubAction;
#[derive(Debug, Clone)]
pub struct ScrubOptions {
    pub dry_run: bool,
    pub verbose: bool,
    pub start_dir: PathBuf,
    pub resume_from: Option<String>,
    pub min_depth: usize,
    pub max_depth: usize,
    pub jobs: usize,
    pub min_size: Option<u64>,
    pub sort_by_size: bool,
    pub export_json: Option<PathBuf>,
    pub interactive: bool,
    pub exclude_patterns: Vec<String>,
    pub stats_only: bool,
}
pub struct CargoScrubber {
    options: ScrubOptions,
}
impl CargoScrubber {
    pub fn new(options: ScrubOptions) -> Self {
        Self { options }
    }
    pub fn scrub(&self) -> Result<()> {
        println!("🧹 Cargo Scrubber called with options:");
        println!("   - Dry run: {}", self.options.dry_run);
        println!("   - Verbose: {}", self.options.verbose);
        println!("   - Start dir: {}", self.options.start_dir.display());
        println!("   - Min depth: {}", self.options.min_depth);
        println!("   - Max depth: {}", self.options.max_depth);
        println!("   - Jobs: {}", self.options.jobs);
        Ok(())
    }
}