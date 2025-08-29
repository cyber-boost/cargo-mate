use super::{Tool, Result, ToolError, common_options, parse_output_format, OutputFormat};
use clap::{Arg, ArgMatches, Command};
use colored::*;
use std::fs;
use std::path::Path;
use serde_json::{Value, Map};
use serde_yaml;
use syn::{parse_file, File, Item, ItemStruct, Fields, Field, Type, PathSegment, Ident};
use quote::quote;
use proc_macro2::TokenStream;
#[derive(Debug, Clone)]
pub struct ProtoBindTool;
#[derive(Debug, Clone)]
struct ProtoSchema {
    messages: Vec<ProtoMessage>,
    services: Vec<ProtoService>,
}
#[derive(Debug, Clone)]
struct ProtoMessage {
    name: String,
    fields: Vec<ProtoField>,
}
#[derive(Debug, Clone)]
struct ProtoField {
    name: String,
    ty: String,
    number: u32,
    repeated: bool,
}
#[derive(Debug, Clone)]
struct ProtoService {
    name: String,
    methods: Vec<ProtoMethod>,
}
#[derive(Debug, Clone)]
struct ProtoMethod {
    name: String,
    input_type: String,
    output_type: String,
}
#[derive(Debug, Clone)]
struct OpenAPISchema {
    components: OpenAPIComponents,
    paths: Vec<OpenAPIPath>,
}
#[derive(Debug, Clone)]
struct OpenAPIComponents {
    schemas: Vec<OpenAPISchemaItem>,
}
#[derive(Debug, Clone)]
struct OpenAPISchemaItem {
    name: String,
    properties: Vec<OpenAPIProperty>,
}
#[derive(Debug, Clone)]
struct OpenAPIProperty {
    name: String,
    ty: String,
    required: bool,
}
#[derive(Debug, Clone)]
struct OpenAPIPath {
    path: String,
    method: String,
    operation_id: String,
    request_body: Option<String>,
    response_body: Option<String>,
}
#[derive(Debug, Clone)]
struct GraphQLSchema {
    types: Vec<GraphQLType>,
    queries: Vec<GraphQLField>,
    mutations: Vec<GraphQLField>,
}
#[derive(Debug, Clone)]
struct GraphQLType {
    name: String,
    fields: Vec<GraphQLField>,
}
#[derive(Debug, Clone)]
struct GraphQLField {
    name: String,
    ty: String,
    args: Vec<GraphQLArg>,
}
#[derive(Debug, Clone)]
struct GraphQLArg {
    name: String,
    ty: String,
}
impl ProtoBindTool {
    pub fn new() -> Self {
        Self
    }
    fn parse_proto_file(&self, file_path: &str) -> Result<ProtoSchema> {
        let content = fs::read_to_string(file_path)
            .map_err(|e| ToolError::ExecutionFailed(
                format!("Failed to read {}: {}", file_path, e),
            ))?;
        let mut messages = Vec::new();
        let mut services = Vec::new();
        let mut lines = content.lines().peekable();
        while let Some(line) = lines.next() {
            let line = line.trim();
            if line.starts_with("message ") {
                if let Some(message) = self.parse_proto_message(line, &mut lines) {
                    messages.push(message);
                }
            } else if line.starts_with("service ") {
                if let Some(service) = self.parse_proto_service(line, &mut lines) {
                    services.push(service);
                }
            }
        }
        Ok(ProtoSchema { messages, services })
    }
    fn parse_proto_message(
        &self,
        first_line: &str,
        lines: &mut std::iter::Peekable<std::str::Lines>,
    ) -> Option<ProtoMessage> {
        let name = first_line
            .strip_prefix("message ")?
            .trim_end_matches(" {")
            .trim()
            .to_string();
        let mut fields = Vec::new();
        while let Some(line) = lines.next() {
            let line = line.trim();
            if line == "}" {
                break;
            }
            if line.is_empty() || line.starts_with("//") {
                continue;
            }
            if let Some(field) = self.parse_proto_field(line) {
                fields.push(field);
            }
        }
        Some(ProtoMessage { name, fields })
    }
    fn parse_proto_field(&self, line: &str) -> Option<ProtoField> {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 3 {
            return None;
        }
        let repeated = parts[0] == "repeated";
        let ty = if repeated { parts[1] } else { parts[0] };
        let name = if repeated { parts[2] } else { parts[1] };
        let number_part = if repeated { parts[3] } else { parts[2] };
        let number_str = number_part
            .strip_prefix("=")
            .unwrap_or("")
            .trim_end_matches(";");
        let number: u32 = number_str.parse().ok()?;
        Some(ProtoField {
            name: name.to_string(),
            ty: ty.to_string(),
            number,
            repeated,
        })
    }
    fn parse_proto_service(
        &self,
        first_line: &str,
        lines: &mut std::iter::Peekable<std::str::Lines>,
    ) -> Option<ProtoService> {
        let name = first_line
            .strip_prefix("service ")?
            .trim_end_matches(" {")
            .trim()
            .to_string();
        let mut methods = Vec::new();
        while let Some(line) = lines.next() {
            let line = line.trim();
            if line == "}" {
                break;
            }
            if line.is_empty() || line.starts_with("//") {
                continue;
            }
            if let Some(method) = self.parse_proto_method(line) {
                methods.push(method);
            }
        }
        Some(ProtoService { name, methods })
    }
    fn parse_proto_method(&self, line: &str) -> Option<ProtoMethod> {
        let line = line.strip_prefix("rpc ")?.trim();
        let paren_pos = line.find('(')?;
        let returns_pos = line.find("returns")?;
        let name = line[..paren_pos].trim().to_string();
        let input_part = &line[paren_pos + 1..returns_pos];
        let output_part = &line[returns_pos + 8..];
        let input_type = input_part.trim_end_matches(')').trim().to_string();
        let output_type = output_part
            .trim_start_matches('(')
            .trim_end_matches(");")
            .trim()
            .to_string();
        Some(ProtoMethod {
            name,
            input_type,
            output_type,
        })
    }
    fn parse_openapi_spec(&self, file_path: &str) -> Result<OpenAPISchema> {
        let content = fs::read_to_string(file_path)
            .map_err(|e| ToolError::ExecutionFailed(
                format!("Failed to read {}: {}", file_path, e),
            ))?;
        let ext = Path::new(file_path).extension().unwrap_or_default();
        let value: Value = if ext == "yaml" || ext == "yml" {
            serde_yaml::from_str(&content)
                .map_err(|e| ToolError::ExecutionFailed(
                    format!("Failed to parse YAML: {}", e),
                ))?
        } else {
            serde_json::from_str(&content)
                .map_err(|e| ToolError::ExecutionFailed(
                    format!("Failed to parse JSON: {}", e),
                ))?
        };
        let mut schemas = Vec::new();
        let mut paths = Vec::new();
        if let Some(components) = value.get("components") {
            if let Some(schemas_obj) = components.get("schemas") {
                if let Some(schemas_map) = schemas_obj.as_object() {
                    for (name, schema) in schemas_map {
                        if let Some(schema_obj) = schema.as_object() {
                            let mut properties = Vec::new();
                            let mut required_fields = Vec::new();
                            if let Some(props) = schema_obj.get("properties") {
                                if let Some(props_map) = props.as_object() {
                                    for (prop_name, prop_schema) in props_map {
                                        if let Some(prop_obj) = prop_schema.as_object() {
                                            let ty = self.extract_openapi_type(prop_obj);
                                            properties
                                                .push(OpenAPIProperty {
                                                    name: prop_name.clone(),
                                                    ty,
                                                    required: false,
                                                });
                                        }
                                    }
                                }
                            }
                            if let Some(required) = schema_obj.get("required") {
                                if let Some(required_arr) = required.as_array() {
                                    for req in required_arr {
                                        if let Some(req_str) = req.as_str() {
                                            required_fields.push(req_str.to_string());
                                        }
                                    }
                                }
                            }
                            for prop in &mut properties {
                                prop.required = required_fields.contains(&prop.name);
                            }
                            schemas
                                .push(OpenAPISchemaItem {
                                    name: name.clone(),
                                    properties,
                                });
                        }
                    }
                }
            }
        }
        if let Some(paths_obj) = value.get("paths") {
            if let Some(paths_map) = paths_obj.as_object() {
                for (path, path_item) in paths_map {
                    if let Some(path_obj) = path_item.as_object() {
                        for (method, operation) in path_obj {
                            if let Some(op_obj) = operation.as_object() {
                                let operation_id = op_obj
                                    .get("operationId")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("unknown")
                                    .to_string();
                                let request_body = self
                                    .extract_openapi_request_body(op_obj);
                                let response_body = self
                                    .extract_openapi_response_body(op_obj);
                                paths
                                    .push(OpenAPIPath {
                                        path: path.clone(),
                                        method: method.to_uppercase(),
                                        operation_id,
                                        request_body,
                                        response_body,
                                    });
                            }
                        }
                    }
                }
            }
        }
        Ok(OpenAPISchema {
            components: OpenAPIComponents { schemas },
            paths,
        })
    }
    fn extract_openapi_type(&self, schema: &Map<String, Value>) -> String {
        if let Some(ty) = schema.get("type") {
            if let Some(type_str) = ty.as_str() {
                match type_str {
                    "string" => "String".to_string(),
                    "integer" => "i64".to_string(),
                    "number" => "f64".to_string(),
                    "boolean" => "bool".to_string(),
                    "array" => {
                        if let Some(items) = schema.get("items") {
                            if let Some(items_obj) = items.as_object() {
                                let item_type = self.extract_openapi_type(items_obj);
                                format!("Vec<{}>", item_type)
                            } else {
                                "Vec<String>".to_string()
                            }
                        } else {
                            "Vec<String>".to_string()
                        }
                    }
                    _ => "String".to_string(),
                }
            } else {
                "String".to_string()
            }
        } else {
            "String".to_string()
        }
    }
    fn extract_openapi_request_body(
        &self,
        operation: &Map<String, Value>,
    ) -> Option<String> {
        operation
            .get("requestBody")
            .and_then(|rb| rb.get("content"))
            .and_then(|content| content.get("application/json"))
            .and_then(|schema| schema.get("schema"))
            .and_then(|schema| schema.get("$ref"))
            .and_then(|ref_str| ref_str.as_str())
            .map(|ref_str| {
                ref_str
                    .strip_prefix("#/components/schemas/")
                    .unwrap_or(ref_str)
                    .to_string()
            })
    }
    fn extract_openapi_response_body(
        &self,
        operation: &Map<String, Value>,
    ) -> Option<String> {
        operation
            .get("responses")
            .and_then(|resp| resp.get("200"))
            .and_then(|resp| resp.get("content"))
            .and_then(|content| content.get("application/json"))
            .and_then(|schema| schema.get("schema"))
            .and_then(|schema| schema.get("$ref"))
            .and_then(|ref_str| ref_str.as_str())
            .map(|ref_str| {
                ref_str
                    .strip_prefix("#/components/schemas/")
                    .unwrap_or(ref_str)
                    .to_string()
            })
    }
    fn parse_graphql_schema(&self, file_path: &str) -> Result<GraphQLSchema> {
        let content = fs::read_to_string(file_path)
            .map_err(|e| ToolError::ExecutionFailed(
                format!("Failed to read {}: {}", file_path, e),
            ))?;
        let mut types = Vec::new();
        let mut queries = Vec::new();
        let mut mutations = Vec::new();
        let mut lines = content.lines().peekable();
        while let Some(line) = lines.next() {
            let line = line.trim();
            if line.starts_with("type ") && !line.contains("Query")
                && !line.contains("Mutation")
            {
                if let Some(ty) = self.parse_graphql_type(line, &mut lines) {
                    types.push(ty);
                }
            } else if line.contains("type Query") {
                queries = self.parse_graphql_fields(&mut lines);
            } else if line.contains("type Mutation") {
                mutations = self.parse_graphql_fields(&mut lines);
            }
        }
        Ok(GraphQLSchema {
            types,
            queries,
            mutations,
        })
    }
    fn parse_graphql_type(
        &self,
        first_line: &str,
        lines: &mut std::iter::Peekable<std::str::Lines>,
    ) -> Option<GraphQLType> {
        let name = first_line
            .strip_prefix("type ")?
            .trim_end_matches(" {")
            .trim()
            .to_string();
        let fields = self.parse_graphql_fields(lines);
        Some(GraphQLType { name, fields })
    }
    fn parse_graphql_fields(
        &self,
        lines: &mut std::iter::Peekable<std::str::Lines>,
    ) -> Vec<GraphQLField> {
        let mut fields = Vec::new();
        while let Some(line) = lines.next() {
            let line = line.trim();
            if line == "}" {
                break;
            }
            if line.is_empty() || line.starts_with("#") {
                continue;
            }
            if let Some(field) = self.parse_graphql_field(line) {
                fields.push(field);
            }
        }
        fields
    }
    fn parse_graphql_field(&self, line: &str) -> Option<GraphQLField> {
        let line = line.trim_end_matches(',').trim();
        let colon_pos = line.find(':')?;
        let name_part = &line[..colon_pos];
        let type_part = &line[colon_pos + 1..];
        let (name, args) = if let Some(paren_pos) = name_part.find('(') {
            let name = name_part[..paren_pos].trim().to_string();
            let args_str = &name_part[paren_pos + 1..name_part.len() - 1];
            let args = self.parse_graphql_args(args_str);
            (name, args)
        } else {
            (name_part.trim().to_string(), Vec::new())
        };
        let ty = self.parse_graphql_type_annotation(type_part.trim());
        Some(GraphQLField { name, ty, args })
    }
    fn parse_graphql_args(&self, args_str: &str) -> Vec<GraphQLArg> {
        let mut args = Vec::new();
        if args_str.is_empty() {
            return args;
        }
        for arg in args_str.split(',') {
            let arg = arg.trim();
            if let Some(colon_pos) = arg.find(':') {
                let name = arg[..colon_pos].trim().to_string();
                let ty = self.parse_graphql_type_annotation(arg[colon_pos + 1..].trim());
                args.push(GraphQLArg { name, ty });
            }
        }
        args
    }
    fn parse_graphql_type_annotation(&self, ty_str: &str) -> String {
        let ty_str = ty_str.trim();
        if ty_str.starts_with('[') && ty_str.ends_with(']') {
            let inner = &ty_str[1..ty_str.len() - 1];
            let inner = inner.trim_end_matches('!');
            let inner_type = self.parse_graphql_type_annotation(inner);
            format!("Vec<{}>", inner_type)
        } else {
            let ty = ty_str.trim_end_matches('!');
            match ty {
                "String" => "String".to_string(),
                "Int" => "i32".to_string(),
                "Float" => "f64".to_string(),
                "Boolean" => "bool".to_string(),
                "ID" => "String".to_string(),
                _ => ty.to_string(),
            }
        }
    }
    fn generate_rust_bindings(
        &self,
        schema: &SchemaType,
        format: &str,
    ) -> Result<String> {
        match schema {
            SchemaType::Proto(proto) => self.generate_proto_rust_bindings(proto, format),
            SchemaType::OpenAPI(openapi) => {
                self.generate_openapi_rust_bindings(openapi, format)
            }
            SchemaType::GraphQL(graphql) => {
                self.generate_graphql_rust_bindings(graphql, format)
            }
        }
    }
    fn generate_proto_rust_bindings(
        &self,
        schema: &ProtoSchema,
        format: &str,
    ) -> Result<String> {
        let mut code = format!(
            "// Generated Rust bindings from Protocol Buffer schema\n\n"
        );
        if format == "serde" {
            code.push_str("use serde::{Deserialize, Serialize};\n\n");
        }
        for message in &schema.messages {
            if format == "serde" {
                code.push_str("#[derive(Debug, Clone, Serialize, Deserialize)]\n");
            } else {
                code.push_str("#[derive(Debug, Clone)]\n");
            }
            code.push_str(&format!("pub struct {} {{\n", message.name));
            for field in &message.fields {
                let rust_type = self.proto_type_to_rust(&field.ty, field.repeated);
                code.push_str(&format!("    pub {}: {},\n", field.name, rust_type));
            }
            code.push_str("}\n\n");
            code.push_str(&format!("impl {} {{\n", message.name));
            code.push_str(&format!("    pub fn new() -> Self {{\n"));
            code.push_str(&format!("        {} {{\n", message.name));
            for field in &message.fields {
                let default_value = self.get_default_value(&field.ty, field.repeated);
                code.push_str(
                    &format!("            {}: {},\n", field.name, default_value),
                );
            }
            code.push_str("        }\n");
            code.push_str("    }\n");
            code.push_str("}\n\n");
        }
        for service in &schema.services {
            code.push_str(&format!("pub trait {} {{\n", service.name));
            for method in &service.methods {
                code.push_str(
                    &format!("    async fn {}(\n", method.name.to_lowercase()),
                );
                code.push_str(&format!("        &mut self,\n"));
                code.push_str(&format!("        request: {},\n", method.input_type));
                code.push_str(
                    &format!("    ) -> Result<{}, tonic::Status>;\n", method.output_type),
                );
            }
            code.push_str("}\n\n");
        }
        Ok(code)
    }
    fn generate_openapi_rust_bindings(
        &self,
        schema: &OpenAPISchema,
        format: &str,
    ) -> Result<String> {
        let mut code = format!("// Generated Rust bindings from OpenAPI schema\n\n");
        if format == "serde" {
            code.push_str("use serde::{Deserialize, Serialize};\n\n");
        }
        for schema_item in &schema.components.schemas {
            if format == "serde" {
                code.push_str("#[derive(Debug, Clone, Serialize, Deserialize)]\n");
            } else {
                code.push_str("#[derive(Debug, Clone)]\n");
            }
            code.push_str(&format!("pub struct {} {{\n", schema_item.name));
            for property in &schema_item.properties {
                let rust_type = if property.required {
                    property.ty.clone()
                } else {
                    format!("Option<{}>", property.ty)
                };
                code.push_str(&format!("    pub {}: {},\n", property.name, rust_type));
            }
            code.push_str("}\n\n");
        }
        code.push_str("pub struct ApiClient {\n");
        code.push_str("    base_url: String,\n");
        code.push_str("    client: reqwest::Client,\n");
        code.push_str("}\n\n");
        code.push_str("impl ApiClient {\n");
        code.push_str("    pub fn new(base_url: String) -> Self {\n");
        code.push_str("        Self {\n");
        code.push_str("            base_url,\n");
        code.push_str("            client: reqwest::Client::new(),\n");
        code.push_str("        }\n");
        code.push_str("    }\n\n");
        for path in &schema.paths {
            let method_name = path.operation_id.replace("-", "_").replace(".", "_");
            let http_method = path.method.to_lowercase();
            code.push_str(&format!("    pub async fn {}(&self", method_name));
            if let Some(req_body) = &path.request_body {
                code.push_str(&format!(", request: &{}", req_body));
            }
            let response_type = path
                .response_body
                .as_ref()
                .map(|s| s.as_str())
                .unwrap_or("String");
            code.push_str(
                &format!(") -> Result<{}, reqwest::Error> {{\n", response_type),
            );
            code.push_str(
                &format!(
                    "        let url = format!(\"{{}}{{}}\", self.base_url, \"{}\");\n",
                    path.path
                ),
            );
            if let Some(req_body) = &path.request_body {
                code.push_str(
                    &format!("        let response = self.client.{}(url)\n", http_method),
                );
                code.push_str("            .json(request)\n");
                code.push_str("            .send()\n");
                code.push_str("            .await?;\n");
            } else {
                code.push_str(
                    &format!("        let response = self.client.{}(url)\n", http_method),
                );
                code.push_str("            .send()\n");
                code.push_str("            .await?;\n");
            }
            if let Some(resp_body) = &path.response_body {
                code.push_str(
                    &format!(
                        "        let result = response.json::<{}>().await?;\n", resp_body
                    ),
                );
                code.push_str("        Ok(result)\n");
            } else {
                code.push_str("        Ok(())\n");
            }
            code.push_str("    }\n\n");
        }
        code.push_str("}\n");
        Ok(code)
    }
    fn generate_graphql_rust_bindings(
        &self,
        schema: &GraphQLSchema,
        format: &str,
    ) -> Result<String> {
        let mut code = format!("// Generated Rust bindings from GraphQL schema\n\n");
        if format == "serde" {
            code.push_str("use serde::{Deserialize, Serialize};\n");
            code.push_str("use graphql_client::{GraphQLQuery, Response};\n\n");
        }
        for ty in &schema.types {
            if format == "serde" {
                code.push_str("#[derive(Debug, Clone, Serialize, Deserialize)]\n");
            } else {
                code.push_str("#[derive(Debug, Clone)]\n");
            }
            code.push_str(&format!("pub struct {} {{\n", ty.name));
            for field in &ty.fields {
                code.push_str(&format!("    pub {}: {},\n", field.name, field.ty));
            }
            code.push_str("}\n\n");
        }
        for query in &schema.queries {
            let query_name = format!("{}Query", query.name);
            let variables_name = format!("{}Variables", query.name);
            if format == "serde" {
                code.push_str(&format!("#[derive(GraphQLQuery))]\n"));
                code.push_str(&format!("#[graphql(\n"));
                code.push_str(&format!("    schema_path = \"schema.json\",\n"));
                code.push_str(
                    &format!(
                        "    query_path = \"{}.graphql\",\n", query.name.to_lowercase()
                    ),
                );
                code.push_str(&format!("    response_derives = Clone\n"));
                code.push_str(&format!(")]\n"));
            }
            code.push_str(&format!("pub struct {};\n\n", query_name));
            if !query.args.is_empty() {
                code.push_str(&format!("pub struct {} {{\n", variables_name));
                for arg in &query.args {
                    code.push_str(&format!("    pub {}: {},\n", arg.name, arg.ty));
                }
                code.push_str("}\n\n");
            }
        }
        Ok(code)
    }
    fn generate_grpc_client(&self, schema: &ProtoSchema) -> Result<String> {
        let mut code = "// Generated gRPC client code\n\n".to_string();
        code.push_str("use tonic::transport::Channel;\n");
        code.push_str("use tonic::{Request, Response, Status};\n\n");
        for service in &schema.services {
            let client_name = format!("{}Client", service.name);
            code.push_str("#[derive(Debug, Clone)]\n");
            code.push_str(&format!("pub struct {} {{\n", client_name));
            code.push_str("    client: Box<dyn ");
            code.push_str(&service.name);
            code.push_str(">,\n");
            code.push_str("}\n\n");
            code.push_str(&format!("impl {} {{\n", client_name));
            code.push_str(
                "    pub async fn connect<D>(dst: D) -> Result<Self, tonic::transport::Error>\n",
            );
            code.push_str("    where\n");
            code.push_str(
                "        D: std::convert::TryInto<tonic::transport::Endpoint>,\n",
            );
            code.push_str(
                "        D::Error: Into<Box<dyn std::error::Error + Send + Sync>>,\n",
            );
            code.push_str("    {\n");
            code.push_str(
                "        let conn = tonic::transport::Endpoint::new(dst)?.connect().await?;\n",
            );
            code.push_str("        Ok(Self {\n");
            code.push_str("            client: Box::new(\n");
            code.push_str("                // Initialize your service client here\n");
            code.push_str("                todo!()\n");
            code.push_str("            ),\n");
            code.push_str("        })\n");
            code.push_str("    }\n\n");
            for method in &service.methods {
                let method_name = method.name.to_lowercase();
                code.push_str(&format!("    pub async fn {}(\n", method_name));
                code.push_str("        &mut self,\n");
                code.push_str(&format!("        request: {},\n", method.input_type));
                code.push_str(
                    &format!("    ) -> Result<{}, Status> {{\n", method.output_type),
                );
                code.push_str("        self.client.");
                code.push_str(&method_name);
                code.push_str("(request).await\n");
                code.push_str("    }\n\n");
            }
            code.push_str("}\n\n");
        }
        Ok(code)
    }
    fn proto_type_to_rust(&self, proto_type: &str, repeated: bool) -> String {
        let base_type = match proto_type {
            "string" => "String",
            "int32" => "i32",
            "int64" => "i64",
            "uint32" => "u32",
            "uint64" => "u64",
            "sint32" => "i32",
            "sint64" => "i64",
            "fixed32" => "u32",
            "fixed64" => "u64",
            "sfixed32" => "i32",
            "sfixed64" => "i64",
            "bool" => "bool",
            "float" => "f32",
            "double" => "f64",
            "bytes" => "Vec<u8>",
            _ => proto_type,
        };
        if repeated { format!("Vec<{}>", base_type) } else { base_type.to_string() }
    }
    fn get_default_value(&self, proto_type: &str, repeated: bool) -> String {
        if repeated {
            "Vec::new()".to_string()
        } else {
            match proto_type {
                "string" => "String::new()".to_string(),
                "int32" | "int64" | "uint32" | "uint64" | "sint32" | "sint64" | "fixed32"
                | "fixed64" | "sfixed32" | "sfixed64" => "0".to_string(),
                "bool" => "false".to_string(),
                "float" | "double" => "0.0".to_string(),
                "bytes" => "Vec::new()".to_string(),
                _ => format!("{}::new()", proto_type),
            }
        }
    }
}
#[derive(Debug)]
enum SchemaType {
    Proto(ProtoSchema),
    OpenAPI(OpenAPISchema),
    GraphQL(GraphQLSchema),
}
impl Tool for ProtoBindTool {
    fn name(&self) -> &'static str {
        "proto-bind"
    }
    fn description(&self) -> &'static str {
        "Generate Rust bindings from Protocol Buffers, OpenAPI specs, or GraphQL schemas"
    }
    fn command(&self) -> Command {
        Command::new(self.name())
            .about(self.description())
            .long_about(
                "Auto-generate Rust bindings from various schema formats including Protocol Buffers (.proto), OpenAPI/Swagger specs (JSON/YAML), and GraphQL schemas (.graphql). Supports Serde derives and gRPC client generation.",
            )
            .args(
                &[
                    Arg::new("input")
                        .long("input")
                        .short('i')
                        .help("Input schema file (.proto, .json, .yaml, .graphql)")
                        .required(true),
                    Arg::new("format")
                        .long("format")
                        .short('f')
                        .help("Input format: proto, openapi, graphql")
                        .required(true),
                    Arg::new("output")
                        .long("output")
                        .short('o')
                        .help("Output file for generated bindings")
                        .default_value("generated/bindings.rs"),
                    Arg::new("grpc")
                        .long("grpc")
                        .help("Generate gRPC client code (proto only)")
                        .action(clap::ArgAction::SetTrue),
                    Arg::new("serde")
                        .long("serde")
                        .help("Add Serde derive macros")
                        .action(clap::ArgAction::SetTrue),
                    Arg::new("package")
                        .long("package")
                        .short('p')
                        .help("Rust package name for generated code"),
                ],
            )
            .args(&common_options())
    }
    fn execute(&self, matches: &ArgMatches) -> Result<()> {
        let input = matches.get_one::<String>("input").unwrap();
        let format = matches.get_one::<String>("format").unwrap();
        let output = matches.get_one::<String>("output").unwrap();
        let grpc = matches.get_flag("grpc");
        let serde = matches.get_flag("serde");
        let package = matches.get_one::<String>("package");
        let dry_run = matches.get_flag("dry-run");
        let verbose = matches.get_flag("verbose");
        let output_format = parse_output_format(matches);
        println!(
            "🔧 {} - {}", "CargoMate ProtoBind".bold().blue(), self.description()
            .cyan()
        );
        if !Path::new(input).exists() {
            return Err(
                ToolError::InvalidArguments(format!("Input file not found: {}", input)),
            );
        }
        let schema = match format.as_str() {
            "proto" => {
                let proto_schema = self.parse_proto_file(input)?;
                if verbose {
                    println!(
                        "   📄 Found {} messages and {} services", proto_schema
                        .messages.len(), proto_schema.services.len()
                    );
                }
                SchemaType::Proto(proto_schema)
            }
            "openapi" => {
                let openapi_schema = self.parse_openapi_spec(input)?;
                if verbose {
                    println!(
                        "   📄 Found {} schemas and {} paths", openapi_schema
                        .components.schemas.len(), openapi_schema.paths.len()
                    );
                }
                SchemaType::OpenAPI(openapi_schema)
            }
            "graphql" => {
                let graphql_schema = self.parse_graphql_schema(input)?;
                if verbose {
                    println!(
                        "   📄 Found {} types, {} queries, and {} mutations",
                        graphql_schema.types.len(), graphql_schema.queries.len(),
                        graphql_schema.mutations.len()
                    );
                }
                SchemaType::GraphQL(graphql_schema)
            }
            _ => {
                return Err(
                    ToolError::InvalidArguments(
                        format!("Unsupported format: {}", format),
                    ),
                );
            }
        };
        let format_option = if serde { "serde" } else { "plain" };
        let mut rust_code = self.generate_rust_bindings(&schema, format_option)?;
        if let Some(pkg) = package {
            let package_decl = format!("// Package: {}\n", pkg);
            rust_code.insert_str(0, &package_decl);
        }
        let mut grpc_code = String::new();
        if grpc && matches!(schema, SchemaType::Proto(_)) {
            if let SchemaType::Proto(proto_schema) = &schema {
                grpc_code = self.generate_grpc_client(proto_schema)?;
            }
        }
        if !grpc_code.is_empty() {
            rust_code.push_str("\n\n");
            rust_code.push_str(&grpc_code);
        }
        match output_format {
            OutputFormat::Human => {
                println!("  ✅ Generated Rust bindings for {} format", format.bold());
                println!("     → {}", output.cyan());
                if grpc && !grpc_code.is_empty() {
                    println!("  ✅ Generated gRPC client code");
                }
                if serde {
                    println!("  ✅ Added Serde derive macros");
                }
                if dry_run {
                    println!("   📋 {}", "Generated code preview:".bold());
                    println!("   {}", "─".repeat(50));
                    for (i, line) in rust_code.lines().take(20).enumerate() {
                        if i < 19 {
                            println!("   {}", line);
                        } else {
                            println!("   ... (truncated)");
                            break;
                        }
                    }
                } else {
                    if let Some(parent) = Path::new(output).parent() {
                        fs::create_dir_all(parent)
                            .map_err(|e| ToolError::ExecutionFailed(
                                format!("Failed to create output directory: {}", e),
                            ))?;
                    }
                    fs::write(output, rust_code)
                        .map_err(|e| ToolError::ExecutionFailed(
                            format!("Failed to write {}: {}", output, e),
                        ))?;
                    println!("  💾 File written successfully");
                }
            }
            OutputFormat::Json => {
                let result = serde_json::json!(
                    { "format" : format, "input" : input, "output" : output,
                    "grpc_generated" : grpc && ! grpc_code.is_empty(), "serde_enabled" :
                    serde, "code_preview" : rust_code.lines().take(10).collect::< Vec < _
                    >> ().join("\n") }
                );
                println!("{}", serde_json::to_string_pretty(& result).unwrap());
            }
            OutputFormat::Table => {
                println!(
                    "{:<15} {:<10} {:<8} {:<8}", "Format", "Input", "gRPC", "Serde"
                );
                println!("{}", "─".repeat(50));
                println!(
                    "{:<15} {:<10} {:<8} {:<8}", format, Path::new(input).file_name()
                    .unwrap_or_default().to_string_lossy(), if grpc { "Yes" } else { "No"
                    }, if serde { "Yes" } else { "No" }
                );
            }
        }
        println!("\n🎉 Binding generation completed!");
        Ok(())
    }
}
impl Default for ProtoBindTool {
    fn default() -> Self {
        Self::new()
    }
}