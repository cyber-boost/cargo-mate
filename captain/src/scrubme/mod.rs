use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::fs;
use clap::{Parser, Subcommand};
pub mod scrub;
pub use scrub::{ScrubOptions, CargoScrubber};
#[derive(Parser, Debug)]
#[command(name = "scrub")]
#[command(about = "🧹 Scrub - System-wide Cargo project cleaner")]
pub struct ScrubCli {
    #[command(subcommand)]
    command: ScrubCommands,
    #[arg(short, long)]
    verbose: bool,
}
#[derive(Subcommand, Debug)]
enum ScrubCommands {
    Run {
        #[arg(short, long, default_value = ".")]
        start: String,
        #[arg(long)]
        dry_run: bool,
        #[arg(short, long)]
        resume: Option<String>,
        #[arg(long, default_value = "1")]
        min_depth: usize,
        #[arg(long, default_value = "10")]
        max_depth: usize,
    },
    Help,
}
pub fn run_scrub(
    dry_run: bool,
    verbose: bool,
    start: String,
    resume: Option<String>,
    min_depth: usize,
    max_depth: usize,
) -> Result<()> {
    let mut args = vec!["scrub"];
    args.push("-d");
    args.push(&start);
    if dry_run {
        args.push("--dry-run");
    }
    if verbose {
        args.push("-v");
    }
    let resume_str = resume.map(|p| p.to_string());
    if let Some(ref resume_str) = resume_str {
        args.push("-r");
        args.push(resume_str);
    }
    let min_depth_str = min_depth.to_string();
    args.push("--min-depth");
    args.push(&min_depth_str);
    let max_depth_str = max_depth.to_string();
    args.push("--max-depth");
    args.push(&max_depth_str);
    execute_scrub_command(&args)
}
fn ensure_scrub_symlink() -> Result<()> {
    let source_bin_path = get_source_binary_path()?;
    let target_link_path = get_shipwreck_bin_path()?;
    if !is_valid_symlink(&target_link_path, &source_bin_path)? {
        create_symlink(&source_bin_path, &target_link_path)?;
    }
    Ok(())
}
fn get_source_binary_path() -> Result<PathBuf> {
    let module_dir = Path::new(file!()).parent().unwrap_or(Path::new("."));
    let target_release_path = module_dir.join("target/release/scrub");
    if target_release_path.exists() {
        return Ok(target_release_path);
    }
    let current_exe = std::env::current_exe()?;
    let exe_dir = current_exe.parent().context("Failed to get parent directory")?;
    let project_root = exe_dir.parent().context("Failed to get project root")?;
    let scrub_path = project_root.join("src/scrubme/target/release/scrub");
    if scrub_path.exists() {
        return Ok(scrub_path);
    }
    Ok(PathBuf::from("scrub"))
}
fn get_shipwreck_bin_path() -> Result<PathBuf> {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let shipwreck_bin_dir = PathBuf::from(&home).join(".shipwreck").join("bin");
    if !shipwreck_bin_dir.exists() {
        fs::create_dir_all(&shipwreck_bin_dir)?;
    }
    Ok(shipwreck_bin_dir.join("scrub"))
}
fn is_valid_symlink(link_path: &Path, target_path: &Path) -> Result<bool> {
    if !link_path.exists() {
        return Ok(false);
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let metadata = fs::symlink_metadata(link_path)?;
        if !metadata.file_type().is_symlink() {
            return Ok(false);
        }
        let real_path = fs::read_link(link_path)?;
        return Ok(real_path == target_path);
    }
    #[cfg(not(unix))] { Ok(link_path.exists()) }
}
fn create_symlink(source: &Path, target: &Path) -> Result<()> {
    if target.exists() {
        fs::remove_file(target)?;
    }
    #[cfg(unix)] std::os::unix::fs::symlink(source, target)?;
    #[cfg(windows)]
    {
        if fs::metadata(source)?.is_dir() {
            std::os::windows::fs::symlink_dir(source, target)?;
        } else {
            std::os::windows::fs::symlink_file(source, target)?;
        }
    }
    Ok(())
}
fn execute_scrub_command(args: &[&str]) -> Result<()> {
    use std::process::Command;
    ensure_scrub_symlink()?;
    let target_link = get_shipwreck_bin_path()?;
    let scrub_path = if target_link.exists() {
        target_link.to_string_lossy().to_string()
    } else {
        std::env::var("SCRUB_BIN_PATH")
            .unwrap_or_else(|_| {
                get_source_binary_path()
                    .unwrap_or_else(|_| PathBuf::from("scrub"))
                    .to_string_lossy()
                    .to_string()
            })
    };
    let output = Command::new(&scrub_path).args(args).output()?;
    if !output.stdout.is_empty() {
        print!("{}", String::from_utf8_lossy(& output.stdout));
    }
    if !output.stderr.is_empty() {
        eprint!("{}", String::from_utf8_lossy(& output.stderr));
    }
    Ok(())
}