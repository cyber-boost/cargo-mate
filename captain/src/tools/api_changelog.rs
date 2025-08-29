use super::{Tool, Result, ToolError, common_options, parse_output_format, OutputFormat};
use clap::{Arg, ArgMatches, Command};
use colored::*;
use std::fs;
use std::path::Path;
use std::process::Command as ProcessCommand;
use syn::{
    parse_file, File, Item, ItemFn, ItemStruct, ItemEnum, Fields, Field, Type,
    PathSegment, Ident, visit::Visit, spanned::Spanned,
};
use quote::quote;
use proc_macro2::TokenStream;
use serde::{Serialize, Deserialize};
#[derive(Debug, Clone)]
pub struct ApiChangelogTool;
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ApiSurface {
    functions: Vec<FunctionInfo>,
    structs: Vec<StructInfo>,
    enums: Vec<EnumInfo>,
    version: String,
    commit: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
struct FunctionInfo {
    name: String,
    params: Vec<ParameterInfo>,
    return_type: Option<String>,
    visibility: String,
    attributes: Vec<String>,
    file_path: String,
    line_number: usize,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
struct StructInfo {
    name: String,
    fields: Vec<FieldInfo>,
    visibility: String,
    attributes: Vec<String>,
    file_path: String,
    line_number: usize,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
struct EnumInfo {
    name: String,
    variants: Vec<VariantInfo>,
    visibility: String,
    attributes: Vec<String>,
    file_path: String,
    line_number: usize,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ParameterInfo {
    name: String,
    type_info: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
struct FieldInfo {
    name: String,
    type_info: String,
    visibility: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
struct VariantInfo {
    name: String,
    fields: Vec<FieldInfo>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ApiComparison {
    breaking_changes: Vec<BreakingChange>,
    new_features: Vec<NewFeature>,
    non_breaking_changes: Vec<NonBreakingChange>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
enum BreakingChange {
    FunctionSignatureChanged {
        function: String,
        old_signature: String,
        new_signature: String,
        changes: Vec<FunctionChange>,
    },
    FunctionRemoved { function: String, file_path: String },
    StructFieldRemoved { struct_name: String, field: String, file_path: String },
    StructFieldTypeChanged {
        struct_name: String,
        field: String,
        old_type: String,
        new_type: String,
        file_path: String,
    },
    EnumVariantRemoved { enum_name: String, variant: String, file_path: String },
    TypeRemoved { type_name: String, type_kind: String, file_path: String },
}
#[derive(Debug, Clone, Serialize, Deserialize)]
enum NewFeature {
    FunctionAdded { function: String, signature: String, file_path: String },
    StructAdded { struct_name: String, file_path: String },
    StructFieldAdded {
        struct_name: String,
        field: String,
        field_type: String,
        file_path: String,
    },
    EnumAdded { enum_name: String, file_path: String },
    EnumVariantAdded { enum_name: String, variant: String, file_path: String },
}
#[derive(Debug, Clone, Serialize, Deserialize)]
enum NonBreakingChange {
    FunctionParameterAdded {
        function: String,
        parameter: String,
        parameter_type: String,
        file_path: String,
    },
    FunctionDocumentationAdded { function: String, file_path: String },
    StructDocumentationAdded { struct_name: String, file_path: String },
}
#[derive(Debug, Clone, Serialize, Deserialize)]
enum FunctionChange {
    ParameterAdded { name: String, ty: String },
    ParameterRemoved { name: String, ty: String },
    ParameterTypeChanged { name: String, old_ty: String, new_ty: String },
    ParameterNameChanged { old_name: String, new_name: String, ty: String },
    ReturnTypeChanged { old_ty: String, new_ty: String },
    VisibilityChanged { old_vis: String, new_vis: String },
}
impl ApiChangelogTool {
    pub fn new() -> Self {
        Self
    }
    fn get_git_commit(&self, version: &str) -> Result<String> {
        let output = ProcessCommand::new("git")
            .args(&["rev-parse", version])
            .output()
            .map_err(|e| ToolError::ExecutionFailed(
                format!("Failed to get git commit: {}", e),
            ))?;
        if output.status.success() {
            let commit = String::from_utf8_lossy(&output.stdout).trim().to_string();
            Ok(commit)
        } else {
            Err(
                ToolError::ExecutionFailed(
                    format!(
                        "Git command failed: {}", String::from_utf8_lossy(& output
                        .stderr)
                    ),
                ),
            )
        }
    }
    fn checkout_version(&self, version: &str) -> Result<()> {
        let output = ProcessCommand::new("git")
            .args(&["checkout", version])
            .output()
            .map_err(|e| ToolError::ExecutionFailed(
                format!("Failed to checkout version: {}", e),
            ))?;
        if output.status.success() {
            Ok(())
        } else {
            Err(
                ToolError::ExecutionFailed(
                    format!(
                        "Git checkout failed: {}", String::from_utf8_lossy(& output
                        .stderr)
                    ),
                ),
            )
        }
    }
    fn get_current_commit(&self) -> Result<String> {
        let output = ProcessCommand::new("git")
            .args(&["rev-parse", "HEAD"])
            .output()
            .map_err(|e| ToolError::ExecutionFailed(
                format!("Failed to get current commit: {}", e),
            ))?;
        if output.status.success() {
            let commit = String::from_utf8_lossy(&output.stdout).trim().to_string();
            Ok(commit)
        } else {
            Err(
                ToolError::ExecutionFailed(
                    format!(
                        "Git command failed: {}", String::from_utf8_lossy(& output
                        .stderr)
                    ),
                ),
            )
        }
    }
    fn analyze_api_surface(
        &self,
        version: &str,
        source_path: &str,
    ) -> Result<ApiSurface> {
        let commit = if version == "HEAD" {
            self.get_current_commit()?
        } else {
            self.get_git_commit(version)?
        };
        let mut functions = Vec::new();
        let mut structs = Vec::new();
        let mut enums = Vec::new();
        self.analyze_directory(source_path, &mut functions, &mut structs, &mut enums)?;
        Ok(ApiSurface {
            functions,
            structs,
            enums,
            version: version.to_string(),
            commit,
        })
    }
    fn analyze_directory(
        &self,
        dir_path: &str,
        functions: &mut Vec<FunctionInfo>,
        structs: &mut Vec<StructInfo>,
        enums: &mut Vec<EnumInfo>,
    ) -> Result<()> {
        let entries = fs::read_dir(dir_path)
            .map_err(|e| ToolError::ExecutionFailed(
                format!("Failed to read directory {}: {}", dir_path, e),
            ))?;
        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                self.analyze_directory(
                    &path.to_string_lossy(),
                    functions,
                    structs,
                    enums,
                )?;
            } else if let Some(ext) = path.extension() {
                if ext == "rs" && !path.ends_with("mod.rs") && !path.ends_with("lib.rs")
                {
                    self.analyze_file(&path, functions, structs, enums)?;
                }
            }
        }
        Ok(())
    }
    fn analyze_file(
        &self,
        file_path: &Path,
        functions: &mut Vec<FunctionInfo>,
        structs: &mut Vec<StructInfo>,
        enums: &mut Vec<EnumInfo>,
    ) -> Result<()> {
        let content = fs::read_to_string(file_path)?;
        let ast = parse_file(&content)?;
        struct ApiVisitor<'a> {
            functions: &'a mut Vec<FunctionInfo>,
            structs: &'a mut Vec<StructInfo>,
            enums: &'a mut Vec<EnumInfo>,
            current_file: String,
        }
        impl<'a> Visit<'_> for ApiVisitor<'a> {
            fn visit_item_fn(&mut self, node: &ItemFn) {
                if matches!(node.vis, syn::Visibility::Public(_)) {
                    let func_info = FunctionInfo {
                        name: node.sig.ident.to_string(),
                        params: Self::extract_parameters(&node.sig.inputs),
                        return_type: Self::extract_return_type(&node.sig.output),
                        visibility: "pub".to_string(),
                        attributes: Self::extract_attributes(&node.attrs),
                        file_path: self.current_file.clone(),
                        line_number: 0,
                    };
                    self.functions.push(func_info);
                }
            }
            fn visit_item_struct(&mut self, node: &ItemStruct) {
                if matches!(node.vis, syn::Visibility::Public(_)) {
                    let struct_info = StructInfo {
                        name: node.ident.to_string(),
                        fields: Self::extract_fields(&node.fields),
                        visibility: "pub".to_string(),
                        attributes: Self::extract_attributes(&node.attrs),
                        file_path: self.current_file.clone(),
                        line_number: 0,
                    };
                    self.structs.push(struct_info);
                }
            }
            fn visit_item_enum(&mut self, node: &ItemEnum) {
                if matches!(node.vis, syn::Visibility::Public(_)) {
                    let enum_info = EnumInfo {
                        name: node.ident.to_string(),
                        variants: Self::extract_variants(&node.variants),
                        visibility: "pub".to_string(),
                        attributes: Self::extract_attributes(&node.attrs),
                        file_path: self.current_file.clone(),
                        line_number: 0,
                    };
                    self.enums.push(enum_info);
                }
            }
        }
        impl ApiVisitor<'_> {
            fn extract_parameters(
                inputs: &syn::punctuated::Punctuated<syn::FnArg, syn::token::Comma>,
            ) -> Vec<ParameterInfo> {
                let mut params = Vec::new();
                for input in inputs {
                    if let syn::FnArg::Typed(pat_type) = input {
                        if let syn::Pat::Ident(pat_ident) = &*pat_type.pat {
                            let name = pat_ident.ident.to_string();
                            let type_info = Self::type_to_string(&*pat_type.ty);
                            params.push(ParameterInfo { name, type_info });
                        }
                    }
                }
                params
            }
            fn extract_return_type(output: &syn::ReturnType) -> Option<String> {
                match output {
                    syn::ReturnType::Default => None,
                    syn::ReturnType::Type(_, ty) => Some(Self::type_to_string(ty)),
                }
            }
            fn extract_fields(fields: &Fields) -> Vec<FieldInfo> {
                let mut field_infos = Vec::new();
                if let Fields::Named(named_fields) = fields {
                    for field in &named_fields.named {
                        if let Some(field_ident) = &field.ident {
                            let visibility = if matches!(
                                field.vis, syn::Visibility::Public(_)
                            ) {
                                "pub".to_string()
                            } else {
                                "private".to_string()
                            };
                            let type_info = Self::type_to_string(&field.ty);
                            field_infos
                                .push(FieldInfo {
                                    name: field_ident.to_string(),
                                    type_info,
                                    visibility,
                                });
                        }
                    }
                }
                field_infos
            }
            fn extract_variants(
                variants: &syn::punctuated::Punctuated<syn::Variant, syn::token::Comma>,
            ) -> Vec<VariantInfo> {
                let mut variant_infos = Vec::new();
                for variant in variants {
                    let fields = Self::extract_fields(&variant.fields);
                    variant_infos
                        .push(VariantInfo {
                            name: variant.ident.to_string(),
                            fields,
                        });
                }
                variant_infos
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
            fn type_to_string(ty: &Type) -> String {
                match ty {
                    Type::Path(type_path) => {
                        type_path
                            .path
                            .segments
                            .iter()
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
        }
        let mut visitor = ApiVisitor {
            functions,
            structs,
            enums,
            current_file: file_path.to_string_lossy().to_string(),
        };
        syn::visit::visit_file(&mut visitor, &ast);
        Ok(())
    }
    fn compare_api_surfaces(
        &self,
        old_api: &ApiSurface,
        new_api: &ApiSurface,
    ) -> Result<ApiComparison> {
        let mut breaking_changes = Vec::new();
        let mut new_features = Vec::new();
        let mut non_breaking_changes = Vec::new();
        self.compare_functions(
            &old_api.functions,
            &new_api.functions,
            &mut breaking_changes,
            &mut new_features,
            &mut non_breaking_changes,
        );
        self.compare_structs(
            &old_api.structs,
            &new_api.structs,
            &mut breaking_changes,
            &mut new_features,
            &mut non_breaking_changes,
        );
        self.compare_enums(
            &old_api.enums,
            &new_api.enums,
            &mut breaking_changes,
            &mut new_features,
        );
        Ok(ApiComparison {
            breaking_changes,
            new_features,
            non_breaking_changes,
        })
    }
    fn compare_functions(
        &self,
        old_functions: &[FunctionInfo],
        new_functions: &[FunctionInfo],
        breaking_changes: &mut Vec<BreakingChange>,
        new_features: &mut Vec<NewFeature>,
        non_breaking_changes: &mut Vec<NonBreakingChange>,
    ) {
        let old_function_map: std::collections::HashMap<&str, &FunctionInfo> = old_functions
            .iter()
            .map(|f| (f.name.as_str(), f))
            .collect();
        let new_function_map: std::collections::HashMap<&str, &FunctionInfo> = new_functions
            .iter()
            .map(|f| (f.name.as_str(), f))
            .collect();
        for old_func in old_functions {
            if !new_function_map.contains_key(old_func.name.as_str()) {
                breaking_changes
                    .push(BreakingChange::FunctionRemoved {
                        function: old_func.name.clone(),
                        file_path: old_func.file_path.clone(),
                    });
            }
        }
        for new_func in new_functions {
            if !old_function_map.contains_key(new_func.name.as_str()) {
                let signature = self.format_function_signature(new_func);
                new_features
                    .push(NewFeature::FunctionAdded {
                        function: new_func.name.clone(),
                        signature,
                        file_path: new_func.file_path.clone(),
                    });
            }
        }
        for new_func in new_functions {
            if let Some(old_func) = old_function_map.get(new_func.name.as_str()) {
                let changes = self.compare_function_signatures(old_func, new_func);
                if !changes.is_empty() {
                    let old_signature = self.format_function_signature(old_func);
                    let new_signature = self.format_function_signature(new_func);
                    breaking_changes
                        .push(BreakingChange::FunctionSignatureChanged {
                            function: new_func.name.clone(),
                            old_signature,
                            new_signature,
                            changes,
                        });
                }
            }
        }
    }
    fn compare_function_signatures(
        &self,
        old_func: &FunctionInfo,
        new_func: &FunctionInfo,
    ) -> Vec<FunctionChange> {
        let mut changes = Vec::new();
        let old_params: std::collections::HashMap<&str, &ParameterInfo> = old_func
            .params
            .iter()
            .map(|p| (p.name.as_str(), p))
            .collect();
        let new_params: std::collections::HashMap<&str, &ParameterInfo> = new_func
            .params
            .iter()
            .map(|p| (p.name.as_str(), p))
            .collect();
        for old_param in &old_func.params {
            if !new_params.contains_key(old_param.name.as_str()) {
                changes
                    .push(FunctionChange::ParameterRemoved {
                        name: old_param.name.clone(),
                        ty: old_param.type_info.clone(),
                    });
            }
        }
        for new_param in &new_func.params {
            if !old_params.contains_key(new_param.name.as_str()) {
                changes
                    .push(FunctionChange::ParameterAdded {
                        name: new_param.name.clone(),
                        ty: new_param.type_info.clone(),
                    });
            }
        }
        for new_param in &new_func.params {
            if let Some(old_param) = old_params.get(new_param.name.as_str()) {
                if old_param.type_info != new_param.type_info {
                    changes
                        .push(FunctionChange::ParameterTypeChanged {
                            name: new_param.name.clone(),
                            old_ty: old_param.type_info.clone(),
                            new_ty: new_param.type_info.clone(),
                        });
                }
            }
        }
        if old_func.return_type != new_func.return_type {
            changes
                .push(FunctionChange::ReturnTypeChanged {
                    old_ty: old_func
                        .return_type
                        .clone()
                        .unwrap_or_else(|| "None".to_string()),
                    new_ty: new_func
                        .return_type
                        .clone()
                        .unwrap_or_else(|| "None".to_string()),
                });
        }
        if old_func.visibility != new_func.visibility {
            changes
                .push(FunctionChange::VisibilityChanged {
                    old_vis: old_func.visibility.clone(),
                    new_vis: new_func.visibility.clone(),
                });
        }
        changes
    }
    fn compare_structs(
        &self,
        old_structs: &[StructInfo],
        new_structs: &[StructInfo],
        breaking_changes: &mut Vec<BreakingChange>,
        new_features: &mut Vec<NewFeature>,
        non_breaking_changes: &mut Vec<NonBreakingChange>,
    ) {
        let old_struct_map: std::collections::HashMap<&str, &StructInfo> = old_structs
            .iter()
            .map(|s| (s.name.as_str(), s))
            .collect();
        let new_struct_map: std::collections::HashMap<&str, &StructInfo> = new_structs
            .iter()
            .map(|s| (s.name.as_str(), s))
            .collect();
        for new_struct in new_structs {
            if !old_struct_map.contains_key(new_struct.name.as_str()) {
                new_features
                    .push(NewFeature::StructAdded {
                        struct_name: new_struct.name.clone(),
                        file_path: new_struct.file_path.clone(),
                    });
            }
        }
        for old_struct in old_structs {
            if !new_struct_map.contains_key(old_struct.name.as_str()) {
                breaking_changes
                    .push(BreakingChange::TypeRemoved {
                        type_name: old_struct.name.clone(),
                        type_kind: "struct".to_string(),
                        file_path: old_struct.file_path.clone(),
                    });
            }
        }
        for new_struct in new_structs {
            if let Some(old_struct) = old_struct_map.get(new_struct.name.as_str()) {
                self.compare_struct_fields(
                    old_struct,
                    new_struct,
                    breaking_changes,
                    new_features,
                );
            }
        }
    }
    fn compare_struct_fields(
        &self,
        old_struct: &StructInfo,
        new_struct: &StructInfo,
        breaking_changes: &mut Vec<BreakingChange>,
        new_features: &mut Vec<NewFeature>,
    ) {
        let old_fields: std::collections::HashMap<&str, &FieldInfo> = old_struct
            .fields
            .iter()
            .map(|f| (f.name.as_str(), f))
            .collect();
        let new_fields: std::collections::HashMap<&str, &FieldInfo> = new_struct
            .fields
            .iter()
            .map(|f| (f.name.as_str(), f))
            .collect();
        for old_field in &old_struct.fields {
            if !new_fields.contains_key(old_field.name.as_str()) {
                breaking_changes
                    .push(BreakingChange::StructFieldRemoved {
                        struct_name: new_struct.name.clone(),
                        field: old_field.name.clone(),
                        file_path: new_struct.file_path.clone(),
                    });
            }
        }
        for new_field in &new_struct.fields {
            if !old_fields.contains_key(new_field.name.as_str()) {
                new_features
                    .push(NewFeature::StructFieldAdded {
                        struct_name: new_struct.name.clone(),
                        field: new_field.name.clone(),
                        field_type: new_field.type_info.clone(),
                        file_path: new_struct.file_path.clone(),
                    });
            }
        }
        for new_field in &new_struct.fields {
            if let Some(old_field) = old_fields.get(new_field.name.as_str()) {
                if old_field.type_info != new_field.type_info {
                    breaking_changes
                        .push(BreakingChange::StructFieldTypeChanged {
                            struct_name: new_struct.name.clone(),
                            field: new_field.name.clone(),
                            old_type: old_field.type_info.clone(),
                            new_type: new_field.type_info.clone(),
                            file_path: new_struct.file_path.clone(),
                        });
                }
            }
        }
    }
    fn compare_enums(
        &self,
        old_enums: &[EnumInfo],
        new_enums: &[EnumInfo],
        breaking_changes: &mut Vec<BreakingChange>,
        new_features: &mut Vec<NewFeature>,
    ) {
        let old_enum_map: std::collections::HashMap<&str, &EnumInfo> = old_enums
            .iter()
            .map(|e| (e.name.as_str(), e))
            .collect();
        let new_enum_map: std::collections::HashMap<&str, &EnumInfo> = new_enums
            .iter()
            .map(|e| (e.name.as_str(), e))
            .collect();
        for new_enum in new_enums {
            if !old_enum_map.contains_key(new_enum.name.as_str()) {
                new_features
                    .push(NewFeature::EnumAdded {
                        enum_name: new_enum.name.clone(),
                        file_path: new_enum.file_path.clone(),
                    });
            }
        }
        for old_enum in old_enums {
            if !new_enum_map.contains_key(old_enum.name.as_str()) {
                breaking_changes
                    .push(BreakingChange::TypeRemoved {
                        type_name: old_enum.name.clone(),
                        type_kind: "enum".to_string(),
                        file_path: old_enum.file_path.clone(),
                    });
            }
        }
    }
    fn format_function_signature(&self, func: &FunctionInfo) -> String {
        let params = func
            .params
            .iter()
            .map(|p| format!("{}: {}", p.name, p.type_info))
            .collect::<Vec<_>>()
            .join(", ");
        match &func.return_type {
            Some(return_type) => {
                format!("fn {}({}) -> {}", func.name, params, return_type)
            }
            None => format!("fn {}({})", func.name, params),
        }
    }
    fn generate_changelog(
        &self,
        comparison: &ApiComparison,
        format: &str,
        old_version: &str,
        new_version: &str,
    ) -> Result<String> {
        match format {
            "markdown" => {
                self.generate_markdown_changelog(comparison, old_version, new_version)
            }
            "json" => self.generate_json_changelog(comparison),
            "html" => self.generate_html_changelog(comparison, old_version, new_version),
            _ => {
                Err(
                    ToolError::InvalidArguments(
                        format!("Unsupported format: {}", format),
                    ),
                )
            }
        }
    }
    fn generate_markdown_changelog(
        &self,
        comparison: &ApiComparison,
        old_version: &str,
        new_version: &str,
    ) -> Result<String> {
        let mut markdown = format!(
            "# API Changelog: {} → {}\n\n", old_version, new_version
        );
        if !comparison.breaking_changes.is_empty() {
            markdown.push_str("## 🚨 Breaking Changes\n\n");
            for change in &comparison.breaking_changes {
                match change {
                    BreakingChange::FunctionSignatureChanged {
                        function,
                        old_signature,
                        new_signature,
                        changes,
                    } => {
                        markdown
                            .push_str(
                                &format!("### Function Signature Changed: `{}`\n", function),
                            );
                        markdown.push_str(&format!("**Old:** `{}`\n", old_signature));
                        markdown.push_str(&format!("**New:** `{}`\n\n", new_signature));
                        markdown.push_str("**Changes:**\n");
                        for change in changes {
                            match change {
                                FunctionChange::ParameterAdded { name, ty } => {
                                    markdown
                                        .push_str(
                                            &format!("- Added parameter: `{}`: `{}`\n", name, ty),
                                        );
                                }
                                FunctionChange::ParameterRemoved { name, ty } => {
                                    markdown
                                        .push_str(
                                            &format!("- Removed parameter: `{}`: `{}`\n", name, ty),
                                        );
                                }
                                FunctionChange::ParameterTypeChanged {
                                    name,
                                    old_ty,
                                    new_ty,
                                } => {
                                    markdown
                                        .push_str(
                                            &format!(
                                                "- Parameter `{}` type changed: `{}` → `{}`\n", name,
                                                old_ty, new_ty
                                            ),
                                        );
                                }
                                FunctionChange::ReturnTypeChanged { old_ty, new_ty } => {
                                    markdown
                                        .push_str(
                                            &format!(
                                                "- Return type changed: `{}` → `{}`\n", old_ty, new_ty
                                            ),
                                        );
                                }
                                _ => {}
                            }
                        }
                        markdown.push_str("\n");
                    }
                    BreakingChange::FunctionRemoved { function, file_path } => {
                        markdown
                            .push_str(
                                &format!("### Function Removed: `{}`\n", function),
                            );
                        markdown.push_str(&format!("**File:** `{}`\n\n", file_path));
                    }
                    BreakingChange::StructFieldRemoved {
                        struct_name,
                        field,
                        file_path,
                    } => {
                        markdown
                            .push_str(
                                &format!(
                                    "### Struct Field Removed: `{}.{}`\n", struct_name, field
                                ),
                            );
                        markdown.push_str(&format!("**File:** `{}`\n\n", file_path));
                    }
                    BreakingChange::TypeRemoved { type_name, type_kind, file_path } => {
                        markdown
                            .push_str(
                                &format!("### {} Removed: `{}`\n", type_kind, type_name),
                            );
                        markdown.push_str(&format!("**File:** `{}`\n\n", file_path));
                    }
                    _ => {}
                }
            }
        }
        if !comparison.new_features.is_empty() {
            markdown.push_str("## ✨ New Features\n\n");
            for feature in &comparison.new_features {
                match feature {
                    NewFeature::FunctionAdded { function, signature, file_path } => {
                        markdown
                            .push_str(&format!("### New Function: `{}`\n", function));
                        markdown.push_str(&format!("**Signature:** `{}`\n", signature));
                        markdown.push_str(&format!("**File:** `{}`\n\n", file_path));
                    }
                    NewFeature::StructAdded { struct_name, file_path } => {
                        markdown
                            .push_str(&format!("### New Struct: `{}`\n", struct_name));
                        markdown.push_str(&format!("**File:** `{}`\n\n", file_path));
                    }
                    NewFeature::StructFieldAdded {
                        struct_name,
                        field,
                        field_type,
                        file_path,
                    } => {
                        markdown
                            .push_str(
                                &format!(
                                    "### New Struct Field: `{}.{}`\n", struct_name, field
                                ),
                            );
                        markdown.push_str(&format!("**Type:** `{}`\n", field_type));
                        markdown.push_str(&format!("**File:** `{}`\n\n", file_path));
                    }
                    _ => {}
                }
            }
        }
        let compatibility_score = self.calculate_compatibility_score(comparison);
        markdown.push_str("## 📊 Compatibility Score\n\n");
        markdown
            .push_str(
                &format!("**Overall Compatibility:** {}%\n\n", compatibility_score),
            );
        markdown
            .push_str(
                &format!("**Breaking Changes:** {}\n", comparison.breaking_changes.len()),
            );
        markdown
            .push_str(&format!("**New Features:** {}\n", comparison.new_features.len()));
        markdown
            .push_str(
                &format!(
                    "**Non-Breaking Changes:** {}\n\n", comparison.non_breaking_changes
                    .len()
                ),
            );
        Ok(markdown)
    }
    fn generate_json_changelog(&self, comparison: &ApiComparison) -> Result<String> {
        let changelog = serde_json::json!(
            { "breaking_changes" : comparison.breaking_changes, "new_features" :
            comparison.new_features, "non_breaking_changes" : comparison
            .non_breaking_changes, "compatibility_score" : self
            .calculate_compatibility_score(comparison) }
        );
        Ok(serde_json::to_string_pretty(&changelog)?)
    }
    fn generate_html_changelog(
        &self,
        comparison: &ApiComparison,
        old_version: &str,
        new_version: &str,
    ) -> Result<String> {
        let mut html = format!(
            "<!DOCTYPE html>\n<html>\n<head>\n<title>API Changelog: {} → {}</title>\n</head>\n<body>\n",
            old_version, new_version
        );
        html.push_str(
            &format!("<h1>API Changelog: {} → {}</h1>\n", old_version, new_version),
        );
        if !comparison.breaking_changes.is_empty() {
            html.push_str("<h2>🚨 Breaking Changes</h2>\n");
            for change in &comparison.breaking_changes {
                match change {
                    BreakingChange::FunctionSignatureChanged { function, .. } => {
                        html.push_str(
                            &format!(
                                "<h3>Function Signature Changed: <code>{}</code></h3>\n",
                                function
                            ),
                        );
                    }
                    BreakingChange::FunctionRemoved { function, .. } => {
                        html.push_str(
                            &format!(
                                "<h3>Function Removed: <code>{}</code></h3>\n", function
                            ),
                        );
                    }
                    _ => {}
                }
            }
        }
        if !comparison.new_features.is_empty() {
            html.push_str("<h2>✨ New Features</h2>\n");
            for feature in &comparison.new_features {
                match feature {
                    NewFeature::FunctionAdded { function, .. } => {
                        html.push_str(
                            &format!(
                                "<h3>New Function: <code>{}</code></h3>\n", function
                            ),
                        );
                    }
                    _ => {}
                }
            }
        }
        html.push_str("</body>\n</html>\n");
        Ok(html)
    }
    fn calculate_compatibility_score(&self, comparison: &ApiComparison) -> u32 {
        let total_changes = comparison.breaking_changes.len()
            + comparison.new_features.len() + comparison.non_breaking_changes.len();
        if total_changes == 0 {
            return 100;
        }
        let breaking_weight = 10;
        let new_feature_weight = 1;
        let non_breaking_weight = 0;
        let breaking_score = comparison.breaking_changes.len() * breaking_weight;
        let new_feature_score = comparison.new_features.len() * new_feature_weight;
        let non_breaking_score = comparison.non_breaking_changes.len()
            * non_breaking_weight;
        let total_score = breaking_score + new_feature_score + non_breaking_score;
        let max_possible_score = total_changes * breaking_weight;
        if max_possible_score == 0 {
            100
        } else {
            ((max_possible_score - total_score) as f64 / max_possible_score as f64
                * 100.0) as u32
        }
    }
}
impl Tool for ApiChangelogTool {
    fn name(&self) -> &'static str {
        "api-changelog"
    }
    fn description(&self) -> &'static str {
        "Generate API changelogs and analyze breaking changes"
    }
    fn command(&self) -> Command {
        Command::new(self.name())
            .about(self.description())
            .long_about(
                "Compare API surfaces between different versions and generate comprehensive changelogs. Automatically detects breaking changes, new features, and analyzes API compatibility.",
            )
            .args(
                &[
                    Arg::new("old-version")
                        .long("old-version")
                        .short('o')
                        .help("Old version to compare from (git tag, commit, or path)")
                        .required(true),
                    Arg::new("new-version")
                        .long("new-version")
                        .short('n')
                        .help("New version to compare to (default: current)")
                        .default_value("HEAD"),
                    Arg::new("source-path")
                        .long("source-path")
                        .short('s')
                        .help("Path to source code")
                        .default_value("src/"),
                    Arg::new("breaking-only")
                        .long("breaking-only")
                        .help("Show only breaking changes")
                        .action(clap::ArgAction::SetTrue),
                    Arg::new("format")
                        .long("format")
                        .short('f')
                        .help("Output format: markdown, json, html")
                        .default_value("markdown"),
                    Arg::new("output")
                        .long("output")
                        .help("Output file for changelog")
                        .default_value("CHANGELOG-API.md"),
                    Arg::new("migration-guide")
                        .long("migration-guide")
                        .help("Generate migration guide for breaking changes")
                        .action(clap::ArgAction::SetTrue),
                    Arg::new("compatibility")
                        .long("compatibility")
                        .help("Analyze API compatibility score")
                        .action(clap::ArgAction::SetTrue),
                    Arg::new("detailed")
                        .long("detailed")
                        .short('d')
                        .help("Include detailed change descriptions")
                        .action(clap::ArgAction::SetTrue),
                    Arg::new("exclude")
                        .long("exclude")
                        .help("Exclude patterns (comma-separated)")
                        .default_value("tests/,examples/"),
                ],
            )
            .args(&common_options())
    }
    fn execute(&self, matches: &ArgMatches) -> Result<()> {
        let old_version = matches.get_one::<String>("old-version").unwrap();
        let new_version = matches.get_one::<String>("new-version").unwrap();
        let source_path = matches.get_one::<String>("source-path").unwrap();
        let breaking_only = matches.get_flag("breaking-only");
        let format = matches.get_one::<String>("format").unwrap();
        let output = matches.get_one::<String>("output").unwrap();
        let migration_guide = matches.get_flag("migration-guide");
        let compatibility = matches.get_flag("compatibility");
        let detailed = matches.get_flag("detailed");
        let dry_run = matches.get_flag("dry-run");
        let verbose = matches.get_flag("verbose");
        let output_format = parse_output_format(matches);
        println!(
            "📊 {} - {}", "CargoMate ApiChangelog".bold().blue(), self.description()
            .cyan()
        );
        if !Path::new(source_path).exists() {
            return Err(
                ToolError::InvalidArguments(
                    format!("Source path not found: {}", source_path),
                ),
            );
        }
        if !Path::new(".git").exists() {
            return Err(
                ToolError::ExecutionFailed(
                    "Not in a git repository. API changelog requires git history."
                        .to_string(),
                ),
            );
        }
        if verbose {
            println!(
                "   📋 Analyzing API changes from {} to {}", old_version, new_version
            );
        }
        let current_commit = self.get_current_commit()?;
        let old_api = if old_version == "HEAD" {
            self.analyze_api_surface(old_version, source_path)?
        } else {
            self.checkout_version(old_version)?;
            let api = self.analyze_api_surface(old_version, source_path)?;
            self.checkout_version(&current_commit)?;
            api
        };
        let new_api = if new_version == "HEAD" {
            self.analyze_api_surface(new_version, source_path)?
        } else {
            self.checkout_version(new_version)?;
            let api = self.analyze_api_surface(new_version, source_path)?;
            self.checkout_version(&current_commit)?;
            api
        };
        let comparison = self.compare_api_surfaces(&old_api, &new_api)?;
        if verbose {
            println!(
                "   📊 Found {} breaking changes, {} new features, {} non-breaking changes",
                comparison.breaking_changes.len(), comparison.new_features.len(),
                comparison.non_breaking_changes.len()
            );
        }
        let filtered_comparison = if breaking_only {
            ApiComparison {
                breaking_changes: comparison.breaking_changes,
                new_features: Vec::new(),
                non_breaking_changes: Vec::new(),
            }
        } else {
            comparison
        };
        let changelog = self
            .generate_changelog(&filtered_comparison, format, old_version, new_version)?;
        match output_format {
            OutputFormat::Human => {
                println!(
                    "  ✅ Generated API changelog: {} → {}", old_version, new_version
                );
                println!("     → {}", output.cyan());
                if breaking_only {
                    println!("  ⚠️  Showing only breaking changes");
                }
                if dry_run {
                    println!("   📋 {}", "Changelog preview:".bold());
                    println!("   {}", "─".repeat(50));
                    for line in changelog.lines().take(20) {
                        println!("   {}", line);
                    }
                    if changelog.lines().count() > 20 {
                        println!("   ... (truncated)");
                    }
                } else {
                    fs::write(output, &changelog)
                        .map_err(|e| ToolError::ExecutionFailed(
                            format!("Failed to write {}: {}", output, e),
                        ))?;
                    println!("  💾 Changelog written successfully");
                    if compatibility {
                        let score = self
                            .calculate_compatibility_score(&filtered_comparison);
                        println!("  📊 API Compatibility Score: {}%", score);
                    }
                }
            }
            OutputFormat::Json => {
                let result = serde_json::json!(
                    { "old_version" : old_version, "new_version" : new_version,
                    "breaking_changes_count" : filtered_comparison.breaking_changes
                    .len(), "new_features_count" : filtered_comparison.new_features
                    .len(), "compatibility_score" : self.calculate_compatibility_score(&
                    filtered_comparison), "changelog" : changelog }
                );
                println!("{}", serde_json::to_string_pretty(& result).unwrap());
            }
            OutputFormat::Table => {
                println!(
                    "{:<20} {:<15} {:<15} {:<12}", "Version Comparison", "Breaking",
                    "New Features", "Compatibility"
                );
                println!("{}", "─".repeat(70));
                let score = self.calculate_compatibility_score(&filtered_comparison);
                println!(
                    "{:<20} {:<15} {:<15} {:<12}%", format!("{} → {}", old_version,
                    new_version), filtered_comparison.breaking_changes.len(),
                    filtered_comparison.new_features.len(), score
                );
            }
        }
        println!("\n🎉 API changelog generation completed!");
        Ok(())
    }
}
impl Default for ApiChangelogTool {
    fn default() -> Self {
        Self::new()
    }
}