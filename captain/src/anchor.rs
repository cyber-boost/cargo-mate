use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use colored::*;
use notify::{Event, RecursiveMode, Watcher};
use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};
use std::collections::HashMap;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::mpsc::channel;
use crate::captain::license;
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Anchor {
    pub name: String,
    pub timestamp: DateTime<Utc>,
    pub description: String,
    pub git_commit: Option<String>,
    pub cargo_lock_hash: String,
    pub files_snapshot: HashMap<String, FileSnapshot>,
    pub environment: HashMap<String, String>,
    pub metadata: AnchorMetadata,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FileSnapshot {
    pub path: PathBuf,
    pub hash: String,
    pub size: u64,
    pub modified: DateTime<Utc>,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AnchorMetadata {
    pub project_name: String,
    pub rust_version: String,
    pub dependencies_count: usize,
    pub total_loc: usize,
}
pub struct AnchorManager {
    anchors_dir: PathBuf,
    snapshots_dir: PathBuf,
}
impl AnchorManager {
    pub fn new() -> Result<Self> {
        let shipwreck = dirs::home_dir()
            .context("Could not find home directory")?
            .join(".shipwreck");
        let anchors_dir = shipwreck.join("anchors");
        let snapshots_dir = shipwreck.join("snapshots");
        fs::create_dir_all(&anchors_dir)?;
        fs::create_dir_all(&snapshots_dir)?;
        Ok(Self { anchors_dir, snapshots_dir })
    }
    pub fn save(&self, name: &str, description: &str) -> Result<()> {
        println!("⚓ Dropping anchor: {}", name.cyan().bold());
        let git_commit = None;
        let cargo_lock_hash = self.hash_cargo_lock()?;
        let files_snapshot = self.create_files_snapshot()?;
        let environment = self.capture_environment();
        let metadata = self.gather_metadata()?;
        let anchor = Anchor {
            name: name.to_string(),
            timestamp: Utc::now(),
            description: description.to_string(),
            git_commit,
            cargo_lock_hash,
            files_snapshot: files_snapshot.clone(),
            environment,
            metadata,
        };
        self.save_anchor(&anchor)?;
        self.save_file_backups(&anchor)?;
        println!("✅ Anchor '{}' saved successfully!", name.green());
        println!("   📁 {} files backed up", files_snapshot.len());
        Ok(())
    }
    pub fn restore(&self, name: &str) -> Result<()> {
        println!("⚓ Restoring anchor: {}", name.cyan().bold());
        let anchor = self.load_anchor(name)?;
        self.restore_cargo_lock(&anchor)?;
        let restored_count = self.restore_files(&anchor)?;
        println!("✅ Anchor '{}' restored successfully!", name.green());
        println!("   📁 {} files restored", restored_count);
        println!("   🕐 From: {}", anchor.timestamp.format("%Y-%m-%d %H:%M:%S"));
        Ok(())
    }
    pub fn update_file(&self, anchor_name: &str, file_path: &Path) -> Result<()> {
        let mut anchor = self.load_anchor(anchor_name)?;
        if let Some(file_key) = anchor
            .files_snapshot
            .keys()
            .find(|&path| path == &file_path.to_string_lossy())
        {
            let file_snapshot = self.create_file_snapshot(file_path)?;
            anchor.files_snapshot.insert(file_key.clone(), file_snapshot);
            anchor.timestamp = Utc::now();
            self.save_anchor(&anchor)?;
            println!("🔄 Updated {} in anchor '{}'", file_path.display(), anchor_name);
        }
        Ok(())
    }
    pub fn start_auto_update(&self, anchor_name: &str) -> Result<()> {
        self.start_auto_update_with_options(anchor_name, false)
    }
    pub fn start_auto_update_background(&self, anchor_name: &str) -> Result<()> {
        self.start_auto_update_with_options(anchor_name, true)
    }
    pub fn start_auto_update_with_options(
        &self,
        anchor_name: &str,
        background: bool,
    ) -> Result<()> {
        let anchor = self.load_anchor(anchor_name)?;
        if background {
            println!(
                "🚀 {}", format!("Starting auto-update for anchor: {}", anchor_name)
                .cyan().bold()
            );
            println!("📁 Setting up file monitoring...");
            let manager = AnchorManager::new()?;
            let anchor_clone = anchor.clone();
            let anchor_name_clone = anchor_name.to_string();
            std::thread::spawn(move || {
                if let Err(e) = manager
                    .run_auto_update_loop(&anchor_clone, &anchor_name_clone)
                {
                    eprintln!("❌ Auto-update error for {}: {}", anchor_name_clone, e);
                }
            });
            println!("✅ {}", "Auto-update STARTED successfully!".green().bold());
            println!("🔄 Files will be updated automatically when changed");
            println!("🛑 Use 'cargo anchor stop {}' to stop monitoring", anchor_name);
            println!();
            println!(
                "💡 {}", format!("Background daemon running for anchor '{}'",
                anchor_name) .dimmed()
            );
            return Ok(());
        } else {
            println!(
                "📁 Monitoring {} files for changes...", anchor.files_snapshot.len()
            );
            println!("💡 Press Ctrl+C to stop auto-update");
            println!();
        }
        let (tx, rx) = channel();
        let mut watcher = notify::recommended_watcher(tx)?;
        let mut watched_dirs = HashMap::new();
        for file_path in anchor.files_snapshot.keys() {
            let path = Path::new(file_path);
            if let Some(parent) = path.parent() {
                if !watched_dirs.contains_key(parent) {
                    watcher.watch(parent, RecursiveMode::NonRecursive)?;
                    watched_dirs.insert(parent.to_path_buf(), true);
                }
            }
        }
        println!("👀 Watching {} directories", watched_dirs.len());
        println!("✅ Auto-update started! Files will be updated automatically.");
        println!();
        loop {
            match rx.recv() {
                Ok(event) => {
                    match event {
                        Ok(Event { paths, kind: _, attrs: _ }) => {
                            for path in paths {
                                let path_str = path.to_string_lossy();
                                if anchor.files_snapshot.contains_key(&path_str.to_string())
                                {
                                    if let Err(e) = self.update_file(anchor_name, &path) {
                                        eprintln!("❌ Failed to update {}: {}", path.display(), e);
                                    } else {
                                        println!(
                                            "🔄 Updated {} in anchor '{}'", path.display(),
                                            anchor_name
                                        );
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("❌ File watcher error: {}", e);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("❌ Channel receive error: {}", e);
                    break;
                }
            }
        }
        Ok(())
    }
    pub fn run_auto_update_loop(
        &self,
        anchor: &Anchor,
        anchor_name: &str,
    ) -> Result<()> {
        let (tx, rx) = channel();
        let mut watcher = notify::recommended_watcher(tx)?;
        let mut watched_dirs = HashMap::new();
        for file_path in anchor.files_snapshot.keys() {
            let path = Path::new(file_path);
            if let Some(parent) = path.parent() {
                if !watched_dirs.contains_key(parent) {
                    watcher.watch(parent, RecursiveMode::NonRecursive)?;
                    watched_dirs.insert(parent.to_path_buf(), true);
                }
            }
        }
        println!("🔄 Auto-update daemon running for '{}'", anchor_name);
        loop {
            match rx.recv() {
                Ok(event) => {
                    match event {
                        Ok(Event { paths, kind: _, attrs: _ }) => {
                            for path in paths {
                                let path_str = path.to_string_lossy();
                                if anchor.files_snapshot.contains_key(&path_str.to_string())
                                {
                                    if let Err(e) = self.update_file(anchor_name, &path) {
                                        eprintln!("❌ Failed to update {}: {}", path.display(), e);
                                    } else {
                                        println!(
                                            "🔄 [{}] Updated {} in anchor '{}'", chrono::Utc::now()
                                            .format("%H:%M:%S"), path.display(), anchor_name
                                        );
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("❌ File watcher error: {}", e);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("❌ Channel receive error: {}", e);
                    break;
                }
            }
        }
        Ok(())
    }
    pub fn stop_auto_update(&self, anchor_name: &str) -> Result<()> {
        println!("🛑 Stopping auto-update for anchor: {}", anchor_name.cyan().bold());
        println!(
            "⚠️  Note: In this implementation, stopping requires restarting the shell"
        );
        println!("💡 Future versions will have proper daemon management");
        Ok(())
    }
    pub fn list(&self) -> Result<Vec<AnchorSummary>> {
        let mut anchors = Vec::new();
        for entry in fs::read_dir(&self.anchors_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension() == Some(std::ffi::OsStr::new("json")) {
                let content = fs::read_to_string(&path)?;
                let anchor: Anchor = serde_json::from_str(&content)?;
                anchors
                    .push(AnchorSummary {
                        name: anchor.name,
                        timestamp: anchor.timestamp,
                        description: anchor.description,
                        files_count: anchor.files_snapshot.len(),
                    });
            }
        }
        anchors.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        Ok(anchors)
    }
    pub fn show(&self, name: &str) -> Result<()> {
        let anchor = self.load_anchor(name)?;
        println!("{}", format!("=== Anchor: {} ===", anchor.name) .blue().bold());
        println!("📅 Created: {}", anchor.timestamp.format("%Y-%m-%d %H:%M:%S"));
        println!("📝 Description: {}", anchor.description);
        if let Some(ref commit) = anchor.git_commit {
            println!("🔗 Git commit: {}", commit.dimmed());
        }
        println!("\n📊 Metadata:");
        println!("   Project: {}", anchor.metadata.project_name);
        println!("   Rust version: {}", anchor.metadata.rust_version);
        println!("   Dependencies: {}", anchor.metadata.dependencies_count);
        println!("   Lines of code: {}", anchor.metadata.total_loc);
        println!("\n📁 Files snapshot ({} files):", anchor.files_snapshot.len());
        let mut files: Vec<_> = anchor.files_snapshot.values().collect();
        files.sort_by(|a, b| a.path.cmp(&b.path));
        for (i, file) in files.iter().enumerate().take(10) {
            println!("   {} {}", if i < 9 { " " } else { "" }, file.path.display());
        }
        if anchor.files_snapshot.len() > 10 {
            println!("   ... and {} more files", anchor.files_snapshot.len() - 10);
        }
        Ok(())
    }
    pub fn diff(&self, name: &str) -> Result<()> {
        let anchor = self.load_anchor(name)?;
        let current_snapshot = self.create_files_snapshot()?;
        println!("{}", format!("=== Diff from anchor '{}' ===", name) .blue().bold());
        let mut added = Vec::new();
        let mut modified = Vec::new();
        let mut deleted = Vec::new();
        for (path, current_file) in &current_snapshot {
            match anchor.files_snapshot.get(path) {
                Some(anchor_file) => {
                    if anchor_file.hash != current_file.hash {
                        modified.push(path.clone());
                    }
                }
                None => added.push(path.clone()),
            }
        }
        for path in anchor.files_snapshot.keys() {
            if !current_snapshot.contains_key(path) {
                deleted.push(path.clone());
            }
        }
        if !added.is_empty() {
            println!("\n✨ Added files:");
            for path in &added {
                println!("   + {}", path.green());
            }
        }
        if !modified.is_empty() {
            println!("\n📝 Modified files:");
            for path in &modified {
                println!("   ~ {}", path.yellow());
            }
        }
        if !deleted.is_empty() {
            println!("\n🗑️  Deleted files:");
            for path in &deleted {
                println!("   - {}", path.red());
            }
        }
        if added.is_empty() && modified.is_empty() && deleted.is_empty() {
            println!("✅ No changes since anchor '{}'", name);
        }
        Ok(())
    }
    fn save_anchor(&self, anchor: &Anchor) -> Result<()> {
        let anchor_file = self.anchors_dir.join(format!("{}.json", anchor.name));
        let json = serde_json::to_string_pretty(anchor)?;
        fs::write(&anchor_file, json)?;
        Ok(())
    }
    fn load_anchor(&self, name: &str) -> Result<Anchor> {
        let anchor_file = self.anchors_dir.join(format!("{}.json", name));
        if !anchor_file.exists() {
            return Err(anyhow::anyhow!("Anchor '{}' not found", name));
        }
        let content = fs::read_to_string(&anchor_file)?;
        let anchor: Anchor = serde_json::from_str(&content)?;
        Ok(anchor)
    }
    fn checkout_git_commit(&self, commit: &str) -> Result<()> {
        let output = Command::new("git").args(&["checkout", commit]).output()?;
        if !output.status.success() {
            return Err(
                anyhow::anyhow!(
                    "Failed to checkout commit: {}", String::from_utf8_lossy(& output
                    .stderr)
                ),
            );
        }
        Ok(())
    }
    fn hash_cargo_lock(&self) -> Result<String> {
        let cargo_lock = Path::new("Cargo.lock");
        if !cargo_lock.exists() {
            return Ok("no-cargo-lock".to_string());
        }
        let mut file = fs::File::open(cargo_lock)?;
        let mut hasher = Sha256::new();
        let mut buffer = [0; 8192];
        loop {
            let bytes_read = file.read(&mut buffer)?;
            if bytes_read == 0 {
                break;
            }
            hasher.update(&buffer[..bytes_read]);
        }
        Ok(format!("{:x}", hasher.finalize()))
    }
    fn create_files_snapshot(&self) -> Result<HashMap<String, FileSnapshot>> {
        let mut snapshot = HashMap::new();
        let patterns = vec![
            "src/**/*.rs", "tests/**/*.rs", "**/*.toml", "**/*.lock", "**/build.rs",
            "**/*.rs", ".env", "**/.env*",
        ];
        for pattern in patterns {
            for entry in glob::glob(pattern)? {
                if let Ok(path) = entry {
                    if path.is_file() {
                        let metadata = fs::metadata(&path)?;
                        let hash = self.hash_file(&path)?;
                        snapshot
                            .insert(
                                path.to_string_lossy().to_string(),
                                FileSnapshot {
                                    path: path.clone(),
                                    hash,
                                    size: metadata.len(),
                                    modified: DateTime::from(metadata.modified()?),
                                },
                            );
                    }
                }
            }
        }
        Ok(snapshot)
    }
    fn create_file_snapshot(&self, path: &Path) -> Result<FileSnapshot> {
        let metadata = fs::metadata(path)?;
        let hash = self.hash_file(path)?;
        Ok(FileSnapshot {
            path: path.to_path_buf(),
            hash,
            size: metadata.len(),
            modified: DateTime::from(metadata.modified()?),
        })
    }
    fn hash_file(&self, path: &Path) -> Result<String> {
        let mut file = fs::File::open(path)?;
        let mut hasher = Sha256::new();
        let mut buffer = [0; 8192];
        loop {
            let bytes_read = file.read(&mut buffer)?;
            if bytes_read == 0 {
                break;
            }
            hasher.update(&buffer[..bytes_read]);
        }
        Ok(format!("{:x}", hasher.finalize()))
    }
    fn save_file_backups(&self, anchor: &Anchor) -> Result<()> {
        let backup_dir = self.snapshots_dir.join(&anchor.name);
        fs::create_dir_all(&backup_dir)?;
        for (_, file) in &anchor.files_snapshot {
            if file.path.exists() {
                let backup_path = backup_dir
                    .join(file.path.strip_prefix("./").unwrap_or(&file.path));
                if let Some(parent) = backup_path.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::copy(&file.path, &backup_path)?;
            }
        }
        Ok(())
    }
    fn restore_cargo_lock(&self, anchor: &Anchor) -> Result<()> {
        let current_hash = self.hash_cargo_lock()?;
        if current_hash != anchor.cargo_lock_hash
            && anchor.cargo_lock_hash != "no-cargo-lock"
        {
            let backup_dir = self.snapshots_dir.join(&anchor.name);
            let backup_cargo_lock = backup_dir.join("Cargo.lock");
            if backup_cargo_lock.exists() {
                fs::copy(&backup_cargo_lock, "Cargo.lock")?;
                println!("   📦 Cargo.lock restored");
            }
        }
        Ok(())
    }
    fn restore_files(&self, anchor: &Anchor) -> Result<usize> {
        let backup_dir = self.snapshots_dir.join(&anchor.name);
        let mut restored_count = 0;
        for (_, file) in &anchor.files_snapshot {
            let backup_path = backup_dir
                .join(file.path.strip_prefix("./").unwrap_or(&file.path));
            if backup_path.exists() {
                let current_hash = if file.path.exists() {
                    self.hash_file(&file.path).unwrap_or_default()
                } else {
                    String::new()
                };
                if current_hash != file.hash {
                    if let Some(parent) = file.path.parent() {
                        fs::create_dir_all(parent)?;
                    }
                    fs::copy(&backup_path, &file.path)?;
                    restored_count += 1;
                }
            }
        }
        Ok(restored_count)
    }
    fn capture_environment(&self) -> HashMap<String, String> {
        let mut env = HashMap::new();
        for (key, value) in std::env::vars() {
            if key.starts_with("CARGO_") || key.starts_with("RUST_") {
                env.insert(key, value);
            }
        }
        env
    }
    fn gather_metadata(&self) -> Result<AnchorMetadata> {
        let cargo_toml = fs::read_to_string("Cargo.toml")?;
        let manifest: toml::Value = toml::from_str(&cargo_toml)?;
        let project_name = manifest
            .get("package")
            .and_then(|p| p.get("name"))
            .and_then(|n| n.as_str())
            .unwrap_or("unknown")
            .to_string();
        let rust_version = Command::new("rustc")
            .arg("--version")
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .unwrap_or_else(|_| "unknown".to_string());
        let dependencies_count = manifest
            .get("dependencies")
            .and_then(|d| d.as_table())
            .map(|t| t.len())
            .unwrap_or(0);
        let total_loc = self.count_lines_of_code()?;
        Ok(AnchorMetadata {
            project_name,
            rust_version,
            dependencies_count,
            total_loc,
        })
    }
    fn count_lines_of_code(&self) -> Result<usize> {
        let mut total = 0;
        for entry in glob::glob("src/**/*.rs")? {
            if let Ok(path) = entry {
                if path.is_file() {
                    let content = fs::read_to_string(&path)?;
                    total += content.lines().count();
                }
            }
        }
        Ok(total)
    }
}
#[derive(Debug)]
pub struct AnchorSummary {
    pub name: String,
    pub timestamp: DateTime<Utc>,
    pub description: String,
    pub files_count: usize,
}
impl AnchorSummary {
    pub fn display(&self) {
        println!(
            "⚓ {} - {} ({} files)", self.name.cyan().bold(), self.timestamp
            .format("%Y-%m-%d %H:%M:%S").to_string().dimmed(), self.files_count
        );
        println!("   {}", self.description.dimmed());
    }
}
pub fn check_license_ahoy(command: &str) -> Result<bool> {
    println!(
        "🏴‍☠️ Ahoy there, matey! Let me check yer license for '{}'", command
        .cyan()
    );
    let license_manager = license::LicenseManager::new()?;
    match license_manager.enforce_license(command) {
        Ok(_) => {
            println!("✅ Aye aye, Captain! License be valid. Full speed ahead!");
            Ok(true)
        }
        Err(e) => {
            if e.to_string().contains("limit") {
                println!("⚠️  Blimey! Ye've hit the daily limit, scallywag!");
                println!("   💰 Walk the plank to Pro: https://cargo.do/checkout");
            } else if e.to_string().contains("License not found") {
                println!("❌ Arr! No license found! Register with 'cm register <key>'");
            } else {
                println!(
                    "❌ Shiver me timbers! License error: {}", e.to_string().red()
                );
            }
            Ok(false)
        }
    }
}