use anyhow::{Context, Result};
use colored::*;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use chrono::{DateTime, Local, TimeZone};

#[derive(Debug, Clone)]
struct TreeOptions {
    no_folders: bool,
    no_files: bool,
    folder_size: bool,
    file_size: bool,
    line_count: bool,
    dates: bool,
    style: TreeStyle,
    yolo: bool,
}

#[derive(Debug, Clone, Copy)]
enum TreeStyle {
    Basic,
    Readme,
    Cm,
    Hard,
    Easy,
}

impl TreeStyle {
    fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "basic" => TreeStyle::Basic,
            "readme" => TreeStyle::Readme,
            "cm" => TreeStyle::Cm,
            "hard" => TreeStyle::Hard,
            "easy" => TreeStyle::Easy,
            _ => TreeStyle::Readme,
        }
    }
}

#[derive(Debug)]
struct FileInfo {
    path: PathBuf,
    size: u64,
    lines: Option<usize>,
    modified: Option<DateTime<Local>>,
}

#[derive(Debug)]
struct DirInfo {
    path: PathBuf,
    size: u64,
    file_count: usize,
    modified: Option<DateTime<Local>>,
}

fn format_size(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit_index = 0;
    
    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }
    
    if unit_index == 0 {
        format!("{} {}", bytes, UNITS[unit_index])
    } else {
        format!("{:.2} {}", size, UNITS[unit_index])
    }
}

fn count_lines(file_path: &Path) -> Option<usize> {
    match fs::read_to_string(file_path) {
        Ok(content) => Some(content.lines().count()),
        Err(_) => None,
    }
}

fn get_file_metadata(path: &Path) -> Result<FileInfo> {
    let metadata = fs::metadata(path)?;
    let size = metadata.len();
    let modified = metadata.modified()
        .ok()
        .and_then(|t| {
            t.duration_since(std::time::UNIX_EPOCH)
                .ok()
                .map(|d| Local.timestamp_opt(d.as_secs() as i64, 0).single())
                .flatten()
        });
    
    Ok(FileInfo {
        path: path.to_path_buf(),
        size,
        lines: None,
        modified,
    })
}

fn build_tree_structure(
    target_dir: &Path,
    options: &TreeOptions,
) -> Result<(BTreeMap<PathBuf, DirInfo>, BTreeMap<PathBuf, FileInfo>)> {
    let mut dirs: BTreeMap<PathBuf, DirInfo> = BTreeMap::new();
    let mut files: BTreeMap<PathBuf, FileInfo> = BTreeMap::new();

    for entry in WalkDir::new(target_dir)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        let relative_path = path.strip_prefix(target_dir).unwrap_or(path);

        if path.is_dir() {
            if !options.no_folders {
                let metadata = fs::metadata(path)?;
                let modified = metadata.modified()
                    .ok()
                    .and_then(|t| {
                        t.duration_since(std::time::UNIX_EPOCH)
                            .ok()
                            .map(|d| Local.timestamp_opt(d.as_secs() as i64, 0).single())
                            .flatten()
                    });
                
                let mut dir_size = 0u64;
                let mut file_count = 0usize;

                if options.folder_size {
                    for file_entry in WalkDir::new(path).into_iter().filter_map(|e| e.ok()) {
                        if file_entry.path().is_file() {
                            if let Ok(file_meta) = fs::metadata(file_entry.path()) {
                                dir_size += file_meta.len();
                                file_count += 1;
                            }
                        }
                    }
                }

                dirs.insert(
                    relative_path.to_path_buf(),
                    DirInfo {
                        path: relative_path.to_path_buf(),
                        size: dir_size,
                        file_count,
                        modified,
                    },
                );
            }
        } else if path.is_file() {
            if !options.no_files {
                let mut file_info = get_file_metadata(path)?;
                
                if options.line_count {
                    file_info.lines = count_lines(path);
                }

                files.insert(relative_path.to_path_buf(), file_info);
            }
        }
    }

    Ok((dirs, files))
}

