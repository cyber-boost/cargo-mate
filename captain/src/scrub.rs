use anyhow::{Context, Result};
use colored::*;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
#[derive(Debug, Clone)]
pub struct ScrubOptions {
    pub dry_run: bool,
    pub verbose: bool,
    pub start_dir: PathBuf,
    pub resume_from: Option<String>,
    pub min_depth: usize,
    pub max_depth: usize,
}
impl Default for ScrubOptions {
    fn default() -> Self {
        Self {
            dry_run: false,
            verbose: false,
            start_dir: PathBuf::from("/"),
            resume_from: None,
            min_depth: 1,
            max_depth: 10,
        }
    }
}
pub struct CargoScrubber {
    options: ScrubOptions,
}
impl CargoScrubber {
    pub fn new(options: ScrubOptions) -> Self {
        Self { options }
    }
    pub fn scrub(&self) -> Result<()> {
        self.print_header();
        if self.options.dry_run {
            println!(
                "{}", "DRY RUN MODE - No actual cleaning will be performed".yellow()
                .bold()
            );
        }
        if let Some(ref resume) = self.options.resume_from {
            println!(
                "{} Starting from projects containing: {}", "RESUME MODE:".cyan(), resume
                .cyan()
            );
        }
        if self.is_running_as_root() {
            println!(
                "{}",
                "WARNING: Running as root - this will clean ALL users' Rust projects"
                .yellow()
            );
            println!("Continue? (y/N): ");
            let mut input = String::new();
            std::io::stdin().read_line(&mut input)?;
            if !input.trim().to_lowercase().starts_with('y') {
                println!("Operation cancelled.");
                return Ok(());
            }
        }
        let projects = self.find_cargo_projects()?;
        if projects.is_empty() {
            println!("No Rust projects found. Exiting.");
            return Ok(());
        }
        let total_space = self.calculate_total_space(&projects)?;
        println!("Potential space to free: {}", self.format_bytes(total_space));
        let results = self.process_projects(projects)?;
        self.print_summary(&results);
        Ok(())
    }
    fn print_header(&self) {
        println!("{}", "🧹 System-wide Cargo Clean".bold());
        println!("{}", "=".repeat(50).cyan());
        println!("Start directory: {}", self.options.start_dir.display());
    }
    fn is_running_as_root(&self) -> bool {
        std::env::var("USER").map_or(false, |user| user == "root")
            || std::env::var("HOME").map_or(false, |home| home == "/root")
    }
    fn find_cargo_projects(&self) -> Result<Vec<PathBuf>> {
        println!("Finding Rust projects...");
        let mut projects = Vec::new();
        let output = Command::new("find")
            .arg(&self.options.start_dir)
            .arg("-name")
            .arg("Cargo.toml")
            .arg("-type")
            .arg("f")
            .arg("-print0")
            .output()
            .context("Failed to run find command")?;
        if !output.status.success() {
            return Err(anyhow::anyhow!("Find command failed"));
        }
        let cargo_toml_paths = String::from_utf8_lossy(&output.stdout);
        for path_str in cargo_toml_paths.split('\0') {
            if path_str.is_empty() {
                continue;
            }
            let cargo_toml = PathBuf::from(path_str);
            let project_dir = cargo_toml.parent().unwrap();
            if self.should_exclude(project_dir) {
                continue;
            }
            let target_dir = project_dir.join("target");
            if target_dir.exists() {
                projects.push(project_dir.to_path_buf());
                if self.options.verbose {
                    println!("Found project: {}", project_dir.display());
                }
            }
        }
        println!("Found {} Rust projects with build artifacts", projects.len());
        Ok(projects)
    }
    fn should_exclude(&self, path: &Path) -> bool {
        let excluded = ["/proc", "/sys", "/dev", "/run", "/tmp", "/mnt", "/media"];
        let path_str = path.to_string_lossy();
        excluded.iter().any(|excl| path_str.starts_with(excl))
    }
    fn calculate_total_space(&self, projects: &[PathBuf]) -> Result<u64> {
        println!("Calculating potential space savings...");
        let mut total = 0u64;
        for project in projects {
            let target_size = self.get_dir_size(&project.join("target"));
            total += target_size;
        }
        Ok(total)
    }
    fn get_dir_size(&self, dir: &Path) -> u64 {
        if !dir.exists() {
            return 0;
        }
        let output = Command::new("du").arg("-sb").arg(dir).output();
        match output {
            Ok(out) if out.status.success() => {
                let size_str = String::from_utf8_lossy(&out.stdout);
                size_str
                    .split_whitespace()
                    .next()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0)
            }
            _ => 0,
        }
    }
    fn format_bytes(&self, bytes: u64) -> String {
        const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
        if bytes == 0 {
            return "0B".to_string();
        }
        let mut size = bytes as f64;
        let mut unit_idx = 0;
        while size >= 1024.0 && unit_idx < UNITS.len() - 1 {
            size /= 1024.0;
            unit_idx += 1;
        }
        if unit_idx == 0 {
            format!("{}B", bytes)
        } else {
            format!("{:.1}{}", size, UNITS[unit_idx])
        }
    }
    fn process_projects(&self, projects: Vec<PathBuf>) -> Result<ScrubResults> {
        let mut results = ScrubResults::default();
        let filtered_projects = self.filter_projects_for_resume(projects);
        for (i, project) in filtered_projects.iter().enumerate() {
            println!(
                "Processing [{}/{}]: {}", i + 1, filtered_projects.len(), project
                .display()
            );
            if self.options.dry_run {
                let target_size = self.get_dir_size(&project.join("target"));
                println!(
                    "Would clean: {} in {}/target", self.format_bytes(target_size),
                    project.display()
                );
                results.total_savings += target_size;
                results.projects_processed += 1;
                continue;
            }
            match self.clean_project(project) {
                Ok(saved) => {
                    if saved > 0 {
                        println!(
                            "Cleaned: {} from {}", self.format_bytes(saved), project
                            .display()
                        );
                        results.total_savings += saved;
                        results.projects_cleaned += 1;
                    } else {
                        println!(
                            "No space saved in {} (already clean?)", project.display()
                        );
                        results.projects_skipped += 1;
                    }
                    results.projects_processed += 1;
                }
                Err(e) => {
                    results.errors.push(format!("{}: {}", project.display(), e));
                    println!("Failed to clean {} ({})", project.display(), e);
                }
            }
        }
        Ok(results)
    }
    fn filter_projects_for_resume(&self, projects: Vec<PathBuf>) -> Vec<PathBuf> {
        if let Some(ref resume_pattern) = self.options.resume_from {
            projects
                .into_iter()
                .filter(|p| p.to_string_lossy().contains(resume_pattern))
                .collect()
        } else {
            projects
        }
    }
    fn clean_project(&self, project: &Path) -> Result<u64> {
        let target_dir = project.join("target");
        if !target_dir.exists() {
            return Ok(0);
        }
        let size_before = self.get_dir_size(&target_dir);
        let original_dir = std::env::current_dir()?;
        std::env::set_current_dir(project)?;
        if self.options.verbose {
            println!("Running: cargo clean in {}", project.display());
        }
        if !self.is_cargo_available() {
            return Err(anyhow::anyhow!("cargo command not found"));
        }
        let clean_result = Command::new("timeout")
            .arg("300")
            .arg("cargo")
            .arg("clean")
            .output()
            .context("Failed to run cargo clean with timeout")?;
        if !clean_result.status.success() {
            let stderr = String::from_utf8_lossy(&clean_result.stderr);
            return Err(anyhow::anyhow!("cargo clean failed: {}", stderr));
        }
        std::env::set_current_dir(original_dir)?;
        let size_after = self.get_dir_size(&target_dir);
        let saved = size_before.saturating_sub(size_after);
        Ok(saved)
    }
    fn is_cargo_available(&self) -> bool {
        Command::new("cargo")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }
    fn print_summary(&self, results: &ScrubResults) {
        println!();
        println!("{}", "=== CLEANUP SUMMARY ===".bold());
        println!(
            "Projects processed: {}/{}", results.projects_processed, results
            .projects_processed + results.projects_skipped
        );
        println!("Space freed: {}", self.format_bytes(results.total_savings));
        if !results.errors.is_empty() {
            println!("Errors encountered:");
            for error in &results.errors {
                println!("  {} {}", "✗".red(), error);
            }
        }
        if self.options.dry_run {
            println!(
                "{}", "This was a dry run. Use without --dry-run to actually clean."
                .yellow()
            );
        }
        println!("{}", "System-wide cargo clean completed!".green());
    }
}
#[derive(Default)]
struct ScrubResults {
    projects_processed: usize,
    projects_cleaned: usize,
    projects_skipped: usize,
    total_savings: u64,
    errors: Vec<String>,
}