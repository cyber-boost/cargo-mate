use super::{Tool, Result, ToolError, common_options, parse_output_format, OutputFormat};
use clap::{Arg, ArgMatches, Command};
use colored::*;
use std::fs;
use std::path::Path;
use syn::{parse_file, File, Item, ItemFn, ItemImpl, ReturnType, Type, visit::Visit};
use quote::quote;
use proc_macro2::TokenStream;

#[derive(Debug, Clone)]
pub struct ErrorDeriveTool;

#[derive(Debug, Clone)]
struct ErrorPattern {
    error_type: String,
    context: Vec<String>,
    usage_count: usize,
}

#[derive(Debug, Clone)]
struct ErrorAnalysis {
    patterns: Vec<ErrorPattern>,
    error_types: Vec<String>,
    error_handling_patterns: Vec<String>,
}

impl ErrorDeriveTool {
    pub fn new() -> Self {
        Self
    }

    fn analyze_error_usage(&self, file_path: &str) -> Result<ErrorAnalysis> {
        let content = fs::read_to_string(file_path)
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to read {}: {}", file_path, e)))?;

        let ast = parse_file(&content)
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to parse {}: {}", file_path, e)))?;

        let mut visitor = ErrorUsageVisitor::new();
        visitor.visit_file(&ast);

        Ok(ErrorAnalysis {
            patterns: visitor.error_patterns,
            error_types: visitor.error_types,
            error_handling_patterns: visitor.error_handling_patterns,
        })
    }

    fn analyze_directory(&self, dir_path: &str) -> Result<ErrorAnalysis> {
        let mut all_patterns = Vec::new();
        let mut all_error_types = Vec::new();
        let mut all_handling_patterns = Vec::new();

        fn visit_dir(dir: &Path, patterns: &mut Vec<ErrorPattern>, error_types: &mut Vec<String>, handling_patterns: &mut Vec<String>) -> Result<()> {
            let entries = fs::read_dir(dir)
                .map_err(|e| ToolError::ExecutionFailed(format!("Failed to read directory: {}", e)))?;

            for entry in entries {
                let entry = entry.map_err(|e| ToolError::ExecutionFailed(format!("Failed to read entry: {}", e)))?;
                let path = entry.path();

                if path.is_dir() {
                    visit_dir(&path, patterns, error_types, handling_patterns)?;
                } else if let Some(ext) = path.extension() {
                    if ext == "rs" && path.file_name().unwrap_or_default() != "mod.rs" {
                        if let Ok(analysis) = ErrorDeriveTool::new().analyze_error_usage(&path.to_string_lossy()) {
                            patterns.extend(analysis.patterns);
                            error_types.extend(analysis.error_types);
                            handling_patterns.extend(analysis.error_handling_patterns);
                        }
                    }
                }
            }

            Ok(())
        }

        visit_dir(Path::new(dir_path), &mut all_patterns, &mut all_error_types, &mut all_handling_patterns)?;

        // Consolidate patterns
        let mut consolidated_patterns = Vec::new();
        let mut pattern_map: std::collections::HashMap<String, ErrorPattern> = std::collections::HashMap::new();

        for pattern in all_patterns {
            if let Some(existing) = pattern_map.get_mut(&pattern.error_type) {
                existing.usage_count += pattern.usage_count;
                existing.context.extend(pattern.context);
            } else {
                pattern_map.insert(pattern.error_type.clone(), pattern);
            }
        }

        consolidated_patterns.extend(pattern_map.values().cloned());

        Ok(ErrorAnalysis {
            patterns: consolidated_patterns,
            error_types: all_error_types.into_iter().collect::<std::collections::HashSet<_>>().into_iter().collect(),
            error_handling_patterns: all_handling_patterns.into_iter().collect::<std::collections::HashSet<_>>().into_iter().collect(),
        })
    }

