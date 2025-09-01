use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;
use std::collections::HashMap;

fn main() {
    println!("Building cargo-mate with dynamic captain binary loading...");
    // No need to embed binaries, we'll download them at runtime
}

fn process_binary(binary_data: &[u8], target: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    // Apply obfuscation and encryption (simplified version)
    let mut processed = Vec::new();
    
    // Simple XOR obfuscation with target-specific key
    let key = generate_target_key(target);
    
    for (i, &byte) in binary_data.iter().enumerate() {
        let obfuscated = byte ^ key[i % key.len()];
        processed.push(obfuscated);
    }
    
    // Add a simple header for identification
    let mut final_binary = Vec::new();
    final_binary.extend_from_slice(b"CAPTAIN");
    final_binary.extend_from_slice(&(processed.len() as u64).to_le_bytes());
    final_binary.extend_from_slice(&processed);
    
    Ok(final_binary)
}

fn generate_target_key(target: &str) -> Vec<u8> {
    // Generate a target-specific obfuscation key
    let mut key = Vec::new();
    let mut hash = 0u64;
    
    for byte in target.bytes() {
        hash = hash.wrapping_mul(31).wrapping_add(byte as u64);
    }
    
    // Generate 32-byte key from hash using a safer approach
    for i in 0..32 {
        // Use modulo to avoid overflow, and rotate the hash
        let rotated_hash = hash.rotate_left(i as u32);
        let key_byte = (rotated_hash & 0xFF) as u8;
        key.push(key_byte);
    }
    
    key
}

fn generate_embedder_module(embedded_dir: &Path, binaries: &HashMap<String, std::path::PathBuf>) -> Result<(), Box<dyn std::error::Error>> {
    let module_content = format!(r#"// Auto-generated embedder module for captain binaries
use anyhow::Result;
use std::path::PathBuf;
use std::process::Command;
use tempfile::NamedTempFile;

// Embedded binary data
{}
"#, generate_binary_constants(binaries));
    
    let embedder_path = embedded_dir
        .parent()
        .ok_or("Failed to get parent directory")?
        .join("embedder.rs");
    fs::write(embedder_path, module_content)?;
    
    Ok(())
}

fn generate_binary_constants(binaries: &HashMap<String, std::path::PathBuf>) -> String {
    let mut constants = String::new();
    
    for (name, path) in binaries {
        // Handle path stripping more safely
        let relative_path = if let Ok(stripped) = path.strip_prefix(Path::new("src").join("captain").join("embedded")) {
            stripped.to_string_lossy().to_string()
        } else {
            // Fallback: just use the filename
            path.file_name()
                .unwrap_or_else(|| path.as_os_str())
                .to_string_lossy()
                .to_string()
        };
        
        constants.push_str(&format!(
            "pub const {}: &[u8] = include_bytes!(\"embedded/{}\");\n",
            name.to_uppercase().replace("-", "_"),
            relative_path
        ));
    }
    
    constants
}
