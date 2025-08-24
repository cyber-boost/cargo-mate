use super::{Tool, ToolError, Result, OutputFormat, parse_output_format};
use clap::{Arg, ArgMatches, Command};
use std::path::Path;
use std::fs;
use std::collections::HashMap;
use colored::*;
use syn::{parse_file, visit::Visit, ItemFn, Lifetime, TypeReference, FnArg, ReturnType, GenericParam};
use quote::ToTokens;
use serde::{Serialize, Deserialize};

/// Lifetime information for a function
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionLifetimeInfo {
    pub name: String,
    pub lifetimes: Vec<String>,
    pub constraints: Vec<String>,
    pub input_lifetimes: Vec<String>,
    pub output_lifetimes: Vec<String>,
    pub issues: Vec<String>,
}

/// Node in the lifetime graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LifetimeNode {
    pub id: usize,
    pub name: String,
    pub node_type: LifetimeNodeType,
}

/// Edge in the lifetime graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LifetimeEdge {
    pub from: usize,
    pub to: usize,
    pub edge_type: LifetimeEdgeType,
}

/// Type of lifetime node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LifetimeNodeType {
    Function,
    Lifetime,
    Borrow,
}

/// Type of lifetime edge
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LifetimeEdgeType {
    Outlives,
    Borrows,
    References,
}

/// The complete lifetime graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LifetimeGraph {
    pub nodes: Vec<LifetimeNode>,
    pub edges: Vec<LifetimeEdge>,
}

/// Lifetime issue detected
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LifetimeIssue {
    pub function: String,
    pub issue: String,
    pub severity: String,
    pub line: usize,
    pub suggestion: String,
}

/// Borrow analysis result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BorrowAnalysis {
    pub immutable_borrows: usize,
    pub mutable_borrows: usize,
    pub borrow_errors: usize,
    pub potential_race_conditions: usize,
}

/// Lifetime improvement suggestion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LifetimeSuggestion {
    pub category: String,
    pub description: String,
    pub before: String,
    pub after: String,
    pub impact: String,
}

pub struct LifetimeVisualizerTool;

impl LifetimeVisualizerTool {
    pub fn new() -> Self {
        Self
    }

    fn parse_lifetimes_in_file(&self, file_path: &str) -> Result<Vec<FunctionLifetimeInfo>> {
        if !Path::new(file_path).exists() {
            return Err(ToolError::InvalidArguments(format!("File not found: {}", file_path)));
        }

        let content = fs::read_to_string(file_path)?;
        let ast = parse_file(&content)
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to parse Rust file: {}", e)))?;

        let mut visitor = FunctionLifetimeVisitor::new();
        visitor.visit_file(&ast);

        Ok(visitor.functions)
    }

    fn build_lifetime_graph(&self, functions: &[FunctionLifetimeInfo]) -> Result<LifetimeGraph> {
        let mut nodes = Vec::new();
        let mut edges = Vec::new();
        let mut node_id = 0;

        // Add function nodes
        for function in functions {
            let function_id = node_id;
            nodes.push(LifetimeNode {
                id: function_id,
                name: function.name.clone(),
                node_type: LifetimeNodeType::Function,
            });
            node_id += 1;

            // Add lifetime nodes for this function
            for lifetime in &function.lifetimes {
                let lifetime_id = node_id;
                nodes.push(LifetimeNode {
                    id: lifetime_id,
                    name: lifetime.clone(),
                    node_type: LifetimeNodeType::Lifetime,
                });

                // Connect function to its lifetimes
                edges.push(LifetimeEdge {
                    from: function_id,
                    to: lifetime_id,
                    edge_type: LifetimeEdgeType::References,
                });
                node_id += 1;
            }
        }

        // Add relationships between lifetimes
        for function in functions {
            for constraint in &function.constraints {
                if let Some((from_lifetime, to_lifetime)) = self.parse_constraint(constraint) {
                    let from_id = nodes.iter().find(|n| n.name == from_lifetime).map(|n| n.id);
                    let to_id = nodes.iter().find(|n| n.name == to_lifetime).map(|n| n.id);

                    if let (Some(from), Some(to)) = (from_id, to_id) {
                        edges.push(LifetimeEdge {
                            from,
                            to,
                            edge_type: LifetimeEdgeType::Outlives,
                        });
                    }
                }
            }
        }

        Ok(LifetimeGraph { nodes, edges })
    }

