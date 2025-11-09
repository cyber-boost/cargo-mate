use anyhow::{Context, Result};
use colored::*;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use syn::{Item, Visibility};
use walkdir::WalkDir;

#[derive(Debug, Clone)]
struct ModuleExports {
    pub structs: BTreeSet<String>,
    pub enums: BTreeSet<String>,
    pub functions: BTreeSet<String>,
    pub constants: BTreeSet<String>,
    pub types: BTreeSet<String>,
    pub traits: BTreeSet<String>,
    pub impls: BTreeSet<String>,
    pub statics: BTreeSet<String>,
    pub unions: BTreeSet<String>,
}

impl ModuleExports {
    fn new() -> Self {
        Self {
            structs: BTreeSet::new(),
            enums: BTreeSet::new(),
            functions: BTreeSet::new(),
            constants: BTreeSet::new(),
            types: BTreeSet::new(),
            traits: BTreeSet::new(),
            impls: BTreeSet::new(),
            statics: BTreeSet::new(),
            unions: BTreeSet::new(),
        }
    }

    fn is_empty(&self) -> bool {
        self.structs.is_empty()
            && self.enums.is_empty()
            && self.functions.is_empty()
            && self.constants.is_empty()
            && self.types.is_empty()
            && self.traits.is_empty()
            && self.impls.is_empty()
            && self.statics.is_empty()
            && self.unions.is_empty()
    }
}

fn is_public(vis: &Visibility) -> bool {
    matches!(vis, Visibility::Public(_))
}

fn extract_exports_from_file(file_path: &Path) -> Result<ModuleExports> {
    let content = fs::read_to_string(file_path)
        .with_context(|| format!("Failed to read file: {}", file_path.display()))?;

    let ast = syn::parse_file(&content)
        .with_context(|| format!("Failed to parse Rust file: {}", file_path.display()))?;

    let mut exports = ModuleExports::new();

    for item in ast.items {
        match item {
            Item::Struct(s) if is_public(&s.vis) => {
                exports.structs.insert(s.ident.to_string());
            }
            Item::Enum(e) if is_public(&e.vis) => {
                exports.enums.insert(e.ident.to_string());
            }
            Item::Fn(f) if is_public(&f.vis) => {
                exports.functions.insert(f.sig.ident.to_string());
            }
            Item::Const(c) if is_public(&c.vis) => {
                exports.constants.insert(c.ident.to_string());
            }
            Item::Type(t) if is_public(&t.vis) => {
                exports.types.insert(t.ident.to_string());
            }
            Item::Trait(t) if is_public(&t.vis) => {
                exports.traits.insert(t.ident.to_string());
            }
            Item::Impl(i) => {
                // For impl blocks, we extract the type being implemented
                if let Some((_, path, _)) = &i.trait_ {
                    // Trait impl
                    if let Some(segment) = path.segments.last() {
                        exports.impls.insert(segment.ident.to_string());
                    }
                } else if let Some(path) = &i.self_ty {
                    // Inherent impl - extract type name
                    if let syn::Type::Path(type_path) = path.as_ref() {
                        if let Some(segment) = type_path.path.segments.last() {
                            exports.impls.insert(segment.ident.to_string());
                        }
                    }
                }
            }
            Item::Static(s) if is_public(&s.vis) => {
                exports.statics.insert(s.ident.to_string());
            }
            Item::Union(u) if is_public(&u.vis) => {
                exports.unions.insert(u.ident.to_string());
            }
            Item::Mod(m) => {
                // Handle inline modules - extract their public items recursively
                if let Some((_, items)) = m.content {
                    for item in items {
                        match item {
                            Item::Struct(s) if is_public(&s.vis) => {
                                exports.structs.insert(s.ident.to_string());
                            }
                            Item::Enum(e) if is_public(&e.vis) => {
                                exports.enums.insert(e.ident.to_string());
                            }
                            Item::Fn(f) if is_public(&f.vis) => {
                                exports.functions.insert(f.sig.ident.to_string());
                            }
                            Item::Const(c) if is_public(&c.vis) => {
                                exports.constants.insert(c.ident.to_string());
                            }
                            Item::Type(t) if is_public(&t.vis) => {
                                exports.types.insert(t.ident.to_string());
                            }
                            Item::Trait(t) if is_public(&t.vis) => {
                                exports.traits.insert(t.ident.to_string());
                            }
                            Item::Static(s) if is_public(&s.vis) => {
                                exports.statics.insert(s.ident.to_string());
                            }
                            Item::Union(u) if is_public(&u.vis) => {
                                exports.unions.insert(u.ident.to_string());
                            }
                            _ => {}
                        }
                    }
                }
            }
            _ => {}
        }
    }

    Ok(exports)
}

fn get_module_name(file_path: &Path, _base_dir: &Path) -> String {
    // Use the file stem as the module name
    // This matches the Python script's approach - each file becomes a module
    file_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_string()
}

