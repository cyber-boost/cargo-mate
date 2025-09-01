use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=src/sweeping/");
    println!("cargo:rerun-if-changed=src/sweeping/Cargo.toml");
    
    // Check if we're building the sweep crate
    if env::var("CARGO_PKG_NAME").unwrap() == "sweep" {
        // This is the sweep crate being built
        return Ok(());
    }
    
    // Build the sweep crate first
    let sweep_dir = Path::new("src/sweeping");
    if !sweep_dir.exists() {
        eprintln!("Sweep directory not found, skipping binary embedding");
        return Ok(());
    }
    
    println!("Building sweep crate...");
    
    // Change to sweep directory and build
    let current_dir = env::current_dir()?;
    env::set_current_dir(sweep_dir)?;
    
    // Build the sweep crate
    let status = Command::new("cargo")
        .args(["build", "--release", "--lib"])
        .status()?;
    
    if !status.success() {
        return Err("Failed to build sweep crate".into());
    }
    
    // Find the compiled static library
    let target_dir = sweep_dir.join("target/release");
    let lib_files: Vec<_> = fs::read_dir(&target_dir)?
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            entry.path()
                .extension()
                .map_or(false, |ext| ext == "a" || ext == "lib")
        })
        .collect();
    
    if lib_files.is_empty() {
        return Err("No static library found after building sweep crate".into());
    }
    
    let lib_path = lib_files[0].path();
    println!("Found sweep library: {:?}", lib_path);
    
    // Read the binary data
    let binary_data = fs::read(&lib_path)?;
    
    // Encrypt the binary (this would use your encryption crate)
    let encrypted_data = encrypt_binary(&binary_data)?;
    
    // Write encrypted data to a file that can be included
    let encrypted_path = sweep_dir.join("encrypted_sweep.bin");
    fs::write(&encrypted_path, encrypted_data)?;
    
    // Set environment variable for the embedder
    println!("cargo:rustc-env=SWEEP_BINARY_PATH={}", encrypted_path.display());
    
    // Return to original directory
    env::set_current_dir(current_dir)?;
    
    Ok(())
}

/// Simple encryption function (in production, use your proper encryption)
fn encrypt_binary(data: &[u8]) -> Result<String, Box<dyn std::error::Error>> {
    use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
    
    // For now, just base64 encode. In production, use proper encryption
    Ok(BASE64.encode(data))
}