    fn parse_constraint(&self, constraint: &str) -> Option<(String, String)> {
        // Parse constraints like "'a: 'b" or "'k: 'v"
        let parts: Vec<&str> = constraint.split(':').map(|s| s.trim().trim_matches('\'')).collect();
        if parts.len() == 2 {
            Some((format!("'{}", parts[0]), format!("'{}", parts[1])))
        } else {
            None
        }
    }

    fn detect_lifetime_issues(&self, functions: &[FunctionLifetimeInfo]) -> Vec<LifetimeIssue> {
        let mut issues = Vec::new();

        for function in functions {
            // Check for unconstrained lifetime relationships
            if function.lifetimes.len() > 1 && function.constraints.is_empty() {
                issues.push(LifetimeIssue {
                    function: function.name.clone(),
                    issue: "Multiple lifetimes without explicit constraints".to_string(),
                    severity: "Medium".to_string(),
                    line: 0, // Would need span information for real implementation
                    suggestion: format!("Add lifetime constraints like {}: {}", function.lifetimes[0], function.lifetimes[1]),
                });
            }

            // Check for unconstrained return lifetimes
            if function.output_lifetimes.len() > 0 {
                let unconstrained_outputs: Vec<_> = function.output_lifetimes.iter()
                    .filter(|lt| !function.input_lifetimes.contains(lt))
                    .collect();

                if !unconstrained_outputs.is_empty() {
                    issues.push(LifetimeIssue {
                        function: function.name.clone(),
                        issue: "Return lifetime not tied to input lifetime".to_string(),
                        severity: "High".to_string(),
                        line: 0,
                        suggestion: "Use HRTB (for<'a>) or tie return lifetime to input".to_string(),
                    });
                }
            }

            // Check for elision opportunities
            if function.lifetimes.len() == 1 &&
               function.input_lifetimes.len() == 1 &&
               function.output_lifetimes.len() == 1 &&
               function.input_lifetimes[0] == function.output_lifetimes[0] {
                issues.push(LifetimeIssue {
                    function: function.name.clone(),
                    issue: "Unnecessary explicit lifetime - elision would work".to_string(),
                    severity: "Low".to_string(),
                    line: 0,
                    suggestion: "Remove explicit lifetimes and let the compiler elide them".to_string(),
                });
            }
        }

        issues
    }

    fn generate_visualization(&self, graph: &LifetimeGraph, format: &str) -> Result<String> {
        match format {
            "mermaid" => self.generate_mermaid_diagram(graph),
            "dot" => self.generate_dot_diagram(graph),
            "graphviz" => self.generate_graphviz_diagram(graph),
            _ => self.generate_mermaid_diagram(graph),
        }
    }

    fn generate_mermaid_diagram(&self, graph: &LifetimeGraph) -> Result<String> {
        let mut diagram = String::from("graph TD\n");

        // Add nodes with styling
        for node in &graph.nodes {
            let style = match node.node_type {
                LifetimeNodeType::Function => format!("📦 {}", node.name),
                LifetimeNodeType::Lifetime => format!("🔗 {}", node.name),
                LifetimeNodeType::Borrow => format!("📍 {}", node.name),
            };

            let node_id = format!("N{}", node.id);
            diagram.push_str(&format!("    {}[\"{}\"]\n", node_id, style));
        }

        // Add edges with styling
        for edge in &graph.edges {
            let from_id = format!("N{}", edge.from);
            let to_id = format!("N{}", edge.to);
            let style = match edge.edge_type {
                LifetimeEdgeType::Outlives => "-->",
                LifetimeEdgeType::Borrows => "-.->",
                LifetimeEdgeType::References => "==>",
            };

            diagram.push_str(&format!("    {}{} {}\n", from_id, style, to_id));
        }

        // Add styling
        diagram.push_str("\n    classDef function fill:#e1f5fe\n");
        diagram.push_str("    classDef lifetime fill:#f3e5f5\n");
        diagram.push_str("    classDef borrow fill:#e8f5e8\n");
        diagram.push_str("    classDef issue fill:#ffebee\n");
        diagram.push_str("    classDef warning fill:#fff3e0\n");
        diagram.push_str("    classDef good fill:#e8f5e8\n");

        Ok(diagram)
    }