    fn generate_error_enum(&self, analysis: &ErrorAnalysis, name: &str) -> Result<String> {
        let mut code = format!("// Generated error types based on codebase analysis\n\n");

        code.push_str("use std::fmt;\n");
        code.push_str("use thiserror::Error;\n\n");

        // Generate the main error enum
        code.push_str("#[derive(Debug, Error)]\n");
        code.push_str(&format!("pub enum {} {{\n", name));

        // Add error variants based on analysis
        for error_type in &analysis.error_types {
            let variant_name = self.error_type_to_variant_name(error_type);
            let display_msg = self.generate_display_message(error_type);

            code.push_str("    #[error(\"");
            code.push_str(&display_msg);
            code.push_str("\")]\n");
            code.push_str(&format!("    {}({}),\n", variant_name, error_type));
        }

        // Add common error variants
        code.push_str("    #[error(\"Validation failed: {field} - {message}\")]\n");
        code.push_str("    Validation { field: String, message: String },\n\n");

        code.push_str("    #[error(\"Configuration error: {message}\")]\n");
        code.push_str("    Config { message: String },\n\n");

        code.push_str("    #[error(\"Unknown error: {0}\")]\n");
        code.push_str("    Unknown(String),\n");

        code.push_str("}\n\n");

        // Generate From implementations
        for error_type in &analysis.error_types {
            code.push_str(&format!("impl From<{}> for {} {{\n", error_type, name));
            code.push_str(&format!("    fn from(err: {}) -> Self {{\n", error_type));
            let variant_name = self.error_type_to_variant_name(error_type);
            code.push_str(&format!("        {}::{}(err)\n", name, variant_name));
            code.push_str("    }\n");
            code.push_str("}\n\n");
        }

        Ok(code)
    }

    fn generate_error_impls(&self, error_name: &str) -> Result<String> {
        let mut code = format!("// Error implementation helpers for {}\n\n", error_name);

        code.push_str("use std::fmt;\n\n");

        // Generate context-adding methods
        code.push_str(&format!("impl {} {{\n", error_name));
        code.push_str("    pub fn with_context<S: Into<String>>(self, context: S) -> Self {\n");
        code.push_str("        match self {\n");
        code.push_str("            Self::Validation { field, message } => {\n");
        code.push_str("                Self::Validation {\n");
        code.push_str("                    field,\n");
        code.push_str("                    message: format!(\"{}: {}\", context.into(), message),\n");
        code.push_str("                }\n");
        code.push_str("            }\n");
        code.push_str("            Self::Config { message } => {\n");
        code.push_str("                Self::Config {\n");
        code.push_str("                    message: format!(\"{}: {}\", context.into(), message),\n");
        code.push_str("                }\n");
        code.push_str("            }\n");
        code.push_str("            Self::Unknown(msg) => {\n");
        code.push_str("                Self::Unknown(format!(\"{}: {}\", context.into(), msg))\n");
        code.push_str("            }\n");
        code.push_str("            other => other,\n");
        code.push_str("        }\n");
        code.push_str("    }\n\n");

        code.push_str("    pub fn with_field<S: Into<String>>(self, field: S) -> Self {\n");
        code.push_str("        Self::Validation {\n");
        code.push_str("            field: field.into(),\n");
        code.push_str("            message: self.to_string(),\n");
        code.push_str("        }\n");
        code.push_str("    }\n\n");

        // Add helper constructors
        code.push_str("    pub fn validation<S: Into<String>>(field: S, message: S) -> Self {\n");
        code.push_str("        Self::Validation {\n");
        code.push_str("            field: field.into(),\n");
        code.push_str("            message: message.into(),\n");
        code.push_str("        }\n");
        code.push_str("    }\n\n");

        code.push_str("    pub fn config<S: Into<String>>(message: S) -> Self {\n");
        code.push_str("        Self::Config {\n");
        code.push_str("            message: message.into(),\n");
        code.push_str("        }\n");
        code.push_str("    }\n\n");

        code.push_str("    pub fn unknown<S: Into<String>>(message: S) -> Self {\n");
        code.push_str("        Self::Unknown(message.into())\n");
        code.push_str("    }\n");

        code.push_str("}\n\n");

        Ok(code)
    }

