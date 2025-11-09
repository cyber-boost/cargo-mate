use anyhow::{Context, Result};
use colored::*;
use regex::Regex;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Debug, Clone)]
struct StubMatch {
    file_path: PathBuf,
    line_number: usize,
    line_content: String,
    pattern_matched: String,
    context_before: Vec<String>,
    context_after: Vec<String>,
}

#[derive(Debug, Clone)]
struct StubOptions {
    extensions: Vec<String>,
    custom_patterns: Vec<String>,
    skip_patterns: Vec<String>,
}

// Default stub patterns (case-insensitive)
const DEFAULT_PATTERNS: &[&str] = &[
    r"\bTODO\b",
    r"\bFIXME\b",
    r"\bXXX\b",
    r"\bHACK\b",
    r"\bSTUB\b",
    r"\bstub\b",
    r"\bplaceholder\b",
    r"\bPLACEHOLDER\b",
    r"\bmock\b",
    r"\bMOCK\b",
    r"\bin real implementation\b",
    r"\bin real Implementation\b",
    r"\bneed to\b",
    r"\bneed to implement\b",
    r"\bremind\b",
    r"\bremind me\b",
    r"\bfix\b",
    r"\bfixme\b",
    r"\bimplement later\b",
    r"\bnot implemented\b",
    r"\bnot yet implemented\b",
    r"\bunimplemented\b",
    r"\bto be implemented\b",
    r"\btbi\b",
    r"\bTBI\b",
    r"\bwip\b",
    r"\bWIP\b",
    r"\bwork in progress\b",
    r"\btemporary\b",
    r"\btemp\b",
    r"\bTEMP\b",
    r"\bskip\b",
    r"\bSKIP\b",
    r"pass\s*#.*implement",
    r"\.\.\.\s*#.*implement",
    r"raise NotImplementedError",
    r"raise NotImplemented",
    r"Some\(",
    r"None\s*#.*placeholder",
    r"None\s*#.*stub",
    r"None\s*#.*TODO",
    r"None\s*#.*FIXME",
    r"return None\s*#.*implement",
    r"return\s+None\s*#.*stub",
    r"return\s+\[\]\s*#.*stub",
    r"return\s+\{\}\s*#.*stub",
    r"return\s+""\s*#.*stub",
    r"print\(".*stub.*"\)",
    r"print\(".*placeholder.*"\)",
    r"print\(".*TODO.*"\)",
    r"print\(".*FIXME.*"\)",
    r"#.*implement.*later",
    r"#.*implement.*soon",
    r"#.*implement.*eventually",
    r"#.*fix.*later",
    r"#.*fix.*soon",
    r"#.*TODO",
    r"#.*FIXME",
    r"#.*XXX",
    r"#.*HACK",
    r"#.*stub",
    r"#.*placeholder",
    r"#.*mock",
    r"#.*not.*implemented",
    r"#.*unimplemented",
    r"#.*temporary",
    r"#.*temp",
    r"#.*wip",
    r"#.*work.*in.*progress",
    r"unimplemented!",
    r"todo!",
    r"unreachable!",
];

fn parse_extensions(ext_str: Option<&String>) -> Vec<String> {
    if let Some(exts) = ext_str {
        exts.split(',')
            .map(|s| s.trim().trim_start_matches('.').to_lowercase())
            .filter(|s| !s.is_empty())
            .collect()
    } else {
        vec!["rs".to_string(), "py".to_string(), "js".to_string(), "html".to_string()]
    }
}

fn parse_skip_patterns(skip_str: Option<&String>) -> Vec<String> {
    if let Some(skip) = skip_str {
        skip.split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    } else {
        Vec::new()
    }
}

