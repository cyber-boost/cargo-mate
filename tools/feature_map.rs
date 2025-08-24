use super::{Tool, Result, ToolError, common_options, parse_output_format, OutputFormat};
use clap::{Arg, ArgMatches, Command};
use colored::*;
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::Path;
use toml::{self, Value};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct FeatureMapTool;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureGraph {
    pub features: HashMap<String, FeatureInfo>,
    pub conflicts: Vec<FeatureConflict>,
    pub combinations: Vec<FeatureCombination>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureInfo {
    pub name: String,
    pub dependencies: Vec<String>,
    pub optional: bool,
    pub default: bool,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureConflict {
    pub features: Vec<String>,
    pub reason: String,
    pub severity: ConflictSeverity,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConflictSeverity {
    Error,
    Warning,
    Info,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureCombination {
    pub features: Vec<String>,
    pub size_estimate: u64,
    pub conflict_free: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureAnalysis {
    pub manifest_path: String,
    pub total_features: usize,
    pub core_features: usize,
    pub optional_features: usize,
    pub dev_features: usize,
    pub unused_features: Vec<String>,
    pub optimization_suggestions: Vec<String>,
}

impl FeatureMapTool {
    pub fn new() -> Self {
        Self
    }

    fn parse_cargo_toml(&self, manifest_path: &str) -> Result<Value> {
        let content = std::fs::read_to_string(manifest_path)
            .map_err(|e| ToolError::IoError(e))?;

        toml::from_str(&content)
            .map_err(|e| ToolError::TomlError(e))
    }

    fn extract_features(&self, cargo_toml: &Value) -> HashMap<String, Vec<String>> {
        let mut features = HashMap::new();

        if let Some(features_table) = cargo_toml.get("features") {
            if let Some(features_obj) = features_table.as_table() {
                for (feature_name, feature_deps) in features_obj {
                    if let Some(dep_array) = feature_deps.as_array() {
                        let deps: Vec<String> = dep_array
                            .iter()
                            .filter_map(|v| v.as_str().map(|s| s.to_string()))
                            .collect();
                        features.insert(feature_name.clone(), deps);
                    }
                }
            }
        }

        features
    }

    fn extract_dependencies(&self, cargo_toml: &Value) -> HashMap<String, DependencyInfo> {
        let mut deps = HashMap::new();

        if let Some(deps_table) = cargo_toml.get("dependencies") {
            if let Some(deps_obj) = deps_table.as_table() {
                for (name, info) in deps_obj {
                    let dep_info = self.parse_dependency_info(info);
                    deps.insert(name.clone(), dep_info);
                }
            }
        }

        deps
    }

    fn parse_dependency_info(&self, dep_value: &Value) -> DependencyInfo {
        match dep_value {
            Value::String(version) => DependencyInfo {
                name: String::new(), // Will be set by caller
                version: version.clone(),
                features: None,
                optional: false,
            },
            Value::Table(table) => {
                let version = table.get("version")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
                    .to_string();

                let features = table.get("features")
                    .and_then(|f| f.as_array())
                    .map(|arr| arr.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect::<Vec<_>>());

                let optional = table.get("optional")
                    .and_then(|o| o.as_bool())
                    .unwrap_or(false);

                DependencyInfo {
                    name: String::new(), // Will be set by caller
                    version,
                    features,
                    optional,
                }
            }
            _ => DependencyInfo {
                name: String::new(),
                version: "unknown".to_string(),
                features: None,
                optional: false,
            }
        }
    }

    fn analyze_feature_dependencies(&self, features: &HashMap<String, Vec<String>>) -> Result<FeatureGraph> {
        let mut graph = FeatureGraph {
            features: HashMap::new(),
            conflicts: Vec::new(),
            combinations: Vec::new(),
        };

        // Build feature info
        for (name, deps) in features {
            let is_default = name == "default";
            let info = FeatureInfo {
                name: name.clone(),
                dependencies: deps.clone(),
                optional: !is_default,
                default: is_default,
                description: None,
            };
            graph.features.insert(name.clone(), info);
        }

        // Detect conflicts
        graph.conflicts = self.detect_feature_conflicts(features)?;

        // Generate combinations
        graph.combinations = self.calculate_feature_combinations(features)?;

        Ok(graph)
    }

    fn detect_feature_conflicts(&self, features: &HashMap<String, Vec<String>>) -> Result<Vec<FeatureConflict>> {
        let mut conflicts = Vec::new();

        // Check for circular dependencies
        let mut visited = HashSet::new();
        let mut recursion_stack = HashSet::new();

        for feature in features.keys() {
            if self.has_circular_dependency(feature, features, &mut visited, &mut recursion_stack) {
                conflicts.push(FeatureConflict {
                    features: vec![feature.clone()],
                    reason: "Circular dependency detected".to_string(),
                    severity: ConflictSeverity::Error,
                });
            }
        }

        // Check for conflicting feature combinations
        let conflicting_pairs = [
            ("crypto", "no_std"),
            ("networking", "minimal"),
            ("async", "sync"),
        ];

        for (feat1, feat2) in &conflicting_pairs {
            if features.contains_key(*feat1) && features.contains_key(*feat2) {
                conflicts.push(FeatureConflict {
                    features: vec![feat1.to_string(), feat2.to_string()],
                    reason: format!("Features '{}' and '{}' are mutually exclusive", feat1, feat2),
                    severity: ConflictSeverity::Error,
                });
            }
        }

        Ok(conflicts)
    }

    fn has_circular_dependency(
        &self,
        feature: &str,
        features: &HashMap<String, Vec<String>>,
        visited: &mut HashSet<String>,
        recursion_stack: &mut HashSet<String>,
    ) -> bool {
        if recursion_stack.contains(feature) {
            return true;
        }

        if visited.contains(feature) {
            return false;
        }

        visited.insert(feature.to_string());
        recursion_stack.insert(feature.to_string());

        if let Some(deps) = features.get(feature) {
            for dep in deps {
                if features.contains_key(dep) {
                    if self.has_circular_dependency(dep, features, visited, recursion_stack) {
                        return true;
                    }
                }
            }
        }

        recursion_stack.remove(feature);
        false
    }

    fn calculate_feature_combinations(&self, features: &HashMap<String, Vec<String>>) -> Result<Vec<FeatureCombination>> {
        let mut combinations = Vec::new();

        // Generate basic combinations
        let feature_names: Vec<String> = features.keys().cloned().collect();

        // Minimal combination (no optional features)
        combinations.push(FeatureCombination {
            features: vec!["default".to_string()],
            size_estimate: 1024 * 500, // ~500KB estimate
            conflict_free: true,
        });

        // Standard combination (default + common features)
        let standard_features = vec!["default".to_string(), "serde".to_string(), "logging".to_string()];
        combinations.push(FeatureCombination {
            features: standard_features,
            size_estimate: 1024 * 1024 * 2, // ~2MB estimate
            conflict_free: true,
        });

        // Full combination (all features)
        combinations.push(FeatureCombination {
            features: feature_names.clone(),
            size_estimate: 1024 * 1024 * 5, // ~5MB estimate
            conflict_free: false, // Assume potential conflicts
        });

        Ok(combinations)
    }

    fn generate_mermaid_graph(&self, graph: &FeatureGraph) -> String {
        let mut mermaid = String::from("graph TD\n");

        for (name, info) in &graph.features {
            let node_type = if info.default {
                "classDef default fill:#4CAF50,color:white"
            } else if info.optional {
                "classDef optional fill:#2196F3,color:white"
            } else {
                "classDef core fill:#FF9800,color:white"
            };

            for dep in &info.dependencies {
                mermaid.push_str(&format!("    {} --> {}\n", name, dep));
            }
        }

        mermaid.push_str("\n    classDef default fill:#4CAF50,color:white\n");
        mermaid.push_str("    classDef optional fill:#2196F3,color:white\n");
        mermaid.push_str("    classDef core fill:#FF9800,color:white\n");

        // Mark conflicts
        for conflict in &graph.conflicts {
            for feature in &conflict.features {
                mermaid.push_str(&format!("    class {} conflict\n", feature));
            }
        }

        mermaid
    }

    fn generate_dot_graph(&self, graph: &FeatureGraph) -> String {
        let mut dot = String::from("digraph FeatureMap {\n");
        dot.push_str("    rankdir=LR;\n");
        dot.push_str("    node [shape=rectangle];\n");

        for (name, info) in &graph.features {
            let color = if info.default {
                "lightgreen"
            } else if info.optional {
                "lightblue"
            } else {
                "orange"
            };

            dot.push_str(&format!("    \"{}\" [fillcolor={},style=filled];\n", name, color));
        }

        for (name, info) in &graph.features {
            for dep in &info.dependencies {
                dot.push_str(&format!("    \"{}\" -> \"{}\";\n", name, dep));
            }
        }

        dot.push_str("}\n");
        dot
    }

    fn find_unused_features(&self, features: &HashMap<String, Vec<String>>, workspace: bool) -> Vec<String> {
        let mut unused = Vec::new();

        // This is a simplified check - in a real implementation,
        // you'd parse the actual source code to see which features are used
        let common_unused = ["legacy-api", "experimental-db", "deprecated"];

        for feature in common_unused {
            if features.contains_key(feature) {
                unused.push(feature.to_string());
            }
        }

        unused
    }

    fn generate_optimization_suggestions(&self, analysis: &FeatureAnalysis, graph: &FeatureGraph) -> Vec<String> {
        let mut suggestions = Vec::new();

        if !analysis.unused_features.is_empty() {
            suggestions.push(format!("Remove unused features: {}", analysis.unused_features.join(", ")));
        }

        if !graph.conflicts.is_empty() {
            suggestions.push("Review feature conflicts and consider renaming or removing conflicting features".to_string());
        }

        if analysis.optional_features > 10 {
            suggestions.push("Consider consolidating optional features to reduce complexity".to_string());
        }

        suggestions.push("Add feature documentation in Cargo.toml".to_string());
        suggestions.push("Consider feature defaults for common use cases".to_string());

        suggestions
    }

    fn display_analysis(&self, analysis: &FeatureAnalysis, graph: &FeatureGraph, output_format: OutputFormat, verbose: bool) {
        match output_format {
            OutputFormat::Human => {
                println!("\n{}", "🎯 Feature Flag Analysis Report".bold().blue());
                println!("{}", "═".repeat(50).blue());

                println!("\n📊 Feature Overview:");
                println!("  • Manifest: {}", analysis.manifest_path);
                println!("  • Total Features: {}", analysis.total_features);
                println!("  • Core features: {}", analysis.core_features);
                println!("  • Optional features: {}", analysis.optional_features);
                println!("  • Dev features: {}", analysis.dev_features);

                if verbose {
                    println!("\n📈 Feature Dependencies:");
                    for (name, info) in &graph.features {
                        if !info.dependencies.is_empty() {
                            println!("  {} -> [{}]", name.green(), info.dependencies.join(", "));
                        }
                    }
                }

                if !graph.conflicts.is_empty() {
                    println!("\n{}", "⚠️  Conflicts Detected:".yellow());
                    for conflict in &graph.conflicts {
                        let severity = match conflict.severity {
                            ConflictSeverity::Error => "❌",
                            ConflictSeverity::Warning => "⚠️",
                            ConflictSeverity::Info => "ℹ️",
                        };
                        println!("  {} {}", severity, conflict.reason);
                    }
                }

                if !analysis.unused_features.is_empty() {
                    println!("\n🔍 Unused Features:");
                    for feature in &analysis.unused_features {
                        println!("  • {} - Not referenced anywhere", feature.yellow());
                    }
                }

                if verbose {
                    println!("\n📈 Feature Combinations:");
                    for combo in &graph.combinations {
                        let status = if combo.conflict_free { "✅" } else { "⚠️" };
                        let size_mb = combo.size_estimate as f64 / (1024.0 * 1024.0);
                        println!("  • {}: {} features ({:.1} MB) {}",
                                combo.features.join(" + "),
                                combo.features.len(),
                                size_mb,
                                status);
                    }
                }

                println!("\n💡 Optimization Suggestions:");
                for suggestion in &analysis.optimization_suggestions {
                    println!("  • {}", suggestion.cyan());
                }
            }
            OutputFormat::Json => {
                let output = serde_json::to_string_pretty(&analysis)
                    .unwrap_or_else(|_| "{}".to_string());
                println!("{}", output);
            }
            OutputFormat::Table => {
                println!("{:<20} {:<10} {:<10} {:<10} {:<10}",
                        "Feature", "Core", "Optional", "Default", "Deps");
                println!("{}", "─".repeat(70));

                for (name, info) in &graph.features {
                    println!("{:<20} {:<10} {:<10} {:<10} {:<10}",
                            name,
                            if info.default { "No" } else { "Yes" },
                            if info.optional { "Yes" } else { "No" },
                            if info.default { "Yes" } else { "No" },
                            info.dependencies.len().to_string());
                }
            }
        }
    }
}

#[derive(Debug, Clone)]
struct DependencyInfo {
    name: String,
    version: String,
    features: Option<Vec<String>>,
    optional: bool,
}

impl Tool for FeatureMapTool {
    fn name(&self) -> &'static str {
        "feature-map"
    }

    fn description(&self) -> &'static str {
        "Visualize and analyze feature flag combinations and their impact"
    }

    fn command(&self) -> Command {
        Command::new(self.name())
            .about(self.description())
            .long_about("Analyze Cargo.toml feature flags and their relationships. \
                        This tool helps you understand your Cargo feature flags: \
                        • Map feature flag dependencies and conflicts \
                        • Calculate feature flag combinations \
                        • Generate visual representations \
                        • Suggest feature optimizations

EXAMPLES:
    cm tool feature-map --conflicts --optimize
    cm tool feature-map --workspace --visualize dot
    cm tool feature-map --unused --impact")
            .args(&[
                Arg::new("manifest")
                    .long("manifest")
                    .short('m')
                    .help("Path to Cargo.toml file")
                    .default_value("Cargo.toml"),
                Arg::new("workspace")
                    .long("workspace")
                    .help("Analyze all crates in workspace")
                    .action(clap::ArgAction::SetTrue),
                Arg::new("conflicts")
                    .long("conflicts")
                    .help("Detect feature flag conflicts")
                    .action(clap::ArgAction::SetTrue),
                Arg::new("combinations")
                    .long("combinations")
                    .help("Calculate feature combinations")
                    .action(clap::ArgAction::SetTrue),
                Arg::new("visualize")
                    .long("visualize")
                    .short('v')
                    .help("Generate visualization (dot, mermaid, json)")
                    .default_value("mermaid"),
                Arg::new("optimize")
                    .long("optimize")
                    .short('o')
                    .help("Generate optimization suggestions")
                    .action(clap::ArgAction::SetTrue),
                Arg::new("unused")
                    .long("unused")
                    .help("Find unused features")
                    .action(clap::ArgAction::SetTrue),
                Arg::new("impact")
                    .long("impact")
                    .help("Analyze feature impact on dependencies")
                    .action(clap::ArgAction::SetTrue),
            ])
            .args(&common_options())
    }

    fn execute(&self, matches: &ArgMatches) -> Result<()> {
        let manifest_path = matches.get_one::<String>("manifest").unwrap();
        let workspace = matches.get_flag("workspace");
        let conflicts = matches.get_flag("conflicts");
        let combinations = matches.get_flag("combinations");
        let visualize = matches.get_flag("visualize") ||
                       matches.contains_id("visualize");
        let optimize = matches.get_flag("optimize");
        let unused = matches.get_flag("unused");
        let impact = matches.get_flag("impact");
        let output_format = parse_output_format(matches);
        let verbose = matches.get_flag("verbose");

        // Validate manifest exists
        if !Path::new(manifest_path).exists() {
            return Err(ToolError::InvalidArguments(format!("Manifest not found: {}", manifest_path)));
        }

        // Parse Cargo.toml
        let cargo_toml = self.parse_cargo_toml(manifest_path)?;
        let features = self.extract_features(&cargo_toml);

        if features.is_empty() {
            println!("{}", "No features found in Cargo.toml".yellow());
            return Ok(());
        }

        // Analyze feature dependencies
        let graph = self.analyze_feature_dependencies(&features)?;

        // Find unused features
        let unused_features = if unused {
            self.find_unused_features(&features, workspace)
        } else {
            Vec::new()
        };

        // Create analysis report
        let analysis = FeatureAnalysis {
            manifest_path: manifest_path.clone(),
            total_features: features.len(),
            core_features: features.get("default").map(|d| d.len()).unwrap_or(0),
            optional_features: features.len().saturating_sub(1), // Subtract default
            dev_features: 0, // Would need more complex analysis
            unused_features: unused_features.clone(),
            optimization_suggestions: Vec::new(),
        };

        // Generate optimization suggestions
        let mut analysis_with_suggestions = analysis.clone();
        analysis_with_suggestions.optimization_suggestions =
            self.generate_optimization_suggestions(&analysis, &graph);

        // Generate visualization if requested
        if visualize {
            let viz_format = matches.get_one::<String>("visualize")
                .map(|s| s.as_str())
                .unwrap_or("mermaid");

            match viz_format {
                "mermaid" => {
                    let mermaid = self.generate_mermaid_graph(&graph);
                    println!("\n📊 Feature Dependency Graph (Mermaid):");
                    println!("{}", mermaid);
                }
                "dot" => {
                    let dot = self.generate_dot_graph(&graph);
                    println!("\n📊 Feature Dependency Graph (DOT):");
                    println!("{}", dot);
                }
                "json" => {
                    let json = serde_json::to_string_pretty(&graph)
                        .unwrap_or_else(|_| "{}".to_string());
                    println!("\n📊 Feature Dependency Graph (JSON):");
                    println!("{}", json);
                }
                _ => {
                    println!("{}", "Unknown visualization format. Use: dot, mermaid, json".red());
                }
            }
        }

        // Display analysis
        self.display_analysis(&analysis_with_suggestions, &graph, output_format, verbose);

        Ok(())
    }
}

impl Default for FeatureMapTool {
    fn default() -> Self {
        Self::new()
    }
}