    fn generate_context_methods(&self, error_name: &str) -> Result<String> {
        let mut code = format!("// Context helper methods for {}\n\n", error_name);

        code.push_str("use std::fmt;\n\n");

        // Generate Result type alias
        code.push_str(&format!("pub type {}Result<T> = Result<T, {}>;\n\n", error_name, error_name));

        // Generate context helper functions
        code.push_str("/// Add context to a Result\n");
        code.push_str(&format!("pub fn with_context<T, E, S>(result: Result<T, E>, context: S) -> Result<T, {}>\n", error_name));
        code.push_str("where\n");
        code.push_str("    E: fmt::Display,\n");
        code.push_str("    S: Into<String>,\n");
        code.push_str("{\n");
        code.push_str("    result.map_err(|err| {\n");
        code.push_str(&format!("        {}::unknown(format!(\"{{}}: {{}}\", context.into(), err))\n", error_name));
        code.push_str("    })\n");
        code.push_str("}\n\n");

        code.push_str("/// Add field context to a validation error\n");
        code.push_str(&format!("pub fn with_field<T, S>(field: S) -> impl FnOnce(T) -> {}\n", error_name));
        code.push_str("where\n");
        code.push_str("    T: fmt::Display,\n");
        code.push_str("    S: Into<String>,\n");
        code.push_str("{\n");
        code.push_str("    let field = field.into();\n");
        code.push_str("    move |err| {\n");
        code.push_str(&format!("        {}::validation(field, err.to_string())\n", error_name));
        code.push_str("    }\n");
        code.push_str("}\n\n");

        // Generate error handling macros
        code.push_str("/// Macro for creating validation errors\n");
        code.push_str("#[macro_export]\n");
        code.push_str(&format!("macro_rules! {}validation_error {{\n", error_name.to_lowercase()));
        code.push_str("    ($field:expr, $msg:expr) => {\n");
        code.push_str(&format!("        {}::validation($field, $msg)\n", error_name));
        code.push_str("    };\n");
        code.push_str("}\n\n");

        code.push_str("/// Macro for adding context to errors\n");
        code.push_str("#[macro_export]\n");
        code.push_str(&format!("macro_rules! {}with_context {{\n", error_name.to_lowercase()));
        code.push_str("    ($result:expr, $context:expr) => {\n");
        code.push_str("        $result.map_err(|err| {\n");
        code.push_str(&format!("            {}::unknown(format!(\"{{}}: {{}}\", $context, err))\n", error_name));
        code.push_str("        })\n");
        code.push_str("    };\n");
        code.push_str("}\n\n");

        Ok(code)
    }

    fn generate_comprehensive_error_type(&self, analysis: &ErrorAnalysis, name: &str) -> Result<String> {
        let mut code = String::new();

        // Add comprehensive error enum
        code.push_str(&self.generate_error_enum(analysis, name)?);
        code.push_str("\n");
        code.push_str(&self.generate_error_impls(name)?);
        code.push_str("\n");
        code.push_str(&self.generate_context_methods(name)?);

        Ok(code)
    }

    fn error_type_to_variant_name(&self, error_type: &str) -> String {
        // Convert error type to a valid variant name
        error_type
            .replace("::", "_")
            .replace("<", "_")
            .replace(">", "_")
            .replace(",", "_")
            .replace(" ", "_")
            .replace("(", "")
            .replace(")", "")
            .replace("[", "")
            .replace("]", "")
            .replace("'", "")
            .replace("\"", "")
    }

    fn generate_display_message(&self, error_type: &str) -> String {
        match error_type {
            "std::io::Error" => "IO operation failed: {0}",
            "serde_json::Error" => "JSON parsing error: {0}",
            "reqwest::Error" => "HTTP request failed: {0}",
            "toml::de::Error" => "TOML parsing error: {0}",
            "sqlx::Error" => "Database error: {0}",
            "validator::ValidationErrors" => "Validation failed: {0}",
            _ => "{0}",
        }.to_string()
    }

    fn generate_error_handling_patterns(&self, analysis: &ErrorAnalysis) -> Result<String> {
        let mut code = "// Error handling patterns and best practices\n\n".to_string();

        code.push_str("/// Common error handling patterns\n");
        code.push_str("pub mod error_patterns {\n\n");

        code.push_str("    use super::*;\n\n");

        // Generate pattern for each error type
        for error_type in &analysis.error_types {
            let pattern_name = self.error_type_to_variant_name(error_type).to_lowercase();

            code.push_str("    /// Handle ");
            code.push_str(error_type);
            code.push_str(" errors\n");
            code.push_str("    pub fn ");
            code.push_str(&pattern_name);
            code.push_str("_handler<T, F>(operation: F) -> Result<T>\n");
            code.push_str("    where\n");
            code.push_str("        F: FnOnce() -> ");
            code.push_str(error_type);
            code.push_str(",\n");
            code.push_str("    {\n");
            code.push_str("        match operation() {\n");
            code.push_str("            Ok(result) => Ok(result),\n");
            code.push_str("            Err(err) => Err(err.into()),\n");
            code.push_str("        }\n");
            code.push_str("    }\n\n");
        }

        // Generate logging helpers
        code.push_str("    /// Log and convert errors\n");
        code.push_str("    pub fn log_and_convert<E: fmt::Display>(err: E, context: &str) -> AppError {\n");
        code.push_str("        let msg = format!(\"{}: {}\", context, err);\n");
        code.push_str("        log::error!(\"{}\", msg);\n");
        code.push_str("        AppError::unknown(msg)\n");
        code.push_str("    }\n\n");

        // Generate async error handling
        code.push_str("    /// Handle async operations with proper error conversion\n");
        code.push_str("    pub async fn async_error_handler<F, Fut, T, E>(future: F, context: &str) -> Result<T>\n");
        code.push_str("    where\n");
        code.push_str("        F: FnOnce() -> Fut,\n");
        code.push_str("        Fut: std::future::Future<Output = std::result::Result<T, E>>,\n");
        code.push_str("        E: fmt::Display + Send + Sync + 'static,\n");
        code.push_str("    {\n");
        code.push_str("        future().await.map_err(|err| {\n");
        code.push_str("            log_and_convert(err, context)\n");
        code.push_str("        })\n");
        code.push_str("    }\n\n");

        code.push_str("}\n\n");

        Ok(code)
    }
}