fn build_patterns(options: &StubOptions) -> Vec<Regex> {
    let mut patterns = Vec::new();

    // Add default patterns
    for pattern in DEFAULT_PATTERNS {
        if let Ok(regex) = Regex::new(&format!("(?i){}", pattern)) {
            patterns.push(regex);
        }
    }

    // Add custom patterns
    for custom in &options.custom_patterns {
        if let Ok(regex) = Regex::new(&format!("(?i){}", custom)) {
            patterns.push(regex);
        }
    }

    // Filter out skip patterns
    if !options.skip_patterns.is_empty() {
        patterns.retain(|regex| {
            let pattern_str = regex.as_str();
            !options.skip_patterns.iter().any(|skip| {
                pattern_str.contains(skip) || skip.contains(pattern_str.trim_start_matches("(?i)"))
            })
        });
    }

    patterns
}

fn find_stubs_in_file(
    file_path: &Path,
    patterns: &[Regex],
    context_lines: usize,
) -> Result<Vec<StubMatch>> {
    let content = fs::read_to_string(file_path)
        .with_context(|| format!("Failed to read file: {}", file_path.display()))?;

    let lines: Vec<&str> = content.lines().collect();
    let mut matches = Vec::new();

    for (line_idx, line) in lines.iter().enumerate() {
        for pattern in patterns {
            if pattern.is_match(line) {
                let context_before: Vec<String> = lines
                    .iter()
                    .skip(line_idx.saturating_sub(context_lines))
                    .take(context_lines.min(line_idx))
                    .map(|s| s.to_string())
                    .collect();

                let context_after: Vec<String> = lines
                    .iter()
                    .skip(line_idx + 1)
                    .take(context_lines)
                    .map(|s| s.to_string())
                    .collect();

                matches.push(StubMatch {
                    file_path: file_path.to_path_buf(),
                    line_number: line_idx + 1,
                    line_content: line.to_string(),
                    pattern_matched: pattern.as_str().to_string(),
                    context_before,
                    context_after,
                });
                break; // Only match once per line
            }
        }
    }

    Ok(matches)
}

fn generate_stub_markdown(
    matches: &[StubMatch],
    options: &StubOptions,
) -> String {
    let mut content = String::new();

    content.push_str("# 🔍 Stub/Placeholder/TODO Finder Report\n\n");
    content.push_str("This report contains all stubs, placeholders, TODOs, and unimplemented code found in the project.\n\n");

    if matches.is_empty() {
        content.push_str("✅ **No stubs found!** Your code is clean.\n");
        return content;
    }

    content.push_str(&format!("**Total stubs found**: {}\n\n", matches.len()));

    // Group by file
    let mut by_file: BTreeMap<&PathBuf, Vec<&StubMatch>> = BTreeMap::new();
    for m in matches {
        by_file.entry(&m.file_path).or_insert_with(Vec::new).push(m);
    }

    content.push_str("## Summary by File\n\n");
    for (file_path, file_matches) in &by_file {
        content.push_str(&format!(
            "- **{}**: {} stub(s)\n",
            file_path.display(),
            file_matches.len()
        ));
    }
    content.push_str("\n");

    // Detailed matches
    content.push_str("## Detailed Matches\n\n");

    for (file_idx, (file_path, file_matches)) in by_file.iter().enumerate() {
        content.push_str(&format!("### {}. {}\n\n", file_idx + 1, file_path.display()));

        for (match_idx, m) in file_matches.iter().enumerate() {
            content.push_str(&format!("#### Match {} (Line {})\n\n", match_idx + 1, m.line_number));
            content.push_str(&format!("**Pattern**: `{}`\n\n", m.pattern_matched));
            content.push_str("**Code**:\n\n");
            content.push_str("```\n");

            // Context before
            for (i, ctx_line) in m.context_before.iter().enumerate() {
                let line_num = m.line_number - m.context_before.len() + i;
                content.push_str(&format!("{:4} | {}\n", line_num, ctx_line));
            }

            // The actual line
            content.push_str(&format!("{:4} | {}\n", m.line_number, m.line_content));
            content.push_str("     | ^^^ MATCH\n");

            // Context after
            for (i, ctx_line) in m.context_after.iter().enumerate() {
                let line_num = m.line_number + 1 + i;
                content.push_str(&format!("{:4} | {}\n", line_num, ctx_line));
            }

            content.push_str("```\n\n");
            content.push_str("---\n\n");
        }
    }

    // Footer
    let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
    content.push_str(&format!("---\n*Generated on {} by Cargo Mate*\n", timestamp));

    content
}

