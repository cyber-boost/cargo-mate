use super::{Tool, Result, ToolError, common_options, parse_output_format, OutputFormat};
use clap::{Arg, ArgMatches, Command};
use colored::*;
use std::fs;
use std::path::Path;
use std::collections::HashMap;
use syn::{
    parse_file, File, Item, ItemTrait, ItemImpl, TraitItem, visit::Visit,
    spanned::Spanned,
};
use quote::quote;
use proc_macro2::TokenStream;
#[derive(Debug, Clone)]
pub struct TraitExplorerTool;
#[derive(Debug, Clone)]
struct TraitDefinition {
    name: String,
    methods: Vec<TraitMethod>,
    supertraits: Vec<String>,
    visibility: String,
    attributes: Vec<String>,
    file_path: String,
    line_number: usize,
}
#[derive(Debug, Clone)]
struct TraitMethod {
    name: String,
    signature: String,
    is_required: bool,
    has_default_implementation: bool,
}
#[derive(Debug, Clone)]
struct TraitImplementation {
    trait_name: String,
    target_type: String,
    methods: Vec<ImplMethod>,
    file_path: String,
    line_number: usize,
    is_foreign: bool,
}
#[derive(Debug, Clone)]
struct ImplMethod {
    name: String,
    is_override: bool,
}
#[derive(Debug, Clone)]
struct TraitAnalysis {
    trait_definitions: Vec<TraitDefinition>,
    trait_implementations: Vec<TraitImplementation>,
    trait_usage_patterns: HashMap<String, Vec<String>>,
    orphan_rules_violations: Vec<String>,
    missing_implementations: Vec<String>,
}
#[derive(Debug, Clone)]
struct TraitGraph {
    nodes: Vec<String>,
    edges: Vec<(String, String)>,
}
#[derive(Debug, Clone)]
struct TraitSuggestion {
    trait_name: String,
    target_type: String,
    reason: String,
    confidence: f64,
}
impl TraitExplorerTool {
    pub fn new() -> Self {
        Self
    }
    fn discover_trait_definitions(
        &self,
        source_path: &str,
    ) -> Result<Vec<TraitDefinition>> {
        let mut traits = Vec::new();
        self.analyze_directory_for_traits(source_path, &mut traits)?;
        Ok(traits)
    }
    fn analyze_directory_for_traits(
        &self,
        dir_path: &str,
        traits: &mut Vec<TraitDefinition>,
    ) -> Result<()> {
        let entries = fs::read_dir(dir_path)
            .map_err(|e| ToolError::ExecutionFailed(
                format!("Failed to read directory {}: {}", dir_path, e),
            ))?;
        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                self.analyze_directory_for_traits(&path.to_string_lossy(), traits)?;
            } else if let Some(ext) = path.extension() {
                if ext == "rs" {
                    self.analyze_file_for_traits(&path, traits)?;
                }
            }
        }
        Ok(())
    }
    fn analyze_file_for_traits(
        &self,
        file_path: &Path,
        traits: &mut Vec<TraitDefinition>,
    ) -> Result<()> {
        let content = fs::read_to_string(file_path)?;
        let ast = parse_file(&content)?;
        struct TraitVisitor<'a> {
            traits: &'a mut Vec<TraitDefinition>,
            current_file: String,
        }
        impl<'a> Visit<'_> for TraitVisitor<'a> {
            fn visit_item_trait(&mut self, node: &ItemTrait) {
                let trait_def = TraitDefinition {
                    name: node.ident.to_string(),
                    methods: Self::extract_trait_methods(&node.items),
                    supertraits: Self::extract_supertraits(&node.supertraits),
                    visibility: if matches!(node.vis, syn::Visibility::Public(_)) {
                        "pub".to_string()
                    } else {
                        "private".to_string()
                    },
                    attributes: Self::extract_attributes(&node.attrs),
                    file_path: self.current_file.clone(),
                    line_number: 0,
                };
                self.traits.push(trait_def);
            }
        }
        impl TraitVisitor<'_> {
            fn extract_trait_methods(items: &[TraitItem]) -> Vec<TraitMethod> {
                let mut methods = Vec::new();
                for item in items {
                    if let TraitItem::Fn(method) = item {
                        let name = method.sig.ident.to_string();
                        let signature = Self::format_method_signature(&method.sig);
                        let (is_required, has_default) = match &method.default {
                            Some(_) => (false, true),
                            None => (true, false),
                        };
                        methods
                            .push(TraitMethod {
                                name,
                                signature,
                                is_required,
                                has_default_implementation: has_default,
                            });
                    }
                }
                methods
            }
            fn extract_supertraits(
                supertraits: &syn::punctuated::Punctuated<
                    syn::TypeParamBound,
                    syn::token::Plus,
                >,
            ) -> Vec<String> {
                let mut traits = Vec::new();
                for bound in supertraits {
                    if let syn::TypeParamBound::Trait(trait_bound) = bound {
                        let trait_name = trait_bound
                            .path
                            .segments
                            .iter()
                            .map(|seg| seg.ident.to_string())
                            .collect::<Vec<_>>()
                            .join("::");
                        traits.push(trait_name);
                    }
                }
                traits
            }
            fn extract_attributes(attrs: &[syn::Attribute]) -> Vec<String> {
                attrs
                    .iter()
                    .map(|attr| {
                        attr
                            .path()
                            .segments
                            .iter()
                            .map(|seg| seg.ident.to_string())
                            .collect::<Vec<_>>()
                            .join("::")
                    })
                    .collect()
            }
            fn format_method_signature(sig: &syn::Signature) -> String {
                let params = sig
                    .inputs
                    .iter()
                    .filter_map(|arg| {
                        match arg {
                            syn::FnArg::Receiver(_) => Some("self".to_string()),
                            syn::FnArg::Typed(pat_type) => {
                                if let syn::Pat::Ident(pat_ident) = &*pat_type.pat {
                                    Some(
                                        format!(
                                            "{}: {}", pat_ident.ident, Self::type_to_string(&* pat_type
                                            .ty)
                                        ),
                                    )
                                } else {
                                    None
                                }
                            }
                        }
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                let return_type = match &sig.output {
                    syn::ReturnType::Default => String::new(),
                    syn::ReturnType::Type(_, ty) => {
                        format!(" -> {}", Self::type_to_string(ty))
                    }
                };
                format!("fn {}({}){}", sig.ident, params, return_type)
            }
            fn type_to_string(ty: &syn::Type) -> String {
                match ty {
                    syn::Type::Path(type_path) => {
                        type_path
                            .path
                            .segments
                            .iter()
                            .map(|seg| seg.ident.to_string())
                            .collect::<Vec<_>>()
                            .join("::")
                    }
                    syn::Type::Reference(type_ref) => {
                        let mut result = "&".to_string();
                        if type_ref.mutability.is_some() {
                            result.push_str("mut ");
                        }
                        result.push_str(&Self::type_to_string(&*type_ref.elem));
                        result
                    }
                    _ => "Unknown".to_string(),
                }
            }
        }
        let mut visitor = TraitVisitor {
            traits,
            current_file: file_path.to_string_lossy().to_string(),
        };
        syn::visit::visit_file(&mut visitor, &ast);
        Ok(())
    }
    fn discover_trait_implementations(
        &self,
        source_path: &str,
    ) -> Result<Vec<TraitImplementation>> {
        let mut implementations = Vec::new();
        self.analyze_directory_for_implementations(source_path, &mut implementations)?;
        Ok(implementations)
    }
    fn analyze_directory_for_implementations(
        &self,
        dir_path: &str,
        implementations: &mut Vec<TraitImplementation>,
    ) -> Result<()> {
        let entries = fs::read_dir(dir_path)
            .map_err(|e| ToolError::ExecutionFailed(
                format!("Failed to read directory {}: {}", dir_path, e),
            ))?;
        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                self.analyze_directory_for_implementations(
                    &path.to_string_lossy(),
                    implementations,
                )?;
            } else if let Some(ext) = path.extension() {
                if ext == "rs" {
                    self.analyze_file_for_implementations(&path, implementations)?;
                }
            }
        }
        Ok(())
    }
    fn analyze_file_for_implementations(
        &self,
        file_path: &Path,
        implementations: &mut Vec<TraitImplementation>,
    ) -> Result<()> {
        let content = fs::read_to_string(file_path)?;
        let ast = parse_file(&content)?;
        struct ImplVisitor<'a> {
            implementations: &'a mut Vec<TraitImplementation>,
            current_file: String,
        }
        impl<'a> Visit<'_> for ImplVisitor<'a> {
            fn visit_item_impl(&mut self, node: &ItemImpl) {
                if let Some((_, trait_path, _)) = &node.trait_ {
                    let trait_name = trait_path
                        .segments
                        .iter()
                        .map(|seg| seg.ident.to_string())
                        .collect::<Vec<_>>()
                        .join("::");
                    let target_type = Self::extract_target_type(&node.self_ty);
                    let methods = node
                        .items
                        .iter()
                        .filter_map(|item| {
                            match item {
                                syn::ImplItem::Fn(method) => {
                                    Some(ImplMethod {
                                        name: method.sig.ident.to_string(),
                                        is_override: method
                                            .attrs
                                            .iter()
                                            .any(|attr| {
                                                attr.path()
                                                    .segments
                                                    .iter()
                                                    .any(|seg| seg.ident == "override")
                                            }),
                                    })
                                }
                                _ => None,
                            }
                        })
                        .collect();
                    let impl_info = TraitImplementation {
                        trait_name,
                        target_type,
                        methods,
                        file_path: self.current_file.clone(),
                        line_number: 0,
                        is_foreign: false,
                    };
                    self.implementations.push(impl_info);
                }
            }
        }
        impl ImplVisitor<'_> {
            fn extract_target_type(ty: &syn::Type) -> String {
                match ty {
                    syn::Type::Path(type_path) => {
                        type_path
                            .path
                            .segments
                            .iter()
                            .map(|seg| seg.ident.to_string())
                            .collect::<Vec<_>>()
                            .join("::")
                    }
                    _ => "Unknown".to_string(),
                }
            }
        }
        let mut visitor = ImplVisitor {
            implementations,
            current_file: file_path.to_string_lossy().to_string(),
        };
        syn::visit::visit_file(&mut visitor, &ast);
        Ok(())
    }
    fn analyze_trait_usage(
        &self,
        implementations: &[TraitImplementation],
    ) -> Result<HashMap<String, Vec<String>>> {
        let mut usage_patterns = HashMap::new();
        for impl_info in implementations {
            usage_patterns
                .entry(impl_info.trait_name.clone())
                .or_insert_with(Vec::new)
                .push(impl_info.target_type.clone());
        }
        Ok(usage_patterns)
    }
    fn find_missing_implementations(
        &self,
        traits: &[TraitDefinition],
        implementations: &[TraitImplementation],
    ) -> Vec<String> {
        let mut missing = Vec::new();
        let implemented_traits: HashMap<String, Vec<String>> = implementations
            .iter()
            .fold(
                HashMap::new(),
                |mut acc, impl_info| {
                    acc.entry(impl_info.trait_name.clone())
                        .or_insert_with(Vec::new)
                        .push(impl_info.target_type.clone());
                    acc
                },
            );
        for trait_def in traits {
            if let Some(implementors) = implemented_traits.get(&trait_def.name) {
                if implementors.is_empty() {
                    missing
                        .push(
                            format!("Trait '{}' has no implementations", trait_def.name),
                        );
                }
            } else {
                missing
                    .push(format!("Trait '{}' has no implementations", trait_def.name));
            }
        }
        missing
    }
    fn generate_trait_documentation(
        &self,
        traits: &[TraitDefinition],
        implementations: &[TraitImplementation],
    ) -> Result<String> {
        let mut docs = String::from("# Trait Implementation Analysis\n\n");
        docs.push_str(
            &format!(
                "Generated on: {}\n\n", chrono::Utc::now().format("%Y-%m-%d %H:%M:%S")
            ),
        );
        docs.push_str("## 📊 Overview\n\n");
        docs.push_str(&format!("- **Total Traits**: {}\n", traits.len()));
        docs.push_str(
            &format!("- **Total Implementations**: {}\n", implementations.len()),
        );
        let trait_usage: HashMap<String, usize> = implementations
            .iter()
            .fold(
                HashMap::new(),
                |mut acc, impl_info| {
                    *acc.entry(impl_info.trait_name.clone()).or_insert(0) += 1;
                    acc
                },
            );
        let most_used_trait = trait_usage
            .iter()
            .max_by_key(|(_, count)| *count)
            .map(|(name, _)| name.as_str())
            .unwrap_or("None");
        docs.push_str(&format!("- **Most Implemented Trait**: {}\n\n", most_used_trait));
        docs.push_str("## 🎯 Trait Implementations\n\n");
        for trait_def in traits {
            docs.push_str(&format!("### `{}` Trait\n", trait_def.name));
            docs.push_str(&format!("**File:** `{}`\n", trait_def.file_path));
            docs.push_str(&format!("**Visibility:** {}\n\n", trait_def.visibility));
            if !trait_def.supertraits.is_empty() {
                docs.push_str("**Supertraits:**\n");
                for supertrait in &trait_def.supertraits {
                    docs.push_str(&format!("- `{}`\n", supertrait));
                }
                docs.push_str("\n");
            }
            docs.push_str("**Methods:**\n");
            for method in &trait_def.methods {
                let required = if method.is_required { "Required" } else { "Optional" };
                let default = if method.has_default_implementation {
                    " (has default)"
                } else {
                    ""
                };
                docs.push_str(
                    &format!("- `{}` - {}{}\n", method.signature, required, default),
                );
            }
            docs.push_str("\n");
            let trait_implementations: Vec<&TraitImplementation> = implementations
                .iter()
                .filter(|impl_info| impl_info.trait_name == trait_def.name)
                .collect();
            if !trait_implementations.is_empty() {
                docs.push_str("**Known Implementations:**\n");
                for impl_info in &trait_implementations {
                    docs.push_str(
                        &format!(
                            "- `{}` in `{}`\n", impl_info.target_type, impl_info
                            .file_path
                        ),
                    );
                }
            } else {
                docs.push_str("**No implementations found**\n");
            }
            docs.push_str("\n");
        }
        let missing = self.find_missing_implementations(traits, implementations);
        if !missing.is_empty() {
            docs.push_str("## ⚠️ Missing Implementations\n\n");
            for issue in &missing {
                docs.push_str(&format!("- {}\n", issue));
            }
            docs.push_str("\n");
        }
        Ok(docs)
    }
    fn generate_trait_visualization(
        &self,
        traits: &[TraitDefinition],
        implementations: &[TraitImplementation],
        format: &str,
    ) -> Result<String> {
        match format {
            "mermaid" => self.generate_mermaid_graph(traits, implementations),
            "dot" => self.generate_dot_graph(traits, implementations),
            _ => {
                Err(
                    ToolError::InvalidArguments(
                        format!("Unsupported visualization format: {}", format),
                    ),
                )
            }
        }
    }
    fn generate_mermaid_graph(
        &self,
        traits: &[TraitDefinition],
        implementations: &[TraitImplementation],
    ) -> Result<String> {
        let mut mermaid = String::from("```mermaid\ngraph TD\n");
        for trait_def in traits {
            mermaid
                .push_str(
                    &format!(
                        "    T{}[\"Trait: {}\"]\n", trait_def.name.replace("::", "_"),
                        trait_def.name
                    ),
                );
        }
        for impl_info in implementations {
            let trait_node = impl_info.trait_name.replace("::", "_");
            let type_node = impl_info.target_type.replace("::", "_");
            mermaid
                .push_str(
                    &format!(
                        "    T{} --> I{}[\"Impl: {}\"]\n", trait_node, type_node,
                        impl_info.target_type
                    ),
                );
        }
        for trait_def in traits {
            for supertrait in &trait_def.supertraits {
                let super_node = supertrait.replace("::", "_");
                let trait_node = trait_def.name.replace("::", "_");
                mermaid
                    .push_str(
                        &format!(
                            "    T{} --> T{}[\"Supertrait: {}\"]\n", trait_node,
                            super_node, supertrait
                        ),
                    );
            }
        }
        mermaid.push_str("```\n");
        Ok(mermaid)
    }
    fn generate_dot_graph(
        &self,
        traits: &[TraitDefinition],
        implementations: &[TraitImplementation],
    ) -> Result<String> {
        let mut dot = String::from("digraph TraitGraph {\n");
        dot.push_str("    rankdir=LR;\n");
        dot.push_str("    node [shape=box];\n\n");
        for trait_def in traits {
            dot.push_str(
                &format!(
                    "    \"{}\" [label=\"Trait: {}\", fillcolor=lightblue, style=filled];\n",
                    trait_def.name, trait_def.name
                ),
            );
        }
        for impl_info in implementations {
            dot.push_str(
                &format!(
                    "    \"{}\" [label=\"Impl: {}\"];\n", impl_info.target_type,
                    impl_info.target_type
                ),
            );
            dot.push_str(
                &format!(
                    "    \"{}\" -> \"{}\" [label=\"implements\"];\n", impl_info
                    .target_type, impl_info.trait_name
                ),
            );
        }
        for trait_def in traits {
            for supertrait in &trait_def.supertraits {
                dot.push_str(
                    &format!(
                        "    \"{}\" -> \"{}\" [label=\"extends\", style=dashed];\n",
                        trait_def.name, supertrait
                    ),
                );
            }
        }
        dot.push_str("}\n");
        Ok(dot)
    }
    fn suggest_trait_implementations(
        &self,
        traits: &[TraitDefinition],
        implementations: &[TraitImplementation],
    ) -> Vec<TraitSuggestion> {
        let mut suggestions = Vec::new();
        let common_types = vec![
            "String", "i32", "i64", "bool", "Vec<T>", "HashMap<K, V>", "Option<T>",
            "Result<T, E>"
        ];
        for trait_def in traits {
            let implementation_count = implementations
                .iter()
                .filter(|impl_info| impl_info.trait_name == trait_def.name)
                .count();
            if implementation_count > 5 {
                continue;
            }
            for common_type in &common_types {
                let already_implemented = implementations
                    .iter()
                    .any(|impl_info| {
                        impl_info.trait_name == trait_def.name
                            && impl_info.target_type == *common_type
                    });
                if !already_implemented {
                    let confidence = if common_type.contains("String")
                        && trait_def.name.contains("Display")
                    {
                        0.9
                    } else if common_type.contains("Vec")
                        && trait_def.name.contains("IntoIterator")
                    {
                        0.8
                    } else {
                        0.5
                    };
                    suggestions
                        .push(TraitSuggestion {
                            trait_name: trait_def.name.clone(),
                            target_type: common_type.to_string(),
                            reason: format!(
                                "{} would benefit from {} implementation", common_type,
                                trait_def.name
                            ),
                            confidence,
                        });
                }
            }
        }
        suggestions
            .sort_by(|a, b| {
                b
                    .confidence
                    .partial_cmp(&a.confidence)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
        suggestions
    }
    fn generate_implementations_report(
        &self,
        _traits: &[TraitDefinition],
        _implementations: &[TraitImplementation],
    ) -> Result<String> {
        Err(
            ToolError::ExecutionFailed(
                "Implementation report feature not yet implemented".to_string(),
            ),
        )
    }
    fn generate_missing_report(&self, _missing: &[String]) -> Result<String> {
        Err(
            ToolError::ExecutionFailed(
                "Missing implementations report feature not yet implemented".to_string(),
            ),
        )
    }
    fn generate_suggestions_report(
        &self,
        _suggestions: &[TraitSuggestion],
    ) -> Result<String> {
        Err(
            ToolError::ExecutionFailed(
                "Suggestions report feature not yet implemented".to_string(),
            ),
        )
    }
}
impl Tool for TraitExplorerTool {
    fn name(&self) -> &'static str {
        "trait-explorer"
    }
    fn description(&self) -> &'static str {
        "Explore and analyze trait implementations across the workspace"
    }
    fn command(&self) -> Command {
        Command::new(self.name())
            .about(self.description())
            .long_about(
                "Discover all trait implementations in your codebase, analyze usage patterns, find missing implementations, and generate comprehensive trait documentation.",
            )
            .args(
                &[
                    Arg::new("input")
                        .long("input")
                        .short('i')
                        .help("Input directory to analyze")
                        .default_value("src/"),
                    Arg::new("trait")
                        .long("trait")
                        .short('t')
                        .help("Specific trait to explore"),
                    Arg::new("implementations")
                        .long("implementations")
                        .help("Show all implementations of specified trait")
                        .action(clap::ArgAction::SetTrue),
                    Arg::new("missing")
                        .long("missing")
                        .help("Find missing trait implementations")
                        .action(clap::ArgAction::SetTrue),
                    Arg::new("usage")
                        .long("usage")
                        .help("Analyze trait usage patterns")
                        .action(clap::ArgAction::SetTrue),
                    Arg::new("visualize")
                        .long("visualize")
                        .short('v')
                        .help("Generate trait relationship visualization")
                        .action(clap::ArgAction::SetTrue),
                    Arg::new("format")
                        .long("format")
                        .help("Output format: markdown, json, mermaid, dot")
                        .default_value("markdown"),
                    Arg::new("output")
                        .long("output")
                        .short('o')
                        .help("Output file for analysis")
                        .default_value("trait-analysis.md"),
                    Arg::new("suggest")
                        .long("suggest")
                        .help("Suggest additional trait implementations")
                        .action(clap::ArgAction::SetTrue),
                    Arg::new("workspace")
                        .long("workspace")
                        .help("Analyze all crates in workspace")
                        .action(clap::ArgAction::SetTrue),
                ],
            )
            .args(&common_options())
    }
    fn execute(&self, matches: &ArgMatches) -> Result<()> {
        let input = matches.get_one::<String>("input").unwrap();
        let specific_trait = matches.get_one::<String>("trait");
        let implementations_only = matches.get_flag("implementations");
        let missing_only = matches.get_flag("missing");
        let usage_analysis = matches.get_flag("usage");
        let visualize = matches.get_flag("visualize");
        let format = matches.get_one::<String>("format").unwrap();
        let output = matches.get_one::<String>("output").unwrap();
        let suggest = matches.get_flag("suggest");
        let workspace = matches.get_flag("workspace");
        let dry_run = matches.get_flag("dry-run");
        let verbose = matches.get_flag("verbose");
        let output_format = parse_output_format(matches);
        println!(
            "🔍 {} - {}", "CargoMate TraitExplorer".bold().blue(), self.description()
            .cyan()
        );
        if !Path::new(input).exists() {
            return Err(
                ToolError::InvalidArguments(format!("Input path not found: {}", input)),
            );
        }
        if verbose {
            println!("   📊 Analyzing trait ecosystem in {}", input);
        }
        let trait_definitions = self.discover_trait_definitions(input)?;
        if verbose {
            println!("   📋 Found {} trait definitions", trait_definitions.len());
        }
        let trait_implementations = self.discover_trait_implementations(input)?;
        if verbose {
            println!(
                "   📋 Found {} trait implementations", trait_implementations.len()
            );
        }
        let (filtered_definitions, filtered_implementations) = if let Some(trait_name) = specific_trait {
            let defs: Vec<TraitDefinition> = trait_definitions
                .into_iter()
                .filter(|t| t.name == *trait_name)
                .collect();
            let impls: Vec<TraitImplementation> = trait_implementations
                .into_iter()
                .filter(|i| i.trait_name == *trait_name)
                .collect();
            (defs, impls)
        } else {
            (trait_definitions, trait_implementations)
        };
        if filtered_definitions.is_empty() {
            println!("{}", "No traits found matching criteria.".yellow());
            return Ok(());
        }
        let usage_patterns = self.analyze_trait_usage(&filtered_implementations)?;
        let missing_implementations = self
            .find_missing_implementations(
                &filtered_definitions,
                &filtered_implementations,
            );
        let suggestions = if suggest {
            self.suggest_trait_implementations(
                &filtered_definitions,
                &filtered_implementations,
            )
        } else {
            Vec::new()
        };
        let mut output_content = String::new();
        if implementations_only {
            output_content = self
                .generate_implementations_report(
                    &filtered_definitions,
                    &filtered_implementations,
                )?;
        } else if missing_only {
            output_content = self.generate_missing_report(&missing_implementations)?;
        } else if visualize {
            output_content = self
                .generate_trait_visualization(
                    &filtered_definitions,
                    &filtered_implementations,
                    format,
                )?;
        } else if suggest {
            output_content = self.generate_suggestions_report(&suggestions)?;
        } else {
            output_content = self
                .generate_trait_documentation(
                    &filtered_definitions,
                    &filtered_implementations,
                )?;
        }
        match output_format {
            OutputFormat::Human => {
                println!("  ✅ Generated trait analysis");
                println!("     → {}", output.cyan());
                if implementations_only {
                    println!(
                        "  📋 Showing implementations for {} traits",
                        filtered_definitions.len()
                    );
                } else if missing_only {
                    println!(
                        "  ⚠️  Found {} potential missing implementations",
                        missing_implementations.len()
                    );
                } else if visualize {
                    println!("  📊 Generated {} visualization", format);
                } else if suggest {
                    println!(
                        "  💡 Generated {} implementation suggestions", suggestions
                        .len()
                    );
                }
                if dry_run {
                    println!("   📋 {}", "Analysis preview:".bold());
                    println!("   {}", "─".repeat(50));
                    for line in output_content.lines().take(15) {
                        println!("   {}", line);
                    }
                    if output_content.lines().count() > 15 {
                        println!("   ... (truncated)");
                    }
                } else {
                    if let Some(parent) = Path::new(output).parent() {
                        fs::create_dir_all(parent)
                            .map_err(|e| ToolError::ExecutionFailed(
                                format!("Failed to create output directory: {}", e),
                            ))?;
                    }
                    fs::write(output, &output_content)
                        .map_err(|e| ToolError::ExecutionFailed(
                            format!("Failed to write {}: {}", output, e),
                        ))?;
                    println!("  💾 Analysis written successfully");
                }
            }
            OutputFormat::Json => {
                let result = serde_json::json!(
                    { "traits_analyzed" : filtered_definitions.len(),
                    "implementations_found" : filtered_implementations.len(),
                    "missing_implementations" : missing_implementations.len(),
                    "suggestions_count" : suggestions.len(), "analysis_content" :
                    output_content }
                );
                println!("{}", serde_json::to_string_pretty(& result).unwrap());
            }
            OutputFormat::Table => {
                println!(
                    "{:<25} {:<15} {:<12} {:<10} {:<12}", "Analysis Type", "Traits",
                    "Impls", "Missing", "Suggestions"
                );
                println!("{}", "─".repeat(80));
                println!(
                    "{:<25} {:<15} {:<12} {:<10} {:<12}", if implementations_only {
                    "Implementations" } else if missing_only { "Missing" } else if
                    visualize { "Visualization" } else if suggest { "Suggestions" } else
                    { "Full Analysis" }, filtered_definitions.len(),
                    filtered_implementations.len(), missing_implementations.len(),
                    suggestions.len()
                );
            }
        }
        println!("\n🎉 Trait exploration completed!");
        Ok(())
    }
}
impl Default for TraitExplorerTool {
    fn default() -> Self {
        Self::new()
    }
}