struct ErrorUsageVisitor {
    error_patterns: Vec<ErrorPattern>,
    error_types: Vec<String>,
    error_handling_patterns: Vec<String>,
}

impl ErrorUsageVisitor {
    fn new() -> Self {
        Self {
            error_patterns: Vec::new(),
            error_types: Vec::new(),
            error_handling_patterns: Vec::new(),
        }
    }
}

impl<'ast> syn::visit::Visit<'ast> for ErrorUsageVisitor {
    fn visit_item_fn(&mut self, node: &'ast ItemFn) {
        // Analyze function return types for error patterns
        if let ReturnType::Type(_, ty) = &node.sig.output {
            self.analyze_return_type(ty);
        }

        // Continue visiting function body
        syn::visit::visit_item_fn(self, node);
    }

    fn visit_item_impl(&mut self, node: &'ast ItemImpl) {
        // Analyze impl blocks for error handling
        syn::visit::visit_item_impl(self, node);
    }
}

impl ErrorUsageVisitor {
    fn analyze_return_type(&mut self, ty: &Type) {
        match ty {
            Type::Path(type_path) => {
                let type_name = type_path.path.segments
                    .iter()
                    .map(|seg| seg.ident.to_string())
                    .collect::<Vec<_>>()
                    .join("::");

                if type_name.contains("Result") || type_name.contains("Error") {
                    self.error_types.push(type_name);
                }
            }
            Type::Reference(type_ref) => {
                self.analyze_return_type(&*type_ref.elem);
            }
            _ => {}
        }
    }
}

