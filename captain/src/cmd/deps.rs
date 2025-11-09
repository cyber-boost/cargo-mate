use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeclaredDeps {
    pub normal: BTreeSet<String>,
    pub dev: BTreeSet<String>,
    pub build: BTreeSet<String>,
    pub renamed: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissingDep {
    pub name: String,
    pub crates_io: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DepsReport {
    pub crate_root: String,
    pub used_crates: BTreeSet<String>,
    pub declared_deps: DeclaredDeps,
    pub missing_deps: Vec<MissingDep>,
}

pub async fn handle_deps_async(path: Option<PathBuf>, json: bool) -> Result<()> {
    let start_dir = path.unwrap_or(std::env::current_dir()?);
    let crate_root = find_crate_root(&start_dir)
        .with_context(|| format!("No Cargo.toml found starting from {}", start_dir.display()))?;
    let used_roots = scan_used_crates(&crate_root)?;
    let declared = parse_declared_dependencies(&crate_root)?;
    let mut declared_union: BTreeSet<String> = declared
        .normal
        .union(&declared.dev)
        .cloned()
        .collect();
    declared_union.extend(declared.build.iter().cloned());

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .unwrap();
    let mut missing: Vec<MissingDep> = Vec::new();
    for name in used_roots.iter() {
        if !declared_union.contains(name) {
            let status = check_crates_io_async(name, &client).await;
            missing.push(MissingDep { name: name.clone(), crates_io: status });
        }
    }

    let report = DepsReport {
        crate_root: crate_root.display().to_string(),
        used_crates: used_roots,
        declared_deps: declared,
        missing_deps: missing,
    };

    if json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        print_human_report(&report);
    }
    Ok(())
}

fn find_crate_root(start: &Path) -> Result<PathBuf> {
    let mut current = if start.is_file() {
        start.parent().map(|p| p.to_path_buf()).unwrap_or_else(|| PathBuf::from("."))
    } else {
        start.to_path_buf()
    };
    loop {
        let candidate = current.join("Cargo.toml");
        if candidate.exists() {
            return Ok(current);
        }
        if !current.pop() {
            break;
        }
    }
    Err(anyhow!("Cargo.toml not found"))
}

fn scan_used_crates(crate_root: &Path) -> Result<BTreeSet<String>> {
    let src_dir = crate_root.join("src");
    let mut roots: BTreeSet<String> = BTreeSet::new();
    if src_dir.exists() {
        for entry in WalkDir::new(&src_dir).into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_file() && path.extension().map(|e| e == "rs").unwrap_or(false) {
                if let Ok(content) = fs::read_to_string(path) {
                    collect_use_roots(&content, &mut roots);
                    collect_extern_crates(&content, &mut roots);
                }
            }
        }
    }
    let build_rs = crate_root.join("build.rs");
    if build_rs.exists() {
        if let Ok(content) = fs::read_to_string(&build_rs) {
            collect_use_roots(&content, &mut roots);
            collect_extern_crates(&content, &mut roots);
        }
    }
    // Filter non-external and local modules
    let reserved = ["crate", "self", "super", "std", "core", "alloc"];
    roots.retain(|name| !reserved.contains(&name.as_str()));
    roots.retain(|name| !is_local_module(crate_root, name));
    Ok(roots)
}

fn collect_use_roots(source: &str, out: &mut BTreeSet<String>) {
    let file = match syn::parse_file(source) {
        Ok(f) => f,
        Err(_) => return,
    };
    for item in file.items {
        if let syn::Item::Use(u) = item {
            walk_use_tree(&u.tree, None, out);
        }
    }
}

fn collect_extern_crates(source: &str, out: &mut BTreeSet<String>) {
    let file = match syn::parse_file(source) {
        Ok(f) => f,
        Err(_) => return,
    };
    for item in file.items {
        if let syn::Item::ExternCrate(ext) = item {
            let used = if let Some((_, rename)) = ext.rename {
                rename.to_string()
            } else {
                ext.ident.to_string()
            };
            out.insert(used);
        }
    }
}

