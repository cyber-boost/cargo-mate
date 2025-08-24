use super::{Tool, Result, ToolError, common_options, parse_output_format, OutputFormat};
use clap::{Arg, ArgMatches, Command};
use colored::*;
use std::fs;
use std::path::Path;
use std::collections::HashMap;
use syn::{parse_file, File, Item, ItemFn, ItemStruct, ItemTrait, Fields, Field, Type, PathSegment, Ident, visit::Visit};
use quote::ToTokens;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone)]
pub struct RustMentorTool;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CodeAnalysis {
    functions: Vec<FunctionAnalysis>,
    structs: Vec<StructAnalysis>,
    traits: Vec<TraitAnalysis>,
    patterns: Vec<PatternAnalysis>,
    issues: Vec<CodeIssue>,
    recommendations: Vec<Recommendation>,
    learning_opportunities: Vec<LearningOpportunity>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct FunctionAnalysis {
    name: String,
    complexity: u32,
    parameters: Vec<ParameterInfo>,
    return_type: Option<String>,
    patterns_used: Vec<String>,
    potential_improvements: Vec<String>,
    explanation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StructAnalysis {
    name: String,
    fields: Vec<FieldInfo>,
    patterns_used: Vec<String>,
    design_considerations: Vec<String>,
    explanation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TraitAnalysis {
    name: String,
    methods: Vec<String>,
    purpose: String,
    common_use_cases: Vec<String>,
    explanation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PatternAnalysis {
    pattern_type: String,
    locations: Vec<String>,
    explanation: String,
    benefits: Vec<String>,
    alternatives: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CodeIssue {
    severity: String,
    category: String,
    location: String,
    message: String,
    explanation: String,
    suggestion: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Recommendation {
    category: String,
    priority: String,
    title: String,
    description: String,
    code_example: Option<String>,
    benefits: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LearningOpportunity {
    topic: String,
    current_level: String,
    next_steps: Vec<String>,
    resources: Vec<String>,
    explanation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ParameterInfo {
    name: String,
    ty: String,
    purpose: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct FieldInfo {
    name: String,
    ty: String,
    purpose: String,
    considerations: Vec<String>,
}

impl RustMentorTool {
    pub fn new() -> Self {
        Self
    }

    fn analyze_codebase(&self, input_path: &str) -> Result<CodeAnalysis> {
        let mut functions = Vec::new();
        let mut structs = Vec::new();
        let mut traits = Vec::new();

        if Path::new(input_path).is_dir() {
            self.analyze_directory(input_path, &mut functions, &mut structs, &mut traits)?;
        } else {
            self.analyze_file(input_path, &mut functions, &mut structs, &mut traits)?;
        }

        let patterns = self.identify_patterns(&functions, &structs, &traits);
        let issues = self.identify_issues(&functions, &structs, &traits);
        let recommendations = self.generate_recommendations(&functions, &structs, &traits, &issues);
        let learning_opportunities = self.identify_learning_opportunities(&functions, &structs, &traits);

        Ok(CodeAnalysis {
            functions,
            structs,
            traits,
            patterns,
            issues,
            recommendations,
            learning_opportunities,
        })
    }

    fn analyze_directory(&self, dir_path: &str, functions: &mut Vec<FunctionAnalysis>,
                        structs: &mut Vec<StructAnalysis>, traits: &mut Vec<TraitAnalysis>) -> Result<()> {
        let entries = fs::read_dir(dir_path)
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to read directory {}: {}", dir_path, e)))?;

        for entry in entries {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                self.analyze_directory(&path.to_string_lossy(), functions, structs, traits)?;
            } else if let Some(ext) = path.extension() {
                if ext == "rs" && !path.ends_with("mod.rs") && !path.ends_with("lib.rs") {
                    self.analyze_file(&path.to_string_lossy(), functions, structs, traits)?;
                }
            }
        }

        Ok(())
    }

    fn analyze_file(&self, file_path: &str, functions: &mut Vec<FunctionAnalysis>,
                   structs: &mut Vec<StructAnalysis>, traits: &mut Vec<TraitAnalysis>) -> Result<()> {
        let content = fs::read_to_string(file_path)?;
        let ast = parse_file(&content)?;

        struct CodeVisitor<'a> {
            functions: &'a mut Vec<FunctionAnalysis>,
            structs: &'a mut Vec<StructAnalysis>,
            traits: &'a mut Vec<TraitAnalysis>,
            current_file: String,
        }

        impl<'a> Visit<'_> for CodeVisitor<'a> {
            fn visit_item_fn(&mut self, node: &ItemFn) {
                if let Ok(analysis) = Self::analyze_function(node) {
                    self.functions.push(analysis);
                }
            }

            fn visit_item_struct(&mut self, node: &ItemStruct) {
                if let Ok(analysis) = Self::analyze_struct(node) {
                    self.structs.push(analysis);
                }
            }

            fn visit_item_trait(&mut self, node: &ItemTrait) {
                if let Ok(analysis) = Self::analyze_trait(node) {
                    self.traits.push(analysis);
                }
            }
        }

        impl CodeVisitor<'_> {
            fn analyze_function(node: &ItemFn) -> Result<FunctionAnalysis> {
                let name = node.sig.ident.to_string();
                let complexity = Self::calculate_complexity(node);
                let parameters = Self::extract_parameters(&node.sig.inputs);
                let return_type = Self::extract_return_type(&node.sig.output);
                let patterns_used = Self::identify_function_patterns(node);
                let potential_improvements = Self::suggest_function_improvements(node);
                let explanation = Self::explain_function(node);

                Ok(FunctionAnalysis {
                    name,
                    complexity,
                    parameters,
                    return_type,
                    patterns_used,
                    potential_improvements,
                    explanation,
                })
            }

            fn analyze_struct(node: &ItemStruct) -> Result<StructAnalysis> {
                let name = node.ident.to_string();
                let fields = Self::extract_fields(&node.fields);
                let patterns_used = Self::identify_struct_patterns(node);
                let design_considerations = Self::struct_design_considerations(node);
                let explanation = Self::explain_struct(node);

                Ok(StructAnalysis {
                    name,
                    fields,
                    patterns_used,
                    design_considerations,
                    explanation,
                })
            }

            fn analyze_trait(node: &ItemTrait) -> Result<TraitAnalysis> {
                let name = node.ident.to_string();
                let methods = Self::extract_trait_methods(node);
                let purpose = Self::identify_trait_purpose(node);
                let common_use_cases = Self::trait_use_cases(node);
                let explanation = Self::explain_trait(node);

                Ok(TraitAnalysis {
                    name,
                    methods,
                    purpose,
                    common_use_cases,
                    explanation,
                })
            }

            fn calculate_complexity(node: &ItemFn) -> u32 {
                // Simple cyclomatic complexity calculation
                let mut complexity = 1u32; // Base complexity

                // Count control flow statements
                let code = node.to_token_stream().to_string();
                let control_flow_keywords = ["if", "else", "for", "while", "loop", "match"];
                for keyword in &control_flow_keywords {
                    complexity += code.matches(keyword).count() as u32;
                }

                complexity
            }

            fn extract_parameters(inputs: &syn::punctuated::Punctuated<syn::FnArg, syn::token::Comma>) -> Vec<ParameterInfo> {
                inputs.iter().filter_map(|arg| {
                    match arg {
                        syn::FnArg::Receiver(_) => Some(ParameterInfo {
                            name: "self".to_string(),
                            ty: "Self".to_string(),
                            purpose: "Reference to the current instance".to_string(),
                        }),
                        syn::FnArg::Typed(pat_type) => {
                            if let syn::Pat::Ident(pat_ident) = &*pat_type.pat {
                                let name = pat_ident.ident.to_string();
                                let ty = Self::type_to_string(&*pat_type.ty);
                                let purpose = Self::infer_parameter_purpose(&name, &ty);
                                Some(ParameterInfo { name, ty, purpose })
                            } else {
                                None
                            }
                        }
                    }
                }).collect()
            }

            fn extract_return_type(output: &syn::ReturnType) -> Option<String> {
                match output {
                    syn::ReturnType::Default => None,
                    syn::ReturnType::Type(_, ty) => Some(Self::type_to_string(ty)),
                }
            }

            fn extract_fields(fields: &Fields) -> Vec<FieldInfo> {
                match fields {
                    Fields::Named(named_fields) => {
                        named_fields.named.iter().filter_map(|field| {
                            field.ident.as_ref().map(|ident| {
                                let name = ident.to_string();
                                let ty = Self::type_to_string(&field.ty);
                                let purpose = Self::infer_field_purpose(&name, &ty);
                                let considerations = Self::field_considerations(&name, &ty);
                                FieldInfo { name, ty, purpose, considerations }
                            })
                        }).collect()
                    }
                    _ => Vec::new(),
                }
            }

            fn extract_trait_methods(node: &ItemTrait) -> Vec<String> {
                node.items.iter().filter_map(|item| {
                    match item {
                        syn::TraitItem::Fn(method) => Some(method.sig.ident.to_string()),
                        _ => None,
                    }
                }).collect()
            }

            fn type_to_string(ty: &Type) -> String {
                match ty {
                    Type::Path(type_path) => {
                        type_path.path.segments.iter()
                            .map(|seg| seg.ident.to_string())
                            .collect::<Vec<_>>()
                            .join("::")
                    }
                    Type::Reference(type_ref) => {
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

            fn identify_function_patterns(node: &ItemFn) -> Vec<String> {
                let mut patterns = Vec::new();
                let code = node.to_token_stream().to_string();

                if code.contains("match") {
                    patterns.push("Pattern Matching".to_string());
                }
                if code.contains("if let") {
                    patterns.push("If-Let Pattern".to_string());
                }
                if code.contains("map") || code.contains("filter") || code.contains("fold") {
                    patterns.push("Iterator Methods".to_string());
                }
                if code.contains("?") {
                    patterns.push("Error Propagation".to_string());
                }
                if node.sig.asyncness.is_some() {
                    patterns.push("Async Function".to_string());
                }

                patterns
            }

            fn identify_struct_patterns(node: &ItemStruct) -> Vec<String> {
                let mut patterns = Vec::new();

                match &node.fields {
                    Fields::Named(_) => patterns.push("Named Fields".to_string()),
                    Fields::Unnamed(_) => patterns.push("Tuple Struct".to_string()),
                    Fields::Unit => patterns.push("Unit Struct".to_string()),
                }

                // Check for common patterns in attributes
                let attrs = node.attrs.iter()
                    .map(|attr| attr.path().segments.iter()
                        .map(|seg| seg.ident.to_string())
                        .collect::<Vec<_>>()
                        .join("::"))
                    .collect::<Vec<_>>();

                if attrs.contains(&"derive".to_string()) {
                    patterns.push("Derive Macros".to_string());
                }
                if attrs.contains(&"serde".to_string()) {
                    patterns.push("Serialization Support".to_string());
                }

                patterns
            }

            fn infer_parameter_purpose(name: &str, ty: &str) -> String {
                match (name, ty) {
                    ("config" | "settings" | "options", _) => "Configuration parameters".to_string(),
                    ("data" | "input", _) => "Input data to process".to_string(),
                    ("callback" | "handler", _) => "Function to call back".to_string(),
                    (_, "String" | "&str") => "Text input or identifier".to_string(),
                    (_, "i32" | "i64" | "u32" | "u64") => "Numeric value or count".to_string(),
                    (_, "&mut") => "Mutable reference for modification".to_string(),
                    _ => format!("{} parameter of type {}", name, ty),
                }
            }

            fn infer_field_purpose(name: &str, ty: &str) -> String {
                match (name, ty) {
                    ("id" | "uuid", _) => "Unique identifier".to_string(),
                    ("name" | "title", _) => "Descriptive name or title".to_string(),
                    ("config" | "settings", _) => "Configuration data".to_string(),
                    ("data" | "content", _) => "Main content or data".to_string(),
                    ("created_at" | "updated_at", _) => "Timestamp information".to_string(),
                    (_, "Vec<T>") => "Collection of items".to_string(),
                    (_, "Option<T>") => "Optional value".to_string(),
                    _ => format!("{} field of type {}", name, ty),
                }
            }

            fn field_considerations(name: &str, ty: &str) -> Vec<String> {
                let mut considerations = Vec::new();

                if ty.contains("&str") && !ty.contains("String") {
                    considerations.push("Consider using owned String for ownership".to_string());
                }
                if name.ends_with("_at") && ty.contains("u64") {
                    considerations.push("Consider using DateTime for better time handling".to_string());
                }
                if name == "id" && ty == "String" {
                    considerations.push("Consider using UUID type for better type safety".to_string());
                }

                considerations
            }

            fn suggest_function_improvements(node: &ItemFn) -> Vec<String> {
                let mut improvements = Vec::new();
                let code = node.to_token_stream().to_string();

                if node.sig.inputs.len() > 4 {
                    improvements.push("Consider grouping parameters into a configuration struct".to_string());
                }

                if code.len() > 1000 {
                    improvements.push("Function is quite long - consider breaking into smaller functions".to_string());
                }

                if !code.contains("Result") && !code.contains("Option") {
                    improvements.push("Consider adding error handling for robustness".to_string());
                }

                improvements
            }

            fn struct_design_considerations(node: &ItemStruct) -> Vec<String> {
                let mut considerations = Vec::new();

                match &node.fields {
                    Fields::Named(named_fields) if named_fields.named.len() > 10 => {
                        considerations.push("Large struct - consider splitting into smaller structs".to_string());
                    }
                    Fields::Unnamed(unnamed_fields) if unnamed_fields.unnamed.len() > 5 => {
                        considerations.push("Many tuple fields - consider using named fields for clarity".to_string());
                    }
                    _ => {}
                }

                considerations
            }

            fn identify_trait_purpose(node: &ItemTrait) -> String {
                let name = node.ident.to_string();
                let method_count = node.items.len();

                match (name.as_str(), method_count) {
                    ("Debug", _) => "Enable debug printing and inspection".to_string(),
                    ("Clone", _) => "Allow creating copies of values".to_string(),
                    ("Display", _) => "Enable user-friendly string representation".to_string(),
                    ("From" | "Into", _) => "Enable type conversions".to_string(),
                    ("Iterator", _) => "Enable iteration over collections".to_string(),
                    (_, 1) => "Single method interface - likely a callback or conversion trait".to_string(),
                    (_, 2..=3) => "Small interface - focused on specific functionality".to_string(),
                    _ => "Larger interface - represents a capability or behavior".to_string(),
                }
            }

            fn trait_use_cases(node: &ItemTrait) -> Vec<String> {
                let name = node.ident.to_string();

                match name.as_str() {
                    "Debug" => vec!["Debugging and logging".to_string(), "Error messages".to_string()],
                    "Clone" => vec!["Creating copies of data".to_string(), "Working with collections".to_string()],
                    "Display" => vec!["User-facing output".to_string(), "Logging and reporting".to_string()],
                    "From" | "Into" => vec!["Type conversions".to_string(), "Builder patterns".to_string()],
                    "Iterator" => vec!["Working with collections".to_string(), "Streaming data".to_string()],
                    _ => vec!["Domain-specific functionality".to_string()],
                }
            }

            fn explain_function(node: &ItemFn) -> String {
                let name = node.sig.ident.to_string();
                let param_count = node.sig.inputs.len();
                let has_return = !matches!(node.sig.output, syn::ReturnType::Default);
                let is_async = node.sig.asyncness.is_some();

                let mut explanation = format!("Function `{}` ", name);

                if is_async {
                    explanation.push_str("is an asynchronous function that ");
                } else {
                    explanation.push_str("is a synchronous function that ");
                }

                if has_return {
                    explanation.push_str("takes ");
                } else {
                    explanation.push_str("performs an operation");
                }

                match param_count {
                    0 => explanation.push_str("no parameters and "),
                    1 => explanation.push_str("one parameter and "),
                    2..=3 => explanation.push_str(&format!("{} parameters and ", param_count)),
                    _ => explanation.push_str(&format!("{} parameters and ", param_count)),
                }

                if has_return {
                    explanation.push_str("returns a value");
                } else {
                    explanation.push_str("doesn't return a value");
                }

                explanation.push_str(". ");

                if node.sig.inputs.iter().any(|arg| matches!(arg, syn::FnArg::Receiver(_))) {
                    explanation.push_str("It operates on an instance of its type (method). ");
                } else {
                    explanation.push_str("It operates as a standalone function. ");
                }

                explanation
            }

            fn explain_struct(node: &ItemStruct) -> String {
                let name = node.ident.to_string();

                match &node.fields {
                    Fields::Named(named_fields) => {
                        let field_count = named_fields.named.len();
                        format!("Struct `{}` has {} named fields, representing a data structure with clear field names for better code readability and maintainability.", name, field_count)
                    }
                    Fields::Unnamed(unnamed_fields) => {
                        let field_count = unnamed_fields.unnamed.len();
                        format!("Struct `{}` is a tuple struct with {} unnamed fields, useful for simple data aggregation where field names aren't needed.", name, field_count)
                    }
                    Fields::Unit => {
                        format!("Struct `{}` is a unit struct with no fields, often used as a marker type or for implementing traits.", name)
                    }
                }
            }

            fn explain_trait(node: &ItemTrait) -> String {
                let name = node.ident.to_string();
                let method_count = node.items.len();

                format!("Trait `{}` defines an interface with {} methods that types can implement to provide specific functionality. It represents a capability that implementing types must provide.", name, method_count)
            }
        }

        let mut visitor = CodeVisitor {
            functions,
            structs,
            traits,
            current_file: file_path.to_string(),
        };
        syn::visit::visit_file(&mut visitor, &ast);

        Ok(())
    }

    fn identify_patterns(&self, functions: &[FunctionAnalysis], structs: &[StructAnalysis], traits: &[TraitAnalysis]) -> Vec<PatternAnalysis> {
        let mut patterns = Vec::new();

        // Analyze function patterns
        let mut pattern_usage = HashMap::new();
        for func in functions {
            for pattern in &func.patterns_used {
                let count = pattern_usage.entry(pattern.clone()).or_insert(0);
                *count += 1;
            }
        }

        for (pattern, count) in pattern_usage {
            if count > 0 {
                patterns.push(PatternAnalysis {
                    pattern_type: pattern.clone(),
                    locations: vec![format!("Found in {} functions", count)],
                    explanation: self.explain_pattern(&pattern),
                    benefits: self.pattern_benefits(&pattern),
                    alternatives: self.pattern_alternatives(&pattern),
                });
            }
        }

        patterns
    }

    fn identify_issues(&self, functions: &[FunctionAnalysis], structs: &[StructAnalysis], traits: &[TraitAnalysis]) -> Vec<CodeIssue> {
        let mut issues = Vec::new();

        // Check for high complexity functions
        for func in functions {
            if func.complexity > 10 {
                issues.push(CodeIssue {
                    severity: "warning".to_string(),
                    category: "complexity".to_string(),
                    location: func.name.clone(),
                    message: format!("High complexity function (score: {})", func.complexity),
                    explanation: "Functions with high complexity are harder to understand and maintain".to_string(),
                    suggestion: "Consider breaking into smaller functions or simplifying logic".to_string(),
                });
            }
        }

        // Check for structs with many fields
        for struct_info in structs {
            if struct_info.fields.len() > 15 {
                issues.push(CodeIssue {
                    severity: "info".to_string(),
                    category: "design".to_string(),
                    location: struct_info.name.clone(),
                    message: format!("Large struct with {} fields", struct_info.fields.len()),
                    explanation: "Large structs can be difficult to work with and may indicate a need for better organization".to_string(),
                    suggestion: "Consider splitting into smaller, more focused structs".to_string(),
                });
            }
        }

        issues
    }

    fn generate_recommendations(&self, functions: &[FunctionAnalysis], structs: &[StructAnalysis], traits: &[TraitAnalysis], issues: &[CodeIssue]) -> Vec<Recommendation> {
        let mut recommendations = Vec::new();

        // Add recommendations based on issues
        for issue in issues {
            recommendations.push(Recommendation {
                category: issue.category.clone(),
                priority: match issue.severity.as_str() {
                    "error" => "high".to_string(),
                    "warning" => "medium".to_string(),
                    _ => "low".to_string(),
                },
                title: format!("Address {}", issue.category),
                description: issue.explanation.clone(),
                code_example: None,
                benefits: vec![
                    "Improved code maintainability".to_string(),
                    "Better developer experience".to_string(),
                    "Reduced bug likelihood".to_string(),
                ],
            });
        }

        // General recommendations
        if functions.iter().any(|f| f.patterns_used.contains(&"Error Propagation".to_string())) {
            recommendations.push(Recommendation {
                category: "error_handling".to_string(),
                priority: "medium".to_string(),
                title: "Consider using thiserror for better error handling".to_string(),
                description: "Using thiserror provides better error messages and easier error handling".to_string(),
                code_example: Some("#[derive(thiserror::Error, Debug)]\npub enum AppError {\n    #[error(\"IO error: {0}\")]\n    Io(#[from] std::io::Error),\n    #[error(\"Parse error: {0}\")]\n    Parse(String),\n}".to_string()),
                benefits: vec![
                    "Better error messages".to_string(),
                    "Easier error handling".to_string(),
                    "Consistent error types".to_string(),
                ],
            });
        }

        if structs.iter().any(|s| s.patterns_used.contains(&"Serialization Support".to_string())) {
            recommendations.push(Recommendation {
                category: "serialization".to_string(),
                priority: "low".to_string(),
                title: "Consider adding validation for serialized data".to_string(),
                description: "Adding validation ensures data integrity when serializing/deserializing".to_string(),
                code_example: Some("#[derive(serde::Deserialize, validator::Validate)]\npub struct User {\n    #[validate(length(min = 1, max = 100))]\n    pub name: String,\n    #[validate(email)]\n    pub email: String,\n}".to_string()),
                benefits: vec![
                    "Data integrity".to_string(),
                    "Better error messages".to_string(),
                    "Security improvements".to_string(),
                ],
            });
        }

        recommendations
    }

    fn identify_learning_opportunities(&self, functions: &[FunctionAnalysis], structs: &[StructAnalysis], traits: &[TraitAnalysis]) -> Vec<LearningOpportunity> {
        let mut opportunities = Vec::new();

        // Check for async functions
        let async_functions = functions.iter().filter(|f| f.patterns_used.contains(&"Async Function".to_string())).count();
        if async_functions == 0 {
            opportunities.push(LearningOpportunity {
                topic: "Asynchronous Programming".to_string(),
                current_level: "Beginner".to_string(),
                next_steps: vec![
                    "Learn async/await syntax".to_string(),
                    "Understand futures and promises".to_string(),
                    "Practice with tokio runtime".to_string(),
                ],
                resources: vec![
                    "https://rust-lang.github.io/async-book/".to_string(),
                    "https://tokio.rs/".to_string(),
                ],
                explanation: "Async programming is becoming increasingly important in Rust for building scalable applications".to_string(),
            });
        }

        // Check for error handling patterns
        let error_handling = functions.iter().filter(|f| f.patterns_used.contains(&"Error Propagation".to_string())).count();
        if error_handling == 0 {
            opportunities.push(LearningOpportunity {
                topic: "Error Handling".to_string(),
                current_level: "Beginner".to_string(),
                next_steps: vec![
                    "Learn Result and Option types".to_string(),
                    "Use ? operator for error propagation".to_string(),
                    "Create custom error types".to_string(),
                    "Use thiserror for better errors".to_string(),
                ],
                resources: vec![
                    "https://doc.rust-lang.org/book/ch09-00-error-handling.html".to_string(),
                    "https://docs.rs/thiserror/latest/thiserror/".to_string(),
                ],
                explanation: "Proper error handling is crucial for robust Rust applications".to_string(),
            });
        }

        // Check for iterator usage
        let iterator_usage = functions.iter().filter(|f| f.patterns_used.contains(&"Iterator Methods".to_string())).count();
        if iterator_usage == 0 {
            opportunities.push(LearningOpportunity {
                topic: "Iterator Patterns".to_string(),
                current_level: "Beginner".to_string(),
                next_steps: vec![
                    "Learn map, filter, fold methods".to_string(),
                    "Understand iterator chains".to_string(),
                    "Create custom iterators".to_string(),
                ],
                resources: vec![
                    "https://doc.rust-lang.org/book/ch13-02-iterators.html".to_string(),
                ],
                explanation: "Iterators provide powerful and efficient ways to work with collections".to_string(),
            });
        }

        opportunities
    }

    fn explain_pattern(&self, pattern: &str) -> String {
        match pattern {
            "Pattern Matching" => "Pattern matching allows you to destructure and match values against patterns, providing a powerful way to handle different cases in your code.".to_string(),
            "If-Let Pattern" => "If-let is a concise way to handle optional values and single-case matches, reducing boilerplate compared to full match statements.".to_string(),
            "Iterator Methods" => "Iterator methods like map, filter, and fold provide functional programming patterns that make data transformation more concise and readable.".to_string(),
            "Error Propagation" => "The ? operator provides concise error propagation, automatically converting errors to the expected return type.".to_string(),
            "Async Function" => "Async functions allow you to write asynchronous code that looks like synchronous code, making concurrent programming more accessible.".to_string(),
            _ => format!("{} is a code pattern used in this codebase.", pattern),
        }
    }

    fn pattern_benefits(&self, pattern: &str) -> Vec<String> {
        match pattern {
            "Pattern Matching" => vec![
                "Exhaustive checking prevents bugs".to_string(),
                "Clear expression of intent".to_string(),
                "Powerful destructuring capabilities".to_string(),
            ],
            "Iterator Methods" => vec![
                "More concise and readable code".to_string(),
                "Composable operations".to_string(),
                "Lazy evaluation for performance".to_string(),
            ],
            "Error Propagation" => vec![
                "Reduces boilerplate error handling".to_string(),
                "Makes error paths more visible".to_string(),
                "Enforces error handling".to_string(),
            ],
            _ => vec!["Improves code quality".to_string()],
        }
    }

    fn pattern_alternatives(&self, pattern: &str) -> Vec<String> {
        match pattern {
            "Pattern Matching" => vec![
                "If-else chains".to_string(),
                "Method dispatch".to_string(),
            ],
            "Iterator Methods" => vec![
                "Traditional loops".to_string(),
                "Manual collection building".to_string(),
            ],
            "Error Propagation" => vec![
                "Manual match statements".to_string(),
                "Panic on error".to_string(),
            ],
            _ => vec!["Alternative approaches available".to_string()],
        }
    }
}

impl Tool for RustMentorTool {
    fn name(&self) -> &'static str {
        "rust-mentor"
    }

    fn description(&self) -> &'static str {
        "Interactive learning and guidance system for Rust developers"
    }

    fn command(&self) -> Command {
        Command::new(self.name())
            .about(self.description())
            .long_about("An interactive learning and guidance system that analyzes your Rust code and provides personalized explanations, best practice suggestions, and learning recommendations. Perfect for developers at all levels looking to improve their Rust skills.")
            .args(&[
                Arg::new("input")
                    .long("input")
                    .short('i')
                    .help("Input Rust file or directory to analyze")
                    .required(true),
                Arg::new("explain")
                    .long("explain")
                    .short('e')
                    .help("Explain what specific code constructs do"),
                Arg::new("suggest")
                    .long("suggest")
                    .help("Show improvement suggestions")
                    .action(clap::ArgAction::SetTrue),
                Arg::new("learn")
                    .long("learn")
                    .help("Show learning opportunities")
                    .action(clap::ArgAction::SetTrue),
                Arg::new("patterns")
                    .long("patterns")
                    .help("Analyze code patterns used")
                    .action(clap::ArgAction::SetTrue),
                Arg::new("focus")
                    .long("focus")
                    .short('f')
                    .help("Focus on specific aspects (functions, structs, traits)")
                    .default_value("all"),
                Arg::new("level")
                    .long("level")
                    .help("Experience level (beginner, intermediate, advanced)")
                    .default_value("intermediate"),
                Arg::new("detailed")
                    .long("detailed")
                    .short('d')
                    .help("Show detailed explanations")
                    .action(clap::ArgAction::SetTrue),
            ])
            .args(&common_options())
    }

    fn execute(&self, matches: &ArgMatches) -> Result<()> {
        let input = matches.get_one::<String>("input").unwrap();
        let explain_target = matches.get_one::<String>("explain");
        let suggest = matches.get_flag("suggest");
        let learn = matches.get_flag("learn");
        let patterns = matches.get_flag("patterns");
        let focus = matches.get_one::<String>("focus").unwrap();
        let level = matches.get_one::<String>("level").unwrap();
        let detailed = matches.get_flag("detailed");
        let dry_run = matches.get_flag("dry-run");
        let verbose = matches.get_flag("verbose");
        let output_format = parse_output_format(matches);

        println!("🎓 {} - {}", "CargoMate RustMentor".bold().blue(), self.description().cyan());

        if !Path::new(input).exists() {
            return Err(ToolError::InvalidArguments(format!("Input not found: {}", input)));
        }

        if verbose {
            println!("   📚 Analyzing codebase for learning opportunities...");
        }

        // Analyze the codebase
        let analysis = self.analyze_codebase(input)?;

        if verbose {
            println!("   📊 Found {} functions, {} structs, {} traits", analysis.functions.len(), analysis.structs.len(), analysis.traits.len());
            println!("   🔍 Identified {} patterns, {} issues, {} recommendations", analysis.patterns.len(), analysis.issues.len(), analysis.recommendations.len());
        }

        match output_format {
            OutputFormat::Human => {
                self.display_human_analysis(&analysis, explain_target, suggest, learn, patterns, focus, level, detailed);
            }
            OutputFormat::Json => {
                let json_analysis = serde_json::to_string_pretty(&analysis)?;
                println!("{}", json_analysis);
            }
            OutputFormat::Table => {
                self.display_table_analysis(&analysis);
            }
        }

        println!("\n🎉 Learning analysis complete! Use the insights above to improve your Rust skills.");

        Ok(())
    }
}

impl RustMentorTool {
    fn display_human_analysis(&self, analysis: &CodeAnalysis, explain_target: Option<&String>,
                             suggest: bool, learn: bool, patterns: bool, focus: &str, level: &str, detailed: bool) {
        println!("\n🧠 {}", "Rust Learning Analysis".bold().underline());

        // Overview
        println!("\n📊 {}", "Codebase Overview".bold());
        println!("   Functions: {}", analysis.functions.len());
        println!("   Structs: {}", analysis.structs.len());
        println!("   Traits: {}", analysis.traits.len());
        println!("   Code Issues: {}", analysis.issues.len());
        println!("   Recommendations: {}", analysis.recommendations.len());
        println!("   Learning Opportunities: {}", analysis.learning_opportunities.len());

        // Focus on specific aspects
        match focus {
            "functions" => self.display_functions(&analysis.functions, detailed),
            "structs" => self.display_structs(&analysis.structs, detailed),
            "traits" => self.display_traits(&analysis.traits, detailed),
            _ => {
                self.display_functions(&analysis.functions, detailed);
                self.display_structs(&analysis.structs, detailed);
                self.display_traits(&analysis.traits, detailed);
            }
        }

        // Explain specific target if requested
        if let Some(target) = explain_target {
            self.explain_specific_target(analysis, target);
        }

        // Show patterns if requested
        if patterns {
            self.display_patterns(&analysis.patterns);
        }

        // Show suggestions if requested
        if suggest {
            self.display_recommendations(&analysis.recommendations);
        }

        // Show learning opportunities if requested
        if learn {
            self.display_learning_opportunities(&analysis.learning_opportunities, level);
        }

        // Show issues
        if !analysis.issues.is_empty() {
            self.display_issues(&analysis.issues);
        }
    }

    fn display_functions(&self, functions: &[FunctionAnalysis], detailed: bool) {
        if functions.is_empty() {
            return;
        }

        println!("\n📝 {}", "Function Analysis".bold());
        for func in functions {
            println!("   🔹 {}", func.name.bold());
            println!("      {}", func.explanation);

            if detailed {
                if func.complexity > 1 {
                    println!("      Complexity: {} (higher = more complex)", func.complexity);
                }
                if !func.patterns_used.is_empty() {
                    println!("      Patterns: {}", func.patterns_used.join(", "));
                }
                if !func.potential_improvements.is_empty() {
                    println!("      💡 Suggestions:");
                    for suggestion in &func.potential_improvements {
                        println!("         • {}", suggestion);
                    }
                }
            }
            println!();
        }
    }

    fn display_structs(&self, structs: &[StructAnalysis], detailed: bool) {
        if structs.is_empty() {
            return;
        }

        println!("\n🏗️  {}", "Struct Analysis".bold());
        for struct_info in structs {
            println!("   🏛️  {}", struct_info.name.bold());
            println!("      {}", struct_info.explanation);

            if detailed {
                if !struct_info.patterns_used.is_empty() {
                    println!("      Patterns: {}", struct_info.patterns_used.join(", "));
                }
                if !struct_info.design_considerations.is_empty() {
                    println!("      💡 Design Notes:");
                    for consideration in &struct_info.design_considerations {
                        println!("         • {}", consideration);
                    }
                }
            }
            println!();
        }
    }

    fn display_traits(&self, traits: &[TraitAnalysis], detailed: bool) {
        if traits.is_empty() {
            return;
        }

        println!("\n🎭 {}", "Trait Analysis".bold());
        for trait_info in traits {
            println!("   🎪 {}", trait_info.name.bold());
            println!("      {}", trait_info.explanation);
            println!("      Purpose: {}", trait_info.purpose);

            if detailed {
                if !trait_info.methods.is_empty() {
                    println!("      Methods: {}", trait_info.methods.join(", "));
                }
                if !trait_info.common_use_cases.is_empty() {
                    println!("      Use Cases: {}", trait_info.common_use_cases.join(", "));
                }
            }
            println!();
        }
    }

    fn explain_specific_target(&self, analysis: &CodeAnalysis, target: &str) {
        println!("\n🔍 {}", format!("Deep Dive: {}", target).bold());

        // Look for the target in functions, structs, or traits
        for func in &analysis.functions {
            if func.name == target {
                println!("   Type: Function");
                println!("   {}", func.explanation);
                if !func.patterns_used.is_empty() {
                    println!("   Patterns Used: {}", func.patterns_used.join(", "));
                }
                return;
            }
        }

        for struct_info in &analysis.structs {
            if struct_info.name == target {
                println!("   Type: Struct");
                println!("   {}", struct_info.explanation);
                if !struct_info.patterns_used.is_empty() {
                    println!("   Patterns Used: {}", struct_info.patterns_used.join(", "));
                }
                return;
            }
        }

        for trait_info in &analysis.traits {
            if trait_info.name == target {
                println!("   Type: Trait");
                println!("   {}", trait_info.explanation);
                println!("   Purpose: {}", trait_info.purpose);
                return;
            }
        }

        println!("   ❌ Target '{}' not found in the analyzed code.", target);
    }

    fn display_patterns(&self, patterns: &[PatternAnalysis]) {
        if patterns.is_empty() {
            return;
        }

        println!("\n🎨 {}", "Code Patterns Used".bold());
        for pattern in patterns {
            println!("   🎭 {}", pattern.pattern_type.bold());
            println!("      {}", pattern.explanation);
            println!("      Benefits: {}", pattern.benefits.join(", "));
            if !pattern.alternatives.is_empty() {
                println!("      Alternatives: {}", pattern.alternatives.join(", "));
            }
            println!();
        }
    }

    fn display_recommendations(&self, recommendations: &[Recommendation]) {
        if recommendations.is_empty() {
            return;
        }

        println!("\n💡 {}", "Recommendations".bold());
        for recommendation in recommendations {
            let priority_icon = match recommendation.priority.as_str() {
                "high" => "🔴",
                "medium" => "🟡",
                "low" => "🟢",
                _ => "⚪",
            };
            println!("   {} {}", priority_icon, recommendation.title.bold());
            println!("      {}", recommendation.description);
            if let Some(code) = &recommendation.code_example {
                println!("      ```rust");
                for line in code.lines() {
                    println!("      {}", line);
                }
                println!("      ```");
            }
            if !recommendation.benefits.is_empty() {
                println!("      Benefits: {}", recommendation.benefits.join(", "));
            }
            println!();
        }
    }

    fn display_learning_opportunities(&self, opportunities: &[LearningOpportunity], level: &str) {
        if opportunities.is_empty() {
            return;
        }

        println!("\n🎓 {}", "Learning Opportunities".bold());
        println!("   Tailored for {} Rust developers:", level);

        for opportunity in opportunities {
            println!("   📚 {}", opportunity.topic.bold());
            println!("      {}", opportunity.explanation);
            println!("      Current Level: {}", opportunity.current_level);

            if !opportunity.next_steps.is_empty() {
                println!("      Next Steps:");
                for step in &opportunity.next_steps {
                    println!("         • {}", step);
                }
            }

            if !opportunity.resources.is_empty() {
                println!("      Resources:");
                for resource in &opportunity.resources {
                    println!("         • {}", resource.cyan());
                }
            }
            println!();
        }
    }

    fn display_issues(&self, issues: &[CodeIssue]) {
        println!("\n⚠️  {}", "Code Issues Found".bold());
        for issue in issues {
            let severity_icon = match issue.severity.as_str() {
                "error" => "❌",
                "warning" => "⚠️ ",
                "info" => "ℹ️ ",
                _ => "•",
            };
            println!("   {} {}", severity_icon, issue.message.bold());
            println!("      Location: {}", issue.location);
            println!("      {}", issue.explanation);
            println!("      💡 {}", issue.suggestion);
            println!();
        }
    }

    fn display_table_analysis(&self, analysis: &CodeAnalysis) {
        println!("{:<20} {:<10} {:<10} {:<10} {:<10}",
                 "Category", "Count", "Patterns", "Issues", "Learning");
        println!("{}", "─".repeat(70));

        println!("{:<20} {:<10} {:<10} {:<10} {:<10}",
                 "Functions", analysis.functions.len(), "-", "-", "-");
        println!("{:<20} {:<10} {:<10} {:<10} {:<10}",
                 "Structs", analysis.structs.len(), "-", "-", "-");
        println!("{:<20} {:<10} {:<10} {:<10} {:<10}",
                 "Traits", analysis.traits.len(), "-", "-", "-");
        println!("{:<20} {:<10} {:<10} {:<10} {:<10}",
                 "Code Patterns", "-", analysis.patterns.len(), "-", "-");
        println!("{:<20} {:<10} {:<10} {:<10} {:<10}",
                 "Issues", "-", "-", analysis.issues.len(), "-");
        println!("{:<20} {:<10} {:<10} {:<10} {:<10}",
                 "Recommendations", "-", "-", "-", analysis.recommendations.len());
        println!("{:<20} {:<10} {:<10} {:<10} {:<10}",
                 "Learning Ops", "-", "-", "-", analysis.learning_opportunities.len());
    }
}

impl Default for RustMentorTool {
    fn default() -> Self {
        Self::new()
    }
}