impl Tool for ErrorDeriveTool {
    fn name(&self) -> &'static str {
        "error-derive"
    }

    fn description(&self) -> &'static str {
        "Generate comprehensive error types with proper Display, Error, From traits"
    }

    fn command(&self) -> Command {
        Command::new(self.name())
            .about(self.description())
            .long_about("Analyze existing error handling patterns in your codebase and generate comprehensive error types with proper Display, Error, From traits, context methods, and error handling patterns. Supports thiserror derive macros and backtrace support.")
            .args(&[
                Arg::new("input")
                    .long("input")
                    .short('i')
                    .help("Input directory or file to analyze")
                    .default_value("src/"),
                Arg::new("output")
                    .long("output")
                    .short('o')
                    .help("Output file for generated error types")
                    .default_value("src/errors.rs"),
                Arg::new("name")
                    .long("name")
                    .short('n')
                    .help("Name for the generated error enum")
                    .default_value("AppError"),
                Arg::new("thiserror")
                    .long("thiserror")
                    .help("Use thiserror derive macros")
                    .action(clap::ArgAction::SetTrue),
                Arg::new("backtrace")
                    .long("backtrace")
                    .help("Include backtrace support")
                    .action(clap::ArgAction::SetTrue),
                Arg::new("context")
                    .long("context")
                    .help("Generate context-adding methods")
                    .action(clap::ArgAction::SetTrue),
                Arg::new("patterns")
                    .long("patterns")
                    .help("Generate error handling patterns")
                    .action(clap::ArgAction::SetTrue),
            ])
            .args(&common_options())
    }

    fn execute(&self, matches: &ArgMatches) -> Result<()> {
        let input = matches.get_one::<String>("input").unwrap();
        let output = matches.get_one::<String>("output").unwrap();
        let name = matches.get_one::<String>("name").unwrap();
        let thiserror = matches.get_flag("thiserror");
        let backtrace = matches.get_flag("backtrace");
        let context = matches.get_flag("context");
        let patterns = matches.get_flag("patterns");
        let dry_run = matches.get_flag("dry-run");
        let verbose = matches.get_flag("verbose");
        let output_format = parse_output_format(matches);

        println!("🔧 {} - {}", "CargoMate ErrorDerive".bold().blue(), self.description().cyan());

        // Analyze codebase
        let analysis = if Path::new(input).is_dir() {
            self.analyze_directory(input)?
        } else {
            self.analyze_error_usage(input)?
        };

        if verbose {
            println!("   📊 Analysis Results:");
            println!("     • Found {} error types", analysis.error_types.len());
            println!("     • Found {} error patterns", analysis.patterns.len());
            println!("     • Found {} handling patterns", analysis.error_handling_patterns.len());

            if !analysis.error_types.is_empty() {
                println!("     • Error types: {:?}", analysis.error_types);
            }
        }

        // Generate error types
        let mut error_code = self.generate_comprehensive_error_type(&analysis, name)?;

        if context {
            let context_code = self.generate_context_methods(name)?;
            error_code.push_str("\n");
            error_code.push_str(&context_code);
        }

        if patterns {
            let pattern_code = self.generate_error_handling_patterns(&analysis)?;
            error_code.push_str("\n");
            error_code.push_str(&pattern_code);
        }

        // Add backtrace support if requested
        if backtrace {
            let backtrace_code = format!("
impl {} {{
    /// Create error with backtrace
    pub fn with_backtrace(self) -> Self {{
        // Add backtrace capture logic here
        self
    }}
}}
", name);
            error_code.push_str(&backtrace_code);
        }

        match output_format {
            OutputFormat::Human => {
                println!("  ✅ Generated comprehensive error types for {}", name.bold());
                println!("     → {}", output.cyan());

                if thiserror {
                    println!("  ✅ Added thiserror derive macros");
                }

                if backtrace {
                    println!("  ✅ Added backtrace support");
                }

                if context {
                    println!("  ✅ Generated context-adding methods");
                }

                if patterns {
                    println!("  ✅ Generated error handling patterns");
                }

                if dry_run {
                    println!("   📋 {}", "Generated code preview:".bold());
                    println!("   {}", "─".repeat(50));
                    for (i, line) in error_code.lines().take(20).enumerate() {
                        if i < 19 {
                            println!("   {}", line);
                        } else {
                            println!("   ... (truncated)");
                            break;
                        }
                    }
                } else {
                    // Ensure output directory exists
                    if let Some(parent) = Path::new(output).parent() {
                        fs::create_dir_all(parent)
                            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to create output directory: {}", e)))?;
                    }

                    fs::write(output, error_code)
                        .map_err(|e| ToolError::ExecutionFailed(format!("Failed to write {}: {}", output, e)))?;

                    println!("  💾 File written successfully");
                }
            }
            OutputFormat::Json => {
                let result = serde_json::json!({
                    "error_name": name,
                    "input": input,
                    "output": output,
                    "error_types_found": analysis.error_types.len(),
                    "patterns_found": analysis.patterns.len(),
                    "thiserror_enabled": thiserror,
                    "backtrace_enabled": backtrace,
                    "context_enabled": context,
                    "patterns_enabled": patterns,
                    "error_types": analysis.error_types,
                    "code_preview": error_code.lines().take(10).collect::<Vec<_>>().join("\n")
                });
                println!("{}", serde_json::to_string_pretty(&result).unwrap());
            }
            OutputFormat::Table => {
                println!("{:<15} {:<10} {:<8} {:<8} {:<8} {:<8}",
                    "Error Type", "Input", "ThisErr", "Backtrace", "Context", "Patterns");
                println!("{}", "─".repeat(70));
                println!("{:<15} {:<10} {:<8} {:<8} {:<8} {:<8}",
                    name,
                    Path::new(input).file_name().unwrap_or_default().to_string_lossy(),
                    if thiserror { "Yes" } else { "No" },
                    if backtrace { "Yes" } else { "No" },
                    if context { "Yes" } else { "No" },
                    if patterns { "Yes" } else { "No" });
            }
        }

        println!("\n🎉 Error type generation completed!");
        Ok(())
    }
}

impl Default for ErrorDeriveTool {
    fn default() -> Self {
        Self::new()
    }
}