    fn generate_dot_diagram(&self, graph: &LifetimeGraph) -> Result<String> {
        let mut diagram = String::from("digraph LifetimeGraph {\n");
        diagram.push_str("    rankdir=LR;\n");
        diagram.push_str("    node [shape=box];\n\n");

        // Add nodes
        for node in &graph.nodes {
            let shape = match node.node_type {
                LifetimeNodeType::Function => "box",
                LifetimeNodeType::Lifetime => "ellipse",
                LifetimeNodeType::Borrow => "diamond",
            };

            diagram.push_str(&format!("    N{} [label=\"{}\", shape={}];\n",
                node.id, node.name, shape));
        }

        diagram.push_str("\n");

        // Add edges
        for edge in &graph.edges {
            let style = match edge.edge_type {
                LifetimeEdgeType::Outlives => "color=blue",
                LifetimeEdgeType::Borrows => "color=green,style=dashed",
                LifetimeEdgeType::References => "color=red,style=bold",
            };

            diagram.push_str(&format!("    N{} -> N{} [{}];\n",
                edge.from, edge.to, style));
        }

        diagram.push_str("}\n");
        Ok(diagram)
    }

    fn generate_graphviz_diagram(&self, graph: &LifetimeGraph) -> Result<String> {
        self.generate_dot_diagram(graph) // GraphViz uses DOT format
    }

    fn analyze_borrow_patterns(&self, file_path: &str) -> Result<BorrowAnalysis> {
        let content = fs::read_to_string(file_path)?;

        let immutable_borrows = content.matches("& ").count() +
                               content.matches("&mut ").count();
        let mutable_borrows = content.matches("&mut ").count();
        let borrow_errors = content.matches("cannot borrow").count() +
                           content.matches("borrowed").count();

        // Simple heuristic for race conditions
        let potential_race_conditions = content.matches("Arc<Mutex<").count() +
                                      content.matches("Arc<RwLock<").count();

        Ok(BorrowAnalysis {
            immutable_borrows,
            mutable_borrows,
            borrow_errors,
            potential_race_conditions,
        })
    }

    fn suggest_lifetime_improvements(&self, issues: &[LifetimeIssue]) -> Vec<LifetimeSuggestion> {
        let mut suggestions = Vec::new();

        for issue in issues {
            match issue.severity.as_str() {
                "High" => {
                    if issue.issue.contains("Return lifetime not tied") {
                        suggestions.push(LifetimeSuggestion {
                            category: "HRTB".to_string(),
                            description: "Use Higher-Ranked Trait Bounds".to_string(),
                            before: "fn cache_get<'k>(key: &'k Key) -> Option<&'k Value>".to_string(),
                            after: "fn cache_get(key: &Key) -> Option<&Value>".to_string(),
                            impact: "High".to_string(),
                        });
                    }
                }
                "Medium" => {
                    if issue.issue.contains("Multiple lifetimes") {
                        suggestions.push(LifetimeSuggestion {
                            category: "Constraints".to_string(),
                            description: "Add explicit lifetime constraints".to_string(),
                            before: "fn process<'a, 'b>(data: &'a mut Vec<T>, config: &'b Config)".to_string(),
                            after: "fn process<'a, 'b: 'a>(data: &'a mut Vec<T>, config: &'b Config)".to_string(),
                            impact: "Medium".to_string(),
                        });
                    }
                }
                "Low" => {
                    if issue.issue.contains("elision") {
                        suggestions.push(LifetimeSuggestion {
                            category: "Elision".to_string(),
                            description: "Use lifetime elision".to_string(),
                            before: "fn get<'a>(&'a self) -> &'a str".to_string(),
                            after: "fn get(&self) -> &str".to_string(),
                            impact: "Low".to_string(),
                        });
                    }
                }
                _ => {}
            }
        }

        suggestions
    }
}