fn save_stub_history(content: &str, timestamp: &str) -> Result<PathBuf> {
    let home = dirs::home_dir()
        .context("Could not find home directory")?;
    let stubs_dir = home.join(".shipwreck").join("stubs");
    fs::create_dir_all(&stubs_dir)?;

    let filename = format!("cm-stubs-{}.md", timestamp);
    let history_path = stubs_dir.join(&filename);
    fs::write(&history_path, content)?;

    Ok(history_path)
}

fn list_stub_history() -> Result<Vec<PathBuf>> {
    let home = dirs::home_dir()
        .context("Could not find home directory")?;
    let stubs_dir = home.join(".shipwreck").join("stubs");

    if !stubs_dir.exists() {
        return Ok(Vec::new());
    }

    let mut stubs: Vec<PathBuf> = fs::read_dir(&stubs_dir)?
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path().is_file()
                && e.path().extension().map(|ext| ext == "md").unwrap_or(false)
                && e.path()
                    .file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n.starts_with("cm-stubs-"))
                    .unwrap_or(false)
        })
        .map(|e| e.path())
        .collect();

    stubs.sort();
    stubs.reverse(); // Most recent first
    Ok(stubs)
}

pub fn handle_stub(
    action: Option<crate::cmd::smune::StubAction>,
    target: PathBuf,
    out: Option<PathBuf>,
    ext: Option<String>,
    custom: Option<String>,
    find: Option<String>,
    skip: Option<String>,
) -> Result<()> {
    // Handle subcommands
    if let Some(action) = action {
        match action {
            crate::cmd::smune::StubAction::History => {
                return handle_stub_history();
            }
            crate::cmd::smune::StubAction::Show { name } => {
                return handle_stub_show(&name);
            }
            crate::cmd::smune::StubAction::Delete { all } => {
                return handle_stub_delete(all);
            }
            crate::cmd::smune::StubAction::Find { pattern } => {
                // Use pattern if provided, otherwise use find/custom from args
                let custom_pattern = pattern.or(custom).or(find);
                return handle_stub_find(target, out, ext, custom_pattern, skip);
            }
            crate::cmd::smune::StubAction::Skip { patterns } => {
                return handle_stub_find(target, out, ext, custom.or(find), Some(patterns));
            }
        }
    }

    // Default: find stubs
    handle_stub_find(target, out, ext, custom.or(find), skip)
}

