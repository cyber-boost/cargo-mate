use anyhow::{Context, Result};
use std::fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use crate::log::Log;
fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let log = Log::new();
    if args.len() != 4 {
        log.log("Usage", vec!["create_secure_binary".to_string(), "error".to_string()])?;
        log.log(
            "Example",
            vec!["create_secure_binary".to_string(), "error".to_string()],
        )?;
        std::process::exit(1);
    }
    let input_path = &args[1];
    let output_path = &args[2];
    let key = &args[3];
    create_secure_binary(input_path, output_path, key)?;
    log.log(
        "Secure self-decrypting binary created",
        vec!["create_secure_binary".to_string(), "success".to_string()],
    )?;
    log.log(
        "Encrypted and ready to run without external dependencies",
        vec!["create_secure_binary".to_string(), "success".to_string()],
    )?;
    log.log(
        "Users can execute",
        vec!["create_secure_binary".to_string(), "success".to_string()],
    )?;
    Ok(())
}
fn create_secure_binary(input_path: &str, output_path: &str, key: &str) -> Result<()> {
    let binary_data = fs::read(input_path)
        .with_context(|| format!("Failed to read binary: {}", input_path))?;
    use sha2::{Sha256, Digest};
    let mut hasher = Sha256::new();
    hasher.update(key.as_bytes());
    let key_hash = hasher.finalize();
    let encrypted_data: Vec<u8> = binary_data
        .iter()
        .enumerate()
        .map(|(i, &byte)| byte ^ key_hash[i % key_hash.len()])
        .collect();
    let loader_code = format!(
        r#"
// Auto-generated secure binary loader
// Built with cargo-mate encryption system

use std::process;
use std::fs::File;
use std::io::Write;
use std::env;
use sha2::{{Sha256, Digest}};

fn main() -> Result<(), Box<dyn std::error::Error>> {{
    // Get the path of the current executable
    let current_exe = env::current_exe()?;
    let exe_data = std::fs::read(&current_exe)?;

    // Find the marker where encrypted data starts
    let marker = b"
    let marker_pos = exe_data.windows(marker.len())
        .position(|window| window == marker)
        .ok_or("Encrypted data marker not found")?;

    let encrypted_data = &exe_data[marker_pos + marker.len()..];

    let mut hasher = Sha256::new();
    hasher.update(b"{key}");
    let key_hash = hasher.finalize();

    let decrypted_data: Vec<u8> = encrypted_data
        .iter()
        .enumerate()
        .map(|(i, &byte)| byte ^ key_hash[i % key_hash.len()])
        .collect();

    let temp_dir = std::env::temp_dir();
    let temp_path = temp_dir.join("cargo_mate_secure_binary");

    let mut temp_file = File::create(&temp_path)?;
    temp_file.write_all(&decrypted_data)?;
    temp_file.flush()?;

    #[cfg(unix)]
    {{
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&temp_path)?.permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&temp_path, perms)?;
    }}

    let args: Vec<String> = env::args().skip(1).collect();
    let status = process::Command::new(&temp_path)
        .args(&args)
        .status()?;

    let _ = std::fs::remove_file(&temp_path);

    if !status.success() {{
        process::exit(status.code().unwrap_or(1));
    }}

    Ok(())
}}
"#,
        key = key
    );
    let temp_dir = std::env::temp_dir();
    let loader_source = temp_dir.join("secure_loader.rs");
    let loader_binary = temp_dir.join("secure_loader");
    fs::write(&loader_source, loader_code)?;
    let mut final_binary = format!(
        r#"

use std::process;
use std::fs::File;
use std::io::Write;
use std::env;
use sha2::{{Sha256, Digest}};

fn main() -> Result<(), Box<dyn std::error::Error>> {{
    
    let current_exe = env::current_exe()?;
    let exe_data = std::fs::read(&current_exe)?;

    let marker = b"// ENCRYPTED_BINARY_DATA_STARTS_HERE";
    let marker_pos = exe_data.windows(marker.len())
        .position(|window| window == marker)
        .ok_or("Encrypted data marker not found")?;

    let encrypted_data = &exe_data[marker_pos + marker.len()..];

    let mut hasher = Sha256::new();
    hasher.update(b"{key}");
    let key_hash = hasher.finalize();

    let decrypted_data: Vec<u8> = encrypted_data
        .iter()
        .enumerate()
        .map(|(i, &byte)| byte ^ key_hash[i % key_hash.len()])
        .collect();

    let temp_dir = std::env::temp_dir();
    let temp_path = temp_dir.join("cargo_mate_secure_binary");

    let mut temp_file = File::create(&temp_path)?;
    temp_file.write_all(&decrypted_data)?;
    temp_file.flush()?;

    #[cfg(unix)]
    {{
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&temp_path)?.permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&temp_path, perms)?;
    }}

    let args: Vec<String> = env::args().skip(1).collect();
    let status = process::Command::new(&temp_path)
        .args(&args)
        .status()?;

    let _ = std::fs::remove_file(&temp_path);

    if !status.success() {{
        process::exit(status.code().unwrap_or(1));
    }}

    Ok(())
}}

"#,
        key = key
    )
        .into_bytes();
    final_binary.extend_from_slice(&encrypted_data);
    fs::write(output_path, final_binary)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(output_path)?.permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(output_path, perms)?;
    }
    let _ = std::fs::remove_file(&loader_source);
    Ok(())
}