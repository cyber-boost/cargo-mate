use anyhow::{Context, Result};
use cargo_metadata::{DependencyKind, Metadata, MetadataCommand, Package, PackageId};
use colored::*;
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::visit::Dfs;
use petgraph::Direction;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use crate::captain::license;
#[derive(Debug, Clone)]
pub struct DependencyNode {
    pub name: String,
    pub version: String,
    pub source: Option<String>,
    pub features: Vec<String>,
    pub size_bytes: Option<u64>,
    pub license: Option<String>,
    pub is_dev: bool,
    pub is_build: bool,
    pub depth: usize,
}
pub struct TreasureMap {
    graph: DiGraph<DependencyNode, DependencyKind>,
    node_map: HashMap<PackageId, NodeIndex>,
    metadata: Metadata,
    root_package: Option<Package>,
}
impl TreasureMap {
    pub fn new() -> Result<Self> {
        let metadata = MetadataCommand::new()
            .exec()
            .context("Failed to get cargo metadata")?;
        let root_package = metadata.root_package().cloned();
        let mut map = Self {
            graph: DiGraph::new(),
            node_map: HashMap::new(),
            metadata: metadata.clone(),
            root_package,
        };
        map.build_graph()?;
        Ok(map)
    }
    fn build_graph(&mut self) -> Result<()> {
        for package in &self.metadata.packages {
            let node = DependencyNode {
                name: package.name.clone(),
                version: package.version.to_string(),
                source: package.source.as_ref().map(|s| s.to_string()),
                features: package.features.keys().cloned().collect(),
                size_bytes: self.estimate_package_size(package),
                license: package.license.clone(),
                is_dev: false,
                is_build: false,
                depth: 0,
            };
            let idx = self.graph.add_node(node);
            self.node_map.insert(package.id.clone(), idx);
        }
        for package in &self.metadata.packages {
            let from_idx = *self.node_map.get(&package.id).unwrap();
            for dep in &package.dependencies {
                if let Some(target_package) = self.find_package(&dep.name, &dep.req) {
                    let to_idx = *self.node_map.get(&target_package.id).unwrap();
                    self.graph.add_edge(from_idx, to_idx, dep.kind);
                    if dep.kind == DependencyKind::Development {
                        self.graph[to_idx].is_dev = true;
                    }
                    if dep.kind == DependencyKind::Build {
                        self.graph[to_idx].is_build = true;
                    }
                }
            }
        }
        self.calculate_depths();
        Ok(())
    }
    fn find_package(&self, name: &str, _req: &semver::VersionReq) -> Option<&Package> {
        self.metadata.packages.iter().find(|p| p.name == name)
    }
    fn estimate_package_size(&self, package: &Package) -> Option<u64> {
        let mut size = 0u64;
        for target in &package.targets {
            if let Ok(metadata) = fs::metadata(&target.src_path) {
                size += metadata.len();
            }
        }
        if size > 0 { Some(size) } else { None }
    }
    fn calculate_depths(&mut self) {
        if let Some(ref root) = self.root_package {
            if let Some(&root_idx) = self.node_map.get(&root.id) {
                let mut visited = HashSet::new();
                self.calculate_depth_recursive(root_idx, 0, &mut visited);
            }
        }
    }
    fn calculate_depth_recursive(
        &mut self,
        node: NodeIndex,
        depth: usize,
        visited: &mut HashSet<NodeIndex>,
    ) {
        if visited.contains(&node) {
            return;
        }
        visited.insert(node);
        self.graph[node].depth = depth;
        let neighbors: Vec<NodeIndex> = self.graph.neighbors(node).collect();
        for neighbor in neighbors {
            self.calculate_depth_recursive(neighbor, depth + 1, visited);
        }
    }
    pub fn show_map(&self) {
        println!("{}", "🗺️  Treasure Map - Dependency Visualization".blue().bold());
        println!();
        if let Some(ref root) = self.root_package {
            if let Some(&root_idx) = self.node_map.get(&root.id) {
                self.print_tree(root_idx, "", true, &mut HashSet::new());
            }
        }
    }
    fn print_tree(
        &self,
        node: NodeIndex,
        prefix: &str,
        is_last: bool,
        visited: &mut HashSet<NodeIndex>,
    ) {
        let dep = &self.graph[node];
        let icon = self.get_node_icon(dep);
        let color = self.get_node_color(dep);
        let node_str = format!("{} {} v{}", icon, dep.name, dep.version);
        let colored_str = match color {
            NodeColor::Green => node_str.green(),
            NodeColor::Yellow => node_str.yellow(),
            NodeColor::Red => node_str.red(),
            NodeColor::Blue => node_str.blue(),
            NodeColor::Gray => node_str.dimmed(),
        };
        let display_str = if visited.contains(&node) {
            format!("{} [circular]", colored_str)
        } else {
            colored_str.to_string()
        };
        println!(
            "{}{}{}", prefix, if is_last { "└── " } else { "├── " },
            display_str
        );
        if visited.contains(&node) {
            return;
        }
        visited.insert(node);
        let mut children: Vec<NodeIndex> = self.graph.neighbors(node).collect();
        children.sort_by_key(|&idx| &self.graph[idx].name);
        for (i, child) in children.iter().enumerate() {
            let is_last_child = i == children.len() - 1;
            let new_prefix = format!(
                "{}{}", prefix, if is_last { "    " } else { "│   " }
            );
            self.print_tree(*child, &new_prefix, is_last_child, visited);
        }
    }
    fn get_node_icon(&self, node: &DependencyNode) -> &str {
        if node.source.is_none() {
            "📦"
        } else if node.is_dev {
            "🔧"
        } else if node.is_build {
            "🔨"
        } else {
            "📚"
        }
    }
    fn get_node_color(&self, node: &DependencyNode) -> NodeColor {
        if node.source.is_none() {
            NodeColor::Green
        } else if node.is_dev {
            NodeColor::Blue
        } else if node.depth > 3 {
            NodeColor::Gray
        } else {
            NodeColor::Yellow
        }
    }
    pub fn analyze(&self) -> DependencyAnalysis {
        let total_dependencies = self.graph.node_count();
        let direct_dependencies = self.count_direct_dependencies();
        let dev_dependencies = self.graph.node_weights().filter(|n| n.is_dev).count();
        let max_depth = self.graph.node_weights().map(|n| n.depth).max().unwrap_or(0);
        let duplicate_deps = self.find_duplicate_dependencies();
        let circular_deps = self.find_circular_dependencies();
        let outdated_deps = self.check_outdated_dependencies();
        let security_issues = self.check_security_issues();
        let total_size = self.graph.node_weights().filter_map(|n| n.size_bytes).sum();
        let largest_deps = self.find_largest_dependencies(10);
        DependencyAnalysis {
            total_dependencies,
            direct_dependencies,
            dev_dependencies,
            max_depth,
            duplicate_deps,
            circular_deps,
            outdated_deps,
            security_issues,
            total_size,
            largest_deps,
        }
    }
    fn count_direct_dependencies(&self) -> usize {
        if let Some(ref root) = self.root_package {
            if let Some(&root_idx) = self.node_map.get(&root.id) {
                return self.graph.neighbors(root_idx).count();
            }
        }
        0
    }
    fn find_duplicate_dependencies(&self) -> Vec<DuplicateDependency> {
        let mut deps_by_name: HashMap<String, Vec<(String, NodeIndex)>> = HashMap::new();
        for idx in self.graph.node_indices() {
            let node = &self.graph[idx];
            deps_by_name
                .entry(node.name.clone())
                .or_default()
                .push((node.version.clone(), idx));
        }
        let mut duplicates = Vec::new();
        for (name, versions) in deps_by_name {
            if versions.len() > 1 {
                duplicates
                    .push(DuplicateDependency {
                        name,
                        versions: versions.into_iter().map(|(v, _)| v).collect(),
                    });
            }
        }
        duplicates
    }
    fn find_circular_dependencies(&self) -> Vec<Vec<String>> {
        let mut cycles = Vec::new();
        let mut visited = HashSet::new();
        let mut stack = Vec::new();
        for idx in self.graph.node_indices() {
            if !visited.contains(&idx) {
                self.dfs_cycles(idx, &mut visited, &mut stack, &mut cycles);
            }
        }
        cycles
    }
    fn dfs_cycles(
        &self,
        node: NodeIndex,
        visited: &mut HashSet<NodeIndex>,
        stack: &mut Vec<NodeIndex>,
        cycles: &mut Vec<Vec<String>>,
    ) {
        visited.insert(node);
        stack.push(node);
        for neighbor in self.graph.neighbors(node) {
            if let Some(pos) = stack.iter().position(|&n| n == neighbor) {
                let cycle: Vec<String> = stack[pos..]
                    .iter()
                    .map(|&idx| self.graph[idx].name.clone())
                    .collect();
                cycles.push(cycle);
            } else if !visited.contains(&neighbor) {
                self.dfs_cycles(neighbor, visited, stack, cycles);
            }
        }
        stack.pop();
    }
    fn check_outdated_dependencies(&self) -> Vec<OutdatedDependency> {
        let output = Command::new("cargo")
            .args(&["outdated", "--format", "json"])
            .output();
        match output {
            Ok(output) if output.status.success() => Vec::new(),
            _ => Vec::new(),
        }
    }
    fn check_security_issues(&self) -> Vec<SecurityIssue> {
        let output = Command::new("cargo").args(&["audit", "--json"]).output();
        match output {
            Ok(output) if output.status.success() => Vec::new(),
            _ => Vec::new(),
        }
    }
    fn find_largest_dependencies(&self, limit: usize) -> Vec<(String, u64)> {
        let mut deps_with_size: Vec<(String, u64)> = self
            .graph
            .node_weights()
            .filter_map(|n| {
                n.size_bytes.map(|s| (format!("{} v{}", n.name, n.version), s))
            })
            .collect();
        deps_with_size.sort_by(|a, b| b.1.cmp(&a.1));
        deps_with_size.truncate(limit);
        deps_with_size
    }
    pub fn export_dot(&self, path: &PathBuf) -> Result<()> {
        let mut dot = String::new();
        dot.push_str("digraph dependencies {\n");
        dot.push_str("    rankdir=LR;\n");
        dot.push_str("    node [shape=box];\n\n");
        for idx in self.graph.node_indices() {
            let node = &self.graph[idx];
            let color = match self.get_node_color(node) {
                NodeColor::Green => "green",
                NodeColor::Yellow => "yellow",
                NodeColor::Red => "red",
                NodeColor::Blue => "blue",
                NodeColor::Gray => "gray",
            };
            dot.push_str(
                &format!(
                    "    \"{}\" [label=\"{}\\nv{}\", color=\"{}\"];\n", node.name, node
                    .name, node.version, color
                ),
            );
        }
        for edge in self.graph.edge_indices() {
            let (from, to) = self.graph.edge_endpoints(edge).unwrap();
            let from_name = &self.graph[from].name;
            let to_name = &self.graph[to].name;
            let style = match self.graph[edge] {
                DependencyKind::Development => "dashed",
                DependencyKind::Build => "dotted",
                _ => "solid",
            };
            dot.push_str(
                &format!(
                    "    \"{}\" -> \"{}\" [style=\"{}\"];\n", from_name, to_name, style
                ),
            );
        }
        dot.push_str("}\n");
        fs::write(path, dot)?;
        println!("✅ Dependency graph exported to {}", path.display());
        Ok(())
    }
    pub fn find_path(&self, from: &str, to: &str) -> Option<Vec<String>> {
        let from_idx = self
            .graph
            .node_indices()
            .find(|&idx| self.graph[idx].name == from)?;
        let to_idx = self.graph.node_indices().find(|&idx| self.graph[idx].name == to)?;
        let path = petgraph::algo::astar(
            &self.graph,
            from_idx,
            |n| n == to_idx,
            |_| 1,
            |_| 0,
        );
        path.map(|(_, p)| {
            p.into_iter().map(|idx| self.graph[idx].name.clone()).collect()
        })
    }
    pub fn find_unused(&self) -> Vec<String> {
        let output = Command::new("cargo").args(&["machete"]).output();
        match output {
            Ok(output) if output.status.success() => {
                String::from_utf8_lossy(&output.stdout)
                    .lines()
                    .filter(|l| !l.is_empty())
                    .map(|l| l.to_string())
                    .collect()
            }
            _ => Vec::new(),
        }
    }
}
#[derive(Debug)]
pub struct DependencyAnalysis {
    pub total_dependencies: usize,
    pub direct_dependencies: usize,
    pub dev_dependencies: usize,
    pub max_depth: usize,
    pub duplicate_deps: Vec<DuplicateDependency>,
    pub circular_deps: Vec<Vec<String>>,
    pub outdated_deps: Vec<OutdatedDependency>,
    pub security_issues: Vec<SecurityIssue>,
    pub total_size: u64,
    pub largest_deps: Vec<(String, u64)>,
}
impl DependencyAnalysis {
    pub fn display(&self) {
        println!("{}", "=== Dependency Analysis ===".blue().bold());
        println!("📊 Total dependencies: {}", self.total_dependencies);
        println!("   Direct: {}", self.direct_dependencies);
        println!("   Dev: {}", self.dev_dependencies);
        println!("   Max depth: {}", self.max_depth);
        if self.total_size > 0 {
            println!("💾 Total size: {}", format_size(self.total_size));
        }
        if !self.duplicate_deps.is_empty() {
            println!(
                "\n⚠️  {} duplicate dependencies found:", self.duplicate_deps.len()
                .to_string().yellow()
            );
            for dup in &self.duplicate_deps[..5.min(self.duplicate_deps.len())] {
                println!(
                    "   {} has versions: {}", dup.name.yellow(), dup.versions.join(", ")
                );
            }
        }
        if !self.circular_deps.is_empty() {
            println!(
                "\n🔄 {} circular dependencies found:", self.circular_deps.len()
                .to_string().red()
            );
            for cycle in &self.circular_deps[..3.min(self.circular_deps.len())] {
                println!("   {}", cycle.join(" → ").red());
            }
        }
        if !self.largest_deps.is_empty() {
            println!("\n📦 Largest dependencies:");
            for (name, size) in &self.largest_deps[..5.min(self.largest_deps.len())] {
                println!("   {} - {}", name, format_size(* size));
            }
        }
        if !self.security_issues.is_empty() {
            println!(
                "\n🚨 {} security issues found!", self.security_issues.len()
                .to_string().red().bold()
            );
            for issue in &self.security_issues[..3.min(self.security_issues.len())] {
                println!("   {} - {}", issue.package.red(), issue.advisory);
            }
        }
    }
}
#[derive(Debug)]
pub struct DuplicateDependency {
    pub name: String,
    pub versions: Vec<String>,
}
#[derive(Debug)]
pub struct OutdatedDependency {
    pub name: String,
    pub current_version: String,
    pub latest_version: String,
}
#[derive(Debug)]
pub struct SecurityIssue {
    pub package: String,
    pub advisory: String,
    pub severity: String,
}
#[derive(Debug)]
enum NodeColor {
    Green,
    Yellow,
    Red,
    Blue,
    Gray,
}
fn format_size(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB"];
    let mut size = bytes as f64;
    let mut unit_idx = 0;
    while size >= 1024.0 && unit_idx < UNITS.len() - 1 {
        size /= 1024.0;
        unit_idx += 1;
    }
    format!("{:.2} {}", size, UNITS[unit_idx])
}
pub fn check_bosun_quotas(command: &str) -> Result<bool> {
    println!("🔨 Bosun checking quotas for command '{}' - tally ho!", command.cyan());
    let license_manager = license::LicenseManager::new()?;
    match license_manager.enforce_license(command) {
        Ok(_) => {
            println!(
                "✅ Bosun reports: Command '{}' within quota limits!", command.green()
            );
            println!("   🔨 All tallies accounted for - ready to proceed!");
            Ok(true)
        }
        Err(e) => {
            if e.to_string().contains("limit") {
                println!("⚠️  Bosun warning: Command quota exceeded!");
                println!("   🔨 Requisition more at: https://cargo.do/checkout");
                println!("   🔨 Upgrade for unlimited command tallies");
            } else if e.to_string().contains("License not found") {
                println!("❌ Bosun emergency: No quota authorization!");
                println!("   🔨 Get requisition with 'cm register <key>'");
            } else {
                println!(
                    "❌ Bosun distress: Quota check failed: {}", e.to_string().red()
                );
                println!("   🔨 Secure the manifest - prepare for inspection!");
            }
            Ok(false)
        }
    }
}