fn generate_tree_markdown(
    target_dir: &Path,
    dirs: &BTreeMap<PathBuf, DirInfo>,
    files: &BTreeMap<PathBuf, FileInfo>,
    options: &TreeOptions,
) -> String {
    let mut content = String::new();

    // Header based on style
    match options.style {
        TreeStyle::Basic => {
            content.push_str("# Directory Tree\n\n");
        }
        TreeStyle::Readme => {
            content.push_str("# 📁 Project Structure\n\n");
            content.push_str("This document shows the directory structure of the project.\n\n");
        }
        TreeStyle::Cm => {
            content.push_str("# 🚢 Cargo Mate Project Tree\n\n");
            content.push_str("Generated by Cargo Mate `tree` command.\n\n");
        }
        TreeStyle::Hard => {
            content.push_str("# ⚡ HARD MODE: Project Structure\n\n");
            content.push_str("**Warning**: This tree contains detailed information. Handle with care.\n\n");
        }
        TreeStyle::Easy => {
            content.push_str("# 🌟 Easy Mode: Project Structure\n\n");
            content.push_str("A simple, easy-to-read directory tree.\n\n");
        }
    }

    if options.yolo {
        content.push_str("> **YOLO MODE ACTIVATED** 🎉\n\n");
    }

    // Generate tree structure
    let root_name = target_dir.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(".")
        .to_string();

    content.push_str("```\n");
    content.push_str(&root_name);
    content.push_str("\n");

    // Build sorted tree structure - simpler approach
    let mut all_paths: Vec<&PathBuf> = dirs.keys().chain(files.keys()).collect();
    all_paths.sort();

    // Simple tree generation
    for path in all_paths {
        let components: Vec<&str> = path.components()
            .filter_map(|c| {
                if let std::path::Component::Normal(name) = c {
                    name.to_str()
                } else {
                    None
                }
            })
            .collect();

        if components.is_empty() {
            continue;
        }

        let depth = components.len();
        let is_dir = dirs.contains_key(path);
        let is_file = files.contains_key(path);

        // Build tree line
        for (i, component) in components.iter().enumerate() {
            let is_last_component = i == components.len() - 1;
            
            // Determine if this is the last sibling
            let mut is_last_sibling = true;
            if !is_last_component {
                // Check if there are more items at this level
                let current_path: PathBuf = components[..=i].iter().collect();
                for other_path in all_paths.iter() {
                    if other_path != &path {
                        let other_components: Vec<&str> = other_path.components()
                            .filter_map(|c| {
                                if let std::path::Component::Normal(name) = c {
                                    name.to_str()
                                } else {
                                    None
                                }
                            })
                            .collect();
                        if other_components.len() > i + 1 {
                            let other_path_at_level: PathBuf = other_components[..=i].iter().collect();
                            if other_path_at_level == current_path {
                                // Same parent, check if there's a later sibling
                                if other_components[i + 1] > component {
                                    is_last_sibling = false;
                                    break;
                                }
                            }
                        }
                    }
                }
            } else {
                // For the last component, check if there are more siblings at this level
                let parent: PathBuf = components[..i].iter().collect();
                for other_path in all_paths.iter() {
                    if other_path != &path {
                        let other_components: Vec<&str> = other_path.components()
                            .filter_map(|c| {
                                if let std::path::Component::Normal(name) = c {
                                    name.to_str()
                                } else {
                                    None
                                }
                            })
                            .collect();
                        if other_components.len() == depth && other_components[..i] == components[..i] {
                            if other_components[i] > component {
                                is_last_sibling = false;
                                break;
                            }
                        }
                    }
                }
            }

            // Generate indent and prefix
            let indent: String = (0..i).map(|_| if i > 0 && is_last_sibling && i == components.len() - 1 { "    " } else { "│   " }).collect();
            let prefix = if is_last_sibling { "└── " } else { "├── " };

            // Only print the last component (the actual file/dir name)
            if is_last_component {
                if is_dir {
                    content.push_str(&format!("{}{}{}/\n", indent, prefix, component));
                } else if is_file {
                    content.push_str(&format!("{}{}{}\n", indent, prefix, component));
                }

                // Add metadata
                let metadata_line = generate_metadata_line(path, dirs, files, options);
                if !metadata_line.is_empty() {
                    let meta_indent: String = (0..depth).map(|_| "    ").collect();
                    content.push_str(&format!("{}{}\n", meta_indent, metadata_line));
                }
            }
        }
    }

    content.push_str("```\n\n");

    // Add summary
    if !options.no_folders || !options.no_files {
        content.push_str("## Summary\n\n");
        if !options.no_folders {
            content.push_str(&format!("- **Directories**: {}\n", dirs.len()));
        }
        if !options.no_files {
            content.push_str(&format!("- **Files**: {}\n", files.len()));
        }
        if options.folder_size || options.file_size {
            let total_size: u64 = dirs.values().map(|d| d.size).sum::<u64>()
                + files.values().map(|f| f.size).sum::<u64>();
            content.push_str(&format!("- **Total Size**: {}\n", format_size(total_size)));
        }
        if options.line_count {
            let total_lines: usize = files.values()
                .filter_map(|f| f.lines)
                .sum();
            content.push_str(&format!("- **Total Lines**: {}\n", total_lines));
        }
        content.push_str("\n");
    }

    // Add footer
    let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S");
    content.push_str(&format!("---\n*Generated on {} by Cargo Mate*\n", timestamp));

    content
}

