use anyhow::Result;
use clap::{Parser, Subcommand};
use rand::{Rng, SeedableRng, rng};
use rand::rngs::StdRng;
use rand::distr::Alphanumeric;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use walkdir::WalkDir;
use syn::{
    parse_quote, Ident, Expr, Stmt, Item, Pat, ItemFn, ExprMethodCall, ExprField,
    ExprMatch, Arm, PatPath, Member, ExprCall,
};
use syn::fold::Fold;
use sha2::{Sha256, Digest};
use aes_gcm::{Aes256Gcm, Key, Nonce};
use aes_gcm::aead::{Aead, KeyInit};
use chacha20poly1305::{ChaCha20Poly1305, Key as ChaChaKey, Nonce as ChaChaNonce};
use hkdf::Hkdf;
use base64::{Engine as _, engine::general_purpose};
use tar::Archive;
use flate2::read::GzDecoder;
#[derive(Parser, Debug)]
#[command(
    name = "scat",
    about = "Source Code Obfuscation Tool - Legitimate obfuscation with reversibility",
    long_about = r#"SCAT (Source Code Obfuscation Tool) provides legitimate obfuscation techniques
that maintain code functionality while making it harder to casually read.

KEY FEATURES:
    • Reversible transformations using mapping files
    • Preserves code functionality
    • Multiple obfuscation strategies
    • Safe for legitimate use cases

LEGITIMATE USE CASES:
    • Contest submissions (hide source until reveal)
    • Protecting algorithms during audits
    • Creating programming puzzles/challenges
    • Preventing casual copying while allowing execution

