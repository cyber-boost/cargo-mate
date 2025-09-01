use anyhow::{Context, Result};
use clap::Parser;
use colored::*;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use rayon::prelude::*;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use walkdir::WalkDir;
#[cfg(feature = "shine")]
mod shine_features;
#[cfg(feature = "shine")]
mod ai_detector;
#[cfg(feature = "shine")]
mod git_integration;
#[cfg(feature = "shine")]
mod html_reports;
#[derive(Parser, Debug)]
#[command(name = "scrub")]
#[command(about = "System-wide Cargo project cleaner", long_about = None)]
struct Cli {
    #[arg(short = 'd', long, default_value = ".")]
    directory: PathBuf,
    #[arg(short = 'n', long)]
    dry_run: bool,
    #[arg(short, long)]
    verbose: bool,
    #[arg(short = 'r', long)]
    resume_from: Option<String>,
    #[arg(long, default_value = "1")]
    min_depth: usize,
    #[arg(long, default_value = "10")]
    max_depth: usize,
    #[arg(short = 'j', long, default_value = "4")]
    jobs: usize,
    #[arg(long)]
    min_size: Option<u64>,
    #[arg(short = 's', long)]
    sort_by_size: bool,
    #[arg(long)]
    export_json: Option<PathBuf>,
    #[arg(short = 'i', long)]
    interactive: bool,
    #[arg(short = 'e', long)]
    exclude: Vec<String>,
    #[arg(long)]
    stats_only: bool,
    #[arg(long)]
    shine: bool,
    #[arg(long)]
    encrypted_backups: bool,
    #[arg(long)]
    profile: bool,
    #[arg(long)]
    html_report: Option<PathBuf>,
    #[arg(long)]
    git_commit: bool,
    #[arg(long, default_value = "30")]
    max_undo_days: u32,
    #[arg(long)]
    ai_detect: bool,
}
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
    pub shine: bool,
    pub encrypted_backups: bool,
    pub profile: bool,
    pub html_report: Option<PathBuf>,
    pub git_commit: bool,
    pub max_undo_days: u32,
    pub ai_detect: bool,
}
impl From<Cli> for ScrubOptions {
    fn from(cli: Cli) -> Self {
        Self {
            dry_run: cli.dry_run,
            verbose: cli.verbose,
            start_dir: cli.directory,
            resume_from: cli.resume_from,
            min_depth: cli.min_depth,
            max_depth: cli.max_depth,
            jobs: cli.jobs,
            min_size: cli.min_size,
            sort_by_size: cli.sort_by_size,
            export_json: cli.export_json,
            interactive: cli.interactive,
            exclude_patterns: cli.exclude,
            stats_only: cli.stats_only,
            shine: cli.shine,
            encrypted_backups: cli.encrypted_backups,
            profile: cli.profile,
            html_report: cli.html_report,
            git_commit: cli.git_commit,
            max_undo_days: cli.max_undo_days,
            ai_detect: cli.ai_detect,
        }
    }
}
#[derive(Debug, Clone)]
struct ProjectInfo {
    path: PathBuf,
    name: String,
    size_bytes: u64,
    #[allow(dead_code)]
    has_lock_file: bool,
    workspace_members: usize,
    #[allow(dead_code)]
    last_modified: Option<std::time::SystemTime>,
}
#[derive(Default, Debug)]
struct ScrubResults {
    projects_processed: usize,
    projects_cleaned: usize,
    projects_skipped: usize,
    total_savings: u64,
    errors: Vec<String>,
    project_details: Vec<ProjectDetail>,
}
#[derive(Debug, Clone)]
struct ProjectDetail {
    path: PathBuf,
    size_before: u64,
    size_after: u64,
    error: Option<String>,
}
pub struct CargoScrubber {
    options: ScrubOptions,
    multi_progress: MultiProgress,
    stats: Arc<ScanStats>,
}
#[derive(Default)]
struct ScanStats {
    dirs_scanned: AtomicUsize,
    cargo_tomls_found: AtomicUsize,
    projects_with_targets: AtomicUsize,
    total_target_size: AtomicU64,
}
impl CargoScrubber {
    pub fn new(options: ScrubOptions) -> Self {
        Self {
            options,
            multi_progress: MultiProgress::new(),
            stats: Arc::new(ScanStats::default()),
        }
    }
    pub fn scrub(&self) -> Result<()> {
        self.print_header();
        if self.options.dry_run {
            println!(
                "{}", "🔍 DRY RUN MODE - No actual cleaning will be performed".yellow()
                .bold()
            );
        }
        if self.options.stats_only {
            println!(
                "{}", "📊 STATS ONLY MODE - Will analyze but not clean".blue().bold()
            );
        }
        if let Some(ref resume) = self.options.resume_from {
            println!(
                "{} Starting from projects containing: {}", "↪️ RESUME MODE:".cyan(),
                resume.cyan()
            );
        }
        if !self.options.stats_only && !self.is_cargo_available() {
            return Err(
                anyhow::anyhow!(
                    "cargo command not found in PATH. Please ensure Rust and Cargo are properly installed.\n\
                 You can install Rust from: https://rustup.rs/"
                ),
            );
        }
        if self.is_running_as_root() {
            println!(
                "{}",
                "⚠️  WARNING: Running as root - this will clean ALL users' Rust projects"
                .yellow()
            );
            if !self.confirm_action("Continue?") {
                println!("Operation cancelled.");
                return Ok(());
            }
        }
        let projects = self.find_cargo_projects()?;
        if projects.is_empty() {
            println!("No Rust projects found. Exiting.");
            return Ok(());
        }
        self.display_project_stats(&projects);
        if self.options.stats_only {
            return Ok(());
        }
        if !self.options.dry_run && !self.options.interactive {
            if !self.confirm_action(&format!("Clean {} projects?", projects.len())) {
                println!("Operation cancelled.");
                return Ok(());
            }
        }
        let results = self.process_projects(projects)?;
        self.print_summary(&results);
        if let Some(export_path) = &self.options.export_json {
            self.export_results_to_json(&results, export_path)?;
        }
        Ok(())
    }
    fn print_header(&self) {
        println!();
        println!("{}", "🧹 Cargo Scrubber - System-wide Cargo Clean".bold().cyan());
        println!("{}", "═".repeat(60).cyan());
        println!(
            "📁 Start directory: {}", self.options.start_dir.display().to_string()
            .green()
        );
        println!("🔧 Parallel jobs: {}", self.options.jobs.to_string().yellow());
        if !self.options.exclude_patterns.is_empty() {
            println!(
                "🚫 Excluding: {}", self.options.exclude_patterns.join(", ").red()
            );
        }
        println!("{}", "─".repeat(60).dimmed());
    }
    fn is_running_as_root(&self) -> bool {
        std::env::var("USER").map_or(false, |user| user == "root")
            || std::env::var("HOME").map_or(false, |home| home == "/root")
    }
    fn find_cargo_projects(&self) -> Result<Vec<ProjectInfo>> {
        println!("\n{} Scanning for Rust projects...", "🔍".cyan());
        let spinner_style = ProgressStyle::default_spinner()
            .template("{prefix:.bold.dim} {spinner} {wide_msg}")
            .unwrap()
            .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈ ");
        let scan_pb = self.multi_progress.add(ProgressBar::new_spinner());
        scan_pb.set_style(spinner_style.clone());
        scan_pb.set_prefix("Scanning");
        scan_pb.enable_steady_tick(Duration::from_millis(100));
        let stats_pb = self.multi_progress.add(ProgressBar::new_spinner());
        stats_pb.set_style(ProgressStyle::default_spinner().template("{msg}").unwrap());
        stats_pb.enable_steady_tick(Duration::from_millis(250));
        let projects = Arc::new(Mutex::new(Vec::new()));
        let stats = Arc::clone(&self.stats);
        let options = self.options.clone();
        let stats_clone = Arc::clone(&stats);
        let stats_pb_clone = stats_pb.clone();
        let stats_handle = std::thread::spawn(move || {
            loop {
                let dirs = stats_clone.dirs_scanned.load(Ordering::Relaxed);
                let tomls = stats_clone.cargo_tomls_found.load(Ordering::Relaxed);
                let targets = stats_clone.projects_with_targets.load(Ordering::Relaxed);
                let size = stats_clone.total_target_size.load(Ordering::Relaxed);
                stats_pb_clone
                    .set_message(
                        format!(
                            "📊 Dirs: {} | Cargo.tomls: {} | With targets: {} | Size: {}",
                            dirs.to_string().yellow(), tomls.to_string().green(), targets
                            .to_string().cyan(), format_bytes(size).magenta()
                        ),
                    );
                std::thread::sleep(Duration::from_millis(100));
                if dirs == 0 {
                    break;
                }
            }
        });
        let walker = WalkDir::new(&options.start_dir)
            .min_depth(options.min_depth)
            .max_depth(options.max_depth);
        for entry in walker {
            let entry = match entry {
                Ok(e) => e,
                Err(e) => {
                    if options.verbose {
                        eprintln!("Warning: {}", e);
                    }
                    continue;
                }
            };
            stats.dirs_scanned.fetch_add(1, Ordering::Relaxed);
            let path = entry.path();
            scan_pb.set_message(format!("Scanning: {}", path.display()));
            if path.file_name() == Some(std::ffi::OsStr::new("Cargo.toml")) {
                stats.cargo_tomls_found.fetch_add(1, Ordering::Relaxed);
                if let Some(project_dir) = path.parent() {
                    if self.should_exclude(project_dir) {
                        continue;
                    }
                    let target_dir = project_dir.join("target");
                    if target_dir.exists() {
                        let target_size = self.get_dir_size(&target_dir);
                        if let Some(min_size) = options.min_size {
                            if target_size < min_size * 1024 * 1024 {
                                continue;
                            }
                        }
                        stats.projects_with_targets.fetch_add(1, Ordering::Relaxed);
                        stats
                            .total_target_size
                            .fetch_add(target_size, Ordering::Relaxed);
                        let lock_file = project_dir.join("Cargo.lock");
                        let workspace_members = self
                            .count_workspace_members(path)
                            .unwrap_or(1);
                        let last_modified = fs::metadata(&target_dir)
                            .ok()
                            .and_then(|m| m.modified().ok());
                        let project_info = ProjectInfo {
                            path: project_dir.to_path_buf(),
                            name: project_dir
                                .file_name()
                                .unwrap()
                                .to_string_lossy()
                                .into_owned(),
                            size_bytes: target_size,
                            has_lock_file: lock_file.exists(),
                            workspace_members,
                            last_modified,
                        };
                        projects.lock().unwrap().push(project_info);
                    }
                }
            }
        }
        stats.dirs_scanned.store(0, Ordering::Relaxed);
        let _ = stats_handle.join();
        scan_pb.finish_with_message("Scan complete!");
        stats_pb.finish_and_clear();
        let mut projects_vec = match Arc::try_unwrap(projects) {
            Ok(mutex) => mutex.into_inner().unwrap(),
            Err(arc) => arc.lock().unwrap().clone(),
        };
        if self.options.sort_by_size {
            projects_vec.sort_by(|a, b| b.size_bytes.cmp(&a.size_bytes));
        }
        println!(
            "\n✅ Found {} Rust projects with build artifacts", projects_vec.len()
            .to_string().green().bold()
        );
        Ok(projects_vec)
    }
    fn should_exclude(&self, path: &Path) -> bool {
        let default_excluded = [
            "/proc",
            "/sys",
            "/dev",
            "/run",
            "/tmp",
            "/mnt",
            "/media",
            "/.cargo",
            "/Library",
            "/Applications",
            "/.rustup",
        ];
        let path_str = path.to_string_lossy();
        if default_excluded.iter().any(|excl| path_str.contains(excl)) {
            return true;
        }
        self.options
            .exclude_patterns
            .iter()
            .any(|pattern| { path_str.contains(pattern) })
    }
    fn count_workspace_members(&self, cargo_toml_path: &Path) -> Option<usize> {
        let content = fs::read_to_string(cargo_toml_path).ok()?;
        if content.contains("[workspace]") {
            let members = content.matches("members").count();
            Some(members.max(1))
        } else {
            Some(1)
        }
    }
    fn display_project_stats(&self, projects: &[ProjectInfo]) {
        println!("\n{}", "📊 Project Statistics".bold().blue());
        println!("{}", "─".repeat(60).dimmed());
        let total_size: u64 = projects.iter().map(|p| p.size_bytes).sum();
        let avg_size = if !projects.is_empty() {
            total_size / projects.len() as u64
        } else {
            0
        };
        println!("Total projects: {}", projects.len().to_string().green());
        println!("Total target size: {}", format_bytes(total_size).yellow());
        println!("Average target size: {}", format_bytes(avg_size).cyan());
        if self.options.sort_by_size && projects.len() > 0 {
            println!("\n{}", "🏆 Top 5 Largest Projects:".bold());
            for (i, project) in projects.iter().take(5).enumerate() {
                println!(
                    "  {}. {} - {}", i + 1, project.path.display().to_string().blue(),
                    format_bytes(project.size_bytes).yellow()
                );
            }
        }
        let workspace_projects: Vec<_> = projects
            .iter()
            .filter(|p| p.workspace_members > 1)
            .collect();
        if !workspace_projects.is_empty() {
            println!(
                "\n{} Found {} workspace projects", "📦".green(), workspace_projects
                .len()
            );
        }
        println!("{}", "─".repeat(60).dimmed());
    }
    fn process_projects(&self, projects: Vec<ProjectInfo>) -> Result<ScrubResults> {
        let results = ScrubResults::default();
        let filtered_projects = self.filter_projects_for_resume(projects);
        if filtered_projects.is_empty() {
            println!("No projects to process after filtering.");
            return Ok(results);
        }
        let total_size: u64 = filtered_projects.iter().map(|p| p.size_bytes).sum();
        println!(
            "\n{} Processing {} projects (potential savings: {})", "🚀".green(),
            filtered_projects.len().to_string().yellow(), format_bytes(total_size).cyan()
        );
        let progress_style = ProgressStyle::default_bar()
            .template(
                "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta}) {msg}",
            )
            .unwrap()
            .progress_chars("#>-");
        let pb = self
            .multi_progress
            .add(ProgressBar::new(filtered_projects.len() as u64));
        pb.set_style(progress_style);
        let results_mutex = Arc::new(Mutex::new(results));
        let pb_clone = pb.clone();
        let chunk_size = (filtered_projects.len() / self.options.jobs).max(1);
        filtered_projects
            .par_chunks(chunk_size)
            .for_each(|chunk| {
                for project in chunk {
                    let project_name = project.name.clone();
                    pb_clone.set_message(format!("Cleaning: {}", project_name));
                    if self.options.interactive {
                        println!(
                            "\n{} {} ({})?", "Clean".yellow(), project.path.display(),
                            format_bytes(project.size_bytes)
                        );
                        if !self.confirm_action("Clean this project?") {
                            pb_clone.inc(1);
                            let mut results = results_mutex.lock().unwrap();
                            results.projects_skipped += 1;
                            continue;
                        }
                    }
                    let detail = if self.options.dry_run {
                        ProjectDetail {
                            path: project.path.clone(),
                            size_before: project.size_bytes,
                            size_after: 0,
                            error: None,
                        }
                    } else {
                        match self.clean_project(&project.path) {
                            Ok(saved) => {
                                ProjectDetail {
                                    path: project.path.clone(),
                                    size_before: project.size_bytes,
                                    size_after: project.size_bytes.saturating_sub(saved),
                                    error: None,
                                }
                            }
                            Err(e) => {
                                ProjectDetail {
                                    path: project.path.clone(),
                                    size_before: project.size_bytes,
                                    size_after: project.size_bytes,
                                    error: Some(e.to_string()),
                                }
                            }
                        }
                    };
                    let mut results = results_mutex.lock().unwrap();
                    if detail.error.is_none() {
                        results.total_savings += detail.size_before - detail.size_after;
                        if detail.size_before > detail.size_after {
                            results.projects_cleaned += 1;
                        } else {
                            results.projects_skipped += 1;
                        }
                    } else {
                        results
                            .errors
                            .push(
                                format!(
                                    "{}: {}", project.path.display(), detail.error.as_ref()
                                    .unwrap()
                                ),
                            );
                    }
                    results.projects_processed += 1;
                    results.project_details.push(detail);
                    pb_clone.inc(1);
                }
            });
        pb.finish_with_message("Processing complete!");
        Arc::try_unwrap(results_mutex)
            .map_err(|_| anyhow::anyhow!("Failed to unwrap results"))
            .map(|mutex| mutex.into_inner().unwrap())
    }
    fn filter_projects_for_resume(
        &self,
        projects: Vec<ProjectInfo>,
    ) -> Vec<ProjectInfo> {
        if let Some(ref resume_pattern) = self.options.resume_from {
            projects
                .into_iter()
                .filter(|p| p.path.to_string_lossy().contains(resume_pattern))
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
        if self.options.verbose {
            println!("Running: cargo clean in {}", project.display());
        }
        let output = Command::new("cargo")
            .current_dir(project)
            .arg("clean")
            .output()
            .context(format!("Failed to run cargo clean in {}", project.display()))?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!("cargo clean failed: {}", stderr.trim()));
        }
        std::thread::sleep(Duration::from_millis(100));
        let size_after = self.get_dir_size(&target_dir);
        Ok(size_before.saturating_sub(size_after))
    }
    fn get_dir_size(&self, dir: &Path) -> u64 {
        WalkDir::new(dir)
            .into_iter()
            .filter_map(|entry| entry.ok())
            .filter_map(|entry| entry.metadata().ok())
            .filter(|metadata| metadata.is_file())
            .map(|metadata| metadata.len())
            .sum()
    }
    fn is_cargo_available(&self) -> bool {
        Command::new("cargo")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }
    fn confirm_action(&self, prompt: &str) -> bool {
        use std::io::{self, Write};
        print!("{} {} (y/N): ", "?".yellow(), prompt);
        io::stdout().flush().unwrap();
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        input.trim().to_lowercase().starts_with('y')
    }
    fn print_summary(&self, results: &ScrubResults) {
        println!();
        println!("{}", "═".repeat(60).green());
        println!("{}", "✨ CLEANUP SUMMARY".bold().green());
        println!("{}", "─".repeat(60).dimmed());
        println!(
            "📦 Projects processed: {}", results.projects_processed.to_string().cyan()
        );
        println!(
            "✅ Projects cleaned: {}", results.projects_cleaned.to_string().green()
        );
        println!(
            "⏭️  Projects skipped: {}", results.projects_skipped.to_string().yellow()
        );
        println!(
            "💾 Space freed: {}", format_bytes(results.total_savings).bold().green()
        );
        if !results.errors.is_empty() {
            println!("\n{}", "❌ Errors encountered:".red());
            for error in results.errors.iter().take(5) {
                println!("  • {}", error.red());
            }
            if results.errors.len() > 5 {
                println!("  ... and {} more", results.errors.len() - 5);
            }
        }
        if self.options.dry_run {
            println!(
                "\n{}",
                "ℹ️  This was a dry run. Use without --dry-run to actually clean."
                .yellow().italic()
            );
        }
        println!("{}", "═".repeat(60).green());
        println!("{}", "🎉 Cargo scrubber completed successfully!".bold().green());
    }
    fn export_results_to_json(&self, results: &ScrubResults, path: &Path) -> Result<()> {
        use serde_json::json;
        let json_data = json!(
            { "timestamp" : chrono::Utc::now().to_rfc3339(), "options" : { "start_dir" :
            self.options.start_dir.display().to_string(), "dry_run" : self.options
            .dry_run, "jobs" : self.options.jobs, }, "summary" : { "projects_processed" :
            results.projects_processed, "projects_cleaned" : results.projects_cleaned,
            "projects_skipped" : results.projects_skipped, "total_savings_bytes" :
            results.total_savings, "total_savings_human" : format_bytes(results
            .total_savings), }, "projects" : results.project_details.iter().map(| p | {
            json!({ "path" : p.path.display().to_string(), "size_before" : p.size_before,
            "size_after" : p.size_after, "saved" : p.size_before - p.size_after, "error"
            : p.error, }) }).collect::< Vec < _ >> (), "errors" : results.errors, }
        );
        let json_string = serde_json::to_string_pretty(&json_data)?;
        fs::write(path, json_string)?;
        println!("\n📄 Results exported to: {}", path.display().to_string().cyan());
        Ok(())
    }
}
fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    if bytes == 0 {
        return "0 B".to_string();
    }
    let mut size = bytes as f64;
    let mut unit_idx = 0;
    while size >= 1024.0 && unit_idx < UNITS.len() - 1 {
        size /= 1024.0;
        unit_idx += 1;
    }
    if unit_idx == 0 {
        format!("{} B", bytes)
    } else {
        format!("{:.2} {}", size, UNITS[unit_idx])
    }
}
fn main() -> Result<()> {
    let cli = Cli::parse();
    let options = ScrubOptions::from(cli);
    let scrubber = CargoScrubber::new(options);
    scrubber.scrub()
}