fn generate_metadata_line(
    path: &PathBuf,
    dirs: &BTreeMap<PathBuf, DirInfo>,
    files: &BTreeMap<PathBuf, FileInfo>,
    options: &TreeOptions,
) -> String {
    let mut parts = Vec::new();

    if let Some(dir_info) = dirs.get(path) {
        if options.folder_size {
            parts.push(format!("[{} files, {}]", dir_info.file_count, format_size(dir_info.size)));
        }
        if options.dates {
            if let Some(modified) = dir_info.modified {
                parts.push(format!("modified: {}", modified.format("%Y-%m-%d")));
            }
        }
    } else if let Some(file_info) = files.get(path) {
        if options.file_size {
            parts.push(format_size(file_info.size));
        }
        if options.line_count {
            if let Some(lines) = file_info.lines {
                parts.push(format!("{} lines", lines));
            }
        }
        if options.dates {
            if let Some(modified) = file_info.modified {
                parts.push(format!("modified: {}", modified.format("%Y-%m-%d")));
            }
        }
    }

    if parts.is_empty() {
        String::new()
    } else {
        format!("  // {}", parts.join(", "))
    }
}

fn save_tree_history(content: &str, timestamp: &str) -> Result<PathBuf> {
    let home = dirs::home_dir()
        .context("Could not find home directory")?;
    let trees_dir = home.join(".shipwreck").join("trees");
    fs::create_dir_all(&trees_dir)?;

    let filename = format!("cm-tree-{}.md", timestamp);
    let history_path = trees_dir.join(&filename);
    fs::write(&history_path, content)?;

    Ok(history_path)
}

fn list_tree_history() -> Result<Vec<PathBuf>> {
    let home = dirs::home_dir()
        .context("Could not find home directory")?;
    let trees_dir = home.join(".shipwreck").join("trees");
    
    if !trees_dir.exists() {
        return Ok(Vec::new());
    }

    let mut trees: Vec<PathBuf> = fs::read_dir(&trees_dir)?
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path().is_file()
                && e.path().extension().map(|ext| ext == "md").unwrap_or(false)
                && e.path().file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n.starts_with("cm-tree-"))
                    .unwrap_or(false)
        })
        .map(|e| e.path())
        .collect();

    trees.sort();
    trees.reverse(); // Most recent first
    Ok(trees)
}