struct FunctionLifetimeVisitor {
    functions: Vec<FunctionLifetimeInfo>,
}

impl FunctionLifetimeVisitor {
    fn new() -> Self {
        Self {
            functions: Vec::new(),
        }
    }
}

impl<'ast> Visit<'ast> for FunctionLifetimeVisitor {
    fn visit_item_fn(&mut self, node: &'ast ItemFn) {
        let lifetime_info = self.extract_function_lifetimes(node);
        self.functions.push(lifetime_info);
        syn::visit::visit_item_fn(self, node);
    }
}

impl FunctionLifetimeVisitor {
    fn extract_function_lifetimes(&self, node: &ItemFn) -> FunctionLifetimeInfo {
        let mut lifetimes = Vec::new();
        let mut constraints = Vec::new();

        // Extract explicit lifetime parameters
        for param in &node.sig.generics.params {
            if let GenericParam::Lifetime(lifetime_def) = param {
                lifetimes.push(format!("'{}", lifetime_def.lifetime.ident));
            }
        }

        // Extract lifetime bounds
        for where_predicate in &node.sig.generics.where_clause {
            for predicate in &where_predicate.predicates {
                if let syn::WherePredicate::Lifetime(lifetime_pred) = predicate {
                    constraints.push(format!("{}: {}",
                        lifetime_pred.lifetime.to_token_stream(),
                        lifetime_pred.bounds.to_token_stream()));
                }
            }
        }

        // Extract input lifetime annotations
        let input_lifetimes = self.extract_input_lifetimes(&node.sig.inputs);

        // Extract output lifetime annotations
        let output_lifetimes = self.extract_output_lifetimes(&node.sig.output);

        FunctionLifetimeInfo {
            name: node.sig.ident.to_string(),
            lifetimes,
            constraints,
            input_lifetimes,
            output_lifetimes,
            issues: Vec::new(),
        }
    }

    fn extract_input_lifetimes(&self, inputs: &syn::punctuated::Punctuated<FnArg, syn::token::Comma>) -> Vec<String> {
        let mut lifetimes = Vec::new();

        for arg in inputs {
            match arg {
                FnArg::Receiver(receiver) => {
                    if let Some((_, lifetime)) = &receiver.reference {
                        if let Some(lt) = lifetime {
                            lifetimes.push(format!("'{}", lt.ident));
                        }
                    }
                }
                FnArg::Typed(pat_type) => {
                    self.extract_lifetimes_from_type(&pat_type.ty, &mut lifetimes);
                }
            }
        }

        lifetimes
    }

    fn extract_output_lifetimes(&self, output: &ReturnType) -> Vec<String> {
        let mut lifetimes = Vec::new();

        if let ReturnType::Type(_, ty) = output {
            self.extract_lifetimes_from_type(ty, &mut lifetimes);
        }

        lifetimes
    }

    fn extract_lifetimes_from_type(&self, ty: &syn::Type, lifetimes: &mut Vec<String>) {
        match ty {
            syn::Type::Reference(type_ref) => {
                if let Some(lifetime) = &type_ref.lifetime {
                    lifetimes.push(format!("'{}", lifetime.ident));
                }
                self.extract_lifetimes_from_type(&type_ref.elem, lifetimes);
            }
            syn::Type::Slice(slice) => {
                self.extract_lifetimes_from_type(&slice.elem, lifetimes);
            }
            syn::Type::Array(array) => {
                self.extract_lifetimes_from_type(&array.elem, lifetimes);
            }
            syn::Type::Tuple(tuple) => {
                for elem in &tuple.elems {
                    self.extract_lifetimes_from_type(elem, lifetimes);
                }
            }
            _ => {}
        }
    }
}