fn generate_lib_rs(
    exports: &BTreeMap<String, ModuleExports>,
    output_path: &Path,
) -> Result<()> {
    let mut content = String::new();

    // Header comment
    content.push_str("//! Auto-generated lib.rs file\n");
    content.push_str("//! Generated by Cargo Mate LIBerate command\n");
    content.push_str("//! This file exports all public items from the project\n\n");

    // Collect all module names and sort them
    let mut module_names: Vec<&String> = exports.keys().collect();
    module_names.sort();

    // Generate module declarations
    for module_name in &module_names {
        let module_exports = &exports[*module_name];
        
        // Skip empty modules
        if module_exports.is_empty() {
            continue;
        }

        content.push_str(&format!("pub mod {};\n", module_name));
    }

    content.push_str("\n");

    // Generate re-exports
    content.push_str("// Re-export all public items\n\n");

    for module_name in &module_names {
        let module_exports = &exports[*module_name];
        
        if module_exports.is_empty() {
            continue;
        }

        let crate_path = format!("crate::{}", module_name);

        // Collect all items to re-export
        let mut all_items: Vec<String> = Vec::new();
        all_items.extend(module_exports.structs.iter().cloned());
        all_items.extend(module_exports.enums.iter().cloned());
        all_items.extend(module_exports.functions.iter().cloned());
        all_items.extend(module_exports.constants.iter().cloned());
        all_items.extend(module_exports.types.iter().cloned());
        all_items.extend(module_exports.traits.iter().cloned());
        all_items.extend(module_exports.statics.iter().cloned());
        all_items.extend(module_exports.unions.iter().cloned());

        if !all_items.is_empty() {
            content.push_str(&format!("// Re-export from {}\n", module_name));
            for item_name in all_items {
                content.push_str(&format!("pub use {}::{};\n", crate_path, item_name));
            }
            content.push_str("\n");
        }
    }

    // Write to file
    fs::write(output_path, content)
        .with_context(|| format!("Failed to write lib.rs to: {}", output_path.display()))?;

    Ok(())
}

pub fn handle_liberate(target: PathBuf, out: Option<PathBuf>) -> Result<()> {
    println!("{}", "🔓 LIBerate - Setting your code free!".bright_cyan().bold());
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

    // Determine output path
    let output_path = if let Some(out_path) = out {
        if out_path.is_absolute() {
            out_path
        } else {
            std::env::current_dir()?.join(out_path)
        }
    } else {
        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
        target_dir.join(format!(".LIBerate-{}.rs", timestamp))
    };

    println!("📂 Scanning directory: {}", target_dir.display().to_string().cyan());
    println!("📝 Output file: {}", output_path.display().to_string().cyan());
    println!();

    // Collect all .rs files
    let mut exports: BTreeMap<String, ModuleExports> = BTreeMap::new();
    let mut file_count = 0;
    let mut error_count = 0;

    for entry in WalkDir::new(&target_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path().is_file()
                && e.path().extension().map(|ext| ext == "rs").unwrap_or(false)
                && !e.path().file_name().map(|n| n.to_string_lossy().starts_with(".LIBerate")).unwrap_or(false)
        })
    {
        file_count += 1;
        let file_path = entry.path();
        
        // Skip lib.rs and main.rs to avoid circular references
        if let Some(file_name) = file_path.file_name().and_then(|n| n.to_str()) {
            if file_name == "lib.rs" || file_name == "main.rs" {
                continue;
            }
        }

        match extract_exports_from_file(file_path) {
            Ok(module_exports) => {
                if !module_exports.is_empty() {
                    let module_name = get_module_name(file_path, &target_dir);
                    // If module already exists, merge exports
                    if let Some(existing) = exports.get_mut(&module_name) {
                        existing.structs.extend(module_exports.structs);
                        existing.enums.extend(module_exports.enums);
                        existing.functions.extend(module_exports.functions);
                        existing.constants.extend(module_exports.constants);
                        existing.types.extend(module_exports.types);
                        existing.traits.extend(module_exports.traits);
                        existing.impls.extend(module_exports.impls);
                        existing.statics.extend(module_exports.statics);
                        existing.unions.extend(module_exports.unions);
                    } else {
                        exports.insert(module_name, module_exports);
                    }
                }
            }
            Err(e) => {
                error_count += 1;
                eprintln!("⚠️  Warning: Failed to parse {}: {}", file_path.display(), e);
            }
        }
    }

    println!("📊 Statistics:");
    println!("   Files scanned: {}", file_count.to_string().bright_white());
    println!("   Modules found: {}", exports.len().to_string().bright_white());
    println!("   Errors: {}", error_count.to_string().bright_red());
    println!();

    if exports.is_empty() {
        println!("{}", "⚠️  No public items found to export".yellow());
        return Ok(());
    }

    // Generate lib.rs
    println!("🔨 Generating lib.rs...");
    generate_lib_rs(&exports, &output_path)?;

    println!();
    println!("{}", "✅ Successfully generated lib.rs!".bright_green().bold());
    println!("   Location: {}", output_path.display().to_string().cyan());
    println!();
    println!("💡 Next steps:");
    println!("   1. Review the generated file");
    println!("   2. Copy it to your src/lib.rs if needed");
    println!("   3. Adjust module paths as necessary");

    Ok(())
}