fn walk_use_tree(tree: &syn::UseTree, current_root: Option<String>, out: &mut BTreeSet<String>) {
    match tree {
        syn::UseTree::Path(p) => {
            let ident = p.ident.to_string();
            let root = current_root.unwrap_or(ident);
            walk_use_tree(&*p.tree, Some(root), out);
        }
        syn::UseTree::Name(n) => {
            let root = current_root.unwrap_or(n.ident.to_string());
            out.insert(root);
        }
        syn::UseTree::Rename(r) => {
            let root = current_root.unwrap_or(r.ident.to_string());
            out.insert(root);
        }
        syn::UseTree::Glob(_) => {
            if let Some(root) = current_root {
                out.insert(root);
            }
        }
        syn::UseTree::Group(g) => {
            for item in &g.items {
                walk_use_tree(item, current_root.clone(), out);
            }
        }
    }
}

fn is_local_module(crate_root: &Path, name: &str) -> bool {
    let src = crate_root.join("src");
    let file_rs = src.join(format!("{}.rs", name));
    let mod_rs = src.join(name).join("mod.rs");
    file_rs.exists() || mod_rs.exists()
}

fn parse_declared_dependencies(crate_root: &Path) -> Result<DeclaredDeps> {
    let cargo_toml_path = crate_root.join("Cargo.toml");
    let content = fs::read_to_string(&cargo_toml_path)
        .with_context(|| format!("Failed to read {}", cargo_toml_path.display()))?;
    let value: toml::Value = toml::from_str(&content)
        .with_context(|| format!("Failed to parse {}", cargo_toml_path.display()))?;
    let mut declared = DeclaredDeps {
        normal: BTreeSet::new(),
        dev: BTreeSet::new(),
        build: BTreeSet::new(),
        renamed: BTreeMap::new(),
    };
    for (kind, set) in [
        ("dependencies", &mut declared.normal),
        ("dev-dependencies", &mut declared.dev),
        ("build-dependencies", &mut declared.build),
    ] {
        if let Some(table) = value.get(kind).and_then(|v| v.as_table()) {
            for (dep_key, spec) in table {
                set.insert(dep_key.clone());
                if let Some(t) = spec.as_table() {
                    if let Some(pkg) = t.get("package").and_then(|v| v.as_str()) {
                        declared.renamed.insert(dep_key.clone(), pkg.to_string());
                    }
                }
            }
        }
    }
    Ok(declared)
}

async fn check_crates_io_async(name: &str, client: &reqwest::Client) -> String {
    let url = format!("https://crates.io/api/v1/crates/{}", name);
    match client.get(&url).send().await {
        Ok(resp) => match resp.status().as_u16() {
            200 => "exists".to_string(),
            404 => "missing".to_string(),
            _ => "error".to_string(),
        },
        Err(_) => "error".to_string(),
    }
}

