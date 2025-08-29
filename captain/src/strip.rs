use anyhow::Result;
use clap::Parser;
use std::fs;
use std::path::PathBuf;
use walkdir::WalkDir;
#[derive(Parser, Debug)]
#[command(
    name = "strip",
    about = "Strip comments, blank lines, and attributes from Rust source code",
    long_about = r#"Remove comments, blank lines, attributes, and other non-essential elements from Rust source code.

MODES:
    Basic:    Remove comments and optionally blank lines
    Minify:   Single-line output where possible
    Aggressive: Maximum stripping - removes attributes, docs, and compresses whitespace

BACKUP SAFETY:
    ✅ By default, backups are created in ~/.shipwreck/strip/
    ❌ Use --no-backup to disable backups
    ⚠️ Use --force to allow overwriting the same file

EXAMPLES:
    cm strip src/main.rs                           # Basic stripping to stdout (with backup)
    cm strip src/main.rs --output main.stripped.rs # Strip to new file (with backup)
    cm strip src/ -r                              # Process directory (with backups)
    cm strip src/ -r -a                           # Aggressive stripping (with backups)
    cm strip main.rs --minify                     # Single-line output (with backup)
    cm strip src/ -r --strip-attrs --strip-docs   # Remove specific elements (with backups)
    cm strip src/ -r --no-backup                  # Process without backups (dangerous!)"#
)]
pub struct StripArgs {
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: Option<PathBuf>,
    #[arg(long)]
    pub target: Option<PathBuf>,
    #[arg(short = 'b', long)]
    pub remove_blanks: bool,
    #[arg(short, long)]
    pub recursive: bool,
    #[arg(long)]
    pub force: bool,
    #[arg(long)]
    pub no_backup: bool,
    #[arg(long)]
    pub src: bool,
    #[arg(long, default_value = "10")]
    pub max_depth: usize,
    #[arg(long, short = 'a')]
    pub aggressive: bool,
    #[arg(long)]
    pub minify: bool,
    #[arg(long, short = 't')]
    pub tease: bool,
    #[arg(long)]
    pub strip_attrs: bool,
    #[arg(long)]
    pub strip_docs: bool,
    #[arg(long)]
    pub inline_uses: bool,
}
pub fn handle_strip_command(args: StripArgs) -> Result<()> {
    show_active_options(&args);
    let input_path = determine_input_path(&args)?;
    if !input_path.exists() {
        return Err(
            anyhow::anyhow!("Input path does not exist: {}", input_path.display()),
        );
    }
    let backup_dir = create_backup_directory()?;
    if args.recursive || input_path.is_dir() {
        process_directory(&input_path, &args, &backup_dir)?;
    } else {
        process_single_file(&input_path, &args, args.output.as_ref(), &backup_dir)?;
    }
    Ok(())
}
fn show_active_options(args: &StripArgs) {
    let mut options = Vec::new();
    if args.tease {
        options.push("🌶️ TEASE mode (remove all comments + blanks)");
    } else if args.aggressive {
        options.push("🔥 Aggressive mode");
    } else {
        if args.remove_blanks {
            options.push("📝 Remove blank lines");
        }
        if args.strip_attrs {
            options.push("🏷️  Strip attributes");
        }
        if args.strip_docs {
            options.push("📖 Strip doc comments");
        }
        if args.minify {
            options.push("🎯 Minify output");
        }
        if args.inline_uses {
            options.push("🔗 Inline use statements");
        }
    }
    if args.no_backup {
        options.push("❌ No backups");
    } else {
        options.push("💾 Auto-backup (default)");
    }
    if args.force {
        options.push("⚠️  Force overwrite");
    }
    if args.recursive {
        options.push("📁 Recursive");
    }
    if !options.is_empty() {
        println!("🚀 Active options:");
        for option in options {
            println!("   {}", option);
        }
        println!();
    }
}
fn determine_input_path(args: &StripArgs) -> Result<PathBuf> {
    if args.src {
        Ok(PathBuf::from("src"))
    } else if let Some(target) = &args.target {
        Ok(target.clone())
    } else {
        Ok(args.input.clone())
    }
}
fn create_backup_directory() -> Result<PathBuf> {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let backup_dir = PathBuf::from(home).join(".shipwreck").join("strip");
    if !backup_dir.exists() {
        fs::create_dir_all(&backup_dir)?;
        println!("📁 Created backup directory: {}", backup_dir.display());
    }
    Ok(backup_dir)
}
fn create_backup(original_path: &PathBuf, backup_dir: &PathBuf) -> Result<PathBuf> {
    let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
    let file_name = original_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown");
    let backup_name = format!("{}_{}.backup", file_name, timestamp);
    let backup_path = backup_dir.join(backup_name);
    fs::copy(original_path, &backup_path)?;
    println!("🔄 Backup created: {}", backup_path.display());
    Ok(backup_path)
}
fn strip_rust(source: &str, args: &StripArgs) -> Result<String> {
    let source_to_parse = if args.tease {
        strip_all_comments_manual(source)
    } else {
        source.to_string()
    };
    let mut syntax_tree = syn::parse_file(&source_to_parse)?;
    if args.aggressive || args.strip_attrs {
        strip_attributes(&mut syntax_tree);
    }
    if args.aggressive || args.strip_docs {
        strip_doc_comments(&mut syntax_tree);
    }
    if args.aggressive || args.inline_uses {
        inline_use_statements(&mut syntax_tree);
    }
    let mut output = if args.minify || args.aggressive {
        prettyplease::unparse(&syntax_tree)
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
    } else {
        prettyplease::unparse(&syntax_tree)
    };
    if args.aggressive {
        output = output
            .replace(" ;", ";")
            .replace(" ,", ",")
            .replace(" :", ":")
            .replace(" {", "{")
            .replace("{ ", "{")
            .replace(" }", "}")
            .replace("} ", "}")
            .replace(" (", "(")
            .replace("( ", "(")
            .replace(" )", ")")
            .replace(") ", ")")
            .replace(" ->", "->")
            .replace("-> ", "->");
    }
    if args.tease || args.remove_blanks {
        let lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
        let mut filtered_lines = Vec::new();
        let mut prev_empty = false;
        for line in lines {
            let is_empty = line.trim().is_empty();
            if !is_empty || !prev_empty {
                filtered_lines.push(line);
            }
            prev_empty = is_empty;
        }
        output = filtered_lines.join("\n");
    }
    Ok(output)
}
fn strip_attributes(syntax_tree: &mut syn::File) {
    use syn::visit_mut::{self, VisitMut};
    struct AttrStripper;
    impl VisitMut for AttrStripper {
        fn visit_item_mut(&mut self, item: &mut syn::Item) {
            match item {
                syn::Item::Fn(f) => {
                    f.attrs
                        .retain(|attr| {
                            attr.path().is_ident("test") || attr.path().is_ident("cfg")
                        });
                }
                _ => {
                    if let Some(attrs) = get_attrs_mut(item) {
                        attrs.clear();
                    }
                }
            }
            visit_mut::visit_item_mut(self, item);
        }
    }
    AttrStripper.visit_file_mut(syntax_tree);
}
fn strip_doc_comments(syntax_tree: &mut syn::File) {
    use syn::visit_mut::{self, VisitMut};
    struct DocStripper;
    impl VisitMut for DocStripper {
        fn visit_item_mut(&mut self, item: &mut syn::Item) {
            match item {
                syn::Item::Fn(f) => {
                    f.attrs.retain(|attr| !is_doc_attr(attr));
                }
                syn::Item::Struct(s) => {
                    s.attrs.retain(|attr| !is_doc_attr(attr));
                }
                syn::Item::Enum(e) => {
                    e.attrs.retain(|attr| !is_doc_attr(attr));
                }
                syn::Item::Trait(t) => {
                    t.attrs.retain(|attr| !is_doc_attr(attr));
                }
                syn::Item::Impl(i) => {
                    i.attrs.retain(|attr| !is_doc_attr(attr));
                }
                syn::Item::Mod(m) => {
                    m.attrs.retain(|attr| !is_doc_attr(attr));
                }
                syn::Item::Type(t) => {
                    t.attrs.retain(|attr| !is_doc_attr(attr));
                }
                syn::Item::Const(c) => {
                    c.attrs.retain(|attr| !is_doc_attr(attr));
                }
                syn::Item::Static(s) => {
                    s.attrs.retain(|attr| !is_doc_attr(attr));
                }
                _ => {
                    if let Some(attrs) = get_attrs_mut(item) {
                        attrs.retain(|attr| !is_doc_attr(attr));
                    }
                }
            }
            visit_mut::visit_item_mut(self, item);
        }
    }
    DocStripper.visit_file_mut(syntax_tree);
}
fn inline_use_statements(_syntax_tree: &mut syn::File) {}
fn strip_all_comments_manual(source: &str) -> String {
    let mut result = String::new();
    let mut chars = source.chars().peekable();
    let mut in_string = false;
    let mut in_char = false;
    let mut escape_next = false;
    while let Some(ch) = chars.next() {
        if escape_next {
            result.push(ch);
            escape_next = false;
            continue;
        }
        if ch == '\\' && (in_string || in_char) {
            result.push(ch);
            escape_next = true;
            continue;
        }
        if ch == '"' && !in_char {
            in_string = !in_string;
            result.push(ch);
            continue;
        }
        if ch == '\'' && !in_string {
            in_char = !in_char;
            result.push(ch);
            continue;
        }
        if !in_string && !in_char {
            if ch == '/' {
                if let Some(&next_ch) = chars.peek() {
                    if next_ch == '/' {
                        chars.next();
                        while let Some(comment_ch) = chars.next() {
                            if comment_ch == '\n' {
                                result.push('\n');
                                break;
                            }
                        }
                        continue;
                    } else if next_ch == '*' {
                        chars.next();
                        let mut prev_ch = ' ';
                        while let Some(comment_ch) = chars.next() {
                            if prev_ch == '*' && comment_ch == '/' {
                                break;
                            }
                            prev_ch = comment_ch;
                        }
                        result.push(' ');
                        continue;
                    }
                }
            }
        }
        result.push(ch);
    }
    result
}
fn is_doc_attr(attr: &syn::Attribute) -> bool {
    attr.path().is_ident("doc")
}
fn get_attrs_mut(item: &mut syn::Item) -> Option<&mut Vec<syn::Attribute>> {
    match item {
        syn::Item::Const(item) => Some(&mut item.attrs),
        syn::Item::Enum(item) => Some(&mut item.attrs),
        syn::Item::ExternCrate(item) => Some(&mut item.attrs),
        syn::Item::Fn(item) => Some(&mut item.attrs),
        syn::Item::ForeignMod(item) => Some(&mut item.attrs),
        syn::Item::Impl(item) => Some(&mut item.attrs),
        syn::Item::Macro(item) => Some(&mut item.attrs),
        syn::Item::Mod(item) => Some(&mut item.attrs),
        syn::Item::Static(item) => Some(&mut item.attrs),
        syn::Item::Struct(item) => Some(&mut item.attrs),
        syn::Item::Trait(item) => Some(&mut item.attrs),
        syn::Item::TraitAlias(item) => Some(&mut item.attrs),
        syn::Item::Type(item) => Some(&mut item.attrs),
        syn::Item::Union(item) => Some(&mut item.attrs),
        syn::Item::Use(item) => Some(&mut item.attrs),
        syn::Item::Verbatim(_) => None,
        _ => None,
    }
}
fn process_single_file(
    input_path: &PathBuf,
    args: &StripArgs,
    output_path: Option<&PathBuf>,
    backup_dir: &PathBuf,
) -> Result<()> {
    println!("📝 Processing single file: {}", input_path.display());
    let original_content = fs::read_to_string(input_path)?;
    if !args.no_backup {
        create_backup(input_path, backup_dir)?;
    }
    let stripped_content = strip_rust(&original_content, args)?;
    let final_output_path = if let Some(output) = output_path {
        output.clone()
    } else {
        let file_stem = input_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");
        let extension = input_path.extension().and_then(|s| s.to_str()).unwrap_or("rs");
        input_path.with_file_name(format!("{}.stripped.{}", file_stem, extension))
    };
    if final_output_path == *input_path && !args.force {
        println!(
            "⚠️  Output path is same as input. Use --force to overwrite or specify different output path."
        );
        println!("   Suggested: --output {}", final_output_path.display());
        return Ok(());
    }
    fs::write(&final_output_path, stripped_content)?;
    if final_output_path != *input_path {
        println!("✅ Stripped code written to: {}", final_output_path.display());
    } else {
        println!("✅ File overwritten: {}", input_path.display());
    }
    let original_lines = original_content.lines().count();
    let stripped_lines = fs::read_to_string(&final_output_path)?.lines().count();
    let reduction = if original_lines > 0 && stripped_lines <= original_lines {
        ((original_lines - stripped_lines) as f64 / original_lines as f64 * 100.0) as i32
    } else if original_lines > 0 && stripped_lines > original_lines {
        0
    } else {
        0
    };
    println!(
        "📊 Lines: {} → {} ({}% reduction)", original_lines, stripped_lines,
        reduction
    );
    Ok(())
}
fn process_directory(
    dir: &PathBuf,
    args: &StripArgs,
    backup_dir: &PathBuf,
) -> Result<()> {
    println!(
        "📁 Processing directory: {} (max depth: {})", dir.display(), args.max_depth
    );
    let output_base = if let Some(output) = &args.output {
        if !output.exists() {
            fs::create_dir_all(output)?;
            println!("📁 Created output directory: {}", output.display());
        }
        Some(output.clone())
    } else {
        None
    };
    let mut processed_count = 0;
    let mut skipped_count = 0;
    let mut error_count = 0;
    let walker = WalkDir::new(dir)
        .max_depth(args.max_depth)
        .into_iter()
        .filter_map(|e| e.ok());
    for entry in walker {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("rs") {
            let output_path = if let Some(ref output_base) = output_base {
                let relative = path.strip_prefix(dir).unwrap_or(path);
                let output_file = output_base.join(relative);
                if let Some(parent) = output_file.parent() {
                    fs::create_dir_all(parent)?;
                }
                Some(output_file)
            } else {
                None
            };
            match process_single_file(
                &path.to_path_buf(),
                args,
                output_path.as_ref(),
                backup_dir,
            ) {
                Ok(_) => {
                    processed_count += 1;
                }
                Err(e) => {
                    println!("❌ Error processing {}: {}", path.display(), e);
                    error_count += 1;
                }
            }
        } else if path.is_dir() {
            continue;
        } else {
            skipped_count += 1;
        }
    }
    println!("📊 Directory processing complete:");
    println!("   ✅ Files processed: {}", processed_count);
    if error_count > 0 {
        println!("   ❌ Errors: {}", error_count);
    }
    if skipped_count > 0 {
        println!("   ⏭️  Files skipped: {}", skipped_count);
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use tempfile::TempDir;
    #[test]
    fn test_strip_comments() {
        let source = r#"
// This is a comment
fn main() {
    // Another comment
    println!("Hello"); // Inline comment
    /* Block comment */
}
"#;
        let args = StripArgs {
            input: PathBuf::from("test.rs"),
            output: None,
            target: None,
            remove_blanks: false,
            recursive: false,
            force: false,
            no_backup: false,
            src: false,
            max_depth: 10,
            aggressive: false,
            minify: false,
            tease: false,
            strip_attrs: false,
            strip_docs: false,
            inline_uses: false,
        };
        let result = strip_rust(source, &args).unwrap();
        assert!(! result.contains("//"));
        assert!(! result.contains("/*"));
        assert!(result.contains("fn main()"));
        assert!(result.contains("println!"));
    }
    #[test]
    fn test_strip_blank_lines() {
        let source = r#"fn main() {

    println!("Hello");

}
"#;
        let args = StripArgs {
            input: PathBuf::from("test.rs"),
            output: None,
            target: None,
            remove_blanks: true,
            recursive: false,
            force: false,
            no_backup: false,
            src: false,
            max_depth: 10,
            aggressive: false,
            minify: false,
            strip_attrs: false,
            strip_docs: false,
            inline_uses: false,
        };
        let result = strip_rust(source, &args).unwrap();
        let lines: Vec<&str> = result.lines().collect();
        let consecutive_empty = lines
            .windows(2)
            .any(|window| window[0].trim().is_empty() && window[1].trim().is_empty());
        assert!(! consecutive_empty);
    }
    #[test]
    fn test_aggressive_stripping() {
        let source = r#"
/// This is a doc comment
#[derive(Debug)]
fn main() {
    println!("Hello");
}
"#;
        let args = StripArgs {
            input: PathBuf::from("test.rs"),
            output: None,
            target: None,
            remove_blanks: false,
            recursive: false,
            force: false,
            no_backup: false,
            src: false,
            max_depth: 10,
            aggressive: true,
            minify: false,
            strip_attrs: false,
            strip_docs: false,
            inline_uses: false,
        };
        let result = strip_rust(source, &args).unwrap();
        assert!(! result.contains("///"));
        assert!(! result.contains("#[derive(Debug)]"));
        assert!(result.contains("fn main()"));
    }
    #[test]
    fn test_determine_input_path() {
        let mut args = StripArgs {
            input: PathBuf::from("test.rs"),
            output: None,
            target: None,
            remove_blanks: false,
            recursive: false,
            force: false,
            no_backup: false,
            src: false,
            max_depth: 10,
            aggressive: false,
            minify: false,
            strip_attrs: false,
            strip_docs: false,
            inline_uses: false,
        };
        assert_eq!(determine_input_path(& args).unwrap(), PathBuf::from("test.rs"));
        args.src = true;
        assert_eq!(determine_input_path(& args).unwrap(), PathBuf::from("src"));
        args.target = Some(PathBuf::from("target/dir"));
        assert_eq!(determine_input_path(& args).unwrap(), PathBuf::from("target/dir"));
    }
    #[test]
    fn test_backup_creation() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.rs");
        let backup_dir = temp_dir.path().join("backups");
        fs::create_dir_all(&backup_dir).unwrap();
        fs::write(&test_file, "fn main() {}").unwrap();
        let backup_path = create_backup(&test_file, &backup_dir).unwrap();
        assert!(backup_path.exists());
        assert!(backup_path.to_string_lossy().contains("test.rs"));
        assert!(backup_path.to_string_lossy().contains(".backup"));
    }
}