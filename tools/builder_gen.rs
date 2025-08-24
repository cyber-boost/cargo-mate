use super::{Tool, Result, ToolError, common_options, parse_output_format, OutputFormat};
use clap::{Arg, ArgMatches, Command};
use colored::*;
use std::fs;
use std::path::Path;
use syn::{parse_file, File, Item, ItemStruct, Fields, Field, Type, PathSegment, Ident, visit::Visit};
use quote::quote;
use proc_macro2::TokenStream;

#[derive(Debug, Clone)]
pub struct BuilderGenTool;

#[derive(Debug, Clone)]
struct StructInfo {
    name: String,
    fields: Vec<FieldInfo>,
}

#[derive(Debug, Clone)]
struct FieldInfo {
    name: String,
    ty: String,
    is_optional: bool,
    has_default: bool,
    validation_rules: Vec<String>,
}

impl BuilderGenTool {
    pub fn new() -> Self {
        Self
    }

    fn parse_struct_from_file(&self, file_path: &str) -> Result<Vec<StructInfo>> {
        let content = fs::read_to_string(file_path)
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to read {}: {}", file_path, e)))?;

        let ast = parse_file(&content)
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to parse {}: {}", file_path, e)))?;

        let mut structs = Vec::new();

        for item in ast.items {
            if let Item::Struct(struct_item) = item {
                if let Some(struct_info) = self.parse_struct_item(&struct_item) {
                    structs.push(struct_info);
                }
            }
        }

        Ok(structs)
    }

    fn parse_struct_item(&self, struct_item: &ItemStruct) -> Option<StructInfo> {
        let name = struct_item.ident.to_string();
        let mut fields = Vec::new();

        if let Fields::Named(named_fields) = &struct_item.fields {
            for field in &named_fields.named {
                if let Some(field_info) = self.parse_field(field) {
                    fields.push(field_info);
                }
            }
        }

        Some(StructInfo { name, fields })
    }

    fn parse_field(&self, field: &Field) -> Option<FieldInfo> {
        let name = field.ident.as_ref()?.to_string();
        let ty = self.type_to_string(&field.ty);
        let is_optional = self.is_optional_type(&field.ty);
        let has_default = self.has_default_value(field);

        // Extract validation rules from attributes
        let validation_rules = self.extract_validation_rules(field);

        Some(FieldInfo {
            name,
            ty,
            is_optional,
            has_default,
            validation_rules,
        })
    }

    fn type_to_string(&self, ty: &Type) -> String {
        match ty {
            Type::Path(type_path) => {
                let mut segments = Vec::new();
                for segment in &type_path.path.segments {
                    segments.push(segment.ident.to_string());
                }
                segments.join("::")
            }
            Type::Reference(type_ref) => {
                let mut result = "&".to_string();
                if type_ref.mutability.is_some() {
                    result.push_str("mut ");
                }
                result.push_str(&self.type_to_string(&*type_ref.elem));
                result
            }
            _ => "Unknown".to_string(),
        }
    }

    fn is_optional_type(&self, ty: &Type) -> bool {
        if let Type::Path(type_path) = ty {
            if let Some(segment) = type_path.path.segments.last() {
                return segment.ident == "Option";
            }
        }
        false
    }

    fn has_default_value(&self, field: &Field) -> bool {
        // Check for #[builder(default)] attribute or similar
        field.attrs.iter().any(|attr| {
            attr.path().segments.iter().any(|seg| {
                seg.ident == "builder" || seg.ident == "default"
            })
        })
    }

    fn extract_validation_rules(&self, field: &Field) -> Vec<String> {
        let mut rules = Vec::new();

        for attr in &field.attrs {
            if let Some(last_seg) = attr.path().segments.last() {
                if last_seg.ident == "validate" {
                    // Parse validation rules from attribute using new syn API
                    if let Ok(()) = attr.parse_nested_meta(|meta| {
                        self.extract_validation_from_nested_meta(&meta, &mut rules)
                    }) {
                        // Successfully parsed
                    }
                }
            }
        }

        rules
    }