impl Tool for LifetimeVisualizerTool {
    fn name(&self) -> &'static str {
        "lifetime-visualizer"
    }

    fn description(&self) -> &'static str {
        "Visualize lifetime relationships in Rust code"
    }

    fn command(&self) -> Command {
        Command::new(self.name())
            .about(self.description())
            .long_about(
                "Visualize lifetime relationships in Rust code, helping developers understand borrowing and ownership patterns.\n\
                 \n\
                 This tool provides deep insights into lifetime patterns:\n\
                 • Analyze lifetime annotations in functions\n\
                 • Build lifetime dependency graphs\n\
                 • Detect potential lifetime issues\n\
                 • Generate visual lifetime flow diagrams\n\
                 \n\
                 EXAMPLES:\n\
                 cm tool lifetime-visualizer --input src/lib.rs --issues --suggest\n\
                 cm tool lifetime-visualizer --input src/main.rs --visualize --format mermaid\n\
                 cm tool lifetime-visualizer --input src/ --borrow-check"
            )
            .args(&[
                Arg::new("input")
                    .long("input")
                    .short('i')
                    .help("Input Rust file or directory to analyze")
                    .default_value("src/"),
                Arg::new("function")
                    .long("function")
                    .short('f')
                    .help("Specific function to analyze"),
                Arg::new("visualize")
                    .long("visualize")
                    .short('v')
                    .help("Generate lifetime visualization")
                    .action(clap::ArgAction::SetTrue),
                Arg::new("issues")
                    .long("issues")
                    .help("Detect lifetime issues")
                    .action(clap::ArgAction::SetTrue),
                Arg::new("suggest")
                    .long("suggest")
                    .help("Generate improvement suggestions")
                    .action(clap::ArgAction::SetTrue),
                Arg::new("format")
                    .long("format")
                    .help("Visualization format: mermaid, dot, graphviz")
                    .default_value("mermaid"),
                Arg::new("borrow-check")
                    .long("borrow-check")
                    .help("Analyze borrowing patterns")
                    .action(clap::ArgAction::SetTrue),
                Arg::new("interactive")
                    .long("interactive")
                    .help("Interactive lifetime exploration")
                    .action(clap::ArgAction::SetTrue),
                Arg::new("output")
                    .long("output")
                    .short('o')
                    .help("Output file for visualization")
                    .default_value("lifetimes.md"),
            ])
            .args(&super::common_options())
    }

    fn execute(&self, matches: &ArgMatches) -> Result<()> {
        let input = matches.get_one::<String>("input").unwrap();
        let specific_function = matches.get_one::<String>("function");
        let visualize = matches.get_flag("visualize");
        let detect_issues = matches.get_flag("issues");
        let suggest = matches.get_flag("suggest");
        let format = matches.get_one::<String>("format").unwrap();
        let borrow_check = matches.get_flag("borrow-check");
        let interactive = matches.get_flag("interactive");
        let output_file = matches.get_one::<String>("output").unwrap();
        let verbose = matches.get_flag("verbose");
        let dry_run = matches.get_flag("dry-run");
        let output_format = parse_output_format(matches);

        if dry_run {
            println!("🔍 Would analyze lifetimes in: {}", input);
            return Ok(());
        }

        // Collect all functions from input
        let functions = if Path::new(input).is_file() {
            self.parse_lifetimes_in_file(input)?
        } else {
            // Directory analysis
            let mut all_functions = Vec::new();
            let rust_files = self.find_rust_files(input)?;

            for file in rust_files {
                match self.parse_lifetimes_in_file(&file) {
                    Ok(mut functions) => all_functions.extend(functions),
                    Err(e) => {
                        if verbose {
                            println!("⚠️  Failed to parse {}: {}", file, e);
                        }
                    }
                }
            }
            all_functions
        };

        // Filter by specific function if requested
        let filtered_functions: Vec<_> = if let Some(func_name) = specific_function {
            functions.into_iter()
                .filter(|f| f.name == *func_name)
                .collect()
        } else {
            functions
        };

        if filtered_functions.is_empty() {
            if let Some(name) = specific_function {
                println!("❌ Function '{}' not found.", name.red());
            } else {
                println!("✅ No functions with explicit lifetimes found.");
            }
            return Ok(());
        }

        match output_format {
            OutputFormat::Human => {
                println!("🔗 {} - {}", "Lifetime Analysis Report".bold(), self.description().cyan());
                println!("\n📁 Files Analyzed: {}", input.bold());
                println!("🔍 Functions Found: {}", filtered_functions.len().to_string().cyan());

                // Summary statistics
                let total_lifetimes: usize = filtered_functions.iter().map(|f| f.lifetimes.len()).sum();
                let total_constraints: usize = filtered_functions.iter().map(|f| f.constraints.len()).sum();

                println!("\n📊 Lifetime Summary:");
                println!("• Explicit lifetimes: {}", total_lifetimes);
                println!("• Lifetime constraints: {}", total_constraints);
                println!("• Complex relationships: {}", total_constraints);

                // Detect issues
                let issues = if detect_issues {
                    self.detect_lifetime_issues(&filtered_functions)
                } else {
                    Vec::new()
                };

                if !issues.is_empty() {
                    println!("\n⚠️  Lifetime Issues Detected:");
                    for (i, issue) in issues.iter().enumerate() {
                        let severity_color = match issue.severity.as_str() {
                            "High" => issue.severity.red().bold(),
                            "Medium" => issue.severity.yellow().bold(),
                            "Low" => issue.severity.green().bold(),
                            _ => issue.severity.normal(),
                        };

                        println!("{}. Function: {}", i + 1, issue.function.bold());
                        println!("   [{}] {}", severity_color, issue.issue);
                        println!("   💡 {}", issue.suggestion.cyan());
                        println!();
                    }
                } else if detect_issues {
                    println!("\n✅ No lifetime issues detected!");
                }

                // Analyze borrowing patterns
                if borrow_check {
                    println!("\n📈 Borrow Analysis:");
                    for function in &filtered_functions {
                        if function.name.contains("process") || function.name.contains("cache") {
                            match self.analyze_borrow_patterns(input) {
                                Ok(analysis) => {
                                    println!("• Immutable borrows: {}", analysis.immutable_borrows);
                                    println!("• Mutable borrows: {}", analysis.mutable_borrows);
                                    println!("• Borrow checker errors: {}", analysis.borrow_errors);
                                    if analysis.potential_race_conditions > 0 {
                                        println!("• Potential race conditions: {}", analysis.potential_race_conditions.to_string().yellow());
                                    }
                                }
                                Err(e) => {
                                    if verbose {
                                        println!("  ⚠️  Borrow analysis failed: {}", e);
                                    }
                                }
                            }
                            break; // Just show for one file for now
                        }
                    }
                }

                // Generate suggestions
                if suggest && !issues.is_empty() {
                    let suggestions = self.suggest_lifetime_improvements(&issues);
                    if !suggestions.is_empty() {
                        println!("\n💡 Improvement Suggestions:");
                        for (i, suggestion) in suggestions.iter().enumerate() {
                            let impact_color = match suggestion.impact.as_str() {
                                "High" => suggestion.impact.red().bold(),
                                "Medium" => suggestion.impact.yellow().bold(),
                                "Low" => suggestion.impact.green().bold(),
                                _ => suggestion.impact.normal(),
                            };

                            println!("{}. [{}] {}", i + 1, impact_color, suggestion.description.bold());
                            println!("   Before: {}", suggestion.before.red());
                            println!("   After:  {}", suggestion.after.green());
                            println!("   Category: {}", suggestion.category.cyan());
                            println!();
                        }
                    }
                }

                // Generate visualization
                if visualize {
                    match self.build_lifetime_graph(&filtered_functions) {
                        Ok(graph) => {
                            match self.generate_visualization(&graph, format) {
                                Ok(visualization) => {
                                    println!("\n📊 Lifetime Visualization ({}):", format.bold());

                                    if format == "mermaid" {
                                        println!("```mermaid");
                                        println!("{}", visualization);
                                        println!("```");
                                    } else {
                                        println!("Generated {} diagram with {} nodes and {} edges",
                                            format, graph.nodes.len(), graph.edges.len());
                                        println!("First few lines:");
                                        for line in visualization.lines().take(10) {
                                            println!("  {}", line);
                                        }
                                    }

                                    // Write to file
                                    if let Err(e) = fs::write(output_file, &visualization) {
                                        if verbose {
                                            println!("⚠️  Could not write visualization: {}", e);
                                        }
                                    } else {
                                        println!("\n💾 Visualization saved to: {}", output_file);
                                    }
                                }
                                Err(e) => {
                                    if verbose {
                                        println!("⚠️  Could not generate visualization: {}", e);
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            if verbose {
                                println!("⚠️  Could not build lifetime graph: {}", e);
                            }
                        }
                    }
                }
            }
            OutputFormat::Json => {
                let issues = self.detect_lifetime_issues(&filtered_functions);
                let mut json_output = serde_json::json!({
                    "files_analyzed": input,
                    "functions_analyzed": filtered_functions.len(),
                    "total_lifetimes": filtered_functions.iter().map(|f| f.lifetimes.len()).sum::<usize>(),
                    "functions": filtered_functions,
                });

                if !issues.is_empty() {
                    json_output["issues"] = serde_json::to_value(&issues).unwrap();
                }

                if suggest {
                    let suggestions = self.suggest_lifetime_improvements(&issues);
                    json_output["suggestions"] = serde_json::to_value(&suggestions).unwrap();
                }

                if visualize {
                    if let Ok(graph) = self.build_lifetime_graph(&filtered_functions) {
                        json_output["lifetime_graph"] = serde_json::to_value(&graph).unwrap();
                    }
                }

                println!("{}", serde_json::to_string_pretty(&json_output).unwrap());
            }
            OutputFormat::Table => {
                println!("┌─ Lifetime Analysis ──────────────────────────────┐");
                println!("│ Files: {:<45} │", input);
                println!("│ Functions: {:<40} │", filtered_functions.len());
                let total_lifetimes: usize = filtered_functions.iter().map(|f| f.lifetimes.len()).sum();
                println!("│ Total Lifetimes: {:<34} │", total_lifetimes);
                if detect_issues {
                    let issues = self.detect_lifetime_issues(&filtered_functions);
                    println!("│ Issues Found: {:<37} │", issues.len());
                }
                if visualize {
                    println!("│ Visualization: {:<35} │", format!("✓ ({})", format));
                }
                println!("└────────────────────────────────────────────────────┘");
            }
        }

        Ok(())
    }
}

impl LifetimeVisualizerTool {
    fn find_rust_files(&self, dir: &str) -> Result<Vec<String>> {
        let mut rust_files = Vec::new();

        fn visit_dir(dir: &str, files: &mut Vec<String>) -> Result<()> {
            let entries = fs::read_dir(dir)?;
            for entry in entries {
                let entry = entry?;
                let path = entry.path();

                if path.is_dir() {
                    if let Some(dir_name) = path.file_name() {
                        if dir_name != "target" && dir_name != ".git" {
                            visit_dir(&path.to_string_lossy(), files)?;
                        }
                    }
                } else if let Some(ext) = path.extension() {
                    if ext == "rs" {
                        files.push(path.to_string_lossy().to_string());
                    }
                }
            }
            Ok(())
        }

        visit_dir(dir, &mut rust_files)?;
        Ok(rust_files)
    }
}