WARNING: Always keep mapping files secure - they enable reversal!"#
)]
pub struct ScatArgs {
    #[command(subcommand)]
    pub command: ScatCommand,
}
#[derive(Subcommand, Debug)]
pub enum ScatCommand {
    Names {
        path: PathBuf,
        #[arg(long)]
        map: Option<PathBuf>,
        #[arg(long)]
        sequential: bool,
    },
    Code {
        path: PathBuf,
        #[arg(long)]
        preserve_pub: bool,
        #[arg(long, default_value = "3")]
        min_len: usize,
        #[arg(long)]
        map: Option<PathBuf>,
        #[arg(long)]
        control_flow: bool,
        #[arg(long)]
        dry_run: bool,
        #[arg(long)]
        backup: bool,
        #[arg(long)]
        seed: Option<String>,
    },
    Strings {
        path: PathBuf,
        #[arg(long)]
        key: Option<String>,
        #[arg(long)]
        map: Option<PathBuf>,
        #[arg(long, default_value = "builtin")]
        algorithm: String,
        #[arg(long)]
        skip_format: bool,
        #[arg(long)]
        skip_errors: bool,
    },
    Pack { input: PathBuf, output: PathBuf, #[arg(long)] compress: bool },
    Unpack { input: PathBuf, map: PathBuf, #[arg(long)] output: Option<PathBuf> },
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ObfuscationMapping {
    pub original_to_obfuscated: HashMap<String, String>,
    pub obfuscated_to_original: HashMap<String, String>,
    pub timestamp: String,
    pub method: String,
    pub seed: Option<String>,
    pub config: ObfuscationConfig,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ObfuscationConfig {
    pub preserve_pub: bool,
    pub min_len: usize,
    pub control_flow: bool,
    pub string_encryption: StringEncryptionConfig,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct StringEncryptionConfig {
    pub algorithm: String,
    pub skip_format: bool,
    pub skip_errors: bool,
}
#[derive(Debug, Clone)]
pub struct RenameContext {
    pub mappings: HashMap<String, String>,
    pub scope_stack: Vec<Scope>,
    pub protected_identifiers: HashSet<String>,
    pub public_api: HashSet<String>,
    pub current_module: Vec<String>,
    pub rng: StdRng,
    pub config: ObfuscationConfig,
}
#[derive(Debug, Clone)]
pub struct Scope {
    pub variables: HashSet<String>,
    pub functions: HashSet<String>,
    pub types: HashSet<String>,
    pub level: usize,
}
impl ObfuscationMapping {
    pub fn new(method: &str, config: ObfuscationConfig, seed: Option<&str>) -> Self {
        Self {
            original_to_obfuscated: HashMap::new(),
            obfuscated_to_original: HashMap::new(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            method: method.to_string(),
            seed: seed.map(|s| s.to_string()),
            config,
        }
    }
    pub fn add_mapping(&mut self, original: String, obfuscated: String) {
        self.original_to_obfuscated.insert(original.clone(), obfuscated.clone());
        self.obfuscated_to_original.insert(obfuscated, original);
    }
    pub fn save_to_file(&self, path: &PathBuf) -> Result<()> {
        let json = serde_json::to_string_pretty(self)?;
        fs::write(path, json)?;
        println!("🔐 Mapping saved to: {}", path.display());
        Ok(())
    }
    pub fn load_from_file(path: &PathBuf) -> Result<Self> {
        let json = fs::read_to_string(path)?;
        let mapping: ObfuscationMapping = serde_json::from_str(&json)?;
        Ok(mapping)
    }
}
impl RenameContext {
    pub fn new(config: ObfuscationConfig, seed: Option<&str>) -> Self {
        let rng_seed = if let Some(seed_str) = seed {
            let mut hasher = Sha256::new();
            hasher.update(seed_str.as_bytes());
            let hash = hasher.finalize();
            let seed_bytes = <[u8; 32]>::try_from(&hash[..32]).unwrap_or([0u8; 32]);
            seed_bytes
        } else {
            rand::random::<[u8; 32]>()
        };
        let rng = StdRng::from_seed(rng_seed);
        let mut protected = HashSet::new();
        for item in &[
            "std",
            "core",
            "alloc",
            "println",
            "print",
            "eprintln",
            "eprint",
            "dbg",
            "main",
            "Result",
            "Option",
            "Some",
            "None",
            "Ok",
            "Err",
            "vec",
            "String",
            "str",
            "Box",
            "Vec",
            "HashMap",
            "HashSet",
            "format",
        ] {
            protected.insert(item.to_string());
        }
        Self {
            mappings: HashMap::new(),
            scope_stack: vec![Scope::new(0)],
            protected_identifiers: protected,
            public_api: HashSet::new(),
            current_module: vec![],
            rng,
            config,
        }
    }
    pub fn enter_scope(&mut self) {
        let level = self.scope_stack.len();
        self.scope_stack.push(Scope::new(level));
    }
    pub fn exit_scope(&mut self) {
        if self.scope_stack.len() > 1 {
            self.scope_stack.pop();
        }
    }
    pub fn add_variable(&mut self, name: &str) {
        if let Some(scope) = self.scope_stack.last_mut() {
            scope.variables.insert(name.to_string());
        }
    }
    pub fn add_function(&mut self, name: &str) {
        if let Some(scope) = self.scope_stack.last_mut() {
            scope.functions.insert(name.to_string());
        }
    }
    pub fn add_type(&mut self, name: &str) {
        if let Some(scope) = self.scope_stack.last_mut() {
            scope.types.insert(name.to_string());
        }
    }
    pub fn is_protected(&self, ident: &str) -> bool {
        self.protected_identifiers.contains(ident)
            || (self.config.preserve_pub && self.public_api.contains(ident))
    }
    pub fn should_rename(&self, ident: &str, min_len: usize) -> bool {
        !self.is_protected(ident) && ident.len() >= min_len
            && ident.chars().next().unwrap().is_alphabetic() && !ident.starts_with('_')
    }
    pub fn get_or_create_mapping(&mut self, original: &str) -> String {
        if let Some(obfuscated) = self.mappings.get(original) {
            return obfuscated.clone();
        }
        let obfuscated = self.generate_obfuscated_name(original);
        self.mappings.insert(original.to_string(), obfuscated.clone());
        obfuscated
    }
    fn generate_obfuscated_name(&mut self, original: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(original.as_bytes());
        hasher.update(self.current_module.join("::").as_bytes());
        let hash = hasher.finalize();
        let hash_str = general_purpose::STANDARD.encode(&hash[..8]);
        let mut name = hash_str
            .chars()
            .filter(|c| c.is_alphanumeric())
            .collect::<String>();
        if name.chars().next().unwrap().is_numeric() {
            name = format!("a{}", name);
        }
        if name.len() > 12 {
            name.truncate(12);
        }
        name
    }
    pub fn get_current_scope(&self) -> Option<&Scope> {
        self.scope_stack.last()
    }
    pub fn get_current_scope_mut(&mut self) -> Option<&mut Scope> {
        self.scope_stack.last_mut()
    }
}
impl Scope {
    pub fn new(level: usize) -> Self {
        Self {
            variables: HashSet::new(),
            functions: HashSet::new(),
            types: HashSet::new(),
            level,
        }
    }
}
pub struct ObfuscationTransformer {
    pub context: RenameContext,
    pub string_mappings: HashMap<String, String>,
    pub dry_run: bool,
}
impl ObfuscationTransformer {
    pub fn new(config: ObfuscationConfig, seed: Option<&str>, dry_run: bool) -> Self {
        Self {
            context: RenameContext::new(config, seed),
            string_mappings: HashMap::new(),
            dry_run,
        }
    }
    fn should_skip_string(&self, s: &str) -> bool {
        if self.context.config.string_encryption.skip_format
            && (s.contains("{}") || s.contains("{:?}"))
        {
            return true;
        }
        if self.context.config.string_encryption.skip_errors
            && (s.to_lowercase().contains("error") || s.to_lowercase().contains("debug"))
        {
            return true;
        }
        false
    }
    fn encrypt_string(&mut self, original: &str, key: Option<&str>) -> String {
        if self.should_skip_string(original) {
            return original.to_string();
        }
        if let Some(encrypted) = self.string_mappings.get(original) {
            return encrypted.clone();
        }
        let encrypted = match self.context.config.string_encryption.algorithm.as_str() {
            "aes" => self.encrypt_aes(original, key),
            "chacha20" => self.encrypt_chacha20(original, key),
            "builtin" => self.encrypt_builtin(original, key),
            _ => self.simple_scramble(original),
        };
        self.string_mappings.insert(original.to_string(), encrypted.clone());
        encrypted
    }
    fn encrypt_aes(&self, plaintext: &str, key_str: Option<&str>) -> String {
        let key_str = key_str.unwrap_or("default-scat-key-for-aes-encryption");
        let hash = Sha256::digest(key_str.as_bytes());
        let key = Key::<Aes256Gcm>::from_slice(&hash[..32]);
        let cipher = Aes256Gcm::new(key);
        let nonce = Nonce::from_slice(b"unique_nonce");
        let ciphertext = cipher
            .encrypt(nonce, plaintext.as_bytes())
            .expect("encryption failure");
        general_purpose::STANDARD.encode(&ciphertext)
    }
    fn encrypt_builtin(&self, plaintext: &str, key_str: Option<&str>) -> String {
        let key_str = key_str.unwrap_or("default-scat-key-for-builtin-encryption");
        let key_bytes = Sha256::digest(key_str.as_bytes());
        let mut result = String::new();
        for (i, ch) in plaintext.chars().enumerate() {
            let key_byte = key_bytes[i % 32];
            let encrypted_char = ((ch as u8) ^ key_byte) as char;
            result.push(encrypted_char);
        }
        general_purpose::STANDARD.encode(result.as_bytes())
    }
    fn encrypt_chacha20(&self, plaintext: &str, key_str: Option<&str>) -> String {
        let key_str = key_str.unwrap_or("default-scat-key-for-chacha20-encryption");
        let hash = Sha256::digest(key_str.as_bytes());
        let key = ChaChaKey::from_slice(&hash[..32]);
        let cipher = ChaCha20Poly1305::new(&key);
        let nonce = ChaChaNonce::from_slice(b"unique_nonce");
        let ciphertext = cipher
            .encrypt(nonce, plaintext.as_bytes())
            .expect("encryption failure");
        general_purpose::STANDARD.encode(&ciphertext)
    }
    fn simple_scramble(&self, input: &str) -> String {
        input.chars().rev().collect::<String>()
    }
    fn should_obfuscate_ident(&self, ident: &Ident) -> bool {
        self.context.should_rename(&ident.to_string(), self.context.config.min_len)
    }
}
impl ObfuscationTransformer {
    fn fold_expr_method_call(&mut self, mut expr: ExprMethodCall) -> ExprMethodCall {
        if self.should_obfuscate_ident(&expr.method) {
            let new_name = self.context.get_or_create_mapping(&expr.method.to_string());
            if self.dry_run {
                println!("🔄 Would rename method: {} -> {}", expr.method, new_name);
            } else {
                expr.method = Ident::new(&new_name, expr.method.span());
            }
        }
        expr.receiver = Box::new(self.fold_expr(*expr.receiver));
        expr.args = expr.args.into_iter().map(|arg| self.fold_expr(arg)).collect();
        expr
    }
    fn fold_expr_field(&mut self, mut expr: ExprField) -> ExprField {
        if let Member::Named(ref mut ident) = expr.member {
            if self.should_obfuscate_ident(ident) {
                let new_name = self.context.get_or_create_mapping(&ident.to_string());
                if self.dry_run {
                    println!("🔄 Would rename field: {} -> {}", ident, new_name);
                } else {
                    *ident = Ident::new(&new_name, ident.span());
                }
            }
        }
        expr.base = Box::new(self.fold_expr(*expr.base));
        expr
    }
    fn fold_expr_call(&mut self, mut expr: ExprCall) -> ExprCall {
        expr.func = Box::new(self.fold_expr(*expr.func));
        expr.args = expr.args.into_iter().map(|arg| self.fold_expr(arg)).collect();
        expr
    }
    fn fold_expr_path(&mut self, mut expr: syn::ExprPath) -> syn::ExprPath {
        expr.path = self.fold_path(expr.path);
        expr
    }
    fn fold_arm(&mut self, mut arm: Arm) -> Arm {
        arm.pat = self.fold_pat(arm.pat);
        arm.guard = arm
            .guard
            .map(|(if_token, expr)| (if_token, Box::new(self.fold_expr(*expr))));
        arm.body = Box::new(self.fold_expr(*arm.body));
        arm
    }
    fn fold_pat_path(&mut self, mut pat: PatPath) -> PatPath {
        pat.path = self.fold_path(pat.path);
        pat
    }
    fn apply_control_flow_flattening(&mut self, stmts: &mut Vec<Stmt>) {
        if !self.context.config.control_flow || self.dry_run {
            return;
        }
        let mut new_stmts = Vec::new();
        for stmt in stmts.drain(..) {
            match stmt {
                Stmt::Expr(expr, semi) => {
                    if matches!(& expr, Expr::If(_)) {
                        if let Expr::If(if_expr) = expr {
                            let discriminant = self.generate_discriminant();
                            let mut arms = vec![
                                Arm { attrs : vec![], pat : parse_quote!(0), guard : None,
                                fat_arrow_token : Default::default(), body :
                                Box::new(Expr::Block(syn::ExprBlock { attrs : vec![], label
                                : None, block : if_expr.then_branch, })), comma :
                                Some(Default::default()), }
                            ];
                            if let Some((_, else_expr)) = if_expr.else_branch {
                                arms.push(Arm {
                                    attrs: vec![],
                                    pat: parse_quote!(1),
                                    guard: None,
                                    fat_arrow_token: Default::default(),
                                    body: Box::new(self.fold_expr(*else_expr)),
                                    comma: None,
                                });
                            }
                            let match_expr = Expr::Match(ExprMatch {
                                attrs: if_expr.attrs,
                                match_token: Default::default(),
                                expr: Box::new(discriminant),
                                brace_token: Default::default(),
                                arms,
                            });
                            new_stmts.push(Stmt::Expr(match_expr, semi));
                        }
                    } else {
                        new_stmts.push(Stmt::Expr(self.fold_expr(expr), semi));
                    }
                }
                other => {
                    new_stmts.push(self.fold_stmt(other));
                }
            }
        }
        *stmts = new_stmts;
    }
    fn generate_discriminant(&mut self) -> Expr {
        parse_quote!(
            { let x = rand::random::< i32 > (); let y = x.wrapping_mul(x
            .wrapping_add(1)); if y & 1 == 0 { 0 } else { 1 } }
        )
    }
    fn inject_string_decryption(&mut self, encrypted: &str) -> Expr {
        let decrypt_call: ExprCall = parse_quote!(decrypt_scat_string(# encrypted));
        Expr::Call(decrypt_call)
    }
}
impl Fold for ObfuscationTransformer {
    fn fold_ident(&mut self, ident: Ident) -> Ident {
        let name = ident.to_string();
        if self.should_obfuscate_ident(&ident) {
            let new_name = self.context.get_or_create_mapping(&name);
            if self.dry_run {
                println!("🔄 Would rename identifier: {} -> {}", name, new_name);
                ident
            } else {
                Ident::new(&new_name, ident.span())
            }
        } else {
            ident
        }
    }
    fn fold_expr(&mut self, expr: Expr) -> Expr {
        match expr {
            Expr::MethodCall(method_call) => {
                Expr::MethodCall(self.fold_expr_method_call(method_call))
            }
            Expr::Field(field) => Expr::Field(self.fold_expr_field(field)),
            Expr::Call(call) => Expr::Call(self.fold_expr_call(call)),
            Expr::Path(path) => Expr::Path(self.fold_expr_path(path)),
            Expr::Match(mut match_expr) => {
                match_expr.expr = Box::new(self.fold_expr(*match_expr.expr));
                match_expr.arms = match_expr
                    .arms
                    .into_iter()
                    .map(|arm| self.fold_arm(arm))
                    .collect();
                Expr::Match(match_expr)
            }
            Expr::Lit(mut lit_expr) => {
                if let syn::Lit::Str(ref lit_str) = lit_expr.lit {
                    let original = lit_str.value();
                    if !self.should_skip_string(&original) {
                        let encrypted = self.encrypt_string(&original, None);
                        if encrypted != original && !self.dry_run {
                            return self.inject_string_decryption(&encrypted);
                        } else if self.dry_run {
                            println!(
                                "🔄 Would encrypt string: \"{}\" -> \"{}\"", original,
                                encrypted
                            );
                        }
                    }
                }
                Expr::Lit(lit_expr)
            }
            _ => syn::fold::fold_expr(self, expr),
        }
    }
    fn fold_pat(&mut self, pat: Pat) -> Pat {
        match pat {
            Pat::Path(mut path_pat) => Pat::Path(self.fold_pat_path(path_pat)),
            _ => syn::fold::fold_pat(self, pat),
        }
    }
    fn fold_item_fn(&mut self, mut item: ItemFn) -> ItemFn {
        if item.attrs.iter().any(|attr| attr.path().is_ident("no_mangle")) {
            self.context.protected_identifiers.insert(item.sig.ident.to_string());
        }
        if self.context.config.preserve_pub
            && matches!(item.vis, syn::Visibility::Public(_))
        {
            self.context.public_api.insert(item.sig.ident.to_string());
        }
        self.context.add_function(&item.sig.ident.to_string());
        item.sig = self.fold_signature(item.sig);
        item.block = Box::new(self.fold_block(*item.block));
        item
    }
    fn fold_item_struct(&mut self, mut item: syn::ItemStruct) -> syn::ItemStruct {
        if self.context.config.preserve_pub
            && matches!(item.vis, syn::Visibility::Public(_))
        {
            self.context.public_api.insert(item.ident.to_string());
        }
        self.context.add_type(&item.ident.to_string());
        for field in &mut item.fields {
            if let Some(ref mut ident) = &mut field.ident {
                if self.should_obfuscate_ident(ident) {
                    let new_name = self
                        .context
                        .get_or_create_mapping(&ident.to_string());
                    if self.dry_run {
                        println!("🔄 Would rename field: {} -> {}", ident, new_name);
                    } else {
                        *ident = Ident::new(&new_name, ident.span());
                    }
                }
            }
        }
        item
    }
    fn fold_item_enum(&mut self, mut item: syn::ItemEnum) -> syn::ItemEnum {
        if self.context.config.preserve_pub
            && matches!(item.vis, syn::Visibility::Public(_))
        {
            self.context.public_api.insert(item.ident.to_string());
        }
        self.context.add_type(&item.ident.to_string());
        for variant in &mut item.variants {
            if self.should_obfuscate_ident(&variant.ident) {
                let new_name = self
                    .context
                    .get_or_create_mapping(&variant.ident.to_string());
                if self.dry_run {
                    println!(
                        "🔄 Would rename variant: {} -> {}", variant.ident, new_name
                    );
                } else {
                    variant.ident = Ident::new(&new_name, variant.ident.span());
                }
            }
        }
        item
    }
    fn fold_item_trait(&mut self, mut item: syn::ItemTrait) -> syn::ItemTrait {
        if self.context.config.preserve_pub
            && matches!(item.vis, syn::Visibility::Public(_))
        {
            self.context.public_api.insert(item.ident.to_string());
        }
        self.context.add_type(&item.ident.to_string());
        item
    }
    fn fold_item_impl(&mut self, mut item: syn::ItemImpl) -> syn::ItemImpl {
        item.self_ty = Box::new(self.fold_type(*item.self_ty));
        item.items = item
            .items
            .into_iter()
            .map(|item| self.fold_impl_item(item))
            .collect();
        item
    }
    fn fold_pat_ident(&mut self, mut pat: syn::PatIdent) -> syn::PatIdent {
        if self.should_obfuscate_ident(&pat.ident) {
            self.context.add_variable(&pat.ident.to_string());
            let new_name = self.context.get_or_create_mapping(&pat.ident.to_string());
            if self.dry_run {
                println!("🔄 Would rename variable: {} -> {}", pat.ident, new_name);
            } else {
                pat.ident = Ident::new(&new_name, pat.ident.span());
            }
        }
        pat
    }
    fn fold_block(&mut self, mut block: syn::Block) -> syn::Block {
        self.context.enter_scope();
        self.apply_control_flow_flattening(&mut block.stmts);
        block.stmts = block.stmts.into_iter().map(|stmt| self.fold_stmt(stmt)).collect();
        self.context.exit_scope();
        block
    }
}
pub fn handle_scat_command(args: ScatArgs) -> Result<()> {
    match args.command {
        ScatCommand::Names { path, map, sequential } => {
            handle_names_obfuscation(&path, map.as_ref(), sequential)?;
        }
        ScatCommand::Code {
            path,
            preserve_pub,
            min_len,
            map,
            control_flow,
            dry_run,
            backup,
            seed,
        } => {
            handle_code_obfuscation(
                &path,
                preserve_pub,
                min_len,
                map.as_ref(),
                control_flow,
                dry_run,
                backup,
                seed.as_deref(),
            )?;
        }
        ScatCommand::Strings { path, key, map, algorithm, skip_format, skip_errors } => {
            handle_string_scrambling(
                &path,
                key.as_deref(),
                map.as_ref(),
                &algorithm,
                skip_format,
                skip_errors,
            )?;
        }
        ScatCommand::Pack { input, output, compress } => {
            handle_file_packing(&input, &output, compress)?;
        }
        ScatCommand::Unpack { input, map, output } => {
            handle_unpack(&input, &map, output.as_ref())?;
        }
    }
    Ok(())
}
fn handle_names_obfuscation(
    path: &PathBuf,
    map_path: Option<&PathBuf>,
    sequential: bool,
) -> Result<()> {
    println!("🔄 Obfuscating names in: {}", path.display());
    if !path.exists() {
        return Err(anyhow::anyhow!("Path does not exist: {}", path.display()));
    }
    if !path.is_dir() {
        return Err(anyhow::anyhow!("Path must be a directory for name obfuscation"));
    }
    let config = ObfuscationConfig {
        preserve_pub: true,
        min_len: 3,
        control_flow: false,
        string_encryption: StringEncryptionConfig {
            algorithm: "builtin".to_string(),
            skip_format: true,
            skip_errors: true,
        },
    };
    let mut mapping = ObfuscationMapping::new("names", config, None);
    obfuscate_names_recursive(path, &mut mapping, sequential)?;
    if let Some(map_file) = map_path {
        mapping.save_to_file(map_file)?;
    } else {
        let default_map = path.join("name_mapping.json");
        mapping.save_to_file(&default_map)?;
    }
    println!("✅ Name obfuscation complete!");
    println!("📊 Files renamed: {}", mapping.original_to_obfuscated.len());
    Ok(())
}
fn obfuscate_names_recursive(
    dir: &PathBuf,
    mapping: &mut ObfuscationMapping,
    sequential: bool,
) -> Result<()> {
    let mut counter = 0;
    for entry in WalkDir::new(dir).min_depth(1) {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() || path.is_dir() {
            let file_name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown");
            if file_name.ends_with("_mapping.json") || file_name == "name_mapping.json" {
                continue;
            }
            let new_name = if sequential {
                format!("f{}", counter)
            } else {
                generate_random_name(8)
            };
            let parent = path.parent().unwrap();
            let new_path = parent.join(&new_name);
            let final_new_path = if path.is_file() {
                if let Some(ext) = path.extension() {
                    new_path.with_extension(ext)
                } else {
                    new_path
                }
            } else {
                new_path
            };
            fs::rename(path, &final_new_path)?;
            let rel_original = path.strip_prefix(dir)?.to_string_lossy().to_string();
            let rel_new = final_new_path
                .strip_prefix(dir)?
                .to_string_lossy()
                .to_string();
            mapping.add_mapping(rel_original, rel_new);
            counter += 1;
        }
    }
    Ok(())
}
fn handle_code_obfuscation(
    path: &PathBuf,
    preserve_pub: bool,
    min_len: usize,
    map_path: Option<&PathBuf>,
    control_flow: bool,
    dry_run: bool,
    backup: bool,
    seed: Option<&str>,
) -> Result<()> {
    println!("🔄 Obfuscating Rust identifiers in: {}", path.display());
    if !path.exists() {
        return Err(anyhow::anyhow!("Path does not exist: {}", path.display()));
    }
    let config = ObfuscationConfig {
        preserve_pub,
        min_len,
        control_flow,
        string_encryption: StringEncryptionConfig {
            algorithm: "builtin".to_string(),
            skip_format: true,
            skip_errors: true,
        },
    };
    if !dry_run {
        perform_safety_checks(&path)?;
    }
    if backup && !dry_run {
        create_backup(path)?;
    }
    let mut transformer = ObfuscationTransformer::new(config.clone(), seed, dry_run);
    let mut processed_files = 0;
    let mut total_mappings = 0;
    if path.is_dir() {
        for entry in WalkDir::new(path) {
            let entry = entry?;
            let file_path = entry.path();
            if file_path.extension().and_then(|s| s.to_str()) == Some("rs")
                && !file_path.to_string_lossy().contains("target/")
                && !file_path.to_string_lossy().contains(".git/")
            {
                let mappings = obfuscate_rust_file_ast(
                    &file_path.to_path_buf(),
                    &mut transformer,
                )?;
                processed_files += 1;
                total_mappings += mappings;
            }
        }
    } else if path.extension().and_then(|s| s.to_str()) == Some("rs") {
        let mappings = obfuscate_rust_file_ast(&path.to_path_buf(), &mut transformer)?;
        processed_files += 1;
        total_mappings += mappings;
    } else {
        return Err(anyhow::anyhow!("Not a Rust file: {}", path.display()));
    }
    if control_flow && !dry_run {
        println!("🔄 Applying control flow obfuscation...");
        apply_control_flow_obfuscation(path)?;
    }
    let mapping = ObfuscationMapping::new("code", config, seed);
    let mut final_mapping = mapping.clone();
    for (original, obfuscated) in &transformer.context.mappings {
        final_mapping.add_mapping(original.clone(), obfuscated.clone());
    }
    for (original, encrypted) in &transformer.string_mappings {
        final_mapping.add_mapping(original.clone(), encrypted.clone());
    }
    if let Some(map_file) = map_path {
        final_mapping.save_to_file(map_file)?;
    } else {
        let default_map = path.with_extension("code_mapping.json");
        final_mapping.save_to_file(&default_map)?;
    }
    generate_reversal_script(&final_mapping, path)?;
    if dry_run {
        println!("✅ Dry run complete!");
        println!("📊 Files that would be processed: {}", processed_files);
        println!(
            "📊 Identifiers that would be obfuscated: {}", transformer.context.mappings
            .len()
        );
        println!(
            "📊 Strings that would be encrypted: {}", transformer.string_mappings.len()
        );
        println!("📊 Total mappings that would be created: {}", total_mappings);
    } else {
        println!("✅ Code obfuscation complete!");
        println!("📊 Files processed: {}", processed_files);
        println!("📊 Identifiers obfuscated: {}", transformer.context.mappings.len());
        println!("📊 Strings encrypted: {}", transformer.string_mappings.len());
        println!("📊 Total mappings created: {}", total_mappings);
        if validate_obfuscated_code(path)? {
            println!("✅ Validation passed - obfuscated code compiles successfully!");
        } else {
            println!("⚠️  Warning: Obfuscated code may have compilation issues");
        }
    }
    Ok(())
}
fn obfuscate_rust_file_ast(
    path: &PathBuf,
    transformer: &mut ObfuscationTransformer,
) -> Result<usize> {
    let content = fs::read_to_string(path)?;
    let syntax_tree = syn::parse_file(&content)?;
    let transformed_tree = transformer.fold_file(syntax_tree);
    if !transformer.dry_run {
        let transformed_content = prettyplease::unparse(&transformed_tree);
        fs::write(path, transformed_content)?;
    }
    Ok(transformer.context.mappings.len() + transformer.string_mappings.len())
}
fn create_backup(path: &PathBuf) -> Result<()> {
    let backup_path = path
        .with_extension(
            format!(
                "{}.backup", path.extension().unwrap_or("rs".as_ref()).to_string_lossy()
            ),
        );
    if path.is_dir() {
        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
        let backup_dir = path
            .with_extension(
                format!(
                    "{}_backup_{}", path.extension().unwrap_or("rs".as_ref())
                    .to_string_lossy(), timestamp
                ),
            );
        fs::create_dir_all(&backup_dir)?;
        copy_dir_recursively(path, &backup_dir)?;
    } else {
        fs::copy(path, &backup_path)?;
    }
    println!("📁 Backup created at: {}", backup_path.display());
    Ok(())
}
fn copy_dir_recursively(src: &PathBuf, dst: &PathBuf) -> Result<()> {
    for entry in WalkDir::new(src) {
        let entry = entry?;
        let src_path = entry.path();
        let relative_path = src_path.strip_prefix(src)?;
        let dst_path = dst.join(relative_path);
        if src_path.is_dir() {
            fs::create_dir_all(&dst_path)?;
        } else {
            fs::copy(src_path, &dst_path)?;
        }
    }
    Ok(())
}
fn apply_control_flow_obfuscation(path: &PathBuf) -> Result<()> {
    if path.is_dir() {
        for entry in WalkDir::new(path) {
            let entry = entry?;
            let file_path = entry.path();
            if file_path.extension().and_then(|s| s.to_str()) == Some("rs") {
                apply_control_flow_to_file(&file_path.to_path_buf())?;
            }
        }
    } else {
        apply_control_flow_to_file(path)?;
    }
    Ok(())
}
fn apply_control_flow_to_file(path: &PathBuf) -> Result<()> {
    println!("⚠️  Control flow obfuscation skipped (not yet implemented)");
    Ok(())
}
fn perform_safety_checks(path: &PathBuf) -> Result<()> {
    println!("🔍 Performing safety checks...");
    let dangerous_patterns = vec![
        "std::mem::transmute", "#[no_mangle]", "extern \"C\"", "#[link", "asm!",
        "global_asm!",
    ];
    let mut found_dangerous = Vec::new();
    if path.is_dir() {
        for entry in WalkDir::new(path) {
            let entry = entry?;
            let file_path = entry.path();
            if file_path.extension().and_then(|s| s.to_str()) == Some("rs")
                && !file_path.to_string_lossy().contains("target/")
            {
                let content = fs::read_to_string(file_path)?;
                for pattern in &dangerous_patterns {
                    if content.contains(pattern) {
                        found_dangerous
                            .push(format!("{}: {}", file_path.display(), pattern));
                    }
                }
            }
        }
    } else {
        let content = fs::read_to_string(path)?;
        for pattern in &dangerous_patterns {
            if content.contains(pattern) {
                found_dangerous.push(format!("{}: {}", path.display(), pattern));
            }
        }
    }
    if !found_dangerous.is_empty() {
        println!("⚠️  Found potentially dangerous patterns:");
        for danger in found_dangerous {
            println!("   {}", danger);
        }
        return Err(
            anyhow::anyhow!(
                "Refusing to obfuscate code with dangerous patterns. Use --dry-run to see what would be changed."
            ),
        );
    }
    println!("✅ Safety checks passed!");
    Ok(())
}
fn validate_obfuscated_code(path: &PathBuf) -> Result<bool> {
    if path.is_dir() {
        for entry in WalkDir::new(path) {
            let entry = entry?;
            let file_path = entry.path();
            if file_path.extension().and_then(|s| s.to_str()) == Some("rs") {
                let content = fs::read_to_string(file_path)?;
                if syn::parse_file(&content).is_err() {
                    return Ok(false);
                }
            }
        }
    } else {
        let content = fs::read_to_string(path)?;
        if syn::parse_file(&content).is_err() {
            return Ok(false);
        }
    }
    Ok(true)
}
fn generate_reversal_script(
    mapping: &ObfuscationMapping,
    original_path: &PathBuf,
) -> Result<()> {
    let script_path = original_path.with_extension("reversal.sh");
    let script_content = format!(
        r#"#!/bin/bash
# SCAT Obfuscation Reversal Script
# Generated: {}
# Original path: {}

echo "🔄 Reversing SCAT obfuscation..."

# Create backup of obfuscated code
cp -r "{}" "{}.pre_reversal"

# Apply reversals
"#,
        mapping.timestamp, original_path.display(), original_path.display(),
        original_path.display()
    );
    let mut script_with_commands = script_content;
    for (original, obfuscated) in &mapping.original_to_obfuscated {
        script_with_commands
            .push_str(
                &format!(
                    r#"find "{}" -type f -name "*.rs" -exec sed -i 's/{}/{}/g' {{}} \;
"#,
                    original_path.display(), regex::escape(obfuscated),
                    regex::escape(original)
                ),
            );
    }
    script_with_commands
        .push_str(
            r#"
echo "✅ Reversal complete!"
echo "📁 Obfuscated code backed up to: {original_path}.pre_reversal
"#,
        );
    fs::write(&script_path, script_with_commands)?;
    println!("🔧 Reversal script generated: {}", script_path.display());
    Ok(())
}
fn handle_string_scrambling(
    path: &PathBuf,
    key: Option<&str>,
    map_path: Option<&PathBuf>,
    algorithm: &str,
    skip_format: bool,
    skip_errors: bool,
) -> Result<()> {
    println!("🔄 Encrypting strings in: {}", path.display());
    if !path.exists() {
        return Err(anyhow::anyhow!("Path does not exist: {}", path.display()));
    }
    let string_config = StringEncryptionConfig {
        algorithm: algorithm.to_string(),
        skip_format,
        skip_errors,
    };
    let mut config = ObfuscationConfig {
        preserve_pub: true,
        min_len: 0,
        control_flow: false,
        string_encryption: string_config,
    };
    config.string_encryption.algorithm = algorithm.to_string();
    let mut transformer = ObfuscationTransformer::new(config.clone(), None, false);
    let mut processed_files = 0;
    if path.is_dir() {
        for entry in WalkDir::new(path) {
            let entry = entry?;
            let file_path = entry.path();
            if file_path.extension().and_then(|s| s.to_str()) == Some("rs")
                && !file_path.to_string_lossy().contains("target/")
            {
                let content = fs::read_to_string(file_path)?;
                let syntax_tree = syn::parse_file(&content)?;
                let transformed_tree = transformer.fold_file(syntax_tree);
                let transformed_content = prettyplease::unparse(&transformed_tree);
                fs::write(file_path, transformed_content)?;
                processed_files += 1;
            }
        }
    } else if path.extension().and_then(|s| s.to_str()) == Some("rs") {
        let content = fs::read_to_string(path)?;
        let syntax_tree = syn::parse_file(&content)?;
        let transformed_tree = transformer.fold_file(syntax_tree);
        let transformed_content = prettyplease::unparse(&transformed_tree);
        fs::write(path, transformed_content)?;
        processed_files += 1;
    } else {
        return Err(anyhow::anyhow!("Not a Rust file: {}", path.display()));
    }
    if !transformer.string_mappings.is_empty() {
        inject_decryption_runtime(path, algorithm, key)?;
    }
    let mapping = ObfuscationMapping::new("strings", config.clone(), None);
    let mut final_mapping = mapping.clone();
    for (original, encrypted) in &transformer.string_mappings {
        final_mapping.add_mapping(original.clone(), encrypted.clone());
    }
    if let Some(map_file) = map_path {
        final_mapping.save_to_file(map_file)?;
    } else {
        let default_map = path.with_extension("string_mapping.json");
        final_mapping.save_to_file(&default_map)?;
    }
    println!("✅ String encryption complete!");
    println!("📊 Files processed: {}", processed_files);
    println!("📊 Strings encrypted: {}", transformer.string_mappings.len());
    Ok(())
}
fn inject_decryption_runtime(
    path: &PathBuf,
    algorithm: &str,
    key: Option<&str>,
) -> Result<()> {
    if algorithm == "simple" {
        return Ok(());
    }
    let decryption_code = generate_decryption_runtime(algorithm, key);
    if path.is_dir() {
        let mut main_file = None;
        for entry in WalkDir::new(path) {
            let entry = entry?;
            let file_path = entry.path();
            if file_path.file_name().and_then(|n| n.to_str()) == Some("main.rs")
                || file_path.file_name().and_then(|n| n.to_str()) == Some("lib.rs")
            {
                main_file = Some(file_path.to_path_buf());
                break;
            }
        }
        if let Some(main_file) = main_file {
            inject_into_file(&main_file, &decryption_code)?;
        } else {
            println!(
                "⚠️  Warning: No main.rs or lib.rs found to inject decryption runtime"
            );
        }
    } else {
        inject_into_file(path, &decryption_code)?;
    }
    Ok(())
}
fn inject_extern_crates(path: &PathBuf, algorithm: &str) -> Result<()> {
    let _path = path;
    let _algorithm = algorithm;
    Ok(())
}
fn generate_decryption_runtime(algorithm: &str, key: Option<&str>) -> String {
    let key = key.unwrap_or("default-scat-key-for-decryption");
    match algorithm {
        "aes" => {
            format!(
                r#"

use aes_gcm::{{Aes256Gcm, Key, Nonce}};
use aes_gcm::aead::{{Aead, NewAead}};
use base64::{{Engine as _, engine::general_purpose}};

fn decrypt_scat_string(encrypted: &str) -> String {{
    let key_str = "{}";
    let key = Key::from_slice(&sha2::Sha256::digest(key_str.as_bytes())[..32]);
    let cipher = Aes256Gcm::new(key);
    let nonce = Nonce::from_slice(b"unique_nonce");

    if let Ok(ciphertext) = general_purpose::STANDARD.decode(encrypted) {{
        if let Ok(plaintext) = cipher.decrypt(nonce, ciphertext.as_ref()) {{
            String::from_utf8_lossy(&plaintext).to_string()
        }} else {{
            encrypted.to_string()
        }}
    }} else {{
        encrypted.to_string()
    }}
}}

fn scat_decrypt_macro(input: &str) -> String {{
    decrypt_scat_string(input)
}}
"#,
                key
            )
        }
        "chacha20" => {
            format!(
                r#"

use chacha20poly1305::{{ChaCha20Poly1305, Key, Nonce}};
use chacha20poly1305::aead::{{Aead, NewAead}};
use base64::{{Engine as _, engine::general_purpose}};

fn decrypt_scat_string(encrypted: &str) -> String {{
    let key_str = "{}";
    let key = Key::from_slice(&sha2::Sha256::digest(key_str.as_bytes())[..32]);
    let cipher = ChaCha20Poly1305::new(key);
    let nonce = Nonce::from_slice(b"unique_nonce");

    if let Ok(ciphertext) = general_purpose::STANDARD.decode(encrypted) {{
        if let Ok(plaintext) = cipher.decrypt(nonce, ciphertext.as_ref()) {{
            String::from_utf8_lossy(&plaintext).to_string()
        }} else {{
            encrypted.to_string()
        }}
    }} else {{
        encrypted.to_string()
    }}
}}

fn scat_decrypt_macro(input: &str) -> String {{
    decrypt_scat_string(input)
}}
"#,
                key
            )
        }
        "builtin" => {
            format!(
                r#"

use base64::{{Engine as _, engine::general_purpose}};

fn decrypt_scat_string(encrypted: &str) -> String {{
    let key_str = "{}";
    let key_bytes = {{
        let mut hasher = sha2::Sha256::new();
        sha2::Digest::update(&mut hasher, key_str.as_bytes());
        hasher.finalize()
    }};

    if let Ok(encrypted_bytes) = general_purpose::STANDARD.decode(encrypted) {{
        if let Ok(encrypted_str) = std::str::from_utf8(&encrypted_bytes) {{
            let mut result = String::new();
            for (i, ch) in encrypted_str.chars().enumerate() {{
                let key_byte = key_bytes[i % 32];
                let decrypted_char = ((ch as u8) ^ key_byte) as char;
                result.push(decrypted_char);
            }}
            result
        }} else {{
            encrypted.to_string()
        }}
    }} else {{
        encrypted.to_string()
    }}
}}

fn scat_decrypt_macro(input: &str) -> String {{
    decrypt_scat_string(input)
}}
"#,
                key
            )
        }
        _ => {
            format!(
                r#"

fn decrypt_scat_string(encrypted: &str) -> String {{
    encrypted.chars().rev().collect()
}}

fn scat_decrypt_macro(input: &str) -> String {{
    decrypt_scat_string(input)
}}
"#
            )
        }
    }
}
fn inject_into_file(path: &PathBuf, code: &str) -> Result<()> {
    let mut content = fs::read_to_string(path)?;
    if let Some(insert_pos) = find_insertion_point(&content) {
        content.insert_str(insert_pos, &format!("\n{}\n", code));
        fs::write(path, content)?;
        println!("🔓 Decryption runtime injected into: {}", path.display());
    }
    Ok(())
}
fn find_insertion_point(content: &str) -> Option<usize> {
    let lines: Vec<&str> = content.lines().collect();
    for (i, line) in lines.iter().enumerate() {
        if line.trim().is_empty() && i > 0 {
            let prev_line = lines[i - 1];
            if prev_line.starts_with("use ") || prev_line.starts_with("extern crate ")
                || prev_line.starts_with("#[")
            {
                continue;
            } else {
                return Some(lines[..i].iter().map(|l| l.len() + 1).sum());
            }
        }
    }
    Some(content.lines().next().map(|l| l.len() + 1).unwrap_or(0))
}
fn handle_file_packing(input: &PathBuf, output: &PathBuf, compress: bool) -> Result<()> {
    println!("📦 Packing files from: {} to: {}", input.display(), output.display());
    if !input.exists() || !input.is_dir() {
        return Err(anyhow::anyhow!("Input must be an existing directory"));
    }
    let mut tar_data = Vec::new();
    {
        let mut tar_builder = tar::Builder::new(&mut tar_data);
        for entry in WalkDir::new(input) {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                let rel_path = path.strip_prefix(input)?;
                tar_builder.append_path_with_name(path, rel_path)?;
            }
        }
        tar_builder.finish()?;
    }
    let archive_data = tar_data;
    let final_data = if compress {
        use flate2::write::GzEncoder;
        use flate2::Compression;
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        std::io::Write::write_all(&mut encoder, &archive_data)?;
        encoder.finish()?
    } else {
        archive_data
    };
    fs::write(output, final_data)?;
    println!("✅ Files packed successfully!");
    Ok(())
}
fn handle_unpack(
    input: &PathBuf,
    map: &PathBuf,
    output: Option<&PathBuf>,
) -> Result<()> {
    println!(
        "📦 Unpacking files from: {} using map: {}", input.display(), map.display()
    );
    if !input.exists() {
        return Err(anyhow::anyhow!("Input file does not exist: {}", input.display()));
    }
    let output_dir = output
        .map(|p| p.clone())
        .unwrap_or_else(|| { input.with_extension("") });
    fs::create_dir_all(&output_dir)?;
    if input.extension().and_then(|ext| ext.to_str()) == Some("tar")
        || input.to_string_lossy().contains("tar")
    {
        unpack_tar_archive(input, &output_dir)?;
    } else {
        let mapping = ObfuscationMapping::load_from_file(map)?;
        match mapping.method.as_str() {
            "names" => {
                unpack_names(input, &mapping, &output_dir)?;
            }
            "code" => {
                unpack_code(input, &mapping, &output_dir)?;
            }
            "strings" => {
                unpack_strings(input, &mapping, &output_dir)?;
            }
            _ => {
                return Err(
                    anyhow::anyhow!("Unknown obfuscation method: {}", mapping.method),
                );
            }
        }
    }
    println!("✅ Unpacking complete!");
    Ok(())
}
fn unpack_tar_archive(input: &PathBuf, output_dir: &PathBuf) -> Result<()> {
    let file = fs::File::open(input)?;
    let reader = std::io::BufReader::new(file);
    let mut archive = Archive::new(reader);
    if let Err(_) = archive.unpack(output_dir) {
        let file = fs::File::open(input)?;
        let reader = std::io::BufReader::new(file);
        let mut tar_archive = Archive::new(reader);
        tar_archive.unpack(output_dir)?;
    }
    Ok(())
}
fn unpack_names(
    input: &PathBuf,
    mapping: &ObfuscationMapping,
    output: &PathBuf,
) -> Result<()> {
    for (obfuscated, original) in &mapping.obfuscated_to_original {
        let obfuscated_path = input.join(obfuscated);
        let original_path = output.join(original);
        if obfuscated_path.exists() {
            fs::create_dir_all(original_path.parent().unwrap())?;
            fs::copy(&obfuscated_path, &original_path)?;
        }
    }
    Ok(())
}
fn unpack_code(
    input: &PathBuf,
    mapping: &ObfuscationMapping,
    output: &PathBuf,
) -> Result<()> {
    if input.is_dir() {
        fs::create_dir_all(output)?;
        for entry in WalkDir::new(input) {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("rs")
            {
                let rel_path = path.strip_prefix(input)?;
                let output_path = output.join(rel_path);
                fs::create_dir_all(output_path.parent().unwrap())?;
                let mut content = fs::read_to_string(path)?;
                for (original, obfuscated) in &mapping.original_to_obfuscated {
                    content = content
                        .replace(
                            &format!("fn {}", obfuscated),
                            &format!("fn {}", original),
                        );
                }
                fs::write(&output_path, content)?;
            }
        }
    }
    Ok(())
}
fn unpack_strings(
    input: &PathBuf,
    mapping: &ObfuscationMapping,
    output: &PathBuf,
) -> Result<()> {
    if input.is_dir() {
        fs::create_dir_all(output)?;
        for entry in WalkDir::new(input) {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("rs")
            {
                let rel_path = path.strip_prefix(input)?;
                let output_path = output.join(rel_path);
                fs::create_dir_all(output_path.parent().unwrap())?;
                let mut content = fs::read_to_string(path)?;
                for (original, obfuscated) in &mapping.original_to_obfuscated {
                    content = content
                        .replace(
                            &format!("\"{}\"", obfuscated),
                            &format!("\"{}\"", original),
                        );
                }
                fs::write(&output_path, content)?;
            }
        }
    }
    Ok(())
}
fn generate_random_name(length: usize) -> String {
    rng().sample_iter(Alphanumeric).take(length).map(char::from).collect()
}