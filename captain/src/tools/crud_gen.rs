use super::{Tool, Result, ToolError, common_options, parse_output_format, OutputFormat};
use clap::{Arg, ArgMatches, Command};
use colored::*;
use std::fs;
use std::path::Path;
use syn::{parse_file, File, Item, ItemStruct, Fields, Field, Type, PathSegment, Ident};
use quote::quote;
use proc_macro2::TokenStream;
#[derive(Debug, Clone)]
pub struct CrudGenTool;
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
    is_primary_key: bool,
}
impl CrudGenTool {
    pub fn new() -> Self {
        Self
    }
    fn parse_struct_from_file(&self, file_path: &str) -> Result<Vec<StructInfo>> {
        let content = fs::read_to_string(file_path)
            .map_err(|e| ToolError::ExecutionFailed(
                format!("Failed to read {}: {}", file_path, e),
            ))?;
        let ast = parse_file(&content)
            .map_err(|e| ToolError::ExecutionFailed(
                format!("Failed to parse {}: {}", file_path, e),
            ))?;
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
        let is_primary_key = name == "id" || name == "uuid" || name.ends_with("_id");
        Some(FieldInfo {
            name,
            ty,
            is_optional,
            is_primary_key,
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
    fn generate_crud_operations(
        &self,
        struct_info: &StructInfo,
        backend: &str,
    ) -> Result<String> {
        match backend {
            "sql" => self.generate_sql_crud(struct_info),
            "diesel" => self.generate_diesel_crud(struct_info),
            "seaorm" => self.generate_seaorm_crud(struct_info),
            _ => {
                Err(
                    ToolError::ExecutionFailed(
                        format!("Unsupported backend: {}", backend),
                    ),
                )
            }
        }
    }
    fn generate_sql_crud(&self, struct_info: &StructInfo) -> Result<String> {
        let struct_name = &struct_info.name;
        let table_name = snake_case(struct_name);
        let primary_key = struct_info
            .fields
            .iter()
            .find(|f| f.is_primary_key)
            .map(|f| f.name.as_str())
            .unwrap_or("id");
        let mut code = format!("// SQL CRUD operations for {}\n", struct_name);
        code.push_str(&format!("// Table: {}\n\n", table_name));
        code.push_str(&format!("CREATE TABLE {} (\n", table_name));
        for (i, field) in struct_info.fields.iter().enumerate() {
            let comma = if i < struct_info.fields.len() - 1 { "," } else { "" };
            let pk = if field.is_primary_key { " PRIMARY KEY" } else { "" };
            let nullable = if field.is_optional { "" } else { " NOT NULL" };
            let sql_type = self.map_rust_type_to_sql(&field.ty);
            code.push_str(
                &format!(
                    "    {} {}{}{}{}\n", snake_case(& field.name), sql_type, nullable,
                    pk, comma
                ),
            );
        }
        code.push_str(");\n\n");
        code.push_str("// Insert operation\n");
        let columns: Vec<String> = struct_info
            .fields
            .iter()
            .map(|f| snake_case(&f.name))
            .collect();
        let placeholders: Vec<String> = (1..=struct_info.fields.len())
            .map(|i| format!("${}", i))
            .collect();
        code.push_str(
            &format!(
                "INSERT INTO {} ({}) VALUES ({});\n\n", table_name, columns.join(", "),
                placeholders.join(", ")
            ),
        );
        code.push_str("// Select by primary key\n");
        code.push_str(
            &format!(
                "SELECT {} FROM {} WHERE {} = $1;\n\n", columns.join(", "), table_name,
                snake_case(primary_key)
            ),
        );
        code.push_str("// Select all\n");
        code.push_str(
            &format!("SELECT {} FROM {};\n\n", columns.join(", "), table_name),
        );
        code.push_str("// Update operation\n");
        let updates: Vec<String> = struct_info
            .fields
            .iter()
            .filter(|f| !f.is_primary_key)
            .enumerate()
            .map(|(i, f)| format!("{} = ${}", snake_case(& f.name), i + 2))
            .collect();
        code.push_str(
            &format!(
                "UPDATE {} SET {} WHERE {} = $1;\n\n", table_name, updates.join(", "),
                snake_case(primary_key)
            ),
        );
        code.push_str("// Delete operation\n");
        code.push_str(
            &format!(
                "DELETE FROM {} WHERE {} = $1;\n", table_name, snake_case(primary_key)
            ),
        );
        Ok(code)
    }
    fn generate_diesel_crud(&self, struct_info: &StructInfo) -> Result<String> {
        let struct_name = &struct_info.name;
        let table_name = snake_case(struct_name);
        let mut code = format!("// Diesel CRUD operations for {}\n\n", struct_name);
        code.push_str("#[macro_use]\nextern crate diesel;\n\n");
        code.push_str(
            &format!(
                "table! {{\n    {} ({}) {{\n", table_name, snake_case(& struct_info
                .fields[0].name)
            ),
        );
        for field in &struct_info.fields[1..] {
            code.push_str(
                &format!(
                    "        {} -> {},\n", snake_case(& field.name), self
                    .map_rust_type_to_diesel(& field.ty)
                ),
            );
        }
        code.push_str("    }\n}\n\n");
        code.push_str("#[derive(Queryable)]\n");
        code.push_str(&format!("pub struct {} {{\n", struct_name));
        for field in &struct_info.fields {
            let vis = if field.name.starts_with('_') { "" } else { "pub " };
            code.push_str(&format!("    {}pub {}: {},\n", vis, field.name, field.ty));
        }
        code.push_str("}\n\n");
        code.push_str("#[derive(Insertable)]\n");
        code.push_str("#[table_name = \"");
        code.push_str(&table_name);
        code.push_str("\"]\n");
        code.push_str(&format!("pub struct New{} {{\n", struct_name));
        for field in &struct_info.fields {
            if !field.is_primary_key {
                let vis = if field.name.starts_with('_') { "" } else { "pub " };
                code.push_str(
                    &format!("    {}pub {}: {},\n", vis, field.name, field.ty),
                );
            }
        }
        code.push_str("}\n\n");
        code.push_str("// CRUD Operations\n");
        code.push_str(
            &format!(
                "pub fn create_{}(conn: &PgConnection, new_{}: &New{}) -> QueryResult<{}> {{\n",
                snake_case(struct_name), snake_case(struct_name), struct_name,
                struct_name
            ),
        );
        code.push_str(&format!("    diesel::insert_into({}::table)\n", table_name));
        code.push_str(&format!("        .values(new_{})\n", snake_case(struct_name)));
        code.push_str("        .get_result(conn)\n");
        code.push_str("}\n\n");
        code.push_str(
            &format!(
                "pub fn get_{}(conn: &PgConnection, id: i32) -> QueryResult<{}> {{\n",
                snake_case(struct_name), struct_name
            ),
        );
        code.push_str(&format!("    {}::table.find(id).first(conn)\n", table_name));
        code.push_str("}\n\n");
        code.push_str(
            &format!(
                "pub fn update_{}(conn: &PgConnection, id: i32, {}_update: &{}) -> QueryResult<{}> {{\n",
                snake_case(struct_name), snake_case(struct_name), struct_name,
                struct_name
            ),
        );
        code.push_str(&format!("    diesel::update({}::table.find(id))\n", table_name));
        code.push_str(&format!("        .set({}_update)\n", snake_case(struct_name)));
        code.push_str("        .get_result(conn)\n");
        code.push_str("}\n\n");
        code.push_str(
            &format!(
                "pub fn delete_{}(conn: &PgConnection, id: i32) -> QueryResult<usize> {{\n",
                snake_case(struct_name)
            ),
        );
        code.push_str(&format!("    diesel::delete({}::table.find(id))\n", table_name));
        code.push_str("        .execute(conn)\n");
        code.push_str("}\n");
        Ok(code)
    }
    fn generate_seaorm_crud(&self, struct_info: &StructInfo) -> Result<String> {
        let struct_name = &struct_info.name;
        let entity_name = format!("{}Entity", struct_name);
        let model_name = format!("{}Model", struct_name);
        let mut code = format!("// SeaORM CRUD operations for {}\n\n", struct_name);
        code.push_str("use sea_orm::entity::prelude::*;\n\n");
        code.push_str("#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]\n");
        code.push_str("#[sea_orm(table_name = \"");
        code.push_str(&snake_case(struct_name));
        code.push_str("\")]\n");
        code.push_str(&format!("pub struct {} {{\n", model_name));
        for field in &struct_info.fields {
            let column_type = self.map_rust_type_to_seaorm(&field.ty);
            code.push_str("    #[sea_orm(");
            if field.is_primary_key {
                code.push_str("primary_key");
            }
            code.push_str(")]\n");
            code.push_str(&format!("    pub {}: {},\n", field.name, column_type));
        }
        code.push_str("}\n\n");
        code.push_str(
            &format!(
                "#[derive(Copy, Clone, Default, Debug, DeriveRelation, EnumIter)]\n"
            ),
        );
        code.push_str(&format!("pub enum Relation {{\n"));
        code.push_str("}\n\n");
        code.push_str(&format!("impl RelationTrait for Relation {{\n"));
        code.push_str("    fn def(&self) -> RelationDef {\n");
        code.push_str("        match self {\n");
        code.push_str("            // Define relations here\n");
        code.push_str("        }\n");
        code.push_str("    }\n");
        code.push_str("}\n\n");
        code.push_str(&format!("impl ActiveModelBehavior for ActiveModel {{\n"));
        code.push_str("}\n\n");
        code.push_str("// CRUD Operations\n");
        code.push_str(&format!("pub async fn create_{}(\n", snake_case(struct_name)));
        code.push_str("    db: &DatabaseConnection,\n");
        code.push_str(&format!("    {}: {}\n", snake_case(struct_name), model_name));
        code.push_str(&format!(") -> Result<{}, DbErr> {{\n", model_name));
        code.push_str(
            &format!(
                "    {}::insert({}.into_active_model())\n", entity_name,
                snake_case(struct_name)
            ),
        );
        code.push_str("        .exec(db)\n");
        code.push_str("        .await\n");
        code.push_str("}\n\n");
        code.push_str(&format!("pub async fn get_{}(\n", snake_case(struct_name)));
        code.push_str("    db: &DatabaseConnection,\n");
        code.push_str("    id: i32,\n");
        code.push_str(&format!(") -> Result<Option<{}>, DbErr> {{\n", model_name));
        code.push_str(&format!("    {}::find_by_id(id).one(db).await\n", entity_name));
        code.push_str("}\n\n");
        code.push_str(&format!("pub async fn update_{}(\n", snake_case(struct_name)));
        code.push_str("    db: &DatabaseConnection,\n");
        code.push_str("    id: i32,\n");
        code.push_str(&format!("    {}: {}\n", snake_case(struct_name), model_name));
        code.push_str(&format!(") -> Result<{}, DbErr> {{\n", model_name));
        code.push_str(
            &format!(
                "    {}::update({}.into_active_model())\n", entity_name,
                snake_case(struct_name)
            ),
        );
        code.push_str("        .exec(db)\n");
        code.push_str("        .await\n");
        code.push_str("}\n\n");
        code.push_str(&format!("pub async fn delete_{}(\n", snake_case(struct_name)));
        code.push_str("    db: &DatabaseConnection,\n");
        code.push_str("    id: i32,\n");
        code.push_str(") -> Result<(), DbErr> {\n");
        code.push_str(
            &format!("    {}::delete_by_id(id).exec(db).await?;\n", entity_name),
        );
        code.push_str("    Ok(())\n");
        code.push_str("}\n");
        Ok(code)
    }
    fn generate_api_endpoints(
        &self,
        struct_info: &StructInfo,
        framework: &str,
    ) -> Result<String> {
        match framework {
            "axum" => self.generate_axum_api(struct_info),
            "actix" => self.generate_actix_api(struct_info),
            "rocket" => self.generate_rocket_api(struct_info),
            _ => {
                Err(
                    ToolError::ExecutionFailed(
                        format!("Unsupported framework: {}", framework),
                    ),
                )
            }
        }
    }
    fn generate_axum_api(&self, struct_info: &StructInfo) -> Result<String> {
        let struct_name = &struct_info.name;
        let route_name = snake_case(struct_name);
        let primary_key = struct_info
            .fields
            .iter()
            .find(|f| f.is_primary_key)
            .map(|f| f.name.as_str())
            .unwrap_or("id");
        let mut code = format!("// Axum API endpoints for {}\n\n", struct_name);
        code.push_str("use axum::{{\n");
        code.push_str("    extract::{Path, State},\n");
        code.push_str("    http::StatusCode,\n");
        code.push_str("    response::Json,\n");
        code.push_str("    routing::{get, post, put, delete},\n");
        code.push_str("    Router,\n");
        code.push_str("}};\n");
        code.push_str("use serde::{Deserialize, Serialize};\n");
        code.push_str("use std::sync::Arc;\n\n");
        code.push_str("#[derive(Debug, Serialize, Deserialize)]\n");
        code.push_str(&format!("pub struct Create{}Request {{\n", struct_name));
        for field in &struct_info.fields {
            if !field.is_primary_key {
                let ty = if field.is_optional {
                    field.ty.clone()
                } else {
                    field.ty.clone()
                };
                code.push_str(&format!("    pub {}: {},\n", field.name, ty));
            }
        }
        code.push_str("}\n\n");
        code.push_str("#[derive(Debug, Serialize, Deserialize)]\n");
        code.push_str(&format!("pub struct Update{}Request {{\n", struct_name));
        for field in &struct_info.fields {
            if !field.is_primary_key {
                code.push_str(
                    &format!("    pub {}: Option<{}>,\n", field.name, field.ty),
                );
            }
        }
        code.push_str("}\n\n");
        code.push_str("// Handler functions\n");
        code.push_str(&format!("pub async fn create_{}(\n", route_name));
        code.push_str("    State(state): State<Arc<AppState>>,\n");
        code.push_str(
            &format!("    Json(payload): Json<Create{}Request>,\n", struct_name),
        );
        code.push_str(&format!(") -> Result<Json<{}>, StatusCode> {{\n", struct_name));
        code.push_str("    // Implementation here\n");
        code.push_str("    // Call your CRUD create function\n");
        code.push_str("    todo!()\n");
        code.push_str("}\n\n");
        code.push_str(&format!("pub async fn get_{}(\n", route_name));
        code.push_str("    State(state): State<Arc<AppState>>,\n");
        code.push_str(
            &format!(
                "    Path({}): Path<{}>,\n", primary_key, self
                .get_primary_key_type(struct_info)
            ),
        );
        code.push_str(&format!(") -> Result<Json<{}>, StatusCode> {{\n", struct_name));
        code.push_str("    // Implementation here\n");
        code.push_str("    // Call your CRUD get function\n");
        code.push_str("    todo!()\n");
        code.push_str("}\n\n");
        code.push_str(&format!("pub async fn update_{}(\n", route_name));
        code.push_str("    State(state): State<Arc<AppState>>,\n");
        code.push_str(
            &format!(
                "    Path({}): Path<{}>,\n", primary_key, self
                .get_primary_key_type(struct_info)
            ),
        );
        code.push_str(
            &format!("    Json(payload): Json<Update{}Request>,\n", struct_name),
        );
        code.push_str(&format!(") -> Result<Json<{}>, StatusCode> {{\n", struct_name));
        code.push_str("    // Implementation here\n");
        code.push_str("    // Call your CRUD update function\n");
        code.push_str("    todo!()\n");
        code.push_str("}\n\n");
        code.push_str(&format!("pub async fn delete_{}(\n", route_name));
        code.push_str("    State(state): State<Arc<AppState>>,\n");
        code.push_str(
            &format!(
                "    Path({}): Path<{}>,\n", primary_key, self
                .get_primary_key_type(struct_info)
            ),
        );
        code.push_str(") -> Result<StatusCode, StatusCode> {\n");
        code.push_str("    // Implementation here\n");
        code.push_str("    // Call your CRUD delete function\n");
        code.push_str("    todo!()\n");
        code.push_str("}\n\n");
        code.push_str(&format!("pub async fn list_{}(\n", route_name));
        code.push_str("    State(state): State<Arc<AppState>>,\n");
        code.push_str(
            &format!(") -> Result<Json<Vec<{}>>, StatusCode> {{\n", struct_name),
        );
        code.push_str("    // Implementation here\n");
        code.push_str("    // Call your CRUD list function\n");
        code.push_str("    todo!()\n");
        code.push_str("}\n\n");
        code.push_str("// Router configuration\n");
        code.push_str(
            &format!("pub fn {}_routes() -> Router<Arc<AppState>> {{\n", route_name),
        );
        code.push_str("    Router::new()\n");
        code.push_str(&format!("        .route(\"/\", post(create_{}))\n", route_name));
        code.push_str(&format!("        .route(\"/\", get(list_{}))\n", route_name));
        code.push_str(
            &format!("        .route(\"/:{}\", get(get_{}))\n", primary_key, route_name),
        );
        code.push_str(
            &format!(
                "        .route(\"/:{}\", put(update_{}))\n", primary_key, route_name
            ),
        );
        code.push_str(
            &format!(
                "        .route(\"/:{}\", delete(delete_{}))\n", primary_key, route_name
            ),
        );
        code.push_str("}\n");
        Ok(code)
    }
    fn generate_actix_api(&self, struct_info: &StructInfo) -> Result<String> {
        let struct_name = &struct_info.name;
        let route_name = snake_case(struct_name);
        let mut code = format!("// Actix Web API endpoints for {}\n\n", struct_name);
        code.push_str("use actix_web::{{\n");
        code.push_str("    web, HttpResponse, Result,\n");
        code.push_str("}};\n");
        code.push_str("use serde::{Deserialize, Serialize};\n\n");
        code.push_str("#[derive(Debug, Serialize, Deserialize)]\n");
        code.push_str(&format!("pub struct Create{}Request {{\n", struct_name));
        for field in &struct_info.fields {
            if !field.is_primary_key {
                let ty = if field.is_optional {
                    field.ty.clone()
                } else {
                    field.ty.clone()
                };
                code.push_str(&format!("    pub {}: {},\n", field.name, ty));
            }
        }
        code.push_str("}\n\n");
        code.push_str("// Handler functions\n");
        code.push_str(&format!("pub async fn create_{}(\n", route_name));
        code.push_str("    pool: web::Data<DbPool>,\n");
        code.push_str(
            &format!("    payload: web::Json<Create{}Request>,\n", struct_name),
        );
        code.push_str(") -> Result<HttpResponse> {\n");
        code.push_str("    // Implementation here\n");
        code.push_str("    // Call your CRUD create function\n");
        code.push_str(
            "    Ok(HttpResponse::Created().json(serde_json::json!({\"status\": \"created\"})))\n",
        );
        code.push_str("}\n\n");
        code.push_str(&format!("pub async fn get_{}(\n", route_name));
        code.push_str("    pool: web::Data<DbPool>,\n");
        code.push_str("    path: web::Path<i32>,\n");
        code.push_str(") -> Result<HttpResponse> {\n");
        code.push_str("    // Implementation here\n");
        code.push_str("    // Call your CRUD get function\n");
        code.push_str(
            "    Ok(HttpResponse::Ok().json(serde_json::json!({\"status\": \"ok\"})))\n",
        );
        code.push_str("}\n\n");
        code.push_str("// Configure routes in your App\n");
        code.push_str("// .service(\n");
        code.push_str("//     web::scope(\"/");
        code.push_str(&route_name);
        code.push_str("\")\n");
        code.push_str("//         .route(\"\", web::post().to(create_");
        code.push_str(&route_name);
        code.push_str("))\n");
        code.push_str("//         .route(\"/{id}\", web::get().to(get_");
        code.push_str(&route_name);
        code.push_str("))\n");
        code.push_str("// )\n");
        Ok(code)
    }
    fn generate_rocket_api(&self, struct_info: &StructInfo) -> Result<String> {
        let struct_name = &struct_info.name;
        let route_name = snake_case(struct_name);
        let mut code = format!("// Rocket API endpoints for {}\n\n", struct_name);
        code.push_str("#[macro_use] extern crate rocket;\n\n");
        code.push_str("use rocket::{{\n");
        code.push_str("    serde::{Deserialize, Serialize, json::Json},\n");
        code.push_str("    State,\n");
        code.push_str("}};\n\n");
        code.push_str("#[derive(Debug, Serialize, Deserialize)]\n");
        code.push_str(&format!("pub struct Create{}Request {{\n", struct_name));
        for field in &struct_info.fields {
            if !field.is_primary_key {
                let ty = if field.is_optional {
                    field.ty.clone()
                } else {
                    field.ty.clone()
                };
                code.push_str(&format!("    pub {}: {},\n", field.name, ty));
            }
        }
        code.push_str("}\n\n");
        code.push_str("// Handler functions\n");
        code.push_str(&format!("#[post(\"/{}\", data = \"<payload>\")]\n", route_name));
        code.push_str(&format!("pub async fn create_{}(\n", route_name));
        code.push_str("    payload: Json<Create");
        code.push_str(struct_name);
        code.push_str("Request>,\n");
        code.push_str("    // Add your state/database pool here\n");
        code.push_str(") -> Json<serde_json::Value> {\n");
        code.push_str("    // Implementation here\n");
        code.push_str("    // Call your CRUD create function\n");
        code.push_str("    Json(serde_json::json!({\"status\": \"created\"}))\n");
        code.push_str("}\n\n");
        code.push_str(&format!("#[get(\"/{}/<id>\")]\n", route_name));
        code.push_str(&format!("pub async fn get_{}(\n", route_name));
        code.push_str("    id: i32,\n");
        code.push_str("    // Add your state/database pool here\n");
        code.push_str(") -> Json<serde_json::Value> {\n");
        code.push_str("    // Implementation here\n");
        code.push_str("    // Call your CRUD get function\n");
        code.push_str("    Json(serde_json::json!({\"status\": \"ok\"}))\n");
        code.push_str("}\n");
        Ok(code)
    }
    fn generate_validation_code(&self, struct_info: &StructInfo) -> Result<String> {
        let struct_name = &struct_info.name;
        let mut code = format!("// Validation code for {}\n\n", struct_name);
        code.push_str("use validator::Validate;\n\n");
        code.push_str("#[derive(Debug, Validate, serde::Deserialize)]\n");
        code.push_str(&format!("pub struct Create{}Request {{\n", struct_name));
        for field in &struct_info.fields {
            if !field.is_primary_key {
                let validation_attrs = self.generate_validation_attrs(field);
                code.push_str(&format!("    #[validate{}]\n", validation_attrs));
                let ty = if field.is_optional {
                    format!("Option<{}>", field.ty)
                } else {
                    field.ty.clone()
                };
                code.push_str(&format!("    pub {}: {},\n", field.name, ty));
            }
        }
        code.push_str("}\n\n");
        code.push_str("impl Create");
        code.push_str(struct_name);
        code.push_str("Request {\n");
        code.push_str("    pub fn validate_and_create(&self) -> Result<");
        code.push_str(struct_name);
        code.push_str(", ValidationError> {\n");
        code.push_str("        self.validate()?;\n");
        code.push_str("        // Create your entity here\n");
        code.push_str("        todo!()\n");
        code.push_str("    }\n");
        code.push_str("}\n\n");
        code.push_str("#[derive(Debug, thiserror::Error)]\n");
        code.push_str("pub enum ValidationError {\n");
        code.push_str("    #[error(\"Validation failed: {0}\")]\n");
        code.push_str("    Validation(#[from] validator::ValidationErrors),\n");
        code.push_str("    #[error(\"Creation failed: {0}\")]\n");
        code.push_str("    Creation(String),\n");
        code.push_str("}\n");
        Ok(code)
    }
    fn generate_validation_attrs(&self, field: &FieldInfo) -> String {
        let mut attrs = Vec::new();
        match field.ty.as_str() {
            "String" => {
                if !field.is_optional {
                    attrs.push("required".to_string());
                }
                attrs.push("length(min = 1, max = 255)".to_string());
            }
            "i32" | "i64" | "u32" | "u64" => {
                if !field.is_optional {
                    attrs.push("required".to_string());
                }
            }
            _ => {
                if !field.is_optional {
                    attrs.push("required".to_string());
                }
            }
        }
        if attrs.is_empty() { String::new() } else { format!("({})", attrs.join(", ")) }
    }
    fn map_rust_type_to_sql(&self, rust_type: &str) -> String {
        match rust_type {
            "String" => "VARCHAR(255)",
            "i32" => "INTEGER",
            "i64" => "BIGINT",
            "u32" => "INTEGER",
            "u64" => "BIGINT",
            "bool" => "BOOLEAN",
            "f32" => "REAL",
            "f64" => "DOUBLE PRECISION",
            "Option<String>" => "VARCHAR(255)",
            "Option<i32>" => "INTEGER",
            "Option<i64>" => "BIGINT",
            "Option<u32>" => "INTEGER",
            "Option<u64>" => "BIGINT",
            "Option<bool>" => "BOOLEAN",
            "Option<f32>" => "REAL",
            "Option<f64>" => "DOUBLE PRECISION",
            _ => "TEXT",
        }
            .to_string()
    }
    fn map_rust_type_to_diesel(&self, rust_type: &str) -> String {
        match rust_type {
            "String" => "Text",
            "i32" => "Integer",
            "i64" => "BigInt",
            "u32" => "Integer",
            "u64" => "BigInt",
            "bool" => "Bool",
            "f32" => "Float",
            "f64" => "Double",
            "Option<String>" => "Nullable<Text>",
            "Option<i32>" => "Nullable<Integer>",
            "Option<i64>" => "Nullable<BigInt>",
            "Option<u32>" => "Nullable<Integer>",
            "Option<u64>" => "Nullable<BigInt>",
            "Option<bool>" => "Nullable<Bool>",
            "Option<f32>" => "Nullable<Float>",
            "Option<f64>" => "Nullable<Double>",
            _ => "Text",
        }
            .to_string()
    }
    fn map_rust_type_to_seaorm(&self, rust_type: &str) -> String {
        match rust_type {
            "String" => "String",
            "i32" => "i32",
            "i64" => "i64",
            "u32" => "u32",
            "u64" => "u64",
            "bool" => "bool",
            "f32" => "f32",
            "f64" => "f64",
            "Option<String>" => "Option<String>",
            "Option<i32>" => "Option<i32>",
            "Option<i64>" => "Option<i64>",
            "Option<u32>" => "Option<u32>",
            "Option<u64>" => "Option<u64>",
            "Option<bool>" => "Option<bool>",
            "Option<f32>" => "Option<f32>",
            "Option<f64>" => "Option<f64>",
            _ => "String",
        }
            .to_string()
    }
    fn get_primary_key_type(&self, struct_info: &StructInfo) -> String {
        if let Some(pk_field) = struct_info.fields.iter().find(|f| f.is_primary_key) {
            pk_field.ty.clone()
        } else {
            "i32".to_string()
        }
    }
}
fn snake_case(s: &str) -> String {
    let mut result = String::new();
    let mut chars = s.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch.is_uppercase() {
            if !result.is_empty() {
                result.push('_');
            }
            result.push(ch.to_lowercase().next().unwrap());
        } else {
            result.push(ch);
        }
    }
    result
}
impl Tool for CrudGenTool {
    fn name(&self) -> &'static str {
        "crud-gen"
    }
    fn description(&self) -> &'static str {
        "Generate CRUD operations from struct definitions"
    }
    fn command(&self) -> Command {
        Command::new(self.name())
            .about(self.description())
            .long_about(
                "Generate complete CRUD (Create, Read, Update, Delete) operations from Rust struct definitions. Supports SQL, Diesel, SeaORM backends and Axum, Actix, Rocket web frameworks.",
            )
            .args(
                &[
                    Arg::new("input")
                        .long("input")
                        .short('i')
                        .help("Input Rust file containing struct definitions")
                        .required(true),
                    Arg::new("backend")
                        .long("backend")
                        .short('b')
                        .help("Storage backend: sql, diesel, seaorm")
                        .default_value("sql"),
                    Arg::new("framework")
                        .long("framework")
                        .short('f')
                        .help("Web framework: axum, actix, rocket")
                        .default_value("axum"),
                    Arg::new("output")
                        .long("output")
                        .short('o')
                        .help("Output directory for generated files")
                        .default_value("generated/"),
                    Arg::new("validation")
                        .long("validation")
                        .help("Generate validation code")
                        .action(clap::ArgAction::SetTrue),
                    Arg::new("tests")
                        .long("tests")
                        .help("Generate unit tests")
                        .action(clap::ArgAction::SetTrue),
                ],
            )
            .args(&common_options())
    }
    fn execute(&self, matches: &ArgMatches) -> Result<()> {
        let input = matches.get_one::<String>("input").unwrap();
        let backend = matches.get_one::<String>("backend").unwrap();
        let framework = matches.get_one::<String>("framework").unwrap();
        let output = matches.get_one::<String>("output").unwrap();
        let dry_run = matches.get_flag("dry-run");
        let verbose = matches.get_flag("verbose");
        let validation = matches.get_flag("validation");
        let tests = matches.get_flag("tests");
        let output_format = parse_output_format(matches);
        println!(
            "🔧 {} - {}", "CargoMate CrudGen".bold().blue(), self.description().cyan()
        );
        if !Path::new(input).exists() {
            return Err(
                ToolError::InvalidArguments(format!("Input file not found: {}", input)),
            );
        }
        if !dry_run {
            fs::create_dir_all(output)
                .map_err(|e| ToolError::ExecutionFailed(
                    format!("Failed to create output directory: {}", e),
                ))?;
        }
        let structs = self.parse_struct_from_file(input)?;
        if structs.is_empty() {
            println!("{}", "No structs found in input file".yellow());
            return Ok(());
        }
        for struct_info in &structs {
            println!("📝 Processing struct: {}", struct_info.name.bold());
            if verbose {
                println!("   Fields:");
                for field in &struct_info.fields {
                    let pk = if field.is_primary_key { " (PK)" } else { "" };
                    let opt = if field.is_optional { " (optional)" } else { "" };
                    println!("     - {}: {}{}{}", field.name, field.ty, pk, opt);
                }
            }
            let crud_code = self.generate_crud_operations(struct_info, backend)?;
            let crud_file = format!(
                "{}/{}_crud_{}.rs", output, snake_case(& struct_info.name), backend
            );
            let api_code = self.generate_api_endpoints(struct_info, framework)?;
            let api_file = format!(
                "{}/{}_api_{}.rs", output, snake_case(& struct_info.name), framework
            );
            let validation_code = if validation {
                Some(self.generate_validation_code(struct_info)?)
            } else {
                None
            };
            let validation_file = format!(
                "{}/{}_validation.rs", output, snake_case(& struct_info.name)
            );
            match output_format {
                OutputFormat::Human => {
                    println!("  ✅ Generated {} backend code", backend.bold());
                    println!("     → {}", crud_file.cyan());
                    println!("  ✅ Generated {} API endpoints", framework.bold());
                    println!("     → {}", api_file.cyan());
                    if validation {
                        println!("  ✅ Generated validation code");
                        println!("     → {}", validation_file.cyan());
                    }
                    if dry_run {
                        println!("   📋 {}", "Generated code preview:".bold());
                        println!("   {}", "─".repeat(50));
                        println!(
                            "{}", crud_code.lines().take(10).collect::< Vec < _ >> ()
                            .join("\n")
                        );
                        println!("   ...");
                    } else {
                        fs::write(&crud_file, crud_code)
                            .map_err(|e| ToolError::ExecutionFailed(
                                format!("Failed to write {}: {}", crud_file, e),
                            ))?;
                        fs::write(&api_file, api_code)
                            .map_err(|e| ToolError::ExecutionFailed(
                                format!("Failed to write {}: {}", api_file, e),
                            ))?;
                        if let Some(validation_code) = validation_code {
                            fs::write(&validation_file, validation_code)
                                .map_err(|e| ToolError::ExecutionFailed(
                                    format!("Failed to write {}: {}", validation_file, e),
                                ))?;
                        }
                        println!("  💾 Files written successfully");
                    }
                }
                OutputFormat::Json => {
                    let result = serde_json::json!(
                        { "struct" : struct_info.name, "backend" : backend, "framework" :
                        framework, "files" : { "crud" : crud_file, "api" : api_file,
                        "validation" : validation_file }, "crud_code" : crud_code,
                        "api_code" : api_code }
                    );
                    println!("{}", serde_json::to_string_pretty(& result).unwrap());
                }
                OutputFormat::Table => {
                    println!("{:<20} {:<15} {:<15}", "Struct", "Backend", "Framework");
                    println!("{}", "─".repeat(50));
                    println!(
                        "{:<20} {:<15} {:<15}", struct_info.name, backend, framework
                    );
                }
            }
        }
        println!("\n🎉 CRUD generation completed!");
        Ok(())
    }
}
impl Default for CrudGenTool {
    fn default() -> Self {
        Self::new()
    }
}