    fn extract_validation_from_nested_meta(&self, meta: &syn::meta::ParseNestedMeta, rules: &mut Vec<String>) -> syn::Result<()> {
        if meta.path.is_ident("length") {
            let value = meta.value()?.parse::<syn::LitInt>()?;
            rules.push(format!("length = {}", value.base10_digits()));
        } else if meta.path.is_ident("range") {
            let value = meta.value()?.parse::<syn::Expr>()?;
            rules.push(format!("range = {}", self.expr_to_string(&value)));
        } else {
            // For simple validation rules without values
            if let Some(ident) = meta.path.get_ident() {
                rules.push(ident.to_string());
            }
        }
        Ok(())
    }

    fn extract_validation_from_meta(&self, meta: &syn::Meta, rules: &mut Vec<String>) {
        match meta {
            syn::Meta::List(meta_list) => {
                // For now, we'll skip complex nested meta parsing
                // In a full implementation, you'd parse meta_list.tokens
                let _tokens = &meta_list.tokens;
            }
            syn::Meta::NameValue(name_value) => {
                if let Some(ident) = name_value.path.get_ident() {
                    let rule = format!("{} = {}", ident, self.expr_to_string(&name_value.value));
                    rules.push(rule);
                }
            }
            syn::Meta::Path(path) => {
                if let Some(ident) = path.get_ident() {
                    rules.push(ident.to_string());
                }
            }
        }
    }

    fn expr_to_string(&self, expr: &syn::Expr) -> String {
        match expr {
            syn::Expr::Lit(lit) => {
                match &lit.lit {
                    syn::Lit::Str(s) => format!("\"{}\"", s.value()),
                    syn::Lit::Int(i) => i.base10_digits().to_string(),
                    syn::Lit::Bool(b) => b.value.to_string(),
                    _ => "unknown".to_string(),
                }
            }
            _ => "unknown".to_string(),
        }
    }

    fn generate_builder_struct(&self, struct_info: &StructInfo, derive_attrs: &str) -> Result<String> {
        let struct_name = &struct_info.name;
        let builder_name = format!("{}Builder", struct_name);

        let mut code = format!("// Builder for {}\n", struct_name);

        // Generate derive attributes
        if !derive_attrs.is_empty() {
            code.push_str(&format!("#[derive({})]\n", derive_attrs));
        } else {
            code.push_str("#[derive(Debug, Clone)]\n");
        }

        code.push_str(&format!("pub struct {} {{\n", builder_name));

        for field in &struct_info.fields {
            let field_type = if field.is_optional {
                field.ty.clone()
            } else {
                format!("Option<{}>", field.ty)
            };

            code.push_str(&format!("    {}: {},\n", field.name, field_type));
        }

        code.push_str("}\n\n");

        Ok(code)
    }

    fn generate_builder_impl(&self, struct_info: &StructInfo, method_prefix: &str) -> Result<String> {
        let struct_name = &struct_info.name;
        let builder_name = format!("{}Builder", struct_name);

        let mut code = format!("impl {} {{\n", builder_name);

        // Generate new() method
        code.push_str("    pub fn new() -> Self {\n");
        code.push_str(&format!("        {} {{\n", builder_name));

        for field in &struct_info.fields {
            let default_value = if field.has_default {
                self.get_default_value_for_type(&field.ty)
            } else if field.is_optional {
                "None".to_string()
            } else {
                format!("None::<{}>", field.ty)
            };

            code.push_str(&format!("            {}: {},\n", field.name, default_value));
        }

        code.push_str("        }\n");
        code.push_str("    }\n\n");

        // Generate setter methods
        for field in &struct_info.fields {
            let method_name = format!("{}{}", method_prefix, field.name);
            let param_type = field.ty.clone();
            let field_assignment = if field.is_optional {
                format!("Some({})", field.name)
            } else {
                field.name.clone()
            };

            code.push_str(&format!("    pub fn {}(mut self, {}: {}) -> Self {{\n", method_name, field.name, param_type));
            code.push_str(&format!("        self.{} = {};\n", field.name, field_assignment));
            code.push_str("        self\n");
            code.push_str("    }\n\n");
        }

        // Generate build method
        code.push_str("    pub fn build(self) -> Result<");
        code.push_str(struct_name);
        code.push_str(", BuilderError> {\n");

        // Generate validation code
        for field in &struct_info.fields {
            if !field.is_optional && !field.has_default {
                code.push_str(&format!("        let {} = self.{}.ok_or(BuilderError::MissingField(\"{}\"))?;\n",
                    field.name, field.name, field.name));
            } else if field.is_optional {
                code.push_str(&format!("        let {} = self.{};\n", field.name, field.name));
            } else {
                code.push_str(&format!("        let {} = self.{}.unwrap_or_else(|| {});\n",
                    field.name, field.name, self.get_default_value_for_type(&field.ty)));
            }
        }

        // Generate struct construction
        code.push_str(&format!("        let result = {} {{\n", struct_name));
        for field in &struct_info.fields {
            code.push_str(&format!("            {},\n", field.name));
        }
        code.push_str("        };\n\n");

        // Add validation if needed
        code.push_str("        // Add custom validation here\n");
        code.push_str("        // if !is_valid(&result) { return Err(BuilderError::ValidationError(\"...\".to_string())); }\n\n");

        code.push_str("        Ok(result)\n");
        code.push_str("    }\n");

        code.push_str("}\n\n");

        Ok(code)
    }

    fn generate_validation_code(&self, struct_info: &StructInfo) -> Result<String> {
        let mut code = "// Builder validation error types\n\n".to_string();

        code.push_str("#[derive(Debug, Clone)]\n");
        code.push_str("pub enum BuilderError {\n");
        code.push_str("    MissingField(&'static str),\n");
        code.push_str("    ValidationError(String),\n");
        code.push_str("    InvalidValue(String),\n");
        code.push_str("}\n\n");

        code.push_str("impl std::fmt::Display for BuilderError {\n");
        code.push_str("    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {\n");
        code.push_str("        match self {\n");
        code.push_str("            BuilderError::MissingField(field) => write!(f, \"Missing required field: {}\", field),\n");
        code.push_str("            BuilderError::ValidationError(msg) => write!(f, \"Validation error: {}\", msg),\n");
        code.push_str("            BuilderError::InvalidValue(msg) => write!(f, \"Invalid value: {}\", msg),\n");
        code.push_str("        }\n");
        code.push_str("    }\n");
        code.push_str("}\n\n");

        code.push_str("impl std::error::Error for BuilderError {}\n\n");

        // Generate validation functions for complex rules
        for field in &struct_info.fields {
            if !field.validation_rules.is_empty() {
                code.push_str(&format!("// Validation for {} field\n", field.name));
                code.push_str(&format!("fn validate_{}(value: &{}) -> Result<(), BuilderError> {{\n",
                    field.name, field.ty));

                for rule in &field.validation_rules {
                    if rule.contains("length") {
                        code.push_str("    // Length validation\n");
                        code.push_str("    if value.to_string().len() < 1 {\n");
                        code.push_str("        return Err(BuilderError::ValidationError(\"Field cannot be empty\".to_string()));\n");
                        code.push_str("    }\n");
                    } else if rule.contains("range") {
                        code.push_str("    // Range validation\n");
                        code.push_str("    // Add range checks here\n");
                    }
                }

                code.push_str("    Ok(())\n");
                code.push_str("}\n\n");
            }
        }

        Ok(code)
    }

    fn generate_conversion_impl(&self, struct_info: &StructInfo) -> Result<String> {
        let struct_name = &struct_info.name;
        let builder_name = format!("{}Builder", struct_name);

        let mut code = format!("// Conversion implementations for {}\n\n", struct_name);

        // Generate From builder
        code.push_str(&format!("impl From<{}> for {} {{\n", builder_name, struct_name));
        code.push_str(&format!("    fn from(builder: {}) -> Self {{\n", builder_name));
        code.push_str("        builder.build().unwrap_or_else(|_| panic!(\"Failed to build from valid builder\"))\n");
        code.push_str("    }\n");
        code.push_str("}\n\n");

        // Generate TryFrom builder
        code.push_str(&format!("impl TryFrom<{}> for {} {{\n", builder_name, struct_name));
        code.push_str("    type Error = BuilderError;\n\n");
        code.push_str(&format!("    fn try_from(builder: {}) -> Result<Self, Self::Error> {{\n", builder_name));
        code.push_str("        builder.build()\n");
        code.push_str("    }\n");
        code.push_str("}\n\n");

        // Generate Default implementation if all fields have defaults
        let all_have_defaults = struct_info.fields.iter().all(|f| f.is_optional || f.has_default);
        if all_have_defaults {
            code.push_str(&format!("impl Default for {} {{\n", builder_name));
            code.push_str("    fn default() -> Self {\n");
            code.push_str("        Self::new()\n");
            code.push_str("    }\n");
            code.push_str("}\n\n");
        }

        Ok(code)
    }

    fn generate_comprehensive_builder(&self, struct_info: &StructInfo, derive_attrs: &str, method_prefix: &str) -> Result<String> {
        let mut code = format!("// Comprehensive Builder Pattern for {}\n\n", struct_info.name);

        // Add validation code
        let validation_code = self.generate_validation_code(struct_info)?;
        code.push_str(&validation_code);

        // Add builder struct
        let builder_struct = self.generate_builder_struct(struct_info, derive_attrs)?;
        code.push_str(&builder_struct);

        // Add builder implementation
        let builder_impl = self.generate_builder_impl(struct_info, method_prefix)?;
        code.push_str(&builder_impl);

        // Add conversion implementations
        let conversion_impl = self.generate_conversion_impl(struct_info)?;
        code.push_str(&conversion_impl);

        Ok(code)
    }

    fn get_default_value_for_type(&self, ty: &str) -> String {
        match ty {
            "String" => "String::new()".to_string(),
            "i32" | "i64" | "u32" | "u64" => "0".to_string(),
            "bool" => "false".to_string(),
            "f32" | "f64" => "0.0".to_string(),
            "Vec<T>" => "Vec::new()".to_string(),
            "Option<T>" => "None".to_string(),
            _ => format!("{}::default()", ty),
        }
    }

    fn generate_nested_builders(&self, struct_info: &StructInfo) -> Result<String> {
        let mut code = format!("// Nested builders for {}\n\n", struct_info.name);

        for field in &struct_info.fields {
            if self.is_complex_type(&field.ty) && !field.is_optional {
                let nested_builder_name = format!("{}Builder", field.ty);

                code.push_str(&format!("impl {} {{\n", struct_info.name));
                code.push_str(&format!("    pub fn {}_builder() -> {} {{\n", field.name, nested_builder_name));
                code.push_str(&format!("        {}::new()\n", nested_builder_name));
                code.push_str("    }\n");
                code.push_str("}\n\n");
            }
        }

        Ok(code)
    }

    fn is_complex_type(&self, ty: &str) -> bool {
        // Consider it complex if it's not a primitive type
        !matches!(ty, "String" | "i32" | "i64" | "u32" | "u64" | "bool" | "f32" | "f64" | "Option<T>")
    }
}

impl Tool for BuilderGenTool {
    fn name(&self) -> &'static str {
        "builder-gen"
    }

    fn description(&self) -> &'static str {
        "Create builder patterns for complex structs automatically"
    }

    fn command(&self) -> Command {
        Command::new(self.name())
            .about(self.description())
            .long_about("Analyze struct definitions and generate comprehensive builder patterns with validation, optional fields, fluent API methods, and support for nested builders. Perfect for complex structs with many optional fields.")
            .args(&[
                Arg::new("input")
                    .long("input")
                    .short('i')
                    .help("Input Rust file containing struct definitions")
                    .required(true),
                Arg::new("output")
                    .long("output")
                    .short('o')
                    .help("Output file for generated builders")
                    .default_value("generated/builders.rs"),
                Arg::new("derive")
                    .long("derive")
                    .short('d')
                    .help("Additional derive macros for builder structs")
                    .default_value("Debug, Clone"),
                Arg::new("validation")
                    .long("validation")
                    .help("Generate validation logic")
                    .action(clap::ArgAction::SetTrue),
                Arg::new("nested")
                    .long("nested")
                    .help("Generate nested builders for complex types")
                    .action(clap::ArgAction::SetTrue),
                Arg::new("prefix")
                    .long("prefix")
                    .short('p')
                    .help("Method prefix for builder methods")
                    .default_value("with"),
            ])
            .args(&common_options())
    }

    fn execute(&self, matches: &ArgMatches) -> Result<()> {
        let input = matches.get_one::<String>("input").unwrap();
        let output = matches.get_one::<String>("output").unwrap();
        let derive_attrs = matches.get_one::<String>("derive").unwrap();
        let validation = matches.get_flag("validation");
        let nested = matches.get_flag("nested");
        let method_prefix = matches.get_one::<String>("prefix").unwrap();
        let dry_run = matches.get_flag("dry-run");
        let verbose = matches.get_flag("verbose");
        let output_format = parse_output_format(matches);

        println!("🔧 {} - {}", "CargoMate BuilderGen".bold().blue(), self.description().cyan());

        if !Path::new(input).exists() {
            return Err(ToolError::InvalidArguments(format!("Input file not found: {}", input)));
        }

        // Parse structs from input file
        let structs = self.parse_struct_from_file(input)?;

        if structs.is_empty() {
            println!("{}", "No structs found in input file".yellow());
            return Ok(());
        }

        let mut all_code = String::new();

        for struct_info in &structs {
            println!("📝 Processing struct: {}", struct_info.name.bold());

            if verbose {
                println!("   Fields:");
                for field in &struct_info.fields {
                    let opt = if field.is_optional { " (optional)" } else { "" };
                    let def = if field.has_default { " (default)" } else { "" };
                    println!("     - {}: {}{}{}", field.name, field.ty, opt, def);
                }
            }

            // Generate comprehensive builder
            let builder_code = self.generate_comprehensive_builder(struct_info, derive_attrs, method_prefix)?;
            all_code.push_str(&builder_code);

            // Generate nested builders if requested
            if nested {
                let nested_code = self.generate_nested_builders(struct_info)?;
                all_code.push_str(&nested_code);
            }

            all_code.push_str("\n");
        }

        match output_format {
            OutputFormat::Human => {
                println!("  ✅ Generated builder patterns for {} structs", structs.len());
                println!("     → {}", output.cyan());

                if validation {
                    println!("  ✅ Added validation logic");
                }

                if nested {
                    println!("  ✅ Generated nested builders");
                }

                if dry_run {
                    println!("   📋 {}", "Generated code preview:".bold());
                    println!("   {}", "─".repeat(50));
                    for (i, line) in all_code.lines().take(20).enumerate() {
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

                    fs::write(output, all_code)
                        .map_err(|e| ToolError::ExecutionFailed(format!("Failed to write {}: {}", output, e)))?;

                    println!("  💾 File written successfully");
                }
            }
            OutputFormat::Json => {
                let result = serde_json::json!({
                    "input": input,
                    "output": output,
                    "structs_processed": structs.len(),
                    "validation_enabled": validation,
                    "nested_enabled": nested,
                    "method_prefix": method_prefix,
                    "derive_attrs": derive_attrs,
                    "struct_names": structs.iter().map(|s| s.name.clone()).collect::<Vec<_>>(),
                    "code_preview": all_code.lines().take(10).collect::<Vec<_>>().join("\n")
                });
                println!("{}", serde_json::to_string_pretty(&result).unwrap());
            }
            OutputFormat::Table => {
                println!("{:<20} {:<15} {:<8} {:<8} {:<10}",
                    "Struct", "Input", "Valid", "Nested", "Prefix");
                println!("{}", "─".repeat(65));
                for struct_info in &structs {
                    println!("{:<20} {:<15} {:<8} {:<8} {:<10}",
                        struct_info.name,
                        Path::new(input).file_name().unwrap_or_default().to_string_lossy(),
                        if validation { "Yes" } else { "No" },
                        if nested { "Yes" } else { "No" },
                        method_prefix);
                }
            }
        }

        println!("\n🎉 Builder generation completed!");
        Ok(())
    }
}

impl Default for BuilderGenTool {
    fn default() -> Self {
        Self::new()
    }
}