fn print_human_report(report: &DepsReport) {
    println!("📦 Crate root: {}", report.crate_root);
    println!("");
    println!("🔎 Detected external crates (from use statements):");
    if report.used_crates.is_empty() {
        println!("  (none)");
    } else {
        for name in &report.used_crates {
            println!("  - {}", name);
        }
    }
    println!("");
    println!("🧾 Declared dependencies:");
    if !report.declared_deps.normal.is_empty() {
        println!("  [dependencies]:");
        for k in &report.declared_deps.normal {
            if let Some(pkg) = report.declared_deps.renamed.get(k) {
                println!("    - {} (package = {})", k, pkg);
            } else {
                println!("    - {}", k);
            }
        }
    }
    if !report.declared_deps.dev.is_empty() {
        println!("  [dev-dependencies]:");
        for k in &report.declared_deps.dev {
            if let Some(pkg) = report.declared_deps.renamed.get(k) {
                println!("    - {} (package = {})", k, pkg);
            } else {
                println!("    - {}", k);
            }
        }
    }
    if !report.declared_deps.build.is_empty() {
        println!("  [build-dependencies]:");
        for k in &report.declared_deps.build {
            if let Some(pkg) = report.declared_deps.renamed.get(k) {
                println!("    - {} (package = {})", k, pkg);
            } else {
                println!("    - {}", k);
            }
        }
    }
    println!("");
    println!("❗ Missing dependencies (not declared in Cargo.toml):");
    if report.missing_deps.is_empty() {
        println!("  (none)");
    } else {
        for m in &report.missing_deps {
            println!("  - {} [{}]", m.name, m.crates_io);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::tempdir;

    fn write(path: &Path, content: &str) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let mut f = std::fs::File::create(path).unwrap();
        f.write_all(content.as_bytes()).unwrap();
    }

    #[test]
    fn scan_used_crates_detects_external_and_filters_reserved_and_local() {
        let dir = tempdir().unwrap();
        // Cargo.toml so crate root is this dir
        write(
            &dir.path().join("Cargo.toml"),
            "[package]\nname='tmp'\nversion='0.1.0'\nedition='2021'\n",
        );
        // Local module 'internal' should be filtered out
        write(&dir.path().join("src/internal.rs"), "pub fn f(){}\n");
        // Use statements include std/self/super/alloc which must be filtered, and external crates
        let lib_rs = r#"
            use std::fs;
            use self::something;
            use super::other;
            use alloc::vec::Vec;
            use serde::Serialize;
            use tokio::io;
            use foo::{bar, baz};
            use crate::internal;
        "#;
        write(&dir.path().join("src/lib.rs"), lib_rs);

        let used = scan_used_crates(dir.path()).unwrap();
        // Should include external roots
        assert!(used.contains("serde"));
        assert!(used.contains("tokio"));
        assert!(used.contains("foo"));
        // Should not include reserved or local module
        assert!(!used.contains("std"));
        assert!(!used.contains("alloc"));
        assert!(!used.contains("internal"));
    }

    #[test]
    fn parse_declared_dependencies_collects_sets_and_renames() {
        let dir = tempdir().unwrap();
        let cargo = r#"
            [package]
            name = "tmp"
            version = "0.1.0"
            edition = "2021"

            [dependencies]
            serde = "1"
            tokio = { version = "1" }
            foo = { package = "bar", version = "1" }

            [dev-dependencies]
            serde_json = "1"

            [build-dependencies]
            cc = "*"
        "#;
        write(&dir.path().join("Cargo.toml"), cargo);
        let declared = parse_declared_dependencies(dir.path()).unwrap();

        assert!(declared.normal.contains("serde"));
        assert!(declared.normal.contains("tokio"));
        assert!(declared.normal.contains("foo"));
        assert_eq!(declared.renamed.get("foo").map(|s| s.as_str()), Some("bar"));
        assert!(declared.dev.contains("serde_json"));
        assert!(declared.build.contains("cc"));
    }

    #[test]
    fn missing_deps_computation_works_without_network() {
        let dir = tempdir().unwrap();
        // Declare only serde; tokio should appear as missing if used
        let cargo = r#"
            [package]
            name = "tmp"
            version = "0.1.0"
            edition = "2021"

            [dependencies]
            serde = "1"
        "#;
        write(&dir.path().join("Cargo.toml"), cargo);
        let src = r#"
            use serde::Serialize;
            use tokio::io;
        "#;
        write(&dir.path().join("src/lib.rs"), src);

        let used = scan_used_crates(dir.path()).unwrap();
        let declared = parse_declared_dependencies(dir.path()).unwrap();
        let mut declared_union: BTreeSet<String> = declared
            .normal
            .union(&declared.dev)
            .cloned()
            .collect();
        declared_union.extend(declared.build.iter().cloned());
        let missing: BTreeSet<String> = used.difference(&declared_union).cloned().collect();

        assert!(missing.contains("tokio"));
        assert!(!missing.contains("serde"));
    }

    #[test]
    fn is_local_module_detects_src_layouts() {
        let dir = tempdir().unwrap();
        write(&dir.path().join("Cargo.toml"), "[package]\nname='x'\nversion='0.1.0'\n");
        // src/foo.rs
        write(&dir.path().join("src/foo.rs"), "");
        // src/bar/mod.rs
        write(&dir.path().join("src/bar/mod.rs"), "");
        assert!(is_local_module(dir.path(), "foo"));
        assert!(is_local_module(dir.path(), "bar"));
        assert!(!is_local_module(dir.path(), "baz"));
    }
}