pub fn handle_tree(
    action: Option<crate::cmd::smune::TreeAction>,
    target: PathBuf,
    out: Option<PathBuf>,
    no_folders: bool,
    no_files: bool,
    folder_size: bool,
    file_size: bool,
    line_count: bool,
    dates: bool,
    style: String,
    yolo: bool,
) -> Result<()> {
    // Handle subcommands
    if let Some(action) = action {
        match action {
            crate::cmd::smune::TreeAction::History => {
                return handle_tree_history();
            }
            crate::cmd::smune::TreeAction::Show { name } => {
                return handle_tree_show(&name);
            }
            crate::cmd::smune::TreeAction::Find { query } => {
                return handle_tree_find(&query);
            }
        }
    }

    // Generate tree
    println!("{}", "🌳 Generating directory tree...".bright_green().bold());
    println!();

    let target_dir = if target.is_absolute() {
        target
    } else {
        std::env::current_dir()?.join(target)
    };

    if !target_dir.exists() {
        return Err(anyhow::anyhow!("Target directory does not exist: {}", target_dir.display()));
    }

    if !target_dir.is_dir() {
        return Err(anyhow::anyhow!("Target path is not a directory: {}", target_dir.display()));
    }

    let options = TreeOptions {
        no_folders,
        no_files,
        folder_size,
        file_size,
        line_count,
        dates,
        style: TreeStyle::from_str(&style),
        yolo,
    };

    let (dirs, files) = build_tree_structure(&target_dir, &options)?;

    let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
    let tree_content = generate_tree_markdown(&target_dir, &dirs, &files, &options);

    // Determine output path
    let output_path = if let Some(out_path) = out {
        if out_path.is_absolute() {
            out_path
        } else {
            std::env::current_dir()?.join(out_path)
        }
    } else {
        std::env::current_dir()?.join(format!("cm-tree-{}.md", timestamp))
    };

    // Write to output file
    fs::write(&output_path, &tree_content)
        .with_context(|| format!("Failed to write tree to: {}", output_path.display()))?;

    // Save to history
    let history_path = save_tree_history(&tree_content, &timestamp.to_string())?;

    println!("✅ Tree generated successfully!");
    println!("   Output: {}", output_path.display().to_string().cyan());
    println!("   History: {}", history_path.display().to_string().dimmed());
    println!();
    println!("📊 Statistics:");
    if !options.no_folders {
        println!("   Directories: {}", dirs.len().to_string().bright_white());
    }
    if !options.no_files {
        println!("   Files: {}", files.len().to_string().bright_white());
    }
    if options.folder_size || options.file_size {
        let total_size: u64 = dirs.values().map(|d| d.size).sum::<u64>()
            + files.values().map(|f| f.size).sum::<u64>();
        println!("   Total Size: {}", format_size(total_size).bright_white());
    }
    if options.line_count {
        let total_lines: usize = files.values()
            .filter_map(|f| f.lines)
            .sum();
        println!("   Total Lines: {}", total_lines.to_string().bright_white());
    }

    Ok(())
}

fn handle_tree_history() -> Result<()> {
    println!("{}", "📚 Tree History".bright_cyan().bold());
    println!();

    let trees = list_tree_history()?;

    if trees.is_empty() {
        println!("No tree history found.");
        println!("💡 Generate a tree with: cm tree");
        return Ok(());
    }

    println!("Found {} tree(s):\n", trees.len());

    for (i, tree_path) in trees.iter().enumerate() {
        if let Some(file_name) = tree_path.file_name().and_then(|n| n.to_str()) {
            // Extract timestamp from filename
            let timestamp = file_name
                .strip_prefix("cm-tree-")
                .and_then(|s| s.strip_suffix(".md"))
                .unwrap_or("unknown");

            println!("  {}. {}", i + 1, file_name.cyan());
            println!("     Path: {}", tree_path.display().to_string().dimmed());
            println!();
        }
    }

    println!("💡 View a tree with: cm tree show <name>");
    println!("💡 Search trees with: cm tree find <query>");

    Ok(())
}

fn handle_tree_show(name: &str) -> Result<()> {
    let trees = list_tree_history()?;

    // Try to find by exact filename or partial match
    let tree_path = trees.iter()
        .find(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.contains(name))
                .unwrap_or(false)
        })
        .or_else(|| trees.first());

    match tree_path {
        Some(path) => {
            let content = fs::read_to_string(path)?;
            println!("{}", content);
            Ok(())
        }
        None => {
            Err(anyhow::anyhow!("Tree '{}' not found", name))
        }
    }
}

fn handle_tree_find(query: &str) -> Result<()> {
    println!("{}", format!("🔍 Searching for: {}", query).bright_cyan().bold());
    println!();

    let trees = list_tree_history()?;
    let mut found = Vec::new();

    for tree_path in &trees {
        if let Ok(content) = fs::read_to_string(tree_path) {
            if content.to_lowercase().contains(&query.to_lowercase()) {
                found.push(tree_path.clone());
            }
        }
    }

    if found.is_empty() {
        println!("No trees found matching '{}'", query);
        return Ok(());
    }

    println!("Found {} matching tree(s):\n", found.len());

    for (i, tree_path) in found.iter().enumerate() {
        if let Some(file_name) = tree_path.file_name().and_then(|n| n.to_str()) {
            println!("  {}. {}", i + 1, file_name.cyan());
            println!("     Path: {}", tree_path.display().to_string().dimmed());
            println!();
        }
    }

    Ok(())
}