fn handle_stub_find(
    target: PathBuf,
    out: Option<PathBuf>,
    ext: Option<String>,
    custom: Option<String>,
    skip: Option<String>,
) -> Result<()> {
    println!("{}", "🔍 Scanning for stubs, placeholders, and TODOs...".bright_yellow().bold());
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

    let extensions = parse_extensions(ext.as_ref());
    let custom_patterns = if let Some(custom) = custom {
        custom.split(',').map(|s| s.trim().to_string()).collect()
    } else {
        Vec::new()
    };
    let skip_patterns = parse_skip_patterns(skip.as_ref());

    let options = StubOptions {
        extensions: extensions.clone(),
        custom_patterns,
        skip_patterns,
    };

    let patterns = build_patterns(&options);

    println!("📂 Scanning directory: {}", target_dir.display().to_string().cyan());
    println!("📝 Extensions: {}", extensions.join(", ").bright_white());
    println!("🔎 Patterns: {} ({} custom, {} skipped)", 
        patterns.len() + DEFAULT_PATTERNS.len(),
        options.custom_patterns.len(),
        options.skip_patterns.len());
    println!();

    // Find all matching files
    let mut all_matches = Vec::new();
    let mut file_count = 0;

    for entry in WalkDir::new(&target_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_file())
    {
        let path = entry.path();
        if let Some(ext_str) = path.extension().and_then(|e| e.to_str()) {
            if extensions.contains(&ext_str.to_lowercase()) {
                file_count += 1;
                match find_stubs_in_file(path, &patterns, 3) {
                    Ok(matches) => {
                        all_matches.extend(matches);
                    }
                    Err(e) => {
                        eprintln!("⚠️  Warning: Failed to scan {}: {}", path.display(), e);
                    }
                }
            }
        }
    }

    println!("📊 Statistics:");
    println!("   Files scanned: {}", file_count.to_string().bright_white());
    println!("   Stubs found: {}", all_matches.len().to_string().bright_yellow());
    println!();

    // Generate markdown
    let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
    let stub_content = generate_stub_markdown(&all_matches, &options);

    // Determine output path
    let output_path = if let Some(out_path) = out {
        if out_path.is_absolute() {
            out_path
        } else {
            std::env::current_dir()?.join(out_path)
        }
    } else {
        std::env::current_dir()?.join(format!("cm-stubs-{}.md", timestamp))
    };

    // Write to output file
    fs::write(&output_path, &stub_content)
        .with_context(|| format!("Failed to write stub report to: {}", output_path.display()))?;

    // Save to history
    let history_path = save_stub_history(&stub_content, &timestamp.to_string())?;

    println!("✅ Stub report generated successfully!");
    println!("   Output: {}", output_path.display().to_string().cyan());
    println!("   History: {}", history_path.display().to_string().dimmed());
    println!();

    if all_matches.is_empty() {
        println!("{}", "🎉 No stubs found! Your code is clean.".bright_green());
    } else {
        println!("{}", format!("⚠️  Found {} stub(s) that need attention", all_matches.len()).bright_yellow());
    }

    Ok(())
}

fn handle_stub_history() -> Result<()> {
    println!("{}", "📚 Stub History".bright_cyan().bold());
    println!();

    let stubs = list_stub_history()?;

    if stubs.is_empty() {
        println!("No stub history found.");
        println!("💡 Generate a stub report with: cm stub");
        return Ok(());
    }

    println!("Found {} stub report(s):\n", stubs.len());

    for (i, stub_path) in stubs.iter().enumerate() {
        if let Some(file_name) = stub_path.file_name().and_then(|n| n.to_str()) {
            let timestamp = file_name
                .strip_prefix("cm-stubs-")
                .and_then(|s| s.strip_suffix(".md"))
                .unwrap_or("unknown");

            println!("  {}. {}", i + 1, file_name.cyan());
            println!("     Path: {}", stub_path.display().to_string().dimmed());
            println!();
        }
    }

    println!("💡 View a report with: cm stub show <name>");
    println!("💡 Delete reports with: cm stub delete --all");

    Ok(())
}

fn handle_stub_show(name: &str) -> Result<()> {
    let stubs = list_stub_history()?;

    let stub_path = stubs
        .iter()
        .find(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.contains(name))
                .unwrap_or(false)
        })
        .or_else(|| stubs.first());

    match stub_path {
        Some(path) => {
            let content = fs::read_to_string(path)?;
            println!("{}", content);
            Ok(())
        }
        None => Err(anyhow::anyhow!("Stub report '{}' not found", name)),
    }
}

fn handle_stub_delete(all: bool) -> Result<()> {
    let stubs = list_stub_history()?;

    if stubs.is_empty() {
        println!("No stub reports to delete.");
        return Ok(());
    }

    if all {
        println!("🗑️  Deleting all {} stub report(s)...", stubs.len());
        for stub_path in &stubs {
            fs::remove_file(stub_path)?;
            println!("   Deleted: {}", stub_path.file_name().and_then(|n| n.to_str()).unwrap_or("unknown"));
        }
        println!("✅ All stub reports deleted.");
    } else {
        println!("💡 Use --all flag to delete all stub reports");
        println!("   Example: cm stub delete --all");
    }

    Ok(